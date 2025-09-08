mod attribute_modifier;
mod calculator;
mod events;

use crate::attributes::Attribute;
use crate::condition::ConditionContext;
use crate::inspector::pretty_type_name;
use crate::prelude::{AttributeModifier, AttributeTypeId};
use crate::{AttributesMut, AttributesRef};
use bevy::prelude::{Commands, Component, Entity, EntityCommands, Reflect, reflect_trait};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

pub mod prelude {
    pub use super::attribute_modifier::AttributeModifier;
    pub use super::calculator::ModOp;
    pub use super::calculator::{AttributeCalculator, AttributeCalculatorCached};
    pub use super::events::{ApplyAttributeModifierEvent, apply_modifier_events};
}

pub type ModifierFn = dyn Fn(&mut EntityCommands, Entity) + Send + Sync;

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
pub struct ModifierMarker;

pub trait Modifier: Send + Sync {
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity;
    fn apply(&self, actor_entity: &mut AttributesMut) -> bool;
    fn write_event(&self, target: Entity, commands: &mut Commands);
    fn who(&self) -> Who;
    fn attribute_type_id(&self) -> AttributeTypeId;
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
    pub fn resolve_entity<'a>(&self, context: &'a ConditionContext<'a>) -> &'a AttributesRef<'a> {
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
