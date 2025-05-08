use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer};
use crate::systems::{
    despawn_instant_effect, tick_effects_duration_timer, tick_effects_periodic_timer,
};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use std::any::TypeId;
use std::marker::PhantomData;

pub mod abilities;
pub mod actors;
pub mod attributes;
pub mod effects;
pub mod evaluators;
pub mod modifiers;
pub mod systems;

use crate::abilities::GameAbilityContainer;
use crate::modifiers::{ModifierOf, Modifiers};
pub use attributes_macro::Attribute;
use bevy::ecs::world::EntityMutExcept;
use bevy::log::tracing::span::Attributes;

pub struct DeathByAttributesPlugin;

impl Plugin for DeathByAttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_effects_duration_timer)
            .add_systems(PreUpdate, tick_effects_periodic_timer)
            .add_systems(PostUpdate, despawn_instant_effect)
            .init_schedule(PreUpdate)
            .init_schedule(PostUpdate);
    }
}

pub type ActorEntityMut<'w> = EntityMutExcept<
    'w,
    (
        GameAbilityContainer,
        // We exclude anything related to effects
        Effect,
        ModifierOf,
        Modifiers,
        EffectPeriodicTimer,
        EffectDuration,
    ),
>;

#[derive(Component, Copy, Clone, Debug)]
pub struct Actor;

#[derive(Component, Copy, Clone, Debug)]
pub(crate) struct ObserverMarker<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for ObserverMarker<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[derive(Component, Copy, Clone, Debug)]
#[component(storage = "SparseSet")]
pub struct Dirty<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for Dirty<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeUpdateSchedule;

#[derive(Event)]
pub struct OnAttributeValueChanged;

#[derive(Event, Debug)]
pub struct OnAttributeChanged<A> {
    phantom_data: PhantomData<A>,
}

#[derive(Event)]
pub struct OnAttributeMutationChanged;

#[derive(Event)]
pub struct OnCurrentValueChanged;

#[derive(Clone, Debug)]
pub enum AttributeEvaluationError {
    ComponentNotPresent(TypeId),
}

#[derive(Resource)]
pub struct RegisteredSystemCache<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for RegisteredSystemCache<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
