use crate::condition::systems::evaluate_effect_conditions;
use bevy::app::{App, Plugin, PreUpdate};
use bevy::prelude::EntityRef;
use std::fmt::{Debug, Formatter};

mod conditions;
mod evaluator;
mod systems;

pub use conditions::{
    AbilityCondition, And, AttributeCondition, ConditionExt, FunctionCondition, Not, Or,
    StackCondition, TagCondition,
};
use crate::AttributesRef;

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        // This system is responsible for checking conditions and
        // activating/deactivating effects.
        app.add_systems(PreUpdate, evaluate_effect_conditions);
    }
}

pub trait Condition: Send + Sync + 'static {
    fn evaluate(&self, context: &ConditionContext) -> bool;
}

pub struct BoxCondition(pub(crate) Box<dyn Condition>);

impl BoxCondition {
    pub fn new<C: Condition + 'static>(condition: C) -> Self {
        Self(Box::new(condition))
    }
}

pub struct ConditionContext<'a> {
    pub target_actor: &'a AttributesRef<'a>,
    pub source_actor: &'a AttributesRef<'a>,
    pub owner: &'a AttributesRef<'a>,
}

pub enum Who {
    Target,
    Source,
    Owner,
}

impl Who {
    /// Resolves the `Who` variant to a specific entity from the context.
    pub fn get_entity<'a>(&self, context: &'a ConditionContext<'a>) -> &'a AttributesRef<'a> {
        match self {
            Who::Target => context.target_actor,
            Who::Source => context.source_actor,
            Who::Owner => context.owner,
        }
    }
}

impl Debug for Who {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Who::Target => write!(f, "Target"),
            Who::Source => write!(f, "Source"),
            Who::Owner => write!(f, "Owner"),
        }
    }
}
