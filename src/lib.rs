use crate::effects::{observe_effect_application, Effect, EffectDuration, EffectPeriodicTimer};
use crate::effects::{EffectOf, Effects};
use crate::modifiers::ModAggregator;
use crate::systems::{
    tick_ability_cooldown, tick_effect_duration_timers, tick_effects_periodic_timer,
};
use bevy::prelude::*;
use std::any::TypeId;
use std::marker::PhantomData;

pub mod abilities;
pub mod actors;
pub mod assets;
pub mod attributes;
pub mod condition;
pub mod context;
pub mod effects;
pub mod modifiers;
pub mod stacks;
pub mod systems;

use crate::abilities::{Abilities, AbilityActivation, AbilityCooldown, AbilityCost, AbilityOf};
use crate::assets::GameEffect;
use crate::attributes::Attribute;
use crate::condition::evaluate_effect_conditions;
pub use attributes_macro::Attribute;
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub struct AttributesPlugin;

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_effect_duration_timers)
            .add_systems(PreUpdate, tick_effects_periodic_timer)
            .add_systems(PreUpdate, tick_ability_cooldown)
            .add_systems(PreUpdate, evaluate_effect_conditions)
            .add_observer(observe_effect_application)
            .init_schedule(PreUpdate)
            .init_schedule(PostUpdate)
            .init_asset::<GameEffect>();
    }
}

pub type ActorEntityMut<'w> = EntityMutExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        Effects,
        EffectPeriodicTimer,
        EffectDuration,
        Abilities,
        AbilityOf,
        AbilityActivation,
        AbilityCost,
        AbilityCooldown,
    ),
>;

pub type ActorEntityRef<'w> = EntityRefExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        Effects,
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
pub struct OnBaseValueChange<A: Attribute> {
    pub phantom_data: PhantomData<A>,
    pub old: f64,
    pub new: f64,
    pub entity: Entity,
}

#[derive(Event, Debug)]
pub struct OnCurrentValueChange<A: Attribute> {
    pub phantom_data: PhantomData<A>,
    pub old: f64,
    pub new: f64,
    pub entity: Entity,
}

#[derive(Event)]
#[event(traversal = &'static EffectOf, auto_propagate)]
pub struct ApplyModifier<T> {
    pub phantom_data: PhantomData<T>,
    pub value: ModAggregator<T>,
}

impl<T> Default for ApplyModifier<T> {
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
