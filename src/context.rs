use crate::assets::GameEffect;
use crate::effects::ApplyEffectEvent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct EffectContext<'w, 's> {
    pub effects: Res<'w, Assets<GameEffect>>,
    pub commands: Commands<'w, 's>,
}

impl<'s, 'w> EffectContext<'w, 's> {
    pub fn apply_effect_to_target(
        &mut self,
        target: Entity,
        source: Entity,
        handle: Handle<GameEffect>,
    ) {
        self.commands.trigger(ApplyEffectEvent {
            target,
            source,
            handle,
        });
    }

    pub fn apply_effect_to_self(&mut self, source: Entity, definition: Handle<GameEffect>) {
        self.apply_effect_to_target(source, source, definition);
    }
}
