use crate::attributes::Attribute;
use crate::condition::EvalContext;
use crate::expression::ExpressionError;
use num_traits::Num;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait RetrieveAttribute<P: Num>: Debug + Send + Sync {
    fn retrieve(&self, context: &EvalContext) -> Result<P, ExpressionError>;
}

#[derive(Debug, Clone, Copy)]
pub struct Src<T: Attribute>(PhantomData<T>);

impl<T: Attribute> RetrieveAttribute<T::Property> for Src<T> {
    fn retrieve(&self, context: &EvalContext) -> Result<T::Property, ExpressionError> {
        Ok(context
            .source_actor
            .get::<T>()
            .ok_or(ExpressionError::AttributeNotFound)?
            .current_value())
    }
}

pub fn src<T: Attribute>() -> Src<T> {
    Src(PhantomData)
}

#[derive(Debug, Clone, Copy)]
pub struct Dst<T: Attribute>(PhantomData<T>);

impl<T: Attribute> RetrieveAttribute<T::Property> for Dst<T> {
    fn retrieve(&self, context: &EvalContext) -> Result<T::Property, ExpressionError> {
        Ok(context
            .target_actor
            .get::<T>()
            .ok_or(ExpressionError::AttributeNotFound)?
            .current_value())
    }
}

pub fn dst<T: Attribute>() -> Dst<T> {
    Dst(PhantomData)
}

#[derive(Debug, Clone, Copy)]
pub struct Parent<T: Attribute>(PhantomData<T>);

impl<T: Attribute> RetrieveAttribute<T::Property> for Parent<T> {
    fn retrieve(&self, context: &EvalContext) -> Result<T::Property, ExpressionError> {
        Ok(context
            .owner
            .get::<T>()
            .ok_or(ExpressionError::AttributeNotFound)?
            .current_value())
    }
}

pub fn parent<T: Attribute>() -> Parent<T> {
    Parent(PhantomData)
}
