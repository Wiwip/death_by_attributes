pub mod attribute;
pub mod float;
pub mod integer;
mod math;

use crate::attributes::Value;
use crate::condition::EvalContext;
use crate::expression::float::{AsFloat, FloatExprNode};
use crate::expression::integer::IntExprNode;
use bevy::prelude::*;
use num_traits::{Num, PrimInt};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
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
struct CastOp<S: ExprNode, P> {
    inner: Expr<S>,
    _phantom: PhantomData<P>,
}

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
    Rem,
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

impl_select_expr!(IntExprNode: i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
impl_select_expr!(FloatExprNode: f32, f64);

#[macro_export]
macro_rules! impl_math_expr {
    // Inner rule for a single implementation
    (@impl $x:ty, $select:ident) => {
        impl std::ops::Add for Expr<$select<$x>> {
            type Output = Expr<$select<$x>>;

            fn add(self, rhs: Self) -> Self::Output {
                Expr(Arc::new(SelectExprNode::<$x>::BinaryOp {
                    lhs: self,
                    op: BinaryOp::Add,
                    rhs,
                }))
            }
        }

        impl std::ops::Sub for Expr<$select<$x>> {
            type Output = Expr<$select<$x>>;

            fn sub(self, rhs: Self) -> Self::Output {
                Expr(Arc::new(SelectExprNode::<$x>::BinaryOp {
                    lhs: self,
                    op: BinaryOp::Sub,
                    rhs,
                }))
            }
        }

        impl std::ops::Mul for Expr<$select<$x>> {
            type Output = Expr<$select<$x>>;

            fn mul(self, rhs: Self) -> Self::Output {
                Expr(Arc::new(SelectExprNode::<$x>::BinaryOp {
                    lhs: self,
                    op: BinaryOp::Mul,
                    rhs,
                }))
            }
        }

        impl std::ops::Div for Expr<$select<$x>> {
            type Output = Expr<$select<$x>>;

            fn div(self, rhs: Self) -> Self::Output {
                Expr(Arc::new(SelectExprNode::<$x>::BinaryOp {
                    lhs: self,
                    op: BinaryOp::Div,
                    rhs,
                }))
            }
        }

        impl std::ops::Rem for Expr<$select<$x>> {
            type Output = Expr<$select<$x>>;

            fn rem(self, rhs: Self) -> Self::Output {
                Expr(Arc::new(SelectExprNode::<$x>::BinaryOp {
                    lhs: self,
                    op: BinaryOp::Rem,
                    rhs,
                }))
            }
        }
    };
    // Batch rule for multiple types mapping to the same Node
    ($select:ident: $($x:ty),+ $(,)?) => {
        $(
            $crate::impl_math_expr!(@impl $x, $select);
        )+
    };
}

impl_math_expr!(IntExprNode: i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
impl_math_expr!(FloatExprNode: f32, f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::prelude::*;
    use crate::{AttributesRef, attribute};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    attribute!(LhsTest, f32);
    attribute!(RhsTest, f32);

    const LHS_TEST_VAL: f32 = 100.0;
    const RHS_TEST_VAL: f32 = 10.0;

    #[test]
    fn test_math_expr() {
        let mut world = World::new();

        world.spawn((LhsTest::new(LHS_TEST_VAL), RhsTest::new(RHS_TEST_VAL)));

        world
            .run_system_once(|actor: Single<AttributesRef>| {
                let ctx = EvalContext {
                    source_actor: &actor,
                    target_actor: &actor,
                    owner: &actor,
                };

                let result = (LhsTest::src() + RhsTest::src()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL + RHS_TEST_VAL, result);

                let result = (LhsTest::src() - RhsTest::src()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL - RHS_TEST_VAL, result);

                let result = (LhsTest::src() * RhsTest::src()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL * RHS_TEST_VAL, result);

                let result = (LhsTest::src() / RhsTest::src()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL / RHS_TEST_VAL, result);

                let result = (LhsTest::src() % RhsTest::src()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL % RHS_TEST_VAL, result);
            })
            .unwrap();
    }
}
