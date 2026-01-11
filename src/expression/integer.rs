use crate::condition::EvalContext;
use crate::expression::float::{AsFloat, FloatExprNode};
use crate::expression::{BinaryOp, Castable, Expr, ExprNode, ExpressionError, UnaryOp};
use crate::prelude::RetrieveAttribute;
use num_traits::{AsPrimitive, Float, Num, PrimInt};
use std::sync::Arc;

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

impl<S> Expr<IntExprNode<S>>
where
    S: Send + Sync + PrimInt + 'static,
{
    pub fn as_<P>(&self) -> Expr<FloatExprNode<P>>
    where
        P: Float + Send + Sync + 'static,
        S: AsPrimitive<P>,
    {
        let inner = self.0.clone();

        Expr(Arc::new(FloatExprNode::Cast(Box::new(AsFloat::new(Expr(inner))))))
    }
}

impl<P: PrimInt + Send + Sync> ExprNode for IntExprNode<P> {
    type Output = P;

    fn eval(&self, ctx: &EvalContext) -> Result<Self::Output, ExpressionError> {
        match self {
            IntExprNode::None => Err(ExpressionError::EmptyExpr),
            IntExprNode::Lit(lit) => Ok(lit.clone()),
            IntExprNode::Attribute(attribute) => Ok(attribute.retrieve(ctx)?),
            IntExprNode::Cast(cast) => Ok(cast.eval_cast(ctx)?),
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
                    BinaryOp::Rem => Ok(l % r),
                }
            }
        }
    }
}
