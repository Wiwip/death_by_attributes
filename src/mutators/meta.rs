use crate::{OnBaseValueChanged};
use crate::attributes::AttributeComponent;
use crate::evaluators::MutatorEvaluator;
use crate::evaluators::meta::MetaEvaluator;
use crate::mutators::mutator::{ModType, MutatorDef, MutatorHelper};
use crate::mutators::{Mutating, Mutator, ObserveActor};
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::log::{debug};
use bevy::prelude::{Command, Commands, Component, Entity, Name, World};
use std::marker::PhantomData;

pub struct AttributeBuilder<A> {
    actor_entity: Entity,
    queue: CommandQueue,
    _marker: PhantomData<A>,
}

impl<C> AttributeBuilder<C>
where
    C: AttributeComponent + Component<Mutability = Mutable>,
{
    pub fn new(actor_entity: Entity) -> Self {
        Self {
            actor_entity,
            queue: Default::default(),
            _marker: Default::default(),
        }
    }

    pub fn mutate_by_attribute<M>(mut self, scaling: f32, mod_type: ModType) -> Self
    where
        M: AttributeComponent + Component<Mutability = Mutable>,
    {
        self.queue.push(MetaAttributeCommand {
            actor_entity: self.actor_entity,
            mutator: MutatorHelper::new::<C>(MetaEvaluator::<M>::new(scaling, mod_type)),
        });
        self
    }

    pub fn build(mut self, commands: &mut Commands) {
        commands.entity(self.actor_entity).insert(C::new(0.0));
        commands.append(&mut self.queue);
    }
}

/// Spawns a mutator entity on a specified effect when applied
///
pub struct MetaAttributeCommand<A, E> {
    pub(crate) actor_entity: Entity,
    pub(crate) mutator: MutatorDef<A, E>,
}

impl<C, E> Command for MetaAttributeCommand<C, E>
where
    C: AttributeComponent + Component<Mutability = Mutable>,
    E: MutatorEvaluator + Clone,
{
    fn apply(self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.actor_entity);
        
        // We attach an observer to the mutator targeting the parent entity
        let mutator_entity = world.spawn_empty().id();
        debug!("Spawned mutator entity {:?}", mutator_entity);

        self.mutator.register_observer::<OnBaseValueChanged>(
            world,
            mutator_entity,
            self.actor_entity,
        );

        let mut entity_mut = world.entity_mut(mutator_entity);
        entity_mut.insert((
            Name::new("Mutator"),
            Mutating(self.actor_entity),
            Mutator::new(self.mutator),
        ));
    }
}
