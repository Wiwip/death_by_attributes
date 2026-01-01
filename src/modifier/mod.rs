mod attribute_modifier;
mod calculator;
mod events;

use crate::condition::{GameplayContext, GameplayContextMut};
use crate::inspector::pretty_type_name;
use crate::prelude::*;
use crate::{AttributesMut, AttributesRef, Spawnable};
use bevy::prelude::{Commands, Component, Entity, EntityCommands, Reflect, reflect_trait};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

pub use attribute_modifier::AttributeModifier;
pub use calculator::{AttributeCalculator, AttributeCalculatorCached, ModOp};
pub use events::{ApplyAttributeModifierMessage, apply_modifier_events};

pub type ModifierFn = dyn Fn(&mut EntityCommands, Entity) + Send + Sync;

#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
pub struct ModifierMarker;

pub trait Modifier: Spawnable + Send + Sync {
    fn apply_immediate(&self, context: &mut GameplayContextMut) -> bool;
    fn apply_delayed(
        &self,
        source: Entity,
        target: Entity,
        effect: Entity,
        commands: &mut Commands,
    );
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

#[derive(Component, Copy, Clone, Reflect, Serialize, Deserialize)]
pub enum Who {
    Target,
    Source,
    Owner,
}

impl Who {
    /// Resolves the `Who` variant to a specific entity from the context.
    pub fn resolve_entity_mut<'a, 'b>(
        &self,
        context: &'b mut GameplayContextMut<'a>,
    ) -> &'b mut AttributesMut<'a> {
        let result = match self {
            Who::Target => &mut *context.target_actor,
            Who::Source => &mut *context.source_actor,
            Who::Owner => &mut *context.owner,
        };
        result
    }

    /// Resolves the `Who` variant to a specific entity from the context.
    pub fn resolve_entity<'a>(&self, context: &'a GameplayContext<'a>) -> &'a AttributesRef<'a> {
        match self {
            Who::Target => context.target_actor,
            Who::Source => context.source_actor,
            Who::Owner => context.owner,
        }
    }
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

/// The target entity of this effect.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = OwnedModifiers)]
pub struct ModifierOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierOf, linked_spawn)]
pub struct OwnedModifiers(Vec<Entity>);

/// The target entity of this modifier.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = ModifierSources)]
pub struct ModifierSource(pub Entity);

/// All modifiers that are sourced from this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierSource)]
pub struct ModifierSources(Vec<Entity>);

/// The target entity of this modifier.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = AppliedModifiers)]
pub struct ModifierTarget(pub Entity);

/// All modifiers that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ModifierTarget)]
pub struct AppliedModifiers(Vec<Entity>);
