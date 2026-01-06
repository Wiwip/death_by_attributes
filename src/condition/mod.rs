use crate::condition::systems::evaluate_effect_conditions;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use std::fmt::Debug;

mod conditions;
mod systems;

use crate::{AttributesMut, AttributesRef};

use crate::modifier::Who;
use crate::schedule::EffectsSet;
pub use conditions::{
    AbilityCondition, And, AttributeCondition, ChanceCondition, ConditionExt, Not, Or,
    StackCondition, TagCondition,
};

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        // This system is responsible for checking conditions and
        // activating/deactivating their related effects.
        app.add_systems(
            Update,
            evaluate_effect_conditions.in_set(EffectsSet::Prepare),
        );
        //app.add_systems(Update, evaluate_effect_conditions.in_set(EffectsSet::Notify));
    }
}

pub trait Condition: Debug + Send + Sync {
    fn eval(&self, context: &GameplayContext) -> Result<bool, BevyError>;
}

#[derive(Debug)]
pub struct BoxCondition(pub Box<dyn Condition>);

impl BoxCondition {
    pub fn new<C: Condition + 'static>(condition: C) -> Self {
        Self(Box::new(condition))
    }
}

pub struct GameplayContextMut<'w, 's> {
    pub source_actor: Entity,
    pub target_actor: Entity,
    pub owner: Entity,

    pub actors: Query<'w, 's, AttributesMut<'static, 'static>>,
}

impl GameplayContextMut<'_, '_> {
    pub fn entity(&self, who: Who) -> Entity {
        match who {
            Who::Target => self.target_actor,
            Who::Source => self.source_actor,
            Who::Owner => self.owner,
        }
    }

    pub fn attribute_ref(&self, who: Who) -> AttributesRef<'_> {
        self.actors.get(self.entity(who)).unwrap()
    }

    pub fn attribute_mut(&mut self, who: Who) -> AttributesMut<'_, '_> {
        self.actors.get_mut(self.entity(who)).unwrap()
    }
}

pub struct GameplayContext<'w> {
    pub source_actor: &'w AttributesRef<'w>,
    pub target_actor: &'w AttributesRef<'w>,
    pub owner: &'w AttributesRef<'w>,
}

impl GameplayContext<'_> {
    pub fn entity(&self, who: Who) -> Entity {
        match who {
            Who::Target => self.target_actor.id(),
            Who::Source => self.source_actor.id(),
            Who::Owner => self.owner.id(),
        }
    }

    pub fn attribute_ref(&self, who: Who) -> &AttributesRef<'_> {
        match who {
            Who::Target => self.target_actor,
            Who::Source => self.source_actor,
            Who::Owner => self.owner,
        }
    }
}
