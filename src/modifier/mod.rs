mod attribute_modifier;
mod calculator;
mod collector;
mod derived_modifier;

use crate::attributes::Attribute;
use crate::condition::ConditionContext;
use crate::inspector::pretty_type_name;
use crate::prelude::AttributeModifier;
use crate::{AttributesMut, AttributesRef};
use bevy::prelude::{Commands, Component, Entity, EntityCommands, Reflect, reflect_trait};
use std::fmt::{Debug, Formatter};

pub mod prelude {
    pub use super::attribute_modifier::AttributeModifier;
    pub use super::calculator::{AttributeCalculatorCached, AttributeCalculator};
    pub use super::calculator::Mod;
    pub use super::collector::collect_entity_modifiers;
    pub use super::derived_modifier::DerivedModifier;
}

pub type ModifierFn = dyn Fn(&mut EntityCommands, Entity) + Send + Sync;

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Modifiers)]
pub struct ModifierOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierOf, linked_spawn)]
pub struct Modifiers(Vec<Entity>);

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
pub struct ModifierMarker;

pub trait Mutator: Send + Sync {
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity;
    fn apply(&self, actor_entity: &mut AttributesMut) -> bool;
    fn who(&self) -> Who;
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
        format!("{}", self.modifier)
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

#[derive(Copy, Clone, Reflect)]
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
