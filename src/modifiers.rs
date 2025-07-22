use crate::attributes::Attribute;
use crate::effects::{EffectTarget, OnEffectStatusChangeEvent};
use crate::inspector::pretty_type_name;
use crate::{ActorEntityMut, ActorEntityRef, Dirty, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::any::{type_name, TypeId};
use std::fmt::Debug;
use std::fmt::Display;
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Mul};

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
pub struct ModifierMarker;

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Modifiers)]
pub struct ModifierOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierOf, linked_spawn)]
pub struct Modifiers(Vec<Entity>);

pub trait Mutator: Send + Sync {
    fn spawn(&self, commands: &mut Commands, actor_entity: ActorEntityRef) -> Entity;
    fn apply(&self, actor_entity: &mut ActorEntityMut) -> bool;
    fn origin(&self) -> ModTarget;
}

#[derive(Default, Copy, Clone, Debug, Reflect)]
pub enum ModTarget {
    #[default]
    Target,
    Source,
}

#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    pub application: ModTarget,
    pub aggregator: ModAggregator<T>,
}

impl<T: 'static> AttributeModifier<T>
where
    T: Attribute,
{
    pub fn new(value: f64, mod_type: ModType, mod_application: ModTarget) -> Self {
        Self {
            application: mod_application,
            aggregator: ModAggregator::new(value, mod_type),
        }
    }

    pub fn target(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T> Default for AttributeModifier<T>
where
    T: Attribute,
{
    fn default() -> Self {
        Self {
            application: ModTarget::Source,
            aggregator: ModAggregator::default(),
        }
    }
}
impl<T> Display for AttributeModifier<T>
where
    T: Attribute,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mod<{}>({:.1})", type_name::<T>(), self.aggregator)
    }
}

impl<T> Mutator for AttributeModifier<T>
where
    T: Attribute,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: ActorEntityRef) -> Entity {
        debug!(
            "Added Mod<{}> [{}] to {}",
            type_name::<T>(),
            self.aggregator.additive,
            actor_entity.id(),
        );

        let mut observer = Observer::new(
            |trigger: Trigger<OnEffectStatusChangeEvent>,
             query: Query<&EffectTarget>,
             mut commands: Commands| {
                println!(
                    "Observer[{}] -> Target[{}] change for {}",
                    trigger.observer(),
                    trigger.target(),
                    type_name::<T>()
                );
                let parent = query.get(trigger.observer()).unwrap();

                // Marks dirty the actor, the effect, and the modifier.
                commands
                    .entity(trigger.target())
                    .insert(Dirty::<T>::default());
                commands.entity(parent.0).insert(Dirty::<T>::default());
                commands
                    .entity(trigger.observer())
                    .insert(Dirty::<T>::default());
            },
        );
        observer.watch_entity(actor_entity.id());

        commands
            .spawn((
                AttributeModifier::<T> {
                    application: self.application,
                    aggregator: self.aggregator.clone(),
                },
                ModAggregator::<T>::default(),
                observer,
                Name::new(format!("Mod<{}>", type_name::<T>())),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut ActorEntityMut) -> bool {
        if let Some(mut attribute) = actor_entity.get_mut::<T>() {
            let new_val = self.aggregator.evaluate(attribute.base_value());

            // Ensure that the modifier meaningfully changed the value before we trigger the event.
            if (new_val - &attribute.base_value()).abs() > f64::EPSILON {
                attribute.set_current_value(new_val);
                true
            } else {
                false
            }
        } else {
            panic!("Could not find attribute {}", type_name::<T>());
        }
    }

    fn origin(&self) -> ModTarget {
        self.application
    }
}

#[derive(Copy, Clone, Debug, Reflect)]
pub struct ModifierRef<T, S> {
    #[reflect(ignore)]
    _target: PhantomData<T>,
    #[reflect(ignore)]
    _source: PhantomData<S>,
    pub mod_target: ModTarget,
    pub scaling_factor: f64,
    pub mod_type: ModType,
}

impl<T, S> ModifierRef<T, S> {
    pub fn new(value: f64, mod_type: ModType, mod_target: ModTarget) -> Self {
        Self {
            _target: Default::default(),
            _source: Default::default(),
            mod_target,
            mod_type,
            scaling_factor: value,
        }
    }
}

impl<T, S> Mutator for ModifierRef<T, S>
where
    T: Attribute + Component<Mutability = Mutable>,
    S: Attribute + Component<Mutability = Mutable>,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: ActorEntityRef) -> Entity {
        debug!(
            "Added modifier<{}> [{}] to {}",
            type_name::<T>(),
            type_name::<S>(),
            actor_entity.id()
        );
        let factor = self.scaling_factor;

        let mut observer = Observer::new(
            // When the source attribute changes, update the modifier of the target attribute.
            move |trigger: Trigger<OnAttributeValueChanged<S>>,
                  attributes: Query<&S>,
                  mut modifiers: Query<&mut AttributeModifier<T>>| {
                let Ok(attribute) = attributes.get(trigger.target()) else {
                    return;
                };
                let Ok(mut modifier) = modifiers.get_mut(trigger.observer()) else {
                    return;
                };

                modifier.aggregator.additive = factor * attribute.current_value(); // modify by scaling factor
            },
        );
        observer.watch_entity(actor_entity.id());

        let Some(attribute_value) = actor_entity.get::<S>() else {
            panic!(
                "Could not find attribute {} on {}",
                type_name::<S>(),
                actor_entity.id(),
            );
        };
        let value = attribute_value.current_value() * self.scaling_factor;

        commands
            .spawn((
                Name::new(format!("{}", type_name::<T>())),
                observer,
                AttributeModifier::<T>::new(value, ModType::Additive, self.mod_target),
                ModAggregator::<T>::default(),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut ActorEntityMut) -> bool {
        let Some(origin_value) = actor_entity.get::<S>() else {
            panic!("Should have found source attribute");
        };
        let value = origin_value.current_value() * self.scaling_factor;

        AttributeModifier::<T>::new(value, ModType::Additive, self.mod_target).apply(actor_entity)
    }

    fn origin(&self) -> ModTarget {
        self.mod_target
    }
}

pub type ModifierFn = dyn Fn(&mut EntityCommands, Entity) + Send + Sync;

#[derive(Default, Debug, Clone, Copy, Reflect)]
pub enum ModType {
    #[default]
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Component, Copy, Reflect)]
pub struct ModAggregator<T> {
    #[reflect(ignore)]
    phantom_data: PhantomData<T>,
    pub additive: f64,
    pub multi: f64,
    pub overrule: Option<f64>,
}

impl<T> Default for ModAggregator<T> {
    fn default() -> Self {
        Self {
            phantom_data: Default::default(),
            additive: 0.0,
            multi: 0.0,
            overrule: None,
        }
    }
}

impl<T> ModAggregator<T> {
    pub(crate) fn new(magnitude: f64, mod_type: ModType) -> ModAggregator<T> {
        match mod_type {
            ModType::Additive => ModAggregator::<T>::additive(magnitude),
            ModType::Multiplicative => ModAggregator::<T>::multiplicative(magnitude),
            ModType::Overrule => ModAggregator::<T>::overrule(magnitude),
        }
    }

    pub fn evaluate(&self, value: f64) -> f64 {
        match self.overrule {
            None => (value + self.additive) * (1.0 + self.multi),
            Some(value) => value,
        }
    }

    pub fn additive(value: f64) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: value,
            multi: 0.0,
            overrule: None,
        }
    }
    pub fn multiplicative(value: f64) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: 0.0,
            multi: value,
            overrule: None,
        }
    }
    pub fn overrule(value: f64) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: 0.0,
            multi: 0.0,
            overrule: Some(value),
        }
    }
}

impl<T> Add for &ModAggregator<T> {
    type Output = ModAggregator<T>;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Add<ModAggregator<T>> for &mut ModAggregator<T> {
    type Output = ModAggregator<T>;

    fn add(self, rhs: ModAggregator<T>) -> Self::Output {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Mul<ModAggregator<T>> for ModAggregator<T> {
    type Output = Self;

    fn mul(self, rhs: ModAggregator<T>) -> Self::Output {
        Self {
            phantom_data: Default::default(),
            additive: self.additive * rhs.additive,
            multi: self.multi * rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Mul<f64> for ModAggregator<T> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            phantom_data: Default::default(),
            additive: self.additive * rhs,
            multi: self.multi * rhs,
            overrule: self.overrule,
        }
    }
}

impl<T> AddAssign for ModAggregator<T> {
    fn add_assign(&mut self, rhs: ModAggregator<T>) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl<T> AddAssign for &mut ModAggregator<T> {
    fn add_assign(&mut self, rhs: &mut ModAggregator<T>) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl<T> Add for ModAggregator<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Clone for ModAggregator<T> {
    fn clone(&self) -> Self {
        ModAggregator::<T> {
            phantom_data: Default::default(),
            additive: self.additive,
            multi: self.multi,
            overrule: self.overrule,
        }
    }
}

impl<T> Sum for ModAggregator<T> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                phantom_data: PhantomData,
                additive: 0.0,
                multi: 0.0,
                overrule: None,
            },
            |a, b| Self {
                phantom_data: PhantomData,
                additive: a.additive + b.additive,
                multi: a.multi + b.multi,
                overrule: a.overrule.or(b.overrule),
            },
        )
    }
}

impl<T> Display for ModAggregator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "+{:.1} x{:.1} (or {:?})",
            self.additive, self.multi, self.overrule
        )
    }
}

impl<T> Debug for ModAggregator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModAggregator")
            .field("additive", &self.additive)
            .field("multiplicative", &self.multi)
            .field("overrule", &self.overrule)
            .finish()
    }
}

/// Aggregates all modifiers for a given entity, returning a combined ModAggregator
pub fn aggregate_entity_modifiers<T: Component<Mutability = Mutable> + Attribute>(
    entity: Entity,
    modifiers_query: &Query<&Modifiers>,
    attribute_modifier_query: &mut Query<&AttributeModifier<T>>,
) -> ModAggregator<T> {
    // Try to get modifiers for the entity
    modifiers_query
        .get(entity)
        .map(|effect_modifiers| {
            // Map through modifiers, extract aggregators, and combine them
            effect_modifiers
                .iter()
                .filter_map(|modifier_entity| {
                    attribute_modifier_query
                        .get(modifier_entity)
                        .ok()
                        .map(|modifier| &modifier.aggregator)
                })
                .fold(ModAggregator::<T>::default(), |acc, agg| &acc + &agg)
        })
        .unwrap_or_default() // Return default ModAggregator if the entity has no modifiers
}

#[reflect_trait] // Generates a `ReflectMyTrait` type
pub trait AccessModifier {
    fn describe(&self) -> String;
    fn name(&self) -> String;
}

impl<T> AccessModifier for AttributeModifier<T>
where
    T: Attribute,
{
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }

    fn describe(&self) -> String {
        format!("{}", self.aggregator)
    }
}
