mod application;
mod builder;
mod execution;
mod stacks;
mod targeting;
mod timing;

use crate::assets::EffectDef;
use crate::effect::application::apply_effect_events;
use crate::effect::stacks::{NotifyAddStackEvent, read_add_stack_event};
use bevy::app::{App, Plugin, PostUpdate, PreUpdate};
use bevy::asset::Handle;
use bevy::ecs::query::QueryData;
use bevy::prelude::{Component, Deref, Entity, Event, Reflect};

pub use application::{
    ApplyEffectEvent, EffectApplicationPolicy, tick_effect_durations, tick_effect_tickers,
};
pub use builder::EffectBuilder;
pub use execution::{EffectCalculationContext, EffectCaptureContext, EffectExecution};
pub use stacks::{EffectStackingPolicy, Stacks};
pub use targeting::EffectTargeting;
pub use timing::{EffectDuration, EffectTicker};

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_effect_tickers)
            .add_systems(PreUpdate, tick_effect_durations)
            .add_systems(PostUpdate, read_add_stack_event)
            .add_observer(apply_effect_events)
            .add_event::<NotifyAddStackEvent>();
    }
}

pub enum EffectStatus {
    Active,
    Inactive,
}

#[derive(Clone, Copy, Debug, Reflect)]
pub enum Target {
    SelfEntity,
    TargetEntity,
}

#[derive(Event)]
pub struct OnEffectStatusChangeEvent(pub EffectStatus);

#[derive(Component, Debug, Default)]
#[component(storage = "SparseSet")]
pub struct EffectInactive;

#[derive(Component, Debug, Default, Deref)]
#[require(Stacks)]
pub struct Effect(pub Handle<EffectDef>);

/// Who created this effect?
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = EffectSources)]
pub struct EffectSource(pub Entity);

/// All effects originating from this entity
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EffectSource, linked_spawn)]
pub struct EffectSources(Vec<Entity>);

/// All effects targeting this entity
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = AppliedEffects)]
pub struct EffectTarget(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EffectTarget, linked_spawn)]
pub struct AppliedEffects(Vec<Entity>);

#[derive(QueryData)]
pub struct EffectStatusParam {
    inactive: Option<&'static EffectInactive>,
    periodic: Option<&'static EffectTicker>,
}

impl EffectStatusParamItem<'_> {
    pub fn is_inactive(&self) -> bool {
        self.inactive.is_some()
    }

    pub fn is_periodic(&self) -> bool {
        self.periodic.is_some()
    }
}








