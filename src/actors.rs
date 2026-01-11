use crate::ability::{AbilityOf, GrantAbilityCommand};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attribute_clamp::{
    Clamp, convert_bounds, observe_current_value_change_for_clamp_bounds,
};
use crate::effect::{ApplyEffectEvent, EffectTargeting};
use crate::graph::NodeType;
use crate::inspector::pretty_type_name;
use crate::modifier::AttributeCalculatorCached;
use crate::mutator::EntityActions;
use crate::prelude::*;
use crate::{CurrentValueChanged, GrantedAbilities};
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use num_traits::{AsPrimitive, Num, Zero};
use std::collections::VecDeque;
use std::ops::RangeBounds;

#[allow(dead_code)]
#[derive(Component, Clone, Debug)]
#[require(GrantedAbilities)]
pub struct Actor(Handle<ActorDef>);

pub struct SpawnActorCommand {
    pub handle: Handle<ActorDef>,
}

impl EntityCommand for SpawnActorCommand {
    fn apply(self, mut entity: EntityWorldMut) -> () {
        debug!("Spawning actor {} {:?}", entity.id(), self.handle);
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
                    for actions in &actor_def.builder_actions {
                        let mut entity_commands = commands.entity(actor_entity);
                        (actions.func)(&mut entity_commands);
                    }

                    // Spawn the granted ability entities
                    for ability in actor_def.abilities.iter() {
                        commands
                            .spawn(AbilityOf(actor_entity))
                            .queue(GrantAbilityCommand {
                                parent: actor_entity,
                                handle: ability.clone(),
                            });
                    }

                    queue
                };

                // Sends the event that will apply the effects to the entity
                for effect in actor_def.effects.iter() {
                    world.trigger(ApplyEffectEvent {
                        entity: actor_entity,
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
    builder_actions: VecDeque<EntityActions>,
    abilities: Vec<Handle<AbilityDef>>,
    effects: Vec<Handle<EffectDef>>,
}

impl ActorBuilder {
    pub fn new() -> ActorBuilder {
        Self {
            name: "Actor".to_string(),
            builder_actions: VecDeque::new(),
            abilities: vec![],
            effects: vec![],
        }
    }

    pub fn name(mut self, name: &str) -> ActorBuilder {
        self.name = name.to_string();
        self
    }

    pub fn with<T: Attribute>(
        mut self,
        value: impl Num + AsPrimitive<T::Property> + Copy + Send + Sync + 'static,
    ) -> ActorBuilder {
        self.builder_actions.push_front(EntityActions::new(
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

    pub fn clamp<T>(
        mut self,
        limits: impl RangeBounds<f64> + Clone + Send + Sync + 'static,
    ) -> ActorBuilder
    where
        T: Attribute,
        f64: AsPrimitive<T::Property>,
    {
        self.builder_actions.push_back(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                let limits = convert_bounds::<f64, T>(limits.clone());

                entity_commands.insert(Clamp::<T> {
                    expression: T::src(),
                    limits,
                    bounds: limits,
                });
            },
        ));

        self
    }

    pub fn clamp_by<T>(self, limits: impl RangeBounds<f64> + Send + Sync + 'static) -> ActorBuilder
    where
        T: Attribute,
        //f64: AsPrimitive<T::Property>,
    {
        let bounds = (limits.start_bound().cloned(), limits.end_bound().cloned());

        /*self.builder_actions.push_back(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                let parent_actor = entity_commands.id();

                let mut observer =
                    Observer::new(observe_current_value_change_for_clamp_bounds::<S, T>);
                observer.watch_entity(parent_actor);

                entity_commands.insert(Clamp::<T>::new(T::src(), bounds));

                entity_commands.commands().spawn((
                    observer,
                    Name::new(format!(
                        "Clamp<{}, {}> Observer",
                        pretty_type_name::<S>(),
                        pretty_type_name::<T>(),
                    )),
                ));

                entity_commands
                    .commands()
                    .trigger(CurrentValueChanged::<S> {
                        phantom_data: Default::default(),
                        old: S::Property::zero(),
                        new: S::Property::zero(),
                        entity: parent_actor,
                    })
            },
        ));*/

        self
    }

    pub fn insert<T: Bundle + Clone + 'static>(mut self, bundle: T) -> ActorBuilder {
        self.builder_actions.push_front(EntityActions::new(
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
            builder_actions: self.builder_actions,
            abilities: self.abilities,
            effects: self.effects,
        }
    }
}
