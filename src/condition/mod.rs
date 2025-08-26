use crate::condition::systems::evaluate_effect_conditions;
use bevy::app::{App, Plugin, PreUpdate};
use fixed::prelude::ToFixed;
use fixed::traits::{Fixed, LossyInto};
use std::collections::Bound;
use std::ops::RangeBounds;

mod conditions;
mod systems;

use crate::AttributesRef;
use crate::attributes::Attribute;
pub use conditions::{
    AbilityCondition, And, AttributeCondition, ConditionExt, FunctionCondition, Not, Or,
    StackCondition, TagCondition,
};

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        // This system is responsible for checking conditions and
        // activating/deactivating their related effects.
        app.add_systems(PreUpdate, evaluate_effect_conditions);
    }
}

pub trait Condition: Send + Sync + 'static {
    fn eval(&self, context: &ConditionContext) -> bool;
}

pub struct BoxCondition(pub Box<dyn Condition>);

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


pub fn convert_bounds<T: Attribute, R>(
    bounds: impl RangeBounds<R>,
) -> (Bound<T::Property>, Bound<T::Property>)
where
    R: ToFixed + Copy,
{
    let start_bound: Bound<T::Property> = match bounds.start_bound() {
        Bound::Included(bound) => Bound::Included(bound.to_fixed()),
        Bound::Excluded(bound) => Bound::Excluded(bound.to_fixed()),
        Bound::Unbounded => Bound::Unbounded,
    };
    let end_bound: Bound<T::Property> = match bounds.end_bound() {
        Bound::Included(bound) => Bound::Included(bound.to_fixed()),
        Bound::Excluded(bound) => Bound::Excluded(bound.to_fixed()),
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
