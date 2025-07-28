use crate::effects::{
    ApplyEffectEvent, Effect, EffectDuration, EffectPeriodicTimer, EffectSource, EffectSources,
    EffectTarget, EffectTargetedBy, OnAddStackEffect, apply_effect_events, read_add_stack_event,
};
use crate::modifiers::{AttributeModifier, ModAggregator, Modifiers};
use crate::systems::{
    apply_periodic_effect, flag_dirty_attribute, flag_dirty_modifier, tick_ability_cooldown,
    tick_effect_duration_timers, tick_effects_periodic_timer, update_effect_tree_system,
};
use bevy::ecs::event::EventRegistry;
use bevy::prelude::*;
use std::any::{TypeId, type_name};
use std::marker::PhantomData;

pub mod abilities;
pub mod actors;
pub mod assets;
pub mod attributes;
pub mod conditions;
pub mod context;
pub mod effects;
pub mod inspector;
pub mod modifiers;
pub mod mutator;
pub mod stacks;
pub mod systems;
pub mod evaluator;

use crate::abilities::{Abilities, Ability, AbilityCooldown, AbilityOf, try_activate_ability_observer};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attributes::{Attribute, ReflectAccessAttribute, clamp_attributes_system};
use crate::conditions::evaluate_effect_conditions;
pub use attributes_macro::Attribute;
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub struct AttributesPlugin;

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_effect_duration_timers)
            .add_systems(PreUpdate, tick_effects_periodic_timer)
            .add_systems(PreUpdate, tick_ability_cooldown)
            .add_systems(PreUpdate, evaluate_effect_conditions)
            .add_observer(try_activate_ability_observer)
            .add_observer(apply_effect_events)
            .add_systems(PostUpdate, read_add_stack_event)
            .init_schedule(PreUpdate)
            .init_schedule(PostUpdate)
            .init_asset::<ActorDef>()
            .init_asset::<EffectDef>()
            .init_asset::<AbilityDef>()
            .register_type::<AbilityOf>()
            .register_type::<Abilities>()
            .register_type::<Modifiers>()
            .add_event::<OnAddStackEffect>();
    }
}

impl AttributesPlugin {
    pub fn default() -> Self {
        Self
    }
}

pub fn init_attribute<T: Attribute>(app: &mut App) {
    let world = app.world_mut();

    world.resource_scope(|_world, type_registry: Mut<AppTypeRegistry>| {
        type_registry.write().register::<AttributeModifier<T>>();
        type_registry.write().register::<T>();

        type_registry
            .write()
            .register_type_data::<T, ReflectAccessAttribute>();
    });

    EventRegistry::register_event::<OnBaseValueChange<T>>(world);

    world.schedule_scope(PreUpdate, |_, schedule| {
        schedule.add_systems(apply_periodic_effect::<T>.after(tick_effects_periodic_timer));
        schedule.add_systems(update_effect_tree_system::<T>.after(apply_periodic_effect::<T>));
    });

    world.schedule_scope(PostUpdate, |_, schedule| {
        schedule.add_systems(flag_dirty_attribute::<T>);
        schedule.add_systems(flag_dirty_modifier::<T>);
        schedule.add_systems(clamp_attributes_system::<T>);
    });

    debug!("Registered Systems for: {}.", type_name::<T>());
}

pub type ActorEntityMut<'w> = EntityMutExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectSource,
        EffectTarget,
        EffectTargetedBy,
        EffectSources,
        EffectPeriodicTimer,
        EffectDuration,
        Ability,
        Abilities,
        AbilityOf,
        AbilityCooldown,
    ),
>;

pub type ActorEntityRef<'w> = EntityRefExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectSource,
        EffectTarget,
        EffectTargetedBy,
        EffectSources,
        EffectPeriodicTimer,
        EffectDuration,
        Ability,
        Abilities,
        AbilityOf,
        AbilityCooldown,
    ),
>;

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
#[event(traversal = &'static EffectTarget, auto_propagate)]
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
#[event(traversal = &'static EffectTarget, auto_propagate)]
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
