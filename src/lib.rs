use crate::systems::{
    apply_periodic_effect, observe_dirty_node_notifications, update_attribute, update_effect_system,
};
use bevy::prelude::*;
use std::any::TypeId;
use std::error::Error;
use std::fmt::Formatter;
use std::marker::PhantomData;

pub mod ability;
pub mod actors;
pub mod assets;
pub mod attributes;
pub mod condition;
pub mod context;
pub mod effect;
pub mod graph;
pub mod inspector;
pub mod math;
mod modifier;
pub mod mutator;
mod registry;
mod schedule;
mod systems;
mod trigger;

use crate::ability::{Abilities, Ability, AbilityCooldown, AbilityOf, AbilityPlugin};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attributes::{
    Attribute, ReflectAccessAttribute, apply_derived_clamp_attributes, clamp_attributes_observer,
    on_add_attribute, on_change_notify_attribute_dependencies, on_change_notify_attribute_parents,
};
use crate::condition::ConditionPlugin;
use crate::effect::{EffectIntensity, EffectsPlugin};
use crate::inspector::pretty_type_name;
use crate::prelude::{
    AppliedEffects, ApplyAttributeModifierEvent, AttributeCalculatorCached, AttributeModifier,
    Effect, EffectDuration, EffectSource, EffectSources, EffectTarget, EffectTicker, Stacks,
    apply_modifier_events,
};
use crate::schedule::EffectsSet;
use bevy::ecs::world::{EntityMutExcept, EntityRefExcept};

pub mod prelude {
    pub use crate::attributes::{
        AccessAttribute, Attribute, AttributeTypeId, ReflectAccessAttribute, Value,
    };
    pub use crate::condition::{ChanceCondition, Condition};
    pub use crate::effect::*;
    pub use crate::modifier::prelude::*;
    pub use crate::modifier::*;
    pub use crate::registry::{
        Registry, RegistryMut, ability_registry::AbilityToken, effect_registry::EffectToken,
    };
    pub use crate::schedule::EffectsSet;
    pub use crate::{AttributesPlugin, attribute};
}

use crate::graph::NodeType;
use crate::modifier::Who;
use crate::registry::RegistryPlugin;
pub use num_traits;

pub struct AttributesPlugin;

impl Plugin for AttributesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AbilityPlugin,
            ConditionPlugin,
            EffectsPlugin,
            RegistryPlugin,
        ))
        .add_plugins((init_attribute::<EffectIntensity>, init_attribute::<Stacks>))
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
    app.add_message::<ApplyAttributeModifierEvent<T>>();

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
        (
            update_effect_system::<T>,
            apply_derived_clamp_attributes::<T>,
        )
            .chain()
            .in_set(EffectsSet::UpdateCurrentValues),
    );

    app.add_systems(
        Update,
        on_change_notify_attribute_dependencies::<T>.in_set(EffectsSet::Notify),
    );

    app.add_systems(
        Update,
        on_change_notify_attribute_parents::<T>.in_set(EffectsSet::Notify),
    );

    app.add_observer(clamp_attributes_observer::<T>);
    app.add_observer(observe_dirty_node_notifications::<T>);
    app.add_observer(on_add_attribute::<T>);
    app.add_observer(update_attribute::<T>);

    debug!(
        "Registered Systems for attribute: {}.",
        pretty_type_name::<T>()
    );
}

pub type AttributesMut<'w> = EntityMutExcept<
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
        Abilities,
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
        Abilities,
        AbilityOf,
        AbilityCooldown,
    ),
>;

pub trait Spawnable: Send + Sync {
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity;
    fn who(&self) -> Who;
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

#[derive(EntityEvent, Clone)]
#[entity_event(propagate = &'static EffectTarget, auto_propagate)]
pub struct AttributeValueChanged<T> {
    entity: Entity,
    _marker: PhantomData<T>,
}

#[derive(EntityEvent, Debug)]
pub struct OnBaseValueChange<T: Attribute> {
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
        }
    }
}

impl Error for AttributeError {}
