use crate::ability::Ability;
use crate::assets::AbilityDef;
use crate::attributes::{Attribute, AttributeAccessor, AttributeExtractor};
use crate::condition::{Condition, ConditionContext, convert_bounds};
use crate::effect::Stacks;
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use bevy::asset::AssetId;
use bevy::log::error;
use bevy::prelude::{Component, TypePath};
use fixed::prelude::ToFixed;
use serde::Serialize;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

pub type StackCondition = AttributeCondition<Stacks>;

#[derive(TypePath, Serialize)]
pub struct AttributeCondition<T: Attribute> {
    who: Who,
    bounds: (Bound<T::Property>, Bound<T::Property>),
}

impl<T: Attribute> AttributeCondition<T> {
    pub fn new<'a, R>(range: impl RangeBounds<R>, who: Who) -> Self
    where
        R: ToFixed + Copy,
    {
        let bounds = convert_bounds::<T, R>(range);
        Self { who, bounds }
    }

    pub fn target(range: impl RangeBounds<T::Property> + Send + Sync + 'static) -> Self {
        AttributeCondition::<T>::new(range, Who::Target)
    }

    pub fn source(range: impl RangeBounds<T::Property> + Send + Sync + 'static) -> Self {
        AttributeCondition::<T>::new(range, Who::Source)
    }
}

impl<T: Attribute> Condition for AttributeCondition<T> {
    fn eval(&self, context: &ConditionContext) -> bool {
        let entity = self.who.resolve_entity(context);

        let extractor = AttributeExtractor::<T>::new();
        match extractor.current_value(entity) {
            Ok(value) => self.bounds.contains(&value),
            Err(e) => {
                error!("Error evaluating attribute condition: {:?}", e);
                false
            }
        }
    }
}

impl<T: Attribute> std::fmt::Display for AttributeCondition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (start, end) = &self.bounds;

        let start_str = match start {
            Bound::Included(v) => format!("[{v}"),
            Bound::Excluded(v) => format!("({v}"),
            Bound::Unbounded => "(-∞".to_string(),
        };

        let end_str = match end {
            Bound::Included(v) => format!("{v}]"),
            Bound::Excluded(v) => format!("{v})"),
            Bound::Unbounded => "∞)".to_string(),
        };

        write!(
            f,
            "Attribute {} on {:?} in range {}, {}",
            pretty_type_name::<T>(),
            self.who,
            start_str,
            end_str
        )
    }
}

pub struct And<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for And<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn eval(&self, value: &ConditionContext) -> bool {
        self.c1.eval(value) && self.c2.eval(value)
    }
}

pub struct Or<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for Or<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn eval(&self, context: &ConditionContext) -> bool {
        self.c1.eval(context) || self.c2.eval(context)
    }
}

pub struct Not<C>(C);

impl<C: Condition> Condition for Not<C> {
    fn eval(&self, context: &ConditionContext) -> bool {
        !self.0.eval(context)
    }
}

/// A condition that wraps a closure or function pointer.
///
/// This allows for creating custom, inline condition logic without needing
/// to define a new struct for every case.
pub struct FunctionCondition<F>
where
    F: Fn(&ConditionContext) -> bool + Send + Sync + 'static,
{
    f: F,
}

impl<F> FunctionCondition<F>
where
    F: Fn(&ConditionContext) -> bool + Send + Sync + 'static,
{
    /// Creates a new `FunctionCondition` from a closure.
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Condition for FunctionCondition<F>
where
    F: Fn(&ConditionContext) -> bool + Send + Sync + 'static,
{
    /// Evaluates the condition by calling the wrapped function.
    fn eval(&self, context: &ConditionContext) -> bool {
        (self.f)(context)
    }
}

pub struct TagCondition<C: Component> {
    target: Who,
    phantom_data: PhantomData<C>,
}

impl<C: Component> TagCondition<C> {
    pub fn new(target: Who) -> Self {
        Self {
            target,
            phantom_data: PhantomData,
        }
    }

    pub fn source() -> Self {
        Self::new(Who::Source)
    }

    pub fn target() -> Self {
        Self::new(Who::Target)
    }

    pub fn owner() -> Self {
        Self::new(Who::Effect)
    }
}

impl<C: Component> Condition for TagCondition<C> {
    fn eval(&self, context: &ConditionContext) -> bool {
        self.target.resolve_entity(context).contains::<C>()
    }
}

pub struct AbilityCondition {
    asset: AssetId<AbilityDef>,
}

impl AbilityCondition {
    pub fn new(asset: AssetId<AbilityDef>) -> Self {
        Self { asset }
    }
}

impl Condition for AbilityCondition {
    fn eval(&self, context: &ConditionContext) -> bool {
        context
            .owner
            .get::<Ability>()
            .map(|ability| ability.0.id() == self.asset)
            .unwrap_or(false)
    }
}

pub trait ConditionExt: Condition + Sized {
    fn and<C: Condition>(self, other: C) -> And<Self, C> {
        And {
            c1: self,
            c2: other,
        }
    }

    fn or<C: Condition>(self, other: C) -> Or<Self, C> {
        Or {
            c1: self,
            c2: other,
        }
    }

    fn not(self) -> Not<Self> {
        Not(self)
    }
}

impl<T: Condition> ConditionExt for T {}

#[cfg(test)]
mod tests {}
