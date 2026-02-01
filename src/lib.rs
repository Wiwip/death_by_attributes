extern crate core;

use bevy::prelude::*;
use std::any::TypeId;
use std::error::Error;
use std::fmt::Formatter;
use std::marker::PhantomData;

pub mod ability;
pub mod actors;
pub mod assets;
mod attribute_clamp;
pub mod attributes;
pub mod condition;
pub mod context;
pub mod effect;
pub mod graph;
pub mod inspector;
pub mod math;
pub mod modifier;
pub mod mutator;
pub mod registry;
mod schedule;
mod systems;
mod trigger;

use crate::ability::{Ability, AbilityCooldown, AbilityOf, AbilityPlugin, GrantedAbilities};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attribute_clamp::apply_clamps;
use crate::attributes::{
    ReflectAccessAttribute, on_add_attribute, on_change_notify_attribute_dependencies,
    on_change_notify_attribute_parents,
};
use crate::condition::ConditionPlugin;
use crate::effect::global_effect::GlobalEffectPlugin;
use crate::effect::{
    AppliedEffects, Effect, EffectDuration, EffectSource, EffectSources, EffectTarget,
    EffectTicker, EffectsPlugin, Stacks,
};
use crate::graph::NodeType;
use crate::inspector::pretty_type_name;
use crate::modifier::{
    ApplyAttributeModifierMessage, AttributeCalculatorCached, apply_modifier_events,
};
use crate::prelude::*;
use crate::registry::RegistryPlugin;
use crate::schedule::EffectsSet;
use crate::systems::{
    apply_periodic_effect, mark_node_dirty_observer, update_attribute, update_effect_system,
};
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub mod prelude {
    pub use crate::attributes::{
        AccessAttribute, Attribute, AttributeTypeId, ReflectAccessAttribute,
    };
    pub use crate::modifier::{AccessModifier, AttributeModifier, Modifier};
}

pub use num_traits;

pub struct AttributesPlugin;

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AbilityPlugin,
            ConditionPlugin,
            EffectsPlugin,
            GlobalEffectPlugin,
            RegistryPlugin,
        ))
        .add_plugins(init_attribute::<Stacks>)
        .init_schedule(PreUpdate)
        .init_schedule(PostUpdate)
        .init_asset::<ActorDef>()
        .init_asset::<EffectDef>()
        .init_asset::<AbilityDef>()
        .register_type::<AppliedEffects>()
        .register_type::<EffectTarget>()
        .register_type::<NodeType>();

        app.configure_sets(
            Update,
            (
                EffectsSet::First,
                EffectsSet::Prepare,
                EffectsSet::UpdateBaseValues,
                EffectsSet::UpdateCurrentValues,
                EffectsSet::Notify,
                EffectsSet::Last,
            )
                .chain(),
        );
    }
}

impl AttributesPlugin {
    pub fn default() -> Self {
        Self
    }
}

pub fn init_attribute<T: Attribute>(app: &mut App) {
    app.register_type::<T>();
    app.register_type::<AttributeModifier<T>>();
    app.register_type::<AttributeCalculatorCached<T>>();
    app.register_type_data::<T, ReflectAccessAttribute>();
    app.add_message::<ApplyAttributeModifierMessage<T>>();

    app.add_systems(
        Update,
        apply_periodic_effect::<T>.in_set(EffectsSet::Prepare),
    );

    app.add_systems(
        Update,
        apply_modifier_events::<T>.in_set(EffectsSet::UpdateBaseValues),
    );

    app.add_systems(
        Update,
        (update_effect_system::<T>, apply_clamps::<T>)
            .chain()
            .in_set(EffectsSet::UpdateCurrentValues),
    );

    app.add_systems(
        Update,
        (
            on_change_notify_attribute_parents::<T>.in_set(EffectsSet::Notify),
            on_change_notify_attribute_dependencies::<T>.in_set(EffectsSet::Notify),
        ),
    );

    app.add_observer(mark_node_dirty_observer::<T>);
    app.add_observer(on_add_attribute::<T>);
    app.add_observer(update_attribute::<T>);

    debug!(
        "Registered Systems for attribute: {}.",
        pretty_type_name::<T>()
    );
}

pub type AttributesMut<'w, 's> = EntityMutExcept<
    'w,
    's,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectDuration,
        EffectTicker,
        EffectSource,
        EffectTarget,
        AppliedEffects,
        EffectSources,
        Ability,
        GrantedAbilities,
        AbilityOf,
        AbilityCooldown,
    ),
>;

pub type AttributesRef<'w> = EntityRefExcept<
    'w,
    'w,
    (
        // We exclude anything related to effects
        ChildOf,
        Effect,
        EffectDuration,
        EffectTicker,
        EffectSource,
        EffectTarget,
        AppliedEffects,
        EffectSources,
        Ability,
        GrantedAbilities,
        AbilityOf,
        AbilityCooldown,
    ),
>;

pub trait Spawnable: Send + Sync {
    fn spawn(&self, commands: &mut EntityCommands);
    //fn who(&self) -> Who;
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

#[derive(EntityEvent, Debug)]
pub struct BaseValueChanged<T: Attribute> {
    pub phantom_data: PhantomData<T>,
    pub old: T::Property,
    pub new: T::Property,
    pub entity: Entity,
}

#[derive(EntityEvent, Debug)]
pub struct CurrentValueChanged<T: Attribute> {
    pub phantom_data: PhantomData<T>,
    pub old: T::Property,
    pub new: T::Property,
    pub entity: Entity,
}

#[derive(Clone, Debug)]
pub enum AttributeError {
    AttributeNotPresent(TypeId),
    PhantomQuery,
}

impl std::fmt::Display for AttributeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeError::AttributeNotPresent(type_id) => {
                write!(
                    f,
                    "Attribute with TypeId {:?} not present on entity.",
                    type_id
                )
            }
            AttributeError::PhantomQuery => {
                write!(f, "Phantom query on entity does not exist.")
            }
        }
    }
}

impl Error for AttributeError {}
