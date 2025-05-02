use crate::{AttributeEntityMut, AttributeEntityRef, Editable};
use bevy::animation::AnimationEvaluationError;
use bevy::ecs::component::{Mutable};
use bevy::platform::hash::Hashed;
use bevy::prelude::{Component, Reflect};
use bevy::reflect::{TypeInfo, Typed};
use std::any::TypeId;
use std::fmt::Debug;
use std::marker::PhantomData;

pub type GameAttribute = Box<dyn AttributeAccessorMut<Property = AttributeDef>>;

#[derive(Default, Reflect, Debug, Clone)]
pub struct AttributeDef {
    pub base_value: f32,
    pub current_value: f32,
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(Component, Attribute, Default, Clone, Reflect, Deref, DerefMut, Debug)]
        #[require(GameEffectContainer, GameAbilityContainer)]
        pub struct $StructName {
            pub attribute: AttributeDef,
        }
    };

    ( $StructName:ident, $($RequireStruct:ident),* ) => {
        #[derive(Component, Attribute, Default, Clone, Reflect, Deref, DerefMut, Debug)]
        #[require(GameEffectContainer, GameAbilityContainer)]
        #[require($($RequireStruct),*)]
        pub struct $StructName {
            pub attribute: AttributeDef,
        }
    };
}

#[macro_export]
macro_rules! attribute_mut {
    ($component:ident) => {
        AttributeMut::new_unchecked(|component: &mut $component| &mut component.attribute)
    };
}

#[macro_export]
macro_rules! attribute_ref {
    ($component:ident) => {
        AttributeRef::new_unchecked(|component: &$component| &component.attribute)
    };
}

pub trait AttributeAccessorMut: Clone + Send + Sync + 'static {
    type Property: Editable;

    fn get_mut<'a>(
        &self,
        entity: &'a mut AttributeEntityMut,
    ) -> Result<&'a mut Self::Property, AnimationEvaluationError>;

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)>;
}

pub trait AttributeAccessorRef: Send + Sync + 'static {
    type Property: Editable;

    fn get<'a>(
        &self,
        entity: &'a AttributeEntityRef,
    ) -> Result<&'a Self::Property, AnimationEvaluationError>;

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)>;
}

#[derive(Clone)]
pub struct AttributeMut<C, A, F: Fn(&mut C) -> &mut A> {
    func: F,
    marker: PhantomData<(C, A)>,
    evaluator_id: Hashed<(TypeId, usize)>,
}

impl<C: Typed, P, F: Fn(&mut C) -> &mut P + 'static> AttributeMut<C, P, F> {
    pub fn new_unchecked(func: F) -> Self {
        let field_index;
        if let TypeInfo::Struct(struct_info) = C::type_info() {
            field_index = struct_info
                .index_of("attribute")
                .expect("Field name should exist");
        } else if let TypeInfo::TupleStruct(struct_info) = C::type_info() {
            field_index = "attribute"
                .parse()
                .expect("Field name should be a valid tuple index");
            if field_index >= struct_info.field_len() {
                panic!("Field name should be a valid tuple index");
            }
        } else {
            panic!("Only structs are supported in `AnimatedField::new_unchecked`")
        }

        Self {
            func,
            marker: PhantomData,
            evaluator_id: Hashed::new((TypeId::of::<C>(), field_index)),
        }
    }
}

impl<C, A, F> AttributeAccessorMut for AttributeMut<C, A, F>
where
    C: Component<Mutability = Mutable> + std::clone::Clone,
    A: Editable + Clone + Sync + Debug,
    F: Fn(&mut C) -> &mut A + Send + Sync + 'static + std::clone::Clone,
{
    type Property = A;

    fn get_mut<'a>(
        &self,
        entity: &'a mut AttributeEntityMut,
    ) -> bevy::prelude::Result<&'a mut A, AnimationEvaluationError> {
        let c = entity
            .get_mut::<C>()
            .ok_or_else(|| AnimationEvaluationError::ComponentNotPresent(TypeId::of::<C>()))?;

        Ok((self.func)(c.into_inner()))
    }

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)> {
        self.evaluator_id
    }
}

#[derive(Clone)]
pub struct AttributeRef<C, A, F: Fn(&C) -> &A> {
    func: F,
    marker: PhantomData<(C, A)>,
    evaluator_id: Hashed<(TypeId, usize)>,
}

impl<C: Typed, P, F: Fn(&C) -> &P + 'static> AttributeRef<C, P, F> {
    pub fn new_unchecked(func: F) -> Self {
        let field_index;
        if let TypeInfo::Struct(struct_info) = C::type_info() {
            field_index = struct_info
                .index_of("attribute")
                .expect("Field name should exist");
        } else if let TypeInfo::TupleStruct(struct_info) = C::type_info() {
            field_index = "attribute"
                .parse()
                .expect("Field name should be a valid tuple index");
            if field_index >= struct_info.field_len() {
                panic!("Field name should be a valid tuple index");
            }
        } else {
            panic!("Only structs are supported in `AnimatedField::new_unchecked`")
        }

        Self {
            func,
            marker: PhantomData,
            evaluator_id: Hashed::new((TypeId::of::<C>(), field_index)),
        }
    }
}

impl<C, A, F> AttributeAccessorRef for AttributeRef<C, A, F>
where
    C: Component<Mutability = Mutable>,
    A: Editable + Clone + Sync + Debug,
    F: Fn(&C) -> &A + Send + Sync + 'static,
{
    type Property = A;

    fn get<'a>(&self, entity: &'a AttributeEntityRef) -> Result<&'a A, AnimationEvaluationError> {
        let c = entity
            .get::<C>()
            .ok_or_else(|| AnimationEvaluationError::ComponentNotPresent(TypeId::of::<C>()))?;

        Ok((self.func)(c))
    }

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)> {
        self.evaluator_id
    }
}
