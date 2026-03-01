use crate::assets::AbilityDef;
use crate::attributes::Attribute;
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use bevy::asset::AssetId;
use bevy::prelude::{Component, TypePath};
use bevy::reflect::Reflect;
use express_it::context::{Path, ReadContext};
use express_it::expr::{Expr, ExprNode, ExpressionError};
use express_it::logic::{BoolExpr, BoolExprNode};
use serde::Serialize;
use std::any::{TypeId};
use std::collections::HashSet;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

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
    fn eval(&self, ctx: &dyn ReadContext) -> Result<bool, ExpressionError> {
        let path = Path::from_id(self.who, T::ID);
        let any = ctx.get_any(&path)?;
        let value = any.downcast_ref::<T::Property>().unwrap();
        
        Ok(self.bounds.contains(&value))
    }

    fn get_dependencies(&self, _deps: &mut HashSet<Path>) {
        todo!()
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
    fn eval(&self, _: &dyn ReadContext) -> Result<bool, ExpressionError> {
        Ok(rand::random::<f32>() < self.0)
    }

    fn get_dependencies(&self, _deps: &mut HashSet<Path>) {
        todo!()
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
    fn eval(&self, ctx: &dyn ReadContext) -> Result<bool, ExpressionError> {
        let any = ctx.get_any_component(Who::Owner.into(), TypeId::of::<C>());
        Ok(any.is_ok())
    }

    fn get_dependencies(&self, _deps: &mut HashSet<Path>) {
        todo!()
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
    fn eval(&self, _context: &dyn ReadContext) -> Result<bool, ExpressionError> {
        /*Ok(context
        .get_any()
        .get::<Ability>()
        .map(|ability| ability.0.id() == self.asset)
        .unwrap_or(false))*/
        unimplemented!()
    }

    fn get_dependencies(&self, _deps: &mut HashSet<Path>) {
        todo!()
    }
}

impl std::fmt::Debug for AbilityCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Is Ability {}", self.asset)
    }
}
