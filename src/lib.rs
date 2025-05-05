use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer, EffectTarget};
use crate::systems::{
    check_duration_effect_expiry, on_attribute_mutation_changed, on_base_value_changed,
    on_duration_effect_applied, on_duration_effect_removed, on_instant_effect_applied,
    tick_ability_cooldowns, tick_effects_duration_timer, tick_effects_periodic_timer,
    trigger_periodic_effects,
};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use std::any::TypeId;
use std::collections::HashMap;

pub mod abilities;
pub mod actor;
pub mod attributes;
pub mod effects;
pub mod evaluators;
pub mod mutator;
pub mod systems;

use crate::abilities::GameAbilityContainer;
use crate::mutator::{EffectMutators, ModAggregator, Mutating, Mutator};
pub use attributes_macro::Attribute;
use bevy::ecs::world::EntityMutExcept;
use bevy::utils::TypeIdMap;

pub struct DeathByAttributesPlugin;

impl Plugin for DeathByAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_ability_cooldowns)
            .add_systems(PreUpdate, tick_effects_duration_timer)
            .add_systems(
                PreUpdate,
                (tick_effects_periodic_timer, trigger_periodic_effects).chain(),
            )
            .add_systems(PostUpdate, check_duration_effect_expiry)
            .add_observer(on_instant_effect_applied)
            .add_observer(on_duration_effect_applied)
            .add_observer(on_base_value_changed)
            .add_observer(on_attribute_mutation_changed)
            .add_observer(on_duration_effect_removed)
            .insert_resource(CachedMutations::default());
    }
}

pub type ActorEntityMut<'w> = EntityMutExcept<
    'w,
    (
        GameAbilityContainer,
        // We exclude anything related to effects
        Effect,
        EffectTarget,
        EffectMutators,
        EffectPeriodicTimer,
        EffectDuration,
        // We exclude anything related to mutators
        Mutator,
        Mutating,
    ),
>;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeUpdateSchedule;

#[derive(Event)]
pub struct OnBaseValueChanged;

#[derive(Event)]
pub struct OnAttributeMutationChanged;

#[derive(Event)]
pub struct OnCurrentValueChanged;

#[derive(Clone, Debug)]
pub enum AttributeEvaluationError {
    ComponentNotPresent(TypeId),
}

/*pub trait Editable: Reflect + Sized + Send + Sync + 'static {
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
}*/

#[derive(Default, Resource)]
pub struct CachedMutations {
    pub evaluators: HashMap<Entity, TypeIdMap<(Mutator, ModAggregator)>>,
}
