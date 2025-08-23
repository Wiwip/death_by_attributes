use fixed::traits::Fixed;
use crate::OnAttributeValueChanged;
use crate::ability::{AbilityOf, GrantAbilityCommand};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attributes::{Attribute, Clamp, DerivedClamp, derived_clamp_attributes_observer};
use crate::condition::convert_bounds;
use crate::effect::EffectTargeting;
use crate::graph::NodeType;
use crate::mutator::EntityActions;
use crate::prelude::{ApplyEffectEvent, AttributeCalculatorCached};
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use fixed::prelude::{LossyInto, ToFixed};
use std::ops::RangeBounds;

#[derive(Component, Clone, Debug)]
pub struct Actor(Handle<ActorDef>);

pub struct SpawnActorCommand {
    pub handle: Handle<ActorDef>,
}

impl EntityCommand for SpawnActorCommand {
    fn apply(self, mut entity: EntityWorldMut) -> () {
        debug!("Spawning actor {:?}", self.handle);
        let actor_entity = entity.id();

        entity.world_scope(|world| {
            world.resource_scope(|world, actor_assets: Mut<Assets<ActorDef>>| {
                actor_assets.get(&self.handle).unwrap();
                let actor_def = actor_assets.get(&self.handle).unwrap();

                let mut queue = {
                    let mut queue = CommandQueue::default();
                    let mut commands = Commands::new(&mut queue, world);

                    commands.entity(actor_entity).insert((
                        NodeType::Actor,
                        Actor(self.handle.clone()),
                        Name::new(actor_def.name.clone()),
                    ));

                    // Apply mutators
                    for mutator in &actor_def.mutators {
                        let mut entity_commands = commands.entity(actor_entity);
                        (mutator.func)(&mut entity_commands);
                    }

                    // Spawn the granted ability entities
                    for ability in actor_def.abilities.iter() {
                        commands
                            .spawn(AbilityOf(actor_entity))
                            .queue(GrantAbilityCommand {
                                handle: ability.clone(),
                            });
                    }

                    queue
                };

                // Sends the event that will apply the effects to the entity
                for effect in actor_def.effects.iter() {
                    world.send_event(ApplyEffectEvent {
                        targeting: EffectTargeting::SelfCast(actor_entity),
                        handle: effect.clone(),
                    });
                }

                // Queue the commands for deferred application
                world.commands().append(&mut queue);
            });
        });
    }
}

pub struct ActorBuilder {
    name: String,
    builder_actions: Vec<EntityActions>,
    abilities: Vec<Handle<AbilityDef>>,
    effects: Vec<Handle<EffectDef>>,
}

impl ActorBuilder {
    pub fn new() -> ActorBuilder {
        Self {
            name: "Actor".to_string(),
            builder_actions: vec![],
            abilities: vec![],
            effects: vec![],
        }
    }

    pub fn with_name(mut self, name: &str) -> ActorBuilder {
        self.name = name.to_string();
        self
    }

    pub fn with<T: Attribute>(
        mut self,
        value: impl ToFixed + Copy + Send + Sync + 'static,
    ) -> ActorBuilder {
        self.builder_actions.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert((T::new(value), AttributeCalculatorCached::<T>::default()));
            },
        ));
        self
    }

    pub fn with_effect(mut self, effect: &Handle<EffectDef>) -> ActorBuilder {
        self.effects.push(effect.clone());
        self
    }

    pub fn clamp<T>(mut self, range: impl RangeBounds<f64>) -> ActorBuilder
    where
        T: Attribute,
    {
        let bounds = convert_bounds::<T, f64>(range);
        let clamp = Clamp::<T>::new(bounds);

        self.builder_actions.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(clamp.clone());
            },
        ));
        self
    }

    pub fn clamp_from<S, T>(
        mut self,
        limits: impl RangeBounds<f64> + Send + Sync + 'static,
    ) -> ActorBuilder
    where
        S: Attribute,
        T: Attribute,
        S::Property: LossyInto<T::Property>,
    {
        let bounds = (limits.start_bound().cloned(), limits.end_bound().cloned());

        self.builder_actions.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                let mut observer = Observer::new(derived_clamp_attributes_observer::<S, T>);
                observer.watch_entity(entity_commands.id());

                entity_commands.insert((
                    DerivedClamp::<T>::new(bounds),
                    children![observer],
                ));
            },
        ));

        self
    }

    pub fn with_component<T: Bundle + Clone + 'static>(mut self, bundle: T) -> ActorBuilder {
        self.builder_actions.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(bundle.clone());
            },
        ));
        self
    }

    pub fn grant_ability(mut self, ability_spec: &Handle<AbilityDef>) -> Self {
        self.abilities.push(ability_spec.clone());
        self
    }

    pub fn build(self) -> ActorDef {
        ActorDef {
            name: self.name,
            description: "".to_string(),
            mutators: self.builder_actions,
            abilities: self.abilities,
            effects: self.effects,
        }
    }
}
