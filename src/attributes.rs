use crate::condition::{convert_bounds, multiply_bounds};
use crate::effect::AttributeDependents;
use crate::inspector::pretty_type_name;
use crate::math::{AbsDiff, SaturatingAttributes};
use crate::prelude::*;
use crate::systems::MarkNodeDirty;
use crate::{AttributeError, AttributesMut, AttributesRef, CurrentValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
pub use num_traits::{
    AsPrimitive, Bounded, FromPrimitive, Num, NumAssign, NumAssignOps, NumOps, Saturating,
    SaturatingAdd, SaturatingMul, Zero,
};
use serde::Serialize;
use std::any::TypeId;
use std::collections::{Bound, HashSet};
use std::fmt::Display;
use std::fmt::{Debug, Formatter};
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::RangeBounds;
use std::sync::Arc;

pub trait Attribute
where
    Self: Component<Mutability = Mutable> + Copy + Clone + Debug + Display,
    Self: Reflect + TypePath + GetTypeRegistration,
    Self: Serialize,
{
    type Property: Num
        + NumOps
        + NumAssign
        + NumAssignOps
        + SaturatingAttributes<Output = Self::Property>
        + Sum
        + Bounded
        + AbsDiff
        + PartialOrd
        + FromPrimitive
        + AsPrimitive<f64>
        + Reflect
        + Copy
        + Clone
        + Debug
        + Display
        + Send
        + Serialize
        + Sync;

    fn new<T: Num + AsPrimitive<Self::Property> + Copy>(value: T) -> Self;
    fn base_value(&self) -> Self::Property;
    fn set_base_value(&mut self, value: Self::Property);
    fn current_value(&self) -> Self::Property;
    fn set_current_value(&mut self, value: Self::Property);
    fn value() -> AttributeValue<Self> {
        AttributeValue {
            value: Self::Property::zero(),
            phantom_data: PhantomData,
        }
    }
    fn attribute_type_id() -> AttributeTypeId;
}

#[macro_export]
macro_rules! attribute_impl {
    ( $StructName:ident, $ValueType:ty ) => {
        #[derive(
            bevy::prelude::Component,
            Clone,
            Copy,
            bevy::prelude::Reflect,
            Debug,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[reflect(AccessAttribute)]
        pub struct $StructName {
            base_value: $ValueType,
            current_value: $ValueType,
        }

        impl $crate::attributes::Attribute for $StructName {
            type Property = $ValueType;

            fn new<T>(value: T) -> Self
            where
                T: $crate::num_traits::Num + $crate::num_traits::AsPrimitive<Self::Property> + Copy,
            {
                Self {
                    base_value: value.as_(),
                    current_value: value.as_(),
                }
            }
            fn base_value(&self) -> $ValueType {
                self.base_value
            }
            fn set_base_value(&mut self, value: $ValueType) {
                self.base_value = value;
            }
            fn current_value(&self) -> $ValueType {
                self.current_value
            }
            fn set_current_value(&mut self, value: $ValueType) {
                self.current_value = value;
            }
            fn attribute_type_id() -> $crate::prelude::AttributeTypeId {
                $crate::prelude::AttributeTypeId::of::<Self>()
            }
        }

        impl std::fmt::Display for $StructName {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}: {}", stringify!($StructName), self.current_value)
            }
        }
    };
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident ) => {
        $crate::attribute_impl!($StructName, f32);
    };
    ( $StructName:ident, $ValueType:ty  ) => {
        $crate::attribute_impl!($StructName, $ValueType);
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct AttributeTypeId(pub u64);

impl AttributeTypeId {
    pub fn of<T: TypePath>() -> Self {
        let mut hasher = DefaultHasher::new();
        T::type_path().hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn from_reflect(reflect: &dyn Reflect) -> Self {
        let mut hasher = DefaultHasher::new();
        reflect.reflect_type_path().hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(QueryData, Debug)]
#[query_data(mutable, derive(Debug))]
pub struct AttributeQueryData<T: Attribute + 'static> {
    pub entity: Entity,
    pub attribute: &'static mut T,
    pub calculator_cache: &'static mut AttributeCalculatorCached<T>,
}

impl<T: Attribute> AttributeQueryDataItem<'_, '_, T> {
    pub fn update_attribute(&mut self, calculator: &AttributeCalculator<T>) -> bool {
        let old_val = self.attribute.current_value();
        let new_val = calculator.eval(self.attribute.base_value());

        let has_changed = old_val.are_different(new_val);
        if has_changed {
            self.attribute.set_current_value(new_val);
        }
        has_changed
    }

    pub fn update_attribute_from_cache(&mut self) -> bool {
        let old_val = self.attribute.current_value();
        let new_val = self
            .calculator_cache
            .calculator
            .eval(self.attribute.base_value());

        let has_changed = old_val.are_different(new_val);
        if has_changed {
            self.attribute.set_current_value(new_val);
        }
        has_changed
    }
}


#[reflect_trait] // Generates a `ReflectMyTrait` type
pub trait AccessAttribute {
    fn access_base_value(&self) -> f64;
    fn access_current_value(&self) -> f64;
    fn name(&self) -> String;
}

impl<T> AccessAttribute for T
where
    T: Attribute,
{
    fn access_base_value(&self) -> f64 {
        self.base_value().as_()
    }
    fn access_current_value(&self) -> f64 {
        self.current_value().as_()
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

pub trait ValueSource: Send + Sync + 'static {
    type Output: Num;

    fn value(&self, entity: &AttributesRef) -> Result<Self::Output, AttributeError>;
    fn insert_dependency(
        &self,
        target: Entity,
        entity_commands: &mut EntityCommands,
        func: fn(Entity, Commands),
    );
    fn describe(&self) -> String;
}

pub trait IntoValue {
    type Out: Num;

    fn into_value(self) -> Value<Self::Out>;
}

/// A [Value] refers to an Attribute value.
/// It can be a literal value, or a reference to an Attribute.
#[derive(Deref, DerefMut)]
pub struct Value<P: Num>(pub Arc<dyn ValueSource<Output = P>>);

impl<P: Num + Display + Debug + Copy + Clone + Send + Sync + 'static> Default for Value<P> {
    fn default() -> Self {
        Value(Arc::new(Lit(P::zero())))
    }
}

impl<P: Num + 'static> Clone for Value<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<P: Num + 'static> Debug for Value<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.describe())
    }
}

impl<P: Num + 'static> Display for Value<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.describe())
    }
}

/// An [AttributeValue] is a dynamic reference to an Attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttributeValue<T: Attribute> {
    pub value: T::Property,
    pub phantom_data: PhantomData<T>,
}

impl<T: Attribute> ValueSource for AttributeValue<T> {
    type Output = T::Property;

    fn value(&self, entity: &AttributesRef) -> Result<Self::Output, AttributeError> {
        Ok(entity
            .get::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .current_value())
    }

    /// Inserts a dependency on the target entity.
    /// This is used to ensure that the target entity is updated when the source attribute changes.
    /// The func serves as a trigger to MarkNodeDirty<T> on the attribute that must be recalculated
    fn insert_dependency(
        &self,
        target: Entity,
        entity_commands: &mut EntityCommands,
        func: fn(Entity, Commands),
    ) {
        entity_commands.insert(AttributeDependency::<T>::new(target));

        let mut observer = Observer::new(
            move |trigger: On<AttributeDependencyChanged<T>>, commands: Commands| {
                func(trigger.entity, commands);
            },
        );
        observer.watch_entity(entity_commands.id());
        entity_commands.commands().spawn(observer);
    }

    fn describe(&self) -> String {
        format!("{}", pretty_type_name::<T>())
    }
}

impl<T: Attribute> IntoValue for AttributeValue<T> {
    type Out = T::Property;

    fn into_value(self) -> Value<Self::Out> {
        Value(Arc::new(AttributeValue::<T> {
            value: Self::Out::zero(),
            phantom_data: Default::default(),
        }))
    }
}

/// A [Lit] is a static value.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit<P: Num>(pub P);

impl<P: Num + Display + Debug + Copy + Clone + Send + Sync + 'static> ValueSource for Lit<P> {
    type Output = P;

    fn value(&self, _: &AttributesRef) -> Result<Self::Output, AttributeError> {
        Ok(self.0)
    }

    fn insert_dependency(
        &self,
        _target: Entity,
        _entity_commands: &mut EntityCommands,
        _func: fn(Entity, Commands),
    ) {
        // Empty implementation
    }

    fn describe(&self) -> String {
        format!("{}", self.0)
    }
}

#[macro_export]
macro_rules! impl_into_value {
    ( $x:ty ) => {
        impl IntoValue for $x {
            type Out = $x;

            fn into_value(self) -> Value<$x> {
                Value(Arc::new(Lit(self)))
            }
        }
    };
}

impl_into_value!(i8);
impl_into_value!(i16);
impl_into_value!(i32);
impl_into_value!(i64);
impl_into_value!(i128);
impl_into_value!(isize);

impl_into_value!(u8);
impl_into_value!(u16);
impl_into_value!(u32);
impl_into_value!(u64);
impl_into_value!(u128);
impl_into_value!(usize);

impl_into_value!(f32);
impl_into_value!(f64);

pub trait AttributeAccessor: Send + Sync + 'static {
    type Output: Num + PartialOrd + Copy + Clone + Display + Debug + Send + Sync;

    fn current_value(&self, entity: &AttributesRef) -> Result<Self::Output, AttributeError>;
    fn set_current_value(
        &self,
        value: Self::Output,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError>;
    fn base_value(&self, entity: &AttributesRef) -> Result<Self::Output, AttributeError>;
    fn set_base_value(
        &self,
        value: Self::Output,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError>;
    fn name(&self) -> &str;
    fn attribute_type_id(&self) -> AttributeTypeId;
}

#[derive(TypePath, Deref, DerefMut)]
pub struct BoxAttributeAccessor<P: Num>(pub Box<dyn AttributeAccessor<Output = P>>);

impl<P: Num> BoxAttributeAccessor<P> {
    pub fn new<T: Attribute<Property = P>>(evaluator: AttributeExtractor<T>) -> Self {
        Self(Box::new(evaluator))
    }
}

impl<P> Debug for BoxAttributeAccessor<P>
where
    P: Num + PartialOrd + Copy + Clone + Display + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxAttributeAccessor")
            .field("name", &self.0.name())
            .field("attribute_type_id", &self.0.attribute_type_id())
            .finish()
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

impl<T: Attribute> AttributeAccessor for AttributeExtractor<T> {
    type Output = T::Property;

    fn current_value(&self, entity: &AttributesRef) -> Result<T::Property, AttributeError> {
        Ok(entity
            .get::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .current_value())
    }

    fn set_current_value(
        &self,
        value: T::Property,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError> {
        entity
            .get_mut::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .set_current_value(value);
        Ok(())
    }

    fn base_value(&self, entity: &AttributesRef) -> Result<T::Property, AttributeError> {
        Ok(entity
            .get::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .base_value())
    }

    fn set_base_value(
        &self,
        value: T::Property,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError> {
        entity
            .get_mut::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .set_base_value(value);
        Ok(())
    }

    fn name(&self) -> &'static str {
        T::type_path()
    }

    fn attribute_type_id(&self) -> AttributeTypeId {
        T::attribute_type_id()
    }
}

pub fn on_add_attribute<T: Attribute>(trigger: On<Insert, T>, mut commands: Commands) {
    commands.trigger(MarkNodeDirty::<T> {
        entity: trigger.event_target(),
        phantom_data: Default::default(),
    });
}

#[derive(EntityEvent)]
pub struct AttributeDependencyChanged<T> {
    pub entity: Entity,
    phantom_data: PhantomData<T>,
}

pub fn on_change_notify_attribute_dependencies<T: Attribute>(
    query: Query<&AttributeDependents<T>, Changed<T>>,
    mut commands: Commands,
) {
    for dependents in query.iter() {
        let unique_entities: HashSet<Entity> = dependents.iter().collect();
        let notify_targets: Vec<Entity> = unique_entities.into_iter().collect();

        notify_targets.iter().for_each(|target| {
            commands.trigger(AttributeDependencyChanged::<T> {
                entity: *target,
                phantom_data: Default::default(),
            });
        });
    }
}

pub fn on_change_notify_attribute_parents<T: Attribute>(
    query: Query<Entity, Changed<T>>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        commands.trigger(MarkNodeDirty::<T> {
            entity,
            phantom_data: Default::default(),
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ReflectAccessAttribute;

    attribute!(TestAttr, u32);

    /*
    #[test]
    fn test_serialize() {
        let attribute = TestAttribute::new(10);
        let json_attribute = serde_json::to_string(&attribute).unwrap();
        let check_json_attribute = r#"{"base_value":{"bits":10},"current_value":{"bits":10}}"#;

        assert_eq!(json_attribute, check_json_attribute);
    }

    #[test]
    fn test_deserialize() {
        let json_attribute = r#"{"base_value":{"bits":50},"current_value":{"bits":500}}"#;

        let attribute: TestAttribute = serde_json::from_str(json_attribute).unwrap();

        assert_eq!(attribute.base_value, 50);
        assert_eq!(attribute.current_value, 500);
    }*/

    #[test]
    fn test_attribute_new_and_setters() {
        // new() sets both base and current to the same value
        let mut a = TestAttr::new(7u32);
        assert_eq!(a.base_value(), 7);
        assert_eq!(a.current_value(), 7);

        // set_base_value should only change the base
        a.set_base_value(10);
        assert_eq!(a.base_value(), 10);
        assert_eq!(a.current_value(), 7);

        // set_current_value should only change the current
        a.set_current_value(12);
        assert_eq!(a.base_value(), 10);
        assert_eq!(a.current_value(), 12);
    }
}
