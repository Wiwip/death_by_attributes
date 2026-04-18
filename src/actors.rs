use crate::ability::{AbilityOf, GrantAbilityCommand};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attribute::clamps::Clamp;
use crate::effect::{ApplyEffectEvent, EffectTargeting};
use crate::graph::NodeType;
use crate::modifier::AttributeCalculatorCached;
use crate::mutator::EntityActions;
use crate::prelude::*;
use crate::GrantedAbilities;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use express_it::expr::{Expr, ExprNode};
use num_traits::{AsPrimitive, Num};
use std::collections::HashSet;
use crate::context::EffectExprSchema;

#[derive(Component, Clone, Debug, Deref)]
#[require(GrantedAbilities)]
pub struct Actor(pub Handle<ActorDef>);

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
    actor: ActorDef,
}

impl ActorBuilder {
    pub fn new() -> ActorBuilder {
        Self {
            actor: ActorDef {
                name: "Actor".to_string(),
                description: "".to_string(),
                builder_actions: Default::default(),
                abilities: vec![],
                effects: vec![],
                clamp_exprs: Default::default(),
                clamp_reverse_lookup: Default::default(),
            },
        }
    }

    pub fn name(mut self, name: &str) -> ActorBuilder {
        self.actor.name = name.to_string();
        self
    }

    pub fn with<T: Attribute>(
        mut self,
        value: impl Num + AsPrimitive<T::Property> + Copy + Send + Sync + 'static,
    ) -> ActorBuilder {
        self.actor.builder_actions.push_front(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert((T::new(value), AttributeCalculatorCached::<T>::default()));
            },
        ));
        self
    }

    pub fn with_effect(mut self, effect: &Handle<EffectDef>) -> ActorBuilder {
        self.actor.effects.push(effect.clone());
        self
    }

    pub fn clamp<T>(
        mut self,
        min_expr: impl Into<Expr<T::Property, EffectExprSchema>> + Send + Sync + 'static,
        max_expr: impl Into<Expr<T::Property, EffectExprSchema>> + Send + Sync + 'static,
    ) -> ActorBuilder
    where
        T: Attribute,
    {
        let min_expr = min_expr.into();
        let max_expr = max_expr.into();

        // Insert dependencies for reverse lookup
        let mut deps = HashSet::default();
        min_expr.inner.get_dependencies(&mut deps);
        max_expr.inner.get_dependencies(&mut deps);
        println!("deps: {:?}", deps);
        for dep in deps {
            self.actor
                .clamp_reverse_lookup
                .entry(dep.id)
                .or_default()
                .push(dep.id);
        }

        // Insert expressions
        self.actor
            .clamp_exprs
            .insert(T::ID, Box::new((min_expr, max_expr)));

        self.actor.builder_actions.push_back(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(Clamp::<T>::new());
            },
        ));

        self
    }

    pub fn insert<T: Bundle + Clone + 'static>(mut self, bundle: T) -> ActorBuilder {
        self.actor.builder_actions.push_front(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(bundle.clone());
            },
        ));
        self
    }

    pub fn grant_ability(mut self, ability_spec: &Handle<AbilityDef>) -> Self {
        self.actor.abilities.push(ability_spec.clone());
        self
    }

    pub fn build(self) -> ActorDef {
        self.actor
    }
}
