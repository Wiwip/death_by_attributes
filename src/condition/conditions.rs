use crate::assets::AbilityDef;
use crate::attributes::Attribute;
use crate::effect::Stacks;
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use bevy::asset::AssetId;
use bevy::prelude::{Component, TypePath};
use bevy::reflect::Reflect;
use express_it::context::{EvalContext, Path};
use express_it::expr::{Expr, ExprNode, ExpressionError};
use express_it::logic::{BoolExpr, BoolExprNode};
use serde::Serialize;
use std::any::TypeId;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

pub type StackCondition = IsAttributeWithinBounds<Stacks>;

#[derive(TypePath)]
pub struct IsAttributeWithinBounds<T: Attribute> {
    who: Who,
    bounds: (Bound<T::Property>, Bound<T::Property>),
}

impl<T: Attribute> IsAttributeWithinBounds<T> {
    pub fn new(range: impl RangeBounds<T::Property>, who: Who) -> Self {
        Self {
            who,
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
        }
    }

    pub fn target(range: impl RangeBounds<T::Property> + Send + Sync + 'static) -> Self {
        IsAttributeWithinBounds::<T>::new(range, Who::Target)
    }

    pub fn source(range: impl RangeBounds<T::Property> + Send + Sync + 'static) -> Self {
        IsAttributeWithinBounds::<T>::new(range, Who::Source)
    }
}

impl<T: Attribute> std::fmt::Debug for IsAttributeWithinBounds<T> {
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

impl<T: Attribute> ExprNode<bool> for IsAttributeWithinBounds<T> {
    fn eval(&self, ctx: &dyn EvalContext) -> Result<bool, ExpressionError> {
        let path = match self.who {
            Who::Target => Path("dst".into()),
            Who::Source => Path("src".into()),
            Who::Owner => Path("parent".into()),
        };
        let any = ctx.get_any(&path, TypeId::of::<T>())?;
        let attribute = any.downcast_ref::<T>().unwrap();

        Ok(self.bounds.contains(&attribute.current_value()))
    }
}

impl<T: Attribute> Into<BoolExpr> for IsAttributeWithinBounds<T> {
    fn into(self) -> BoolExpr {
        let node = BoolExprNode::Boxed(Box::new(self));
        Expr::new(Arc::new(node))
    }
}

impl<T: Attribute> std::fmt::Display for IsAttributeWithinBounds<T> {
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

impl ExprNode<bool> for ChanceCondition {
    fn eval(&self, _: &dyn EvalContext) -> Result<bool, ExpressionError> {
        Ok(rand::random::<f32>() < self.0)
    }
}

impl std::fmt::Debug for ChanceCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chance: {:.3}", self.0)
    }
}

#[derive(Serialize)]
pub struct HasComponent<C: Component> {
    who: Who,
    phantom_data: PhantomData<C>,
}

impl<C: Component> HasComponent<C> {
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

impl<C: Component + Reflect> ExprNode<bool> for HasComponent<C> {
    fn eval(&self, ctx: &dyn EvalContext) -> Result<bool, ExpressionError> {
        let any = ctx.get_any(&Path("parent".into()), TypeId::of::<C>());
        Ok(any.is_ok())
    }
}

impl<C: Component> std::fmt::Debug for HasComponent<C> {
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

impl ExprNode<bool> for AbilityCondition {
    fn eval(&self, context: &dyn EvalContext) -> Result<bool, ExpressionError> {
        /*Ok(context
        .get_any()
        .get::<Ability>()
        .map(|ability| ability.0.id() == self.asset)
        .unwrap_or(false))*/
        unimplemented!()
    }
}

impl std::fmt::Debug for AbilityCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Is Ability {}", self.asset)
    }
}
