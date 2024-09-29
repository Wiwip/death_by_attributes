use crate::effect::GameEffectEvent;
use crate::systems::{
    handle_apply_effect_events, tick_ability_cooldowns, tick_active_effects,
    update_attribute_base_value, update_attribute_current_value,
};
use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

pub mod abilities;
pub mod attributes;
pub mod context;
pub mod effect;
pub mod events;
pub mod modifiers;
pub mod systems;

pub use attributes_macro::Attribute;

pub struct DeathByAttributesPlugin;

impl Plugin for DeathByAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(BaseValueUpdate)
            .add_event::<GameEffectEvent>()
            .add_systems(
                BaseValueUpdate,
                (
                    handle_apply_effect_events,
                    tick_active_effects,
                    update_attribute_base_value,
                )
                    .chain(),
            )
            .add_systems(CurrentValueUpdate, update_attribute_current_value)
            .add_systems(BaseValueUpdate, tick_ability_cooldowns);

        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(StateTransition, BaseValueUpdate);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(BaseValueUpdate, CurrentValueUpdate);
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaseValueUpdate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrentValueUpdate;
