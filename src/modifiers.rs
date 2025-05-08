use std::fmt::Display;
use std::any::{type_name, TypeId};
use std::fmt::Debug;
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use crate::attributes::AttributeComponent;
use crate::Dirty;

#[derive(Component, Copy, Clone, Debug)]
pub struct Modifier<T> {
    _phantom: PhantomData<T>,
    pub value: ModAggregator<T>,
}

impl<T: 'static> Modifier<T> {
    pub fn new(value: f32, mod_type: ModType) -> Self {
        Self {
            _phantom: Default::default(),
            value: ModAggregator::new(value, mod_type),
        }
    }

    pub fn target(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T> Default for Modifier<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
            value: ModAggregator::default(),
        }
    }
}

/// Spawns a modifier entity attached to an effect
pub struct ModifierCommand<C> {
    pub(crate) effect_entity: Entity,
    pub(crate) actor_entity: Entity,
    pub(crate) modifier: Modifier<C>,
    pub(crate) observer: Option<Observer>,
}

impl<C> Command for ModifierCommand<C>
where
    C: AttributeComponent + Component<Mutability = Mutable>,
{
    fn apply(self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.effect_entity);
        assert_ne!(Entity::PLACEHOLDER, self.actor_entity);
        // We attach an observer to the mutator targeting the parent entity
        let entity = world.spawn_empty().id();
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert((
            Name::new(format!("{}", type_name::<C>())),
            self.modifier,
            ModAggregator::<C>::default(),
            EffectOf(self.effect_entity),
            ModifierOf(self.effect_entity),
            Dirty::<C>::default(),
        ));
        if let Some(observer) = self.observer {
            entity_mut.insert(observer);
        }
    }
}


/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Modifiers)]
pub struct ModifierOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierOf, linked_spawn)]
pub struct Modifiers(Vec<Entity>);

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Effects)]
pub struct EffectOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EffectOf, linked_spawn)]
pub struct Effects(Vec<Entity>);

#[derive(Default, Debug, Clone, Copy, Reflect)]
pub enum ModType {
    #[default]
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Component, Copy, Reflect)]
pub struct ModAggregator<T> {
    phantom_data: PhantomData<T>,
    pub additive: f32,
    pub multi: f32,
    pub overrule: Option<f32>,
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
    pub(crate) fn new(magnitude: f32, mod_type: ModType) -> ModAggregator<T> {
        match mod_type {
            ModType::Additive => ModAggregator::<T>::additive(magnitude),
            ModType::Multiplicative => ModAggregator::<T>::multiplicative(magnitude),
            ModType::Overrule => ModAggregator::<T>::overrule(magnitude),
        }
    }

    pub fn evaluate(&self, value: f32) -> f32 {
        match self.overrule {
            None => (value + self.additive) * (1.0 + self.multi),
            Some(value) => value,
        }
    }

    pub fn additive(value: f32) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: value,
            multi: 0.0,
            overrule: None,
        }
    }
    pub fn multiplicative(value: f32) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: 0.0,
            multi: value,
            overrule: None,
        }
    }
    pub fn overrule(value: f32) -> Self {
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
            multi: self.additive + rhs.additive,
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
            multi: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
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
