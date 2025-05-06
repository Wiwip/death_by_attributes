pub use crate::mutators::mutator::ModAggregator;
use crate::{ActorEntityMut, AttributeEvaluationError};
use bevy::prelude::{Component, Deref, DerefMut, Entity, Event, World};
use bevy::reflect::{Reflect, TypePath};
use std::any::TypeId;
use std::fmt::Debug;

pub mod meta;
pub mod mutator;

pub trait EvaluateMutator: Debug + std::fmt::Display + Send + Sync + 'static {
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
}

pub trait ObserveActor: Send + Sync + 'static {
    fn register_observer<'a, O: Event>(
        &'a self,
        world: &'a mut World,
        owner: Entity,
        target: Entity,
    );
}

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = EffectMutators)]
pub struct Mutating(Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = Mutating, linked_spawn)]
pub struct EffectMutators(Vec<Entity>);

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

impl std::fmt::Display for Mutator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
