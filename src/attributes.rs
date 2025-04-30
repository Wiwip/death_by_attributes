use crate::{AttributeEvaluationError, Editable, AttributeEntityMut};
use bevy::animation::AnimationEvaluationError;
use bevy::ecs::component::Mutable;
use bevy::platform::hash::Hashed;
use bevy::prelude::{Component, EvaluatorId, Reflect};
use bevy::reflect::{TypeInfo, Typed};
use std::any::TypeId;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Default, Reflect, Debug, Clone)]
pub struct AttributeDef {
    pub base_value: f32,
    pub current_value: f32,
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(Component, Attribute, Clone, Reflect, Deref, DerefMut, Debug)]
        pub struct $StructName {
            pub attribute: AttributeDef,
        }
    };
}

#[macro_export]
macro_rules! attribute_field {
    ($component:ident) => {
        AttributeRef::new_unchecked(|component: &mut $component| &mut component.attribute);
    };
}

pub trait EditableAttribute: Send + Sync + 'static {
    type Property: Editable;

    fn get_mut<'a>(
        &self,
        entity: &'a mut AttributeEntityMut,
    ) -> Result<&'a mut Self::Property, AnimationEvaluationError>;
    fn evaluator_id(&self) -> Hashed<(TypeId, usize)>;
}
