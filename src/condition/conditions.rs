use crate::ability::Ability;
use crate::assets::AbilityDef;
use crate::attributes::Attribute;
use crate::condition::{Condition, EvalContext};
use crate::effect::Stacks;
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use bevy::asset::AssetId;
use bevy::log::error;
use bevy::prelude::{BevyError, Component, TypePath};
use serde::Serialize;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

pub type StackCondition = AttributeCondition<Stacks>;

#[derive(TypePath)]
pub struct AttributeCondition<T: Attribute> {
    who: Who,
    bounds: (Bound<T::Property>, Bound<T::Property>),
}

impl<T: Attribute> AttributeCondition<T> {
    pub fn new(range: impl RangeBounds<T::Property>, who: Who) -> Self {
        Self {
            who,
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
        }
    }

    pub fn target(range: impl RangeBounds<T::Property> + Send + Sync + 'static) -> Self {
        AttributeCondition::<T>::new(range, Who::Target)
    }

    pub fn source(range: impl RangeBounds<T::Property> + Send + Sync + 'static) -> Self {
        AttributeCondition::<T>::new(range, Who::Source)
    }
}

impl<T: Attribute> std::fmt::Debug for AttributeCondition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Attribute {} on {:?} in range {:?}",
            pretty_type_name::<T>(),
            self.who,
            self.bounds
        )
    }
}

impl<T: Attribute> Condition for AttributeCondition<T> {
    fn eval(&self, context: &EvalContext) -> Result<bool, BevyError> {
        let attributes = context.attribute_ref(self.who);

        match attributes.get::<T>() {
            Some(value) => Ok(self.bounds.contains(&value.current_value())),
            None => {
                error!("Error evaluating attribute condition:{}", self);
                Ok(false)
            }
        }
    }
}

impl<T: Attribute> std::fmt::Display for AttributeCondition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (start, end) = &self.bounds;

        let start_str = match start {
            Bound::Included(v) => format!("[{v}"),
            Bound::Excluded(v) => format!("]{v}"),
            Bound::Unbounded => "(-∞".to_string(),
        };

        let end_str = match end {
            Bound::Included(v) => format!("{v}]"),
            Bound::Excluded(v) => format!("{v}["),
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

#[derive(Serialize)]
pub struct ChanceCondition(pub f32);

impl Condition for ChanceCondition {
    fn eval(&self, _: &EvalContext) -> Result<bool, BevyError> {
        Ok(rand::random::<f32>() < self.0)
    }
}

impl std::fmt::Debug for ChanceCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chance: {:.3}", self.0)
    }
}

#[derive(Debug, Serialize)]
pub struct And<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for And<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn eval(&self, value: &EvalContext) -> Result<bool, BevyError> {
        Ok(self.c1.eval(value)? && self.c2.eval(value)?)
    }
}

#[derive(Debug, Serialize)]
pub struct Or<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for Or<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn eval(&self, context: &EvalContext) -> Result<bool, BevyError> {
        Ok(self.c1.eval(context)? || self.c2.eval(context)?)
    }
}

#[derive(Debug, Serialize)]
pub struct Not<C>(C);

impl<C: Condition> Condition for Not<C> {
    fn eval(&self, context: &EvalContext) -> Result<bool, BevyError> {
        Ok(!self.0.eval(context)?)
    }
}

#[derive(Serialize)]
pub struct TagCondition<C: Component> {
    who: Who,
    phantom_data: PhantomData<C>,
}

impl<C: Component> TagCondition<C> {
    pub fn new(target: Who) -> Self {
        Self {
            who: target,
            phantom_data: PhantomData,
        }
    }

    pub fn source() -> Self {
        Self::new(Who::Source)
    }

    pub fn target() -> Self {
        Self::new(Who::Target)
    }

    pub fn effect() -> Self {
        Self::new(Who::Owner)
    }
}

impl<C: Component> Condition for TagCondition<C> {
    fn eval(&self, context: &EvalContext) -> Result<bool, BevyError> {
        let entity = context.attribute_ref(self.who);
        Ok(entity.contains::<C>())
    }
}

impl<C: Component> std::fmt::Debug for TagCondition<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Has Tag {} on {}", pretty_type_name::<C>(), self.who)
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
    fn eval(&self, context: &EvalContext) -> Result<bool, BevyError> {
        Ok(context
            .owner
            .get::<Ability>()
            .map(|ability| ability.0.id() == self.asset)
            .unwrap_or(false))
    }
}

impl std::fmt::Debug for AbilityCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Is Ability {}", self.asset)
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
