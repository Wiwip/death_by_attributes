use crate::modifier::Modifiers;
use crate::systems::{apply_periodic_effect, flag_dirty_modifier, observe_dirty_effect_notifications, observe_dirty_modifier_notifications, update_changed_attributes, update_effect_tree_system};
use bevy::ecs::event::EventRegistry;
use bevy::prelude::*;
use std::any::{TypeId, type_name};
use std::marker::PhantomData;

pub mod ability;
pub mod actors;
pub mod assets;
pub mod attributes;
pub mod condition;
pub mod context;
pub mod effect;
pub mod inspector;
mod modifier;
pub mod mutator;
mod systems;

use crate::ability::{Abilities, Ability, AbilityCooldown, AbilityOf, AbilityPlugin};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attributes::{Attribute, ReflectAccessAttribute, clamp_attributes_system};
use crate::condition::ConditionPlugin;
use crate::effect::EffectsPlugin;
use crate::prelude::{
    AttributeModifier, Effect, EffectDuration, EffectSource, EffectSources, EffectTarget,
    EffectTicker, Effects, Mod, tick_effect_tickers,
};
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub mod prelude {
    pub use crate::attributes::Attribute;
    pub use crate::effect::*;
    pub use crate::modifier::prelude::*;
    pub use crate::modifier::*;
}

pub struct AttributesPlugin;

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((AbilityPlugin, ConditionPlugin, EffectsPlugin))
            .init_schedule(PreUpdate)
            .init_schedule(PostUpdate)
            .init_asset::<ActorDef>()
            .init_asset::<EffectDef>()
            .init_asset::<AbilityDef>()
            .register_type::<Modifiers>();
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
        schedule.add_systems(apply_periodic_effect::<T>.after(tick_effect_tickers));
        schedule.add_systems(update_effect_tree_system::<T>.after(apply_periodic_effect::<T>));
    });

    world.schedule_scope(PostUpdate, |_, schedule| {
        schedule.add_systems(flag_dirty_modifier::<T>);
        schedule.add_systems(clamp_attributes_system::<T>);
        schedule.add_systems(update_changed_attributes::<T>);
    });

    world.add_observer(observe_dirty_modifier_notifications::<T>);
    world.add_observer(observe_dirty_effect_notifications::<T>);

    debug!("Registered Systems for: {}.", type_name::<T>());
}

pub type AttributesMut<'w> = EntityMutExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectDuration,
        EffectTicker,
        EffectSource,
        EffectTarget,
        Effects,
        EffectSources,
        Ability,
        Abilities,
        AbilityOf,
        AbilityCooldown,
    ),
>;

pub type AttributesRef<'w> = EntityRefExcept<
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectDuration,
        EffectTicker,
        EffectSource,
        EffectTarget,
        Effects,
        EffectSources,
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
    pub modifier: Mod,
}

#[derive(Clone, Debug)]
pub enum AttributeEvaluationError {
    ComponentNotPresent(TypeId),
}
