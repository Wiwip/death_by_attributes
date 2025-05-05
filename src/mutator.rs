use crate::attributes::AttributeComponent;
use crate::evaluators::MutatorEvaluator;
use crate::{ActorEntityMut, AttributeEvaluationError};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::any::{TypeId, type_name};
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, DerefMut, Neg, SubAssign};

pub trait EvaluateMutator: Debug + Send + Sync + 'static {
    fn clone_value(&self) -> Box<dyn EvaluateMutator>;
    fn apply_mutator(&self, entity_mut: ActorEntityMut);
    fn apply_aggregator(&self, entity_mut: ActorEntityMut, aggregator: ModAggregator);
    fn update_current_value(&self, entity_mut: ActorEntityMut, aggregator: ModAggregator) -> bool;

    fn target(&self) -> TypeId;

    fn to_aggregator(&self) -> ModAggregator;

    fn get_current_value(
        &self,
        entity_mut: ActorEntityMut,
    ) -> Result<f32, AttributeEvaluationError>;
    fn get_base_value(&self, entity_mut: ActorEntityMut) -> Result<f32, AttributeEvaluationError>;

    fn get_magnitude(&self) -> f32;
    fn set_magnitude(&mut self, magnitude: f32);
    fn register_observer<'a>(&'a self, world: &'a mut World, owner: Entity, target: Entity);
}

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = EffectMutators)]
pub struct Mutating(Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = Mutating, linked_spawn)]
pub struct EffectMutators(Vec<Entity>);

pub struct MutatorHelper<A> {
    _phantom: PhantomData<A>,
}

impl<E> MutatorHelper<E> {
    pub fn new<A>(evaluator: E) -> MutatorDef<A, E> {
        MutatorDef {
            attribute: Default::default(),
            evaluator,
        }
    }
}

pub struct MutatorDef<A, E> {
    attribute: PhantomData<A>,
    evaluator: E,
}

impl<A, E> Debug for MutatorDef<A, E>
where
    A: AttributeComponent + Component<Mutability = Mutable>,
    E: MutatorEvaluator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutatorDef")
            .field("attribute", &type_name::<A>())
            .field("evaluator", &self.evaluator)
            .finish()
    }
}

impl<P, C> EvaluateMutator for MutatorDef<P, C>
where
    P: Component<Mutability = Mutable> + AttributeComponent,
    C: MutatorEvaluator + Clone,
{
    fn clone_value(&self) -> Box<dyn EvaluateMutator> {
        Box::new(self.clone())
    }

    fn apply_mutator(&self, mut entity_mut: ActorEntityMut) {
        let mut attribute = entity_mut.get_mut::<P>().unwrap(); // This is what I want to avoid
        let attribute = attribute.deref_mut();
        let aggregator = self.evaluator.get_aggregator();
        let new_value = aggregator.evaluate(attribute.get_base_value());
        let base_value = attribute.get_base_value_mut();
        *base_value = new_value;
    }

    fn apply_aggregator(&self, mut entity_mut: ActorEntityMut, aggregator: ModAggregator) {
        let Some(mut attribute) = entity_mut.get_mut::<P>() else {
            warn_once!("Error getting mutable attribute in apply_aggregator");
            return;
        };
        let attribute = attribute.deref_mut();

        let new_value = aggregator.evaluate(attribute.get_base_value());
        let base_value = attribute.get_base_value_mut();
        *base_value = new_value;
    }

    fn update_current_value(
        &self,
        mut entity_mut: ActorEntityMut,
        aggregator: ModAggregator,
    ) -> bool {
        let mut attribute = entity_mut
            .get_mut::<P>()
            .expect("Error getting mutable attribute in update_current_value");
        let attribute = attribute.deref_mut();

        let new_value = aggregator.evaluate(attribute.get_base_value());
        let old_value = attribute.get_current_value_mut();
        *old_value = new_value;

        new_value == *old_value
    }

    fn target(&self) -> TypeId {
        TypeId::of::<P>()
    }

    fn to_aggregator(&self) -> ModAggregator {
        self.evaluator.get_aggregator()
    }

    fn get_current_value(
        &self,
        mut entity_mut: ActorEntityMut,
    ) -> Result<f32, AttributeEvaluationError> {
        let mut attribute = entity_mut.get_mut::<P>().unwrap();
        let attribute = attribute.deref_mut();
        Ok(attribute.get_current_value())
    }

    fn get_base_value(
        &self,
        mut entity_mut: ActorEntityMut,
    ) -> Result<f32, AttributeEvaluationError> {
        let mut attribute = entity_mut.get_mut::<P>().unwrap();
        let attribute = attribute.deref_mut();
        Ok(attribute.get_base_value())
    }

    fn get_magnitude(&self) -> f32 {
        self.evaluator.get_magnitude()
    }

    fn set_magnitude(&mut self, magnitude: f32) {
        self.evaluator.set_magnitude(magnitude)
    }

    fn register_observer<'a>(&'a self, world: &'a mut World, owner: Entity, target: Entity) {
        let Some(mut observer) = self.evaluator.get_observer() else {
            return;
        };
        observer.watch_entity(target);

        let mut entity_mut = world.entity_mut(owner);
        entity_mut.insert(observer);
    }
}

impl<P, C> Clone for MutatorDef<P, C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            attribute: self.attribute.clone(),
            evaluator: self.evaluator.clone(),
        }
    }
}

/// Spawns a mutator entity on a specified effect when applied
///
pub struct MutatorCommand<A, E> {
    pub(crate) effect_entity: Entity,
    pub(crate) actor_entity: Entity,
    pub(crate) mutator: MutatorDef<A, E>,
}

impl<A, E> Command for MutatorCommand<A, E>
where
    A: AttributeComponent + Component<Mutability = Mutable>,
    E: MutatorEvaluator + Clone,
{
    fn apply(self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.effect_entity);
        assert_ne!(Entity::PLACEHOLDER, self.actor_entity);

        // We attach an observer to the mutator targeting the parent entity
        let mutator_entity = world.spawn_empty().id();
        info!("Created mutator entity {}", mutator_entity);
        self.mutator
            .register_observer(world, mutator_entity, self.actor_entity);

        let mut entity_mut = world.entity_mut(mutator_entity);
        entity_mut.insert((
            Name::new("Mutator"),
            Mutating(self.effect_entity),
            Mutator::new(self.mutator),
        ));
    }
}

#[derive(Default, Debug, Clone, Copy, Reflect)]
pub enum ModType {
    #[default]
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ModAggregator {
    pub additive: f32,
    pub multi: f32,
    pub overrule: Option<f32>,
}

impl ModAggregator {
    pub fn evaluate(self, value: f32) -> f32 {
        match self.overrule {
            None => (value + self.additive) * (1.0 + self.multi),
            Some(value) => value,
        }
    }

    pub fn additive(value: f32) -> Self {
        ModAggregator {
            additive: value,
            multi: 0.0,
            overrule: None,
        }
    }
    pub fn multiplicative(value: f32) -> Self {
        ModAggregator {
            additive: 0.0,
            multi: value,
            overrule: None,
        }
    }
    pub fn overrule(value: f32) -> Self {
        ModAggregator {
            additive: 0.0,
            multi: 0.0,
            overrule: Some(value),
        }
    }
}

impl Add for &ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multi: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl Add<ModAggregator> for &mut ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: ModAggregator) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multi: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl AddAssign for ModAggregator {
    fn add_assign(&mut self, rhs: ModAggregator) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl AddAssign for &mut ModAggregator {
    fn add_assign(&mut self, rhs: &mut ModAggregator) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl Add for ModAggregator {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl SubAssign for ModAggregator {
    fn sub_assign(&mut self, rhs: Self) {
        self.additive -= rhs.additive;
        self.multi -= rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl Sum for ModAggregator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                additive: 0.0,
                multi: 0.0,
                overrule: None,
            },
            |a, b| Self {
                additive: a.additive + b.additive,
                multi: a.multi + b.multi,
                overrule: a.overrule.or(b.overrule),
            },
        )
    }
}

impl Neg for ModAggregator {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            additive: -self.additive,
            multi: -self.multi,
            overrule: self.overrule,
        }
    }
}

#[derive(Component, Debug, Deref, DerefMut, TypePath)]
pub struct Mutator(pub Box<dyn EvaluateMutator>);

impl Clone for Mutator {
    fn clone(&self) -> Self {
        Self(EvaluateMutator::clone_value(&*self.0))
    }
}

impl Mutator {
    pub fn new(effect: impl EvaluateMutator) -> Self {
        Self(Box::new(effect))
    }
}
