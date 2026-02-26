pub mod attribute_modifier;
mod calculator;
mod events;

use crate::condition::BevyContextMut;
use crate::inspector::pretty_type_name;
use crate::prelude::*;
use crate::{AppTypeIdBindings, Spawnable, TypeIdBindings};
pub use attribute_modifier::AttributeModifier;
use bevy::prelude::{Commands, Component, Entity, EntityCommands, Reflect, reflect_trait};
use bevy::reflect::TypeRegistryArc;
pub use calculator::{AttributeCalculator, AttributeCalculatorCached, ModOp};
pub use events::{ApplyAttributeModifierMessage, apply_modifier_events};
use express_it::context::ScopeId;
use express_it::frame::LazyPlan;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

pub type ModifierFn = dyn Fn(&mut EntityCommands, Entity) + Send + Sync;

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
pub struct ModifierMarker;

/*pub trait Modifier: Spawnable + Send + Sync {
    fn apply_immediate(
        &self,
        context: &mut BevyContextMut,
        type_registry: TypeRegistryArc,
        type_bindings: AppTypeIdBindings,
    ) -> bool;
    fn apply_delayed(
        &self,
        source: Entity,
        target: Entity,
        effect: Entity,
        commands: &mut Commands,
    );
}*/

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

#[derive(Component, Copy, Clone, Reflect, Serialize, Deserialize)]
pub enum Who {
    Target,
    Source,
    Owner,
}

impl Debug for Who {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Who {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Who::Target => write!(f, "Target"),
            Who::Source => write!(f, "Source"),
            Who::Owner => write!(f, "Owner"),
        }
    }
}

impl Into<ScopeId> for Who {
    fn into(self) -> ScopeId {
        ScopeId(self as u8)
    }
}

impl TryFrom<u8> for Who {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Who::Target),
            1 => Ok(Who::Source),
            2 => Ok(Who::Owner),
            _ => Err(()),
        }
    }
}

/// The target entity of this effect.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = OwnedModifiers)]
pub struct ModifierOf(pub Entity);

/// All modifiers belonging to this effect.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierOf, linked_spawn)]
pub struct OwnedModifiers(Vec<Entity>);
