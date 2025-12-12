use crate::condition::systems::evaluate_effect_conditions;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use num_traits::{AsPrimitive, Num};
use std::collections::Bound;
use std::fmt::Debug;
use std::ops::RangeBounds;

mod conditions;
mod systems;

use crate::attributes::Attribute;
use crate::{AttributesMut, AttributesRef};

pub use conditions::{
    AbilityCondition, And, AttributeCondition, ChanceCondition, ConditionExt, Not, Or,
    StackCondition, TagCondition,
};
use crate::prelude::EffectsSet;

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        // This system is responsible for checking conditions and
        // activating/deactivating their related effects.
        app.add_systems(Update, evaluate_effect_conditions.in_set(EffectsSet::Prepare));
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

pub struct GameplayContextMut<'a> {
    pub target_actor: &'a AttributesMut<'a>,
    pub source_actor: &'a AttributesMut<'a>,
    pub owner: &'a AttributesMut<'a>,
}

pub struct GameplayContext<'a> {
    pub target_actor: &'a AttributesRef<'a>,
    pub source_actor: &'a AttributesRef<'a>,
    pub owner: &'a AttributesRef<'a>,
}

pub fn convert_bounds<S, T>(bounds: impl RangeBounds<S>) -> (Bound<T::Property>, Bound<T::Property>)
where
    S: Num + AsPrimitive<T::Property> + Copy + 'static,
    T: Attribute,
{
    let start_bound: Bound<T::Property> = match bounds.start_bound() {
        Bound::Included(&bound) => Bound::Included(bound.as_()),
        Bound::Excluded(&bound) => Bound::Excluded(bound.as_()),
        Bound::Unbounded => Bound::Unbounded,
    };
    let end_bound: Bound<T::Property> = match bounds.end_bound() {
        Bound::Included(bound) => Bound::Included(bound.as_()),
        Bound::Excluded(bound) => Bound::Excluded(bound.as_()),
        Bound::Unbounded => Bound::Unbounded,
    };
    (start_bound, end_bound)
}

pub fn multiply_bounds<T: Attribute>(
    bounds: impl RangeBounds<T::Property>,
    multiplier: T::Property,
) -> (Bound<T::Property>, Bound<T::Property>) {
    let start_bound: Bound<T::Property> = match bounds.start_bound() {
        Bound::Included(&bound) => Bound::Included(bound * multiplier),
        Bound::Excluded(&bound) => Bound::Excluded(bound * multiplier),
        Bound::Unbounded => Bound::Unbounded,
    };
    let end_bound: Bound<T::Property> = match bounds.end_bound() {
        Bound::Included(&bound) => Bound::Included(bound * multiplier),
        Bound::Excluded(&bound) => Bound::Excluded(bound * multiplier),
        Bound::Unbounded => Bound::Unbounded,
    };
    (start_bound, end_bound)
}
