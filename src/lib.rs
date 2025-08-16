use crate::systems::{
    apply_periodic_effect, observe_dirty_node_notifications, update_effect_system,
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
mod modifier;
pub mod mutator;
mod schedule;
mod systems;

use crate::ability::{Abilities, Ability, AbilityCooldown, AbilityOf, AbilityPlugin};
use crate::assets::{AbilityDef, ActorDef, EffectDef};
use crate::attributes::{
    Attribute, ReflectAccessAttribute, clamp_attributes_observer, on_add_attribute,
    on_change_notify_attribute_dependencies, on_change_notify_attribute_parents,
};
use crate::condition::ConditionPlugin;
use crate::effect::{EffectIntensity, EffectsPlugin};
use crate::inspector::pretty_type_name;
use crate::prelude::{
    AppliedEffects, ApplyAttributeModifierEvent, AttributeModifier, Effect, EffectDuration,
    EffectSource, EffectSources, EffectTarget, EffectTicker, Stacks, apply_modifier_events,
};
use crate::schedule::EffectsSet;
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
            .add_plugins((init_attribute::<EffectIntensity>, init_attribute::<Stacks>))
            .init_schedule(PreUpdate)
            .init_schedule(PostUpdate)
            .init_asset::<ActorDef>()
            .init_asset::<EffectDef>()
            .init_asset::<AbilityDef>()
            .add_event::<ApplyAttributeModifierEvent>()
            .register_type::<AppliedEffects>()
            .register_type::<EffectTarget>();

        app.add_systems(
            Update,
            apply_modifier_events.in_set(EffectsSet::UpdateBaseValues),
        );

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
    app.register_type_data::<T, ReflectAccessAttribute>();

    app.add_systems(
        Update,
        apply_periodic_effect::<T>.in_set(EffectsSet::Prepare),
    );

    app.add_systems(
        Update,
        update_effect_system::<T>.in_set(EffectsSet::UpdateCurrentValues),
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

    //EventRegistry::register_event::<OnBaseValueChange<T>>(world);
    //EventRegistry::register_event::<OnCurrentValueChanged<T>>(world);
    //EventRegistry::register_event::<OnAttributeValueChanged<T>>(world);

    /*world.schedule_scope(PreUpdate, |_, schedule| {
        //schedule.add_systems(apply_periodic_effect::<T>.after(tick_effect_tickers));
        //schedule.add_systems(update_effect_system::<T>.after(apply_periodic_effect::<T>));
        //schedule.add_systems(update_effect_graph::<T>.after(apply_periodic_effect::<T>));
    });

    world.schedule_scope(Update, |_, schedule| {});

    world.schedule_scope(PostUpdate, |_, schedule| {
        //schedule.add_systems(flag_dirty_modifier::<T>);
        //schedule.add_systems();
        //schedule.add_systems(update_changed_attributes::<T>);
    });*/

    //world.add_observer(observe_dirty_modifier_notifications::<T>);

    debug!(
        "Registered Systems for attribute: {}.",
        pretty_type_name::<T>()
    );
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
pub struct OnCurrentValueChanged<A: Attribute> {
    pub phantom_data: PhantomData<A>,
    pub old: f64,
    pub new: f64,
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
                write!(f, "Attribute with TypeId {:?} not present", type_id)
            }
        }
    }
}

impl Error for AttributeError {}
