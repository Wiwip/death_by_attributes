use crate::ability::Ability;
use crate::assets::AbilityDef;
use crate::attributes::{Attribute, AttributeAccessor, AttributeExtractor};
use crate::condition::{Condition, ConditionContext, convert_bounds};
use crate::effect::Stacks;
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use bevy::asset::AssetId;
use bevy::log::error;
use bevy::prelude::{BevyError, Component, Deref, TypePath};
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
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        let entity = self.who.resolve_entity(context);

        let extractor = AttributeExtractor::<T>::new();
        match extractor.current_value(entity) {
            Ok(value) => Ok(self.bounds.contains(&value)),
            Err(e) => {
                error!("Error evaluating attribute condition: {:?}", e);
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

pub struct And<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for And<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn eval(&self, value: &ConditionContext) -> Result<bool, BevyError> {
        Ok(self.c1.eval(value)? && self.c2.eval(value)?)
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
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        Ok(self.c1.eval(context)? || self.c2.eval(context)?)
    }
}

pub struct Not<C>(C);

impl<C: Condition> Condition for Not<C> {
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        Ok(!self.0.eval(context)?)
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
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        Ok(self.target.resolve_entity(context).contains::<C>())
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
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        Ok(context
            .owner
            .get::<Ability>()
            .map(|ability| ability.0.id() == self.asset)
            .unwrap_or(false))
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

/// A condition that wraps a closure or function pointer.
///
/// This allows for creating custom, inline condition logic without needing
/// to define a new struct for every case.
#[derive(Debug, Serialize)]
pub struct FunctionCondition<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

pub trait EffectParam: Send + Sync {
    type Item<'new>;

    fn retrieve<'r>(context: &'r ConditionContext) -> Self::Item<'r>;
}

#[derive(Deref)]
struct Dst<'a, T: 'static> {
    value: &'a T,
}

impl<'res, T: 'static + Component> EffectParam for Dst<'res, T> {
    type Item<'new> = Dst<'new, T>;

    fn retrieve<'r>(context: &'r ConditionContext) -> Self::Item<'r> {
        Dst {
            value: context
                .target_actor
                .get::<T>()
                .expect("Missing target attribute"),
        }
    }
}

#[derive(Deref)]
struct Src<'a, T: 'static> {
    value: &'a T,
}

impl<'res, T: 'static + Component> EffectParam for Src<'res, T> {
    type Item<'new> = Src<'new, T>;

    fn retrieve<'r>(context: &'r ConditionContext) -> Self::Item<'r> {
        Src {
            value: context
                .source_actor
                .get::<T>()
                .expect("Missing source attribute"),
        }
    }
}

impl<F: Send + Sync, T1: EffectParam> Condition for FunctionCondition<(T1,), F>
where
    for<'a, 'b> &'a F: Fn(T1) -> Result<bool, BevyError>
        + Fn(<T1 as EffectParam>::Item<'b>) -> Result<bool, BevyError>,
{
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        fn call_inner<T1>(
            f: impl Fn(T1) -> Result<bool, BevyError>,
            _0: T1,
        ) -> Result<bool, BevyError> {
            f(_0)
        }

        let _0 = T1::retrieve(context);
        call_inner(&self.f, _0)
    }
}

pub trait IntoGameplayCondition<Input> {
    type ExecFunction: Condition;

    fn into_condition(self) -> Self::ExecFunction;
}

impl<F: Fn(T1) -> Result<bool, BevyError> + Send + Sync, T1: EffectParam>
    IntoGameplayCondition<(T1,)> for F
where
    for<'a, 'b> &'a F: Fn(T1) -> Result<bool, BevyError>
        + Fn(<T1 as EffectParam>::Item<'b>) -> Result<bool, BevyError>,
{
    type ExecFunction = FunctionCondition<(T1,), Self>;

    fn into_condition(self) -> Self::ExecFunction {
        FunctionCondition {
            f: self,
            marker: PhantomData,
        }
    }
}

