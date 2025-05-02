use crate::effects::{GameEffectContainer, GameEffectEvent};
use crate::systems::{
    handle_apply_effect_events, tick_ability_cooldowns, tick_active_effects,
    update_attribute_base_value, update_attribute_current_value,
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

pub type AttributeEntityMut<'w> = EntityMutExcept<'w, (GameEffectContainer, GameAbilityContainer)>;
pub type AttributeEntityRef<'w> = EntityRefExcept<'w, (GameEffectContainer, GameAbilityContainer)>;

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

pub trait Editable: Reflect + Sized + Send + Sync + 'static {
    fn get_base_value(&self) -> f32;
    fn get_current_value(&self) -> f32;

    //fn set_base_value(&mut self, value: f32);
    //fn set_current_value(&mut self, value: f32);
}
impl Editable for AttributeDef {
    fn get_base_value(&self) -> f32 {
        self.base_value
    }
    fn get_current_value(&self) -> f32 {
        self.current_value
    }

    /*fn set_base_value(&mut self, value: f32) {
        self.base_value = value;
    }

    fn set_current_value(&mut self, value: f32) {
        self.current_value = value;
    }*/
}
