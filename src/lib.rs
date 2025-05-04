use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer, MutationAggregatorCache};
use crate::systems::{
    on_effect_removed, on_instant_effect_added, tick_ability_cooldowns,
    tick_effects_duration_timer, tick_effects_periodic_timer, update_base_values,
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
pub mod actor;

use crate::abilities::GameAbilityContainer;
use crate::attributes::AttributeDef;
pub use attributes_macro::Attribute;
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub struct DeathByAttributesPlugin;

impl Plugin for DeathByAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(AttributeUpdate)
            .add_systems(
                AttributeUpdate,
                (
                    tick_effects_periodic_timer,
                    tick_ability_cooldowns,
                    tick_effects_duration_timer,
                ),
            )
            .add_systems(
                AttributeUpdate,
                (update_base_values, update_current_values).chain(),
            )
            .add_observer(on_instant_effect_added)
            .add_observer(on_effect_removed)
            .insert_resource(MutationAggregatorCache::default());

        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Update, AttributeUpdate);
    }
}

pub type AttributeEntityMut<'w> = EntityMutExcept<
    'w,
    (
        GameAbilityContainer,
        Effect,
        EffectPeriodicTimer,
        EffectDuration,
        Children,
        ChildOf,
    ),
>;
pub type AttributeEntityRef<'w> = EntityRefExcept<
    'w,
    (
        GameAbilityContainer,
        Effect,
        EffectPeriodicTimer,
        EffectDuration,
        Children,
        ChildOf,
    ),
>;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeUpdate;

#[derive(Event)]
pub struct BaseValueChanged;

#[derive(Event)]
pub struct CurrentValueChanged;

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
    fn set_current_value(&mut self, value: f32);
}
impl Editable for AttributeDef {
    fn get_base_value(&self) -> f32 {
        self.base_value
    }
    fn get_current_value(&self) -> f32 {
        self.current_value
    }

    fn set_current_value(&mut self, value: f32) {
        self.current_value = value;
    }
}
