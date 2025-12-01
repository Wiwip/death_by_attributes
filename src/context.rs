use crate::actors::SpawnActorCommand;
use crate::assets::{ActorDef, EffectDef};
use crate::effect::EffectTargeting;
use crate::prelude::ApplyEffectEvent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct EffectContext<'w, 's> {
    commands: Commands<'w, 's>,
    effects: ResMut<'w, Assets<EffectDef>>,
}

impl<'s, 'w> EffectContext<'w, 's> {
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

    pub fn spawn_actor(&mut self, handle: &Handle<ActorDef>) -> EntityCommands<'_> {
        let mut entity_commands = self.commands.spawn_empty();
        entity_commands.queue(SpawnActorCommand {
            handle: handle.clone(),
        });
        entity_commands
    }

    pub fn insert_actor(&mut self, entity: Entity, handle: &Handle<ActorDef>) {
        self.commands.entity(entity).queue(SpawnActorCommand {
            handle: handle.clone(),
        });
    }
}
