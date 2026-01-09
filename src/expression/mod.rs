mod attribute;
mod math;

use crate::condition::EvalContext;
use crate::prelude::*;
use num_traits::Num;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Default, Debug, Clone)]
pub struct Expr<P: Num>(pub Arc<ExprNode<P>>);

impl<P: Num + Debug + Copy + 'static> Expr<P> {
    pub fn cast<T: Num + Debug + Send + Sync + 'static>(self) -> Expr<T>
    where
        P: Into<T> + Debug + Send + Sync,
    {
        Expr(Arc::new(ExprNode::Cast(Box::new(CastOp {
            inner: self,
            _phantom: PhantomData,
        }))))
    }

    pub fn eval(&self, ctx: &EvalContext) -> Result<P, ExpressionError> {
        match self.0.as_ref() {
            ExprNode::None => Err(ExpressionError::NoneNode),
            ExprNode::Lit(n) => Ok(*n),
            ExprNode::Attribute(attr) => attr.retrieve(ctx),
            ExprNode::BinaryOp { lhs, op, rhs } => match op {
                BinaryOp::Add => Ok(lhs.eval(ctx)? + rhs.eval(ctx)?),
                BinaryOp::Mul => Ok(lhs.eval(ctx)? * rhs.eval(ctx)?),
            },
            ExprNode::Cast(expr) => expr.eval_cast(ctx),
            _ => todo!("Missing Node Implementation"),
        }
    }

    pub fn lit(value: P) -> Expr<P> {
        Expr(Arc::new(ExprNode::Lit(value)))
    }
}

#[derive(Default, Debug)]
pub enum ExprNode<P: Num> {
    #[default]
    None,
    Lit(P),
    Attribute(Box<dyn RetrieveAttribute<P>>),
    Cast(Box<dyn Castable<P>>),
    BinaryOp {
        lhs: Expr<P>,
        op: BinaryOp,
        rhs: Expr<P>,
    },
}

#[derive(Debug)]
pub enum ExpressionError {
    AttributeError,
    NoneNode,
}

impl Display for ExpressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionError::AttributeError => {
                write!(
                    f,
                    "Attribute error: Failed to retrieve attribute from context"
                )
            }
            ExpressionError::NoneNode => {
                write!(f, "A NoneNode was present.")
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
#[derive(Debug)]
struct CastOp<S: Num, P: Num + Debug> {
    inner: Expr<S>,
    _phantom: PhantomData<P>,
}

impl<S, P> Castable<P> for CastOp<S, P>
where
    S: Num + Debug + Copy + Into<P> + Send + Sync + 'static,
    P: Num + Debug + Send + Sync + 'static,
{
    fn eval_cast(&self, ctx: &EvalContext) -> Result<P, ExpressionError> {
        Ok(self.inner.eval(ctx)?.into())
    }
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Mul,
}

impl<P: Num + Debug> std::ops::Add for Expr<P> {
    type Output = Expr<P>;

    fn add(self, rhs: Expr<P>) -> Self::Output {
        Expr(Arc::new(ExprNode::BinaryOp {
            lhs: self,
            op: BinaryOp::Add,
            rhs,
        }))
    }
}

impl<P: Num + Debug> std::ops::Mul for Expr<P> {
    type Output = Expr<P>;

    fn mul(self, rhs: Expr<P>) -> Self::Output {
        Expr(Arc::new(ExprNode::BinaryOp {
            lhs: self,
            op: BinaryOp::Mul,
            rhs,
        }))
    }
}

impl std::ops::Mul<f32> for Expr<f32> {
    type Output = Expr<f32>;
    fn mul(self, rhs: f32) -> Self::Output {
        self * Expr(Arc::new(ExprNode::Lit(rhs)))
    }
}

impl std::ops::Mul<Expr<f32>> for f32 {
    type Output = Expr<f32>;
    fn mul(self, rhs: Expr<f32>) -> Self::Output {
        Expr(Arc::new(ExprNode::Lit(self))) * rhs
    }
}

pub trait RetrieveAttribute<P: Num>: Debug + Send + Sync {
    fn retrieve(&self, context: &EvalContext) -> Result<P, ExpressionError>;
}

#[derive(Debug)]
pub struct Src<T: Attribute>(PhantomData<T>);

impl<T: Attribute> RetrieveAttribute<T::Property> for Src<T> {
    fn retrieve(&self, context: &EvalContext) -> Result<T::Property, ExpressionError> {
        Ok(context
            .source_actor
            .get::<T>()
            .ok_or(ExpressionError::AttributeError)?
            .current_value())
    }
}

pub fn src<T: Attribute>() -> Src<T> {
    Src(PhantomData)
}

#[derive(Debug)]
pub struct Dst<T: Attribute>(PhantomData<T>);

impl<T: Attribute> RetrieveAttribute<T::Property> for Dst<T> {
    fn retrieve(&self, context: &EvalContext) -> Result<T::Property, ExpressionError> {
        Ok(context
            .target_actor
            .get::<T>()
            .ok_or(ExpressionError::AttributeError)?
            .current_value())
    }
}

pub fn dst<T: Attribute>() -> Dst<T> {
    Dst(PhantomData)
}

#[derive(Debug)]
pub struct Parent<T: Attribute>(PhantomData<T>);

impl<T: Attribute> RetrieveAttribute<T::Property> for Parent<T> {
    fn retrieve(&self, context: &EvalContext) -> Result<T::Property, ExpressionError> {
        Ok(context
            .owner
            .get::<T>()
            .ok_or(ExpressionError::AttributeError)?
            .current_value())
    }
}

pub fn parent<T: Attribute>() -> Parent<T> {
    Parent(PhantomData)
}

#[macro_export]
macro_rules! impl_into_expr {
    ( $x:ty ) => {
        impl From<$x> for Expr<$x> {
            fn from(value: $x) -> Self {
                Expr(Arc::new(ExprNode::Lit(value)))
            }
        }
    };
}

impl_into_expr!(i8);
impl_into_expr!(i16);
impl_into_expr!(i32);
impl_into_expr!(i64);
impl_into_expr!(i128);
impl_into_expr!(isize);

impl_into_expr!(u8);
impl_into_expr!(u16);
impl_into_expr!(u32);
impl_into_expr!(u64);
impl_into_expr!(u128);
impl_into_expr!(usize);

impl_into_expr!(f32);
impl_into_expr!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::{AttributesRef, attribute};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{Single, World};

    attribute!(Test, f32);
    attribute!(TestF64, f64);

    #[test]
    fn test() {
        let mut world = World::new();

        world.spawn((Test::new(100.0), TestF64::new(50.0)));

        world
            .run_system_once(|actor: Single<AttributesRef>| {
                /*let a = Test::value();
                let b = TestF64::value();

                //let _add_expr_inv = a + b;

                let a = AttributeValueExpr::<Test> {
                    who: Who::Target,
                    phantom_data: Default::default(),
                }
                .expr();
                let b = AttributeValueExpr::<Test> {
                    who: Who::Target,
                    phantom_data: Default::default(),
                }
                .expr();

                let add_expr = a + b;

                let context = GameplayContext {
                    target_actor: &actor,
                    source_actor: &actor,
                    owner: &actor,
                };

                let result = add_expr.eval(&context).unwrap();

                println!("Result: {}", result)*/
            })
            .unwrap();
    }
}
