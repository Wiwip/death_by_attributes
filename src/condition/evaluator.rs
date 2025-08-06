
use crate::attributes::Attribute;
use bevy::prelude::*;
use std::marker::PhantomData;
use crate::AttributesRef;

pub trait Extractor: Send + Sync + 'static {
    fn extract_value(&self, entity: &AttributesRef) -> Result<f64, BevyError>;
    fn name(&self) -> &str;
}

#[derive(TypePath)]
pub struct BoxExtractor(pub Box<dyn Extractor>);

impl BoxExtractor {
    pub fn new<T: Extractor + 'static>(evaluator: T) -> Self {
        Self(Box::new(evaluator))
    }
}

pub struct AttributeExtractor<A: Attribute> {
    phantom_data: PhantomData<A>,
}

impl<A: Attribute> AttributeExtractor<A> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

impl<A: Attribute> Extractor for AttributeExtractor<A> {
    fn extract_value(&self, entity: &AttributesRef) -> Result<f64, BevyError> {
        Ok(entity
            .get::<A>()
            .ok_or("Attribute not found")?
            .current_value())
    }

    fn name(&self) -> &'static str {
        A::type_path()
    }

}
