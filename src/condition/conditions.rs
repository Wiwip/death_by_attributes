use crate::ability::Ability;
use crate::assets::AbilityDef;
use crate::attributes::{AccessAttribute, Attribute, AttributeExtractor, BoxAttributeAccessor};
use crate::condition::{Condition, ConditionContext};
use crate::effect::Stacks;
use crate::modifier::Who;
use bevy::asset::AssetId;
use bevy::log::error;
use bevy::prelude::{Component, TypePath};
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

#[derive(TypePath)]
pub struct AttributeCondition {
    who: Who,
    extractor: BoxAttributeAccessor,
    bounds: (Bound<f64>, Bound<f64>),
}

impl AttributeCondition {
    pub fn new<'a, A: Attribute>(
        range: impl RangeBounds<f64> + Send + Sync + 'static,
        who: Who,
    ) -> Self {
        Self {
            who,
            extractor: BoxAttributeAccessor::new(AttributeExtractor::<A>::new()),
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
        }
    }

    pub fn target<A: Attribute>(range: impl RangeBounds<f64> + Send + Sync + 'static) -> Self {
        Self::new::<A>(range, Who::Target)
    }

    pub fn source<A: Attribute>(range: impl RangeBounds<f64> + Send + Sync + 'static) -> Self {
        Self::new::<A>(range, Who::Source)
    }
}

impl Condition for AttributeCondition {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        let entity = self.who.get_entity(context);

        match self.extractor.0.current_value(entity) {
            Ok(value) => self.bounds.contains(&value),
            Err(e) => {
                error!("Error evaluating attribute condition: {:?}", e);
                false
            }
        }
    }
}

impl std::fmt::Display for AttributeCondition {
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
            self.extractor.0.name(),
            self.who,
            start_str,
            end_str
        )
    }
}

#[derive(Clone)]
pub struct StackCondition {
    pub bounds: (Bound<u32>, Bound<u32>),
}

impl StackCondition {
    pub fn new(range: impl RangeBounds<u32> + Send + Sync + 'static) -> Self {
        Self {
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
        }
    }
}

impl std::fmt::Display for StackCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "StackCondition with bounds: {:?}", self.bounds)
    }
}

impl Condition for StackCondition {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        match context.owner.get::<Stacks>() {
            Some(value) => self.bounds.contains(&(value.current_value() as u32)),
            None => {
                error!(
                    "Effect {}: StackCondition requires a Stacks component.",
                    context.owner.id()
                );
                false
            }
        }
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
    fn evaluate(&self, value: &ConditionContext) -> bool {
        self.c1.evaluate(value) && self.c2.evaluate(value)
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
    fn evaluate(&self, context: &ConditionContext) -> bool {
        self.c1.evaluate(context) || self.c2.evaluate(context)
    }
}

pub struct Not<C>(C);

impl<C: Condition> Condition for Not<C> {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        !self.0.evaluate(context)
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
    fn evaluate(&self, context: &ConditionContext) -> bool {
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
    fn evaluate(&self, context: &ConditionContext) -> bool {
        self.target.get_entity(context).contains::<C>()
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
    fn evaluate(&self, context: &ConditionContext) -> bool {
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
