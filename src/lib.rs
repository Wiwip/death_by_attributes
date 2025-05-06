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
use std::marker::PhantomData;

pub mod abilities;
pub mod attributes;
pub mod effects;
pub mod evaluators;
pub mod mutators;
pub mod systems;

use crate::abilities::GameAbilityContainer;
use crate::mutators::mutator::ModAggregator;
use crate::mutators::{EffectMutators, Mutating, Mutator};
pub use attributes_macro::Attribute;
use bevy::ecs::world::EntityMutExcept;
use bevy::platform::collections::HashMap;
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
            .insert_resource(CachedMutations::default())
            .insert_resource(MetaCache::default());
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

#[derive(Event, Debug)]
pub struct OnAttributeChanged<A> {
    phantom_data: PhantomData<A>,
    pub aggregator: ModAggregator,
    pub entity: Entity,
}

#[derive(Event)]
pub struct OnAttributeMutationChanged;

#[derive(Event)]
pub struct OnCurrentValueChanged;

#[derive(Clone, Debug)]
pub enum AttributeEvaluationError {
    ComponentNotPresent(TypeId),
}

#[derive(Default, Resource)]
pub struct CachedMutations {
    pub evaluators: HashMap<Entity, TypeIdMap<(Mutator, ModAggregator)>>,
}

/// Caller entity (mutator observer) / TypeId (Attribute)
#[derive(Default, Resource, Deref, DerefMut)]
#[derive(Debug)]
pub struct MetaCache(HashMap<(Entity, TypeId), (Mutator, ModAggregator)>);
