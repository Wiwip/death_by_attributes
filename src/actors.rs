use crate::ability::{AbilityOf, GrantAbilityCommand};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attribute_clamp::Clamp;
use crate::attributes::AttributeId;
use crate::effect::{ApplyEffectEvent, EffectTargeting};
use crate::graph::NodeType;
use crate::modifier::AttributeCalculatorCached;
use crate::mutator::EntityActions;
use crate::prelude::*;
use crate::GrantedAbilities;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use express_it::expr::Expr;
use num_traits::{AsPrimitive, Num};
use std::any::Any;
use std::collections::{HashMap, VecDeque};

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

    // The value is actually Box<(Expr<T>, Expr<T>)>, but hidden behind 'Any'.
    clamp_exprs: HashMap<AttributeId, Box<dyn Any + Send + Sync>>,
}

impl ActorBuilder {
    pub fn new() -> ActorBuilder {
        Self {
            name: "Actor".to_string(),
            builder_actions: VecDeque::new(),
            abilities: vec![],
            effects: vec![],
            clamp_exprs: Default::default(),
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

    /*pub fn clamp<T>(
        self,
        _limits: impl RangeBounds<f64> + Clone + Send + Sync + 'static,
    ) -> ActorBuilder
    where
        T: Attribute,
        f64: AsPrimitive<T::Property>,
    {
        unimplemented!();
        /*self.builder_actions.push_back(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                let limits = convert_bounds::<f64, T>(limits.clone());

                /*entity_commands.insert(Clamp::<T> {
                    expression: T::src(),
                    limits,
                    bounds: limits,
                });*/
            },
        ));

        self
        */
    }*/

    pub fn clamp<T>(
        mut self,
        min_expr: impl Into<Expr<T::Property>> + Send + Sync + 'static,
        max_expr: impl Into<Expr<T::Property>> + Send + Sync + 'static,
    ) -> ActorBuilder
    where
        T: Attribute,
        //f64: AsPrimitive<T::Property>,
    {
        self.clamp_exprs
            .insert(T::ID, Box::new((min_expr, max_expr)));

        self.builder_actions.push_back(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                let parent_actor = entity_commands.id();

                //let mut observer = Observer::new(observe_current_value_change_for_clamp_bounds::<S, T>);
                //observer.watch_entity(parent_actor);

                entity_commands.insert(Clamp::<T>::new());

                /*entity_commands.commands().spawn((
                    observer,
                    Name::new(format!(
                        "Clamp<{}, {}> Observer",
                        pretty_type_name::<S>(),
                        pretty_type_name::<T>(),
                    )),
                ));*/

                /*entity_commands
                .commands()
                .trigger(CurrentValueChanged::<S> {
                    phantom_data: Default::default(),
                    old: S::Property::zero(),
                    new: S::Property::zero(),
                    entity: parent_actor,
                })*/
            },
        ));

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
            clamp_exprs: self.clamp_exprs,
        }
    }
}
