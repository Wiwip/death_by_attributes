use crate::actors::SpawnActorCommand;
use crate::assets::{ActorDef, EffectDef};
use crate::effect::{ApplyEffectEvent, EffectTargeting};
use crate::effect::global_effect::{GlobalActor, GlobalEffects};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct EffectContext<'w, 's> {
    commands: Commands<'w, 's>,
    global_actor: Query<'w, 's, Entity, With<GlobalActor>>,
    global_effects: ResMut<'w, GlobalEffects>,
    effects: ResMut<'w, Assets<EffectDef>>,
    actors: ResMut<'w, Assets<ActorDef>>,
}

impl<'s, 'w> EffectContext<'w, 's> {
    pub fn add_effect(&mut self, effect: EffectDef) -> Handle<EffectDef> {
        self.effects.add(effect)
    }

    pub fn apply_effect_to_target(
        &mut self,
        target: Entity,
        source: Entity,
        handle: &Handle<EffectDef>,
    ) {
        self.commands.trigger(ApplyEffectEvent {
            entity: target,
            targeting: EffectTargeting::new(source, target),
            handle: handle.clone(),
        });
    }

    pub fn apply_effect_to_self(&mut self, source: Entity, handle: &Handle<EffectDef>) {
        self.apply_effect_to_target(source, source, handle);
    }

    pub fn apply_dynamic_effect_to_target(
        &mut self,
        target: Entity,
        source: Entity,
        effect: EffectDef,
    ) -> Handle<EffectDef> {
        let handle = self.effects.add(effect);

        self.commands.trigger(ApplyEffectEvent {
            entity: target,
            targeting: EffectTargeting::new(source, target),
            handle: handle.clone(),
        });
        handle
    }

    pub fn apply_dynamic_effect_to_self(
        &mut self,
        source: Entity,
        effect: EffectDef,
    ) -> Handle<EffectDef> {
        self.apply_dynamic_effect_to_target(source, source, effect)
    }

    pub fn add_actor(&mut self, actor: ActorDef) -> Handle<ActorDef> {
        self.actors.add(actor)
    }

    pub fn spawn_actor(&mut self, handle: &Handle<ActorDef>) -> EntityCommands<'_> {
        let mut entity_commands = self.commands.spawn_empty();
        entity_commands.queue(SpawnActorCommand {
            handle: handle.clone(),
        });
        entity_commands
    }

    pub fn add_spawn_actor(&mut self, actor: ActorDef) -> EntityCommands<'_> {
        let handle = self.actors.add(actor);
        self.spawn_actor(&handle)
    }

    pub fn insert_actor(&mut self, entity: Entity, handle: &Handle<ActorDef>) {
        self.commands.entity(entity).queue(SpawnActorCommand {
            handle: handle.clone(),
        });
    }

    pub fn add_global_effect(&mut self, handle: Handle<EffectDef>) {
        self.global_effects.push(handle);
    }

    /// Gets or create the global effect actor.
    /// Global effects are attached to this actor and applied to all existing actors.
    /// This actor can serve as a game state tracker, and the effects can depend on its attributes.
    pub fn get_global_actor(&mut self) -> Entity {
        self.global_actor.single().unwrap()
    }

    pub fn spawn_global_effects(&mut self, target_actor: Entity) {
        let global_actor = self.get_global_actor();
        let effects: Vec<_> = self.global_effects.clone();

        for handle in effects.iter() {
            self.apply_effect_to_target(target_actor, global_actor, &handle);
        }
    }
}
