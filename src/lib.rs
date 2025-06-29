use crate::effects::EffectOf;
use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer};
use crate::modifiers::ModAggregator;
use crate::systems::{
    despawn_instant_effect, tick_ability_cooldown, tick_effects_duration_timer,
    tick_effects_periodic_timer,
};
use bevy::prelude::*;
use std::any::TypeId;
use std::marker::PhantomData;

pub mod abilities;
pub mod actors;
pub mod attributes;
pub mod effects;
pub mod modifiers;
pub mod systems;

use crate::abilities::{Abilities, AbilityActivation, AbilityCooldown, AbilityCost, AbilityOf};
use crate::attributes::AttributeComponent;
pub use attributes_macro::Attribute;
use bevy::ecs::world::EntityMutExcept;

pub struct AttributesPlugin;

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_effects_duration_timer)
            .add_systems(PreUpdate, tick_effects_periodic_timer)
            .add_systems(PreUpdate, tick_ability_cooldown)
            .add_systems(PostUpdate, despawn_instant_effect)
            .init_schedule(PreUpdate)
            .init_schedule(PostUpdate);
    }
}

pub type ActorEntityMut<'w> = EntityMutExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectPeriodicTimer,
        EffectDuration,
        Abilities,
        AbilityOf,
        AbilityActivation,
        AbilityCost,
        AbilityCooldown,
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

#[derive(Event, Clone)]
#[event(traversal = &'static EffectOf, auto_propagate)]
pub struct OnAttributeValueChanged<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for OnAttributeValueChanged<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Event, Debug)]
pub struct OnBaseValueChange<A: AttributeComponent> {
    pub phantom_data: PhantomData<A>,
    pub old: f32,
    pub new: f32,
    pub entity: Entity,
}

#[derive(Event, Debug)]
pub struct OnCurrentValueChange<A: AttributeComponent> {
    pub phantom_data: PhantomData<A>,
    pub old: f32,
    pub new: f32,
    pub entity: Entity,
}

#[derive(Event)]
#[event(traversal = &'static EffectOf, auto_propagate)]
pub struct OnModifierApplied<T> {
    pub phantom_data: PhantomData<T>,
    pub value: ModAggregator<T>,
}

impl<T> Default for OnModifierApplied<T> {
    fn default() -> Self {
        Self {
            phantom_data: PhantomData,
            value: ModAggregator::<T>::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum AttributeEvaluationError {
    ComponentNotPresent(TypeId),
}

#[derive(Resource)]
pub struct RegisteredPhantomSystem<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for RegisteredPhantomSystem<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
