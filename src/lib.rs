use crate::effects::EvalStruct;
use crate::effects::{Effect, EffectDuration, EffectPeriodicApplication, EffectTarget, GameEffectEvent};
use crate::systems::{
    on_duration_effect_added, on_effect_removed, on_instant_effect_added,
    tick_ability_cooldowns, tick_effects_duration, tick_effects_periodic_timer, update_base_values,
    update_current_values,
};
use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use std::any::TypeId;

pub mod abilities;
pub mod attributes;
pub mod effects;
pub mod evaluators;
pub mod mutator;
pub mod systems;

use crate::abilities::GameAbilityContainer;
use crate::attributes::AttributeDef;
pub use attributes_macro::Attribute;
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub struct DeathByAttributesPlugin;

impl Plugin for DeathByAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(AttributeUpdate)
            .add_event::<GameEffectEvent>()
            .add_systems(
                AttributeUpdate,
                (
                    tick_effects_periodic_timer,
                    tick_ability_cooldowns,
                    tick_effects_duration,
                ),
            )
            .add_systems(
                AttributeUpdate,
                (update_base_values, update_current_values).chain(),
            )
            .insert_resource(EvalStruct::default())
            .add_observer(on_instant_effect_added)
            .add_observer(on_duration_effect_added)
            .add_observer(on_effect_removed);

        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Update, AttributeUpdate);
    }
}

pub type AttributeEntityMut<'w> = EntityMutExcept<
    'w,
    (
        GameAbilityContainer,
        EffectTarget,
        Effect,
        EffectPeriodicApplication,
        EffectDuration,
    ),
>;
pub type AttributeEntityRef<'w> = EntityRefExcept<
    'w,
    (
        GameAbilityContainer,
        EffectTarget,
        Effect,
        EffectPeriodicApplication,
        EffectDuration,
    ),
>;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeUpdate;

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

pub trait Editable: Reflect + Sized + Send + Sync + 'static {
    fn get_base_value(&self) -> f32;
    fn get_current_value(&self) -> f32;
}
impl Editable for AttributeDef {
    fn get_base_value(&self) -> f32 {
        self.base_value
    }
    fn get_current_value(&self) -> f32 {
        self.current_value
    }
}
