use crate::actors::SpawnActorCommand;
use crate::assets::{ActorDef, EffectDef};
use crate::effect::EffectTargeting;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::prelude::ApplyEffectEvent;

#[derive(SystemParam)]
pub struct EffectContext<'w, 's> {
    pub commands: Commands<'w, 's>,
}

impl<'s, 'w> EffectContext<'w, 's> {
    pub fn apply_effect_to_target(
        &mut self,
        target: Entity,
        source: Entity,
        handle: &Handle<EffectDef>,
    ) {
        self.commands.trigger_targets(
            ApplyEffectEvent {
                targeting: EffectTargeting::new(source, target),
                handle: handle.clone(),
            },
            target,
        );
    }

    pub fn apply_effect_to_self(&mut self, source: Entity, handle: &Handle<EffectDef>) {
        self.apply_effect_to_target(source, source, handle);
    }

    pub fn spawn_actor(&mut self, handle: &Handle<ActorDef>) -> EntityCommands {
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

    //pub fn grant_ability(&mut self, entity: Entity, ability: Handle<AbilityDef>) {}
    //pub fn remove_ability(&mut self, entity: Entity, ability: Entity) {}
}
