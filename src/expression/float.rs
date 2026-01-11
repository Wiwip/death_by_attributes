use crate::condition::EvalContext;
use crate::expression::integer::IntExprNode;
use crate::expression::{BinaryOp, Castable, ExprNode, ExpressionError, UnaryOp};
use crate::prelude::{Expr, RetrieveAttribute};
use num_traits::real::Real;
use num_traits::{AsPrimitive, Float, Num, PrimInt};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

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
            FloatExprNode::Cast(cast) => Ok(cast.eval_cast(ctx)?),
            FloatExprNode::UnaryOp { op, expr } => {
                let value = expr.eval(ctx)?;
                match op {
                    UnaryOp::Sin => Ok(value.sin()),
                    UnaryOp::Asin => Ok(value.asin()),
                    UnaryOp::Cos => Ok(value.cos()),
                    UnaryOp::Acos => Ok(value.acos()),
                }
            }
            FloatExprNode::BinaryOp { lhs, op, rhs } => {
                let l = lhs.eval(ctx)?;
                let r = rhs.eval(ctx)?;
                match op {
                    BinaryOp::Add => Ok(l + r),
                    BinaryOp::Sub => Ok(l - r),
                    BinaryOp::Mul => Ok(l * r),
                    BinaryOp::Div => Ok(l / r),
                    BinaryOp::Rem => Ok(l % r),
                }
            }
        }
    }
}

pub struct AsFloat<P, S: Send + Sync + PrimInt> {
    inner: Expr<IntExprNode<S>>,
    phantom: PhantomData<P>,
}

impl<P, S> AsFloat<P, S>
where
    S: Send + Sync + PrimInt,
{
    pub fn new(inner: Expr<IntExprNode<S>>) -> Self {
        Self {
            inner,
            phantom: Default::default(),
        }
    }
}

impl<P, S> Debug for AsFloat<P, S>
where
    S: Send + Sync + PrimInt,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<P, S> Castable<P> for AsFloat<P, S>
where
    P: Num + Copy + Send + Sync + 'static,
    S: Send + Sync + PrimInt + AsPrimitive<P>,
{
    fn eval_cast(&self, ctx: &EvalContext) -> bevy::prelude::Result<P, ExpressionError> {
        Ok(self.inner.eval(ctx)?.as_())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::expression::CastOp;
    use crate::prelude::*;
    use crate::{AttributesRef, attribute};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    attribute!(LhsTest, f32);
    attribute!(RhsTest, u32);

    const LHS_TEST_VAL: f32 = 100.0;
    const RHS_TEST_VAL: u32 = 10;

    #[test]
    fn test_as_float_expr() {
        let mut world = World::new();

        world.spawn((LhsTest::new(LHS_TEST_VAL), RhsTest::new(RHS_TEST_VAL)));

        world
            .run_system_once(|actor: Single<AttributesRef>| {
                let ctx = EvalContext {
                    source_actor: &actor,
                    target_actor: &actor,
                    owner: &actor,
                };

                let as_float = RhsTest::src().as_::<f32>();
                let result = as_float.eval(&ctx).unwrap();


                assert_eq!(result, RHS_TEST_VAL as f32);

                let result = (LhsTest::src() + RhsTest::src().as_()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL + RHS_TEST_VAL as f32, result);

                let result = (LhsTest::src() - RhsTest::src().as_()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL - RHS_TEST_VAL as f32, result);

                let result = (LhsTest::src() * RhsTest::src().as_()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL * RHS_TEST_VAL as f32, result);

                let result = (LhsTest::src() / RhsTest::src().as_()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL / RHS_TEST_VAL as f32, result);

                let result = (LhsTest::src() % RhsTest::src().as_()).eval(&ctx).unwrap();
                assert_eq!(LHS_TEST_VAL % RHS_TEST_VAL as f32, result);
            })
            .unwrap();
    }
}
