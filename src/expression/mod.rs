pub mod attribute;
mod math;

use crate::condition::EvalContext;
use crate::prelude::RetrieveAttribute;
use bevy::prelude::*;
use num_traits::{Float, Num, PrimInt};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub trait ExprNode: Send + Sync {
    type Output;
    fn eval(&self, ctx: &EvalContext) -> Result<Self::Output, ExpressionError>;
}

#[derive(Default, Deref, Debug, Clone)]
pub struct Expr<N: ExprNode>(pub Arc<N>);

impl<N: ExprNode> Expr<N> {
    pub fn eval(&self, ctx: &EvalContext) -> Result<N::Output, ExpressionError> {
        self.0.eval(ctx)
    }
}

/*impl<P: Value> Expr<P> {
    pub fn cast<T: Value>(self) -> Expr<T>
    where
        P: Into<T> + Debug + Send + Sync,
    {
        Expr(Arc::new(FloatExprNode::Cast(Box::new(CastOp {
            inner: self,
            _phantom: PhantomData,
        }))))
    }

    pub fn eval(&self, ctx: &EvalContext) -> Result<P, ExpressionError> {
        match self.0.as_ref() {
            FloatExprNode::None => Err(ExpressionError::EmptyExpr),
            FloatExprNode::Lit(n) => Ok(*n),
            FloatExprNode::Attribute(attr) => attr.retrieve(ctx),
            FloatExprNode::UnaryOp { op, expr } => {
                let value: f64 = expr.eval(ctx)?.as_();
                let out = match op {
                    UnaryOp::Acos => value.acos(),
                    UnaryOp::Asin => value.asin(),
                    UnaryOp::Cos => value.cos(),
                    UnaryOp::Sin => value.sin(),
                };
                P::from(out).ok_or(ExpressionError::InvalidTypes)
            }
            FloatExprNode::BinaryOp { lhs, op, rhs } => {
                let left = lhs.eval(ctx)?;
                let right = rhs.eval(ctx)?;
                match op {
                    BinaryOp::Add => Ok(left + right),
                    BinaryOp::Sub => Ok(left - right),
                    BinaryOp::Mul => Ok(left * right),
                    BinaryOp::Div => Ok(left / right),
                    BinaryOp::Remainder => Ok(left % right),
                }
            }
            FloatExprNode::Cast(expr) => expr.eval_cast(ctx),
        }
    }

    pub fn lit(value: P) -> Expr<P> {
        Expr(Arc::new(FloatExprNode::Lit(value)))
    }
}*/

/*impl<P: Num +Debug> std::ops::Add for Expr<P> {
    type Output = Expr<P>;

    fn add(self, rhs: Expr<P>) -> Self::Output {
        Expr(Arc::new(FloatExprNode::BinaryOp {
            lhs: self,
            op: BinaryOp::Add,
            rhs,
        }))
    }
}*/

/*impl<P: Num + PrimInt + Debug> std::ops::Add for Expr<P> {
    type Output = Expr<P>;

    fn add(self, rhs: Expr<P>) -> Self::Output {
        Expr(Arc::new(IntExprNode::BinaryOp {
            lhs: self,
            op: BinaryOp::Add,
            rhs,
        }))
    }
}*/

/*impl<P: Num + Float + Debug> std::ops::Mul for Expr<P> {
    type Output = Expr<P>;

    fn mul(self, rhs: Expr<P>) -> Self::Output {
        Expr(Arc::new(FloatExprNode::BinaryOp {
            lhs: self,
            op: BinaryOp::Mul,
            rhs,
        }))
    }
}*/

/*impl std::ops::Mul<f32> for Expr<f32> {
    type Output = Expr<f32>;
    fn mul(self, rhs: f32) -> Self::Output {
        self * Expr(Arc::new(FloatExprNode::Lit(rhs)))
    }
}

impl std::ops::Mul<Expr<f32>> for f32 {
    type Output = Expr<f32>;
    fn mul(self, rhs: Expr<f32>) -> Self::Output {
        Expr(Arc::new(FloatExprNode::Lit(self))) * rhs
    }
}*/

#[derive(Default)]
pub enum FloatExprNode<P: Float + Send + Sync> {
    #[default]
    None,
    Lit(P),
    Attribute(Box<dyn RetrieveAttribute<P>>),
    Cast(Box<dyn Castable<P>>),
    UnaryOp {
        op: UnaryOp,
        expr: Expr<FloatExprNode<P>>,
    },
    BinaryOp {
        lhs: Expr<FloatExprNode<P>>,
        op: BinaryOp,
        rhs: Expr<FloatExprNode<P>>,
    },
}

impl<P: Float + Send + Sync> ExprNode for FloatExprNode<P> {
    type Output = P;

    fn eval(&self, ctx: &EvalContext) -> Result<Self::Output, ExpressionError> {
        match self {
            FloatExprNode::None => Err(ExpressionError::EmptyExpr),
            FloatExprNode::Lit(lit) => Ok(lit.clone()),
            FloatExprNode::Attribute(attribute) => Ok(attribute.retrieve(ctx)?),
            FloatExprNode::Cast(_) => {
                unimplemented!()
            }
            FloatExprNode::UnaryOp { op, expr } => match op {
                UnaryOp::Sin => unimplemented!(),
                UnaryOp::Acos => unimplemented!(),
                UnaryOp::Asin => unimplemented!(),
                UnaryOp::Cos => unimplemented!(),
            },
            FloatExprNode::BinaryOp { lhs, op, rhs } => {
                let l = lhs.eval(ctx)?;
                let r = rhs.eval(ctx)?;
                match op {
                    BinaryOp::Add => Ok(l + r),
                    BinaryOp::Sub => Ok(l - r),
                    BinaryOp::Mul => Ok(l * r),
                    BinaryOp::Div => Ok(l / r),
                    BinaryOp::Remainder => Ok(l % r),
                }
            }
        }
    }
}

#[derive(Default)]
pub enum IntExprNode<P: PrimInt + Send + Sync> {
    #[default]
    None,
    Lit(P),
    Attribute(Box<dyn RetrieveAttribute<P>>),
    Cast(Box<dyn Castable<P>>),
    UnaryOp {
        op: UnaryOp,
        expr: Expr<IntExprNode<P>>,
    },
    BinaryOp {
        lhs: Expr<IntExprNode<P>>,
        op: BinaryOp,
        rhs: Expr<IntExprNode<P>>,
    },
}

impl<P: PrimInt + Send + Sync> ExprNode for IntExprNode<P> {
    type Output = P;

    fn eval(&self, ctx: &EvalContext) -> Result<Self::Output, ExpressionError> {
        match self {
            IntExprNode::None => Err(ExpressionError::EmptyExpr),
            IntExprNode::Lit(lit) => Ok(lit.clone()),
            IntExprNode::Attribute(attribute) => Ok(attribute.retrieve(ctx)?),
            IntExprNode::Cast(_) => {
                unimplemented!()
            }
            IntExprNode::UnaryOp { op, expr } => match op {
                UnaryOp::Sin => unimplemented!(),
                UnaryOp::Acos => unimplemented!(),
                UnaryOp::Asin => unimplemented!(),
                UnaryOp::Cos => unimplemented!(),
            },
            IntExprNode::BinaryOp { lhs, op, rhs } => {
                let l = lhs.eval(ctx)?;
                let r = rhs.eval(ctx)?;
                match op {
                    BinaryOp::Add => Ok(l + r),
                    BinaryOp::Sub => Ok(l - r),
                    BinaryOp::Mul => Ok(l * r),
                    BinaryOp::Div => Ok(l / r),
                    BinaryOp::Remainder => Ok(l % r),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ExpressionError {
    AttributeNotFound,
    EmptyExpr,
    InvalidTypes,
}

impl Display for ExpressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionError::AttributeNotFound => {
                write!(
                    f,
                    "Attribute error: Failed to retrieve attribute from context"
                )
            }
            ExpressionError::EmptyExpr => {
                write!(f, "An Empty Expression was found.")
            }
            ExpressionError::InvalidTypes => {
                write!(f, "Invalid expression type.")
            }
        }
    }
}

impl Error for ExpressionError {}

/// New trait: Allows an expression of any source type 'S' to be evaluated as 'P'
pub trait Castable<P: Num>: Debug + Send + Sync {
    fn eval_cast(&self, ctx: &EvalContext) -> Result<P, ExpressionError>;
}

/// Implementation: Bridge between Source type (S) and Target type (P)
/*#[derive(Debug)]
struct CastOp<S: Value, P: Value> {
    inner: Expr<S>,
    _phantom: PhantomData<P>,
}

impl<S, P> Castable<P> for CastOp<S, P>
where
    S: Value + Into<P>,
    P: Value,
{
    fn eval_cast(&self, ctx: &EvalContext) -> Result<P, ExpressionError> {
        Ok(self.inner.eval(ctx)?.into())
    }
}*/

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Acos,
    Asin,
    Cos,
    Sin,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Remainder,
}

#[macro_export]
macro_rules! impl_into_expr {
    // Inner rule for a single implementation
    (@impl $x:ty, $node:ident) => {
        impl From<$x> for Expr<$node<$x>> {
            fn from(value: $x) -> Self {
                Expr(Arc::new($node::Lit(value)))
            }
        }
    };
    // Batch rule for multiple types mapping to the same Node
    ($node:ident: $($x:ty),+ $(,)?) => {
        $(
            $crate::impl_into_expr!(@impl $x, $node);
        )+
    };
}

impl_into_expr!(IntExprNode: i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
impl_into_expr!(FloatExprNode: f32, f64);

pub trait SelectExprNodeImpl {
    type Property;
    type Node: ExprNode<Output = Self::Property>;
}

pub type SelectExprNode<T> = <T as SelectExprNodeImpl>::Node;

#[macro_export]
macro_rules! impl_select_expr {
    // Inner rule for a single implementation
    (@impl $x:ty, $select:ident) => {
        impl SelectExprNodeImpl for $x {
            type Property = $x;
            type Node = $select<Self::Property>;
        }
    };
    // Batch rule for multiple types mapping to the same Node
    ($select:ident: $($x:ty),+ $(,)?) => {
        $(
            $crate::impl_select_expr!(@impl $x, $select);
        )+
    };
}

// Grouped declarations are cleaner and easier to maintain
impl_select_expr!(IntExprNode: i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
impl_select_expr!(FloatExprNode: f32, f64);

impl std::ops::Add for Expr<FloatExprNode<f32>> {
    type Output = Expr<FloatExprNode<f32>>;

    fn add(self, rhs: Self) -> Self::Output {
        Expr(Arc::new(SelectExprNode::<f32>::BinaryOp {
            lhs: self,
            op: BinaryOp::Add,
            rhs,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::prelude::*;
    use crate::{AttributesRef, attribute};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    attribute!(Test, f32);
    attribute!(TestF64, f64);

    #[test]
    fn test() {
        let mut world = World::new();

        world.spawn((Test::new(100.0), TestF64::new(50.0)));

        world
            .run_system_once(|actor: Single<AttributesRef>| {
                let a = Test::src();
                let b = Test::src();

                let c = a + b;

                let ctx = EvalContext {
                    source_actor: &actor,
                    target_actor: &actor,
                    owner: &actor,
                };

                let result = c.eval(&ctx).unwrap();

                println!("Result: {}", result)
            })
            .unwrap();
    }
}
