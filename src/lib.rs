use crate::effects::{GameEffectContainer, GameEffectEvent};
use crate::systems::{
    handle_apply_effect_events, tick_ability_cooldowns,
    tick_active_effects, update_attribute_base_value, update_attribute_current_value,
};
use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use std::any::TypeId;

pub mod abilities;
pub mod attributes;
pub mod context;
pub mod evaluators;
pub mod effects;
pub mod modifiers;
pub mod systems;

use crate::attributes::AttributeDef;
pub use attributes_macro::Attribute;
use bevy::ecs::world::EntityMutExcept;

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

type AttributeEntityMut<'w> = EntityMutExcept<'w, (GameEffectContainer)>;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaseValueUpdate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrentValueUpdate;

#[derive(Event)]
pub struct BaseValueUpdateTrigger;

#[derive(Event)]
pub struct CurrentValueUpdateTrigger;

/// Why Bevy failed to evaluate an animation.
#[derive(Clone, Debug)]
pub enum AttributeEvaluationError {
    /// The component to be animated isn't present on the animation target.
    ///
    /// To fix this error, make sure the entity to be animated contains all
    /// components that have animation curves.
    ComponentNotPresent(TypeId),

    /// The component to be animated was present, but the property on the
    /// component wasn't present.
    AttributeNotPresent(TypeId),
}

trait Editable: Reflect + Sized + Send + Sync + 'static {}
impl Editable for AttributeDef {}
