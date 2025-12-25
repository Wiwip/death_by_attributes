mod attribute_modifier;
mod calculator;
mod events;

use crate::condition::GameplayContext;
use crate::inspector::pretty_type_name;
use crate::prelude::*;
use crate::{AttributesMut, AttributesRef, Spawnable};
use bevy::prelude::{reflect_trait, Commands, Component, Entity, EntityCommands, Reflect};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

pub use attribute_modifier::AttributeModifier;
pub use calculator::{AttributeCalculator, AttributeCalculatorCached, ModOp};
pub use events::{apply_modifier_events, ApplyAttributeModifierMessage};

pub type ModifierFn = dyn Fn(&mut EntityCommands, Entity) + Send + Sync;

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
pub struct ModifierMarker;

pub trait Modifier: Spawnable + Send + Sync {
    fn apply_immediate(&self, actor_entity: &mut AttributesMut) -> bool;
    fn apply_delayed(&self, target: Entity, commands: &mut Commands);
}

#[reflect_trait] // Generates a `ReflectMyTrait` type
pub trait AccessModifier {
    fn describe(&self) -> String;
    fn name(&self) -> String;
}

impl<T> AccessModifier for AttributeModifier<T>
where
    T: Attribute,
{
    fn describe(&self) -> String {
        format!("{}", self)
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

#[derive(Copy, Clone, Reflect, Serialize, Deserialize)]
pub enum Who {
    Target,
    Source,
    Effect,
}

impl Who {
    /// Resolves the `Who` variant to a specific entity from the context.
    pub fn resolve_entity<'a>(&self, context: &'a GameplayContext<'a>) -> &'a AttributesRef<'a> {
        match self {
            Who::Target => context.target_actor,
            Who::Source => context.source_actor,
            Who::Effect => context.owner,
        }
    }
}

impl Debug for Who {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Who::Target => write!(f, "Target"),
            Who::Source => write!(f, "Source"),
            Who::Effect => write!(f, "Owner"),
        }
    }
}

impl Display for Who {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Who::Target => write!(f, "Target"),
            Who::Source => write!(f, "Source"),
            Who::Effect => write!(f, "Owner"),
        }
    }
}
