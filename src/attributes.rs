use crate::condition::{convert_bounds, multiply_bounds};
use crate::effect::AttributeDependencies;
use crate::inspector::pretty_type_name;
use crate::prelude::*;
use crate::systems::{NotifyAttributeDependencyChanged, NotifyDirtyNode};
use crate::{AttributeError, AttributesMut, AttributesRef, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, Typed};
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

pub trait Attribute
where
    Self: Component<Mutability = Mutable> + Copy + Clone + Debug,
    Self: Reflect + TypePath + GetTypeRegistration,
    Self: Serialize,
{
    type Property: Num
        + NumOps
        + NumAssign
        + NumAssignOps
        + Sum
        + Bounded
        + PartialOrd
        + FromPrimitive
        + AsPrimitive<f64>
        + FromReflect
        + GetTypeRegistration
        + Typed
        + Copy
        + Clone
        + Display
        + Debug
        + Send
        + Serialize
        + Sync;

    fn new<T: Num + AsPrimitive<Self::Property> + Copy>(value: T) -> Self;
    fn base_value(&self) -> Self::Property;
    fn set_base_value(&mut self, value: Self::Property);
    fn current_value(&self) -> Self::Property;
    fn set_current_value(&mut self, value: Self::Property);
    fn attribute_type_id() -> AttributeTypeId;
}

/*#[macro_export]
macro_rules! attribute {
    ( $StructName:ident ) => {
        $crate::attribute!($StructName, f32);
    };
    ( $StructName:ident, $ValueType:ident ) => {
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

            fn new<
                T: $crate::num_traits::Num + $crate::num_traits::AsPrimitive<Self::Property> + Copy,
            >(
                value: T,
            ) -> Self {
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
    };
}*/

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

            fn new<
                T: $crate::num_traits::Num + $crate::num_traits::AsPrimitive<Self::Property> + Copy,
            >(
                value: T,
            ) -> Self {
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

impl<T: Attribute> AttributeQueryDataItem<'_, T> {
    pub fn update_attribute(&mut self, calculator: &AttributeCalculator<T>) -> bool {
        let new_val = calculator.eval(self.attribute.base_value());

        //let has_changed = new_val.abs_sub(self.attribute.current_value()) > 0.as_();
        //if has_changed {
        self.attribute.set_current_value(new_val);
        //}
        //has_changed
        true
    }

    pub fn update_attribute_from_cache(&mut self) -> bool {
        let new_val = self
            .calculator_cache
            .calculator
            .eval(self.attribute.base_value());

        //let has_changed = new_val.abs_sub(self.attribute.current_value()) > 0;
        //if has_changed {
        //    self.attribute.set_current_value(new_val);
        //}
        //has_changed
        true
    }
}

#[derive(Component, Clone)]
pub struct Clamp<A: Attribute> {
    bounds: (Bound<A::Property>, Bound<A::Property>),
    phantom_data: PhantomData<A>,
}

impl<A: Attribute> Clamp<A> {
    pub fn new(range: impl RangeBounds<A::Property> + Send + Sync + 'static) -> Self {
        Self {
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Component)]
pub struct DerivedClamp<T>
where
    T: Attribute,
{
    limits: (Bound<T::Property>, Bound<T::Property>),
    bounds: (Bound<T::Property>, Bound<T::Property>),
}

impl<T> DerivedClamp<T>
where
    T: Attribute,
    f64: AsPrimitive<T::Property>,
{
    pub fn new(limits: impl RangeBounds<f64> + Send + Sync + Copy + 'static) -> Self {
        Self {
            limits: convert_bounds::<f64, T>(limits),
            bounds: (Bound::Unbounded, Bound::Unbounded),
        }
    }
}

/// When the Source attribute changes, we update the bounds of the target attribute
pub fn derived_clamp_attributes_observer<S, T>(
    trigger: Trigger<OnAttributeValueChanged<S>>,
    mut query: Query<(&mut DerivedClamp<T>, &S)>,
) where
    S: Attribute,
    T: Attribute,
    S::Property: AsPrimitive<T::Property>,
{
    let Ok((mut derived_clamp, source_attribute)) = query.get_mut(trigger.target()) else {
        return;
    };
    let source_value: T::Property = source_attribute.current_value().as_();

    // Multiply the source value by the limit to get the derived limit
    let limit_bounds = multiply_bounds::<T>(derived_clamp.limits, source_value);
    derived_clamp.bounds = limit_bounds;
}

pub fn apply_derived_clamp_attributes<T>(mut query: Query<(&mut T, &DerivedClamp<T>), Changed<T>>)
where
    T: Attribute,
{
    for (mut attribute, clamp) in query.iter_mut() {
        let clamp_value = bound_clamp(attribute.base_value(), clamp.bounds);
        attribute.set_base_value(clamp_value);
    }
}

pub(crate) fn clamp_attributes_observer<T: Attribute>(
    trigger: Trigger<OnAttributeValueChanged<T>>,
    mut query: Query<(&mut T, &Clamp<T>)>,
) {
    let Ok((mut attribute, clamp)) = query.get_mut(trigger.target()) else {
        return;
    };

    let clamp_value = bound_clamp(attribute.base_value(), clamp.bounds);
    attribute.set_base_value(clamp_value);
}

fn bound_clamp<V: Num + PartialOrd + Bounded + Copy>(value: V, clamp: impl RangeBounds<V>) -> V {
    let value = match clamp.start_bound() {
        Bound::Included(&min) => {
            if value < min {
                min
            } else {
                value
            }
        }
        Bound::Excluded(&min) => {
            if value <= min {
                min + V::min_value()
            } else {
                value
            }
        }
        Bound::Unbounded => value,
    };

    let value = match clamp.end_bound() {
        Bound::Included(&max) => {
            if value > max {
                max
            } else {
                value
            }
        }
        Bound::Excluded(&max) => {
            if value >= max {
                max - V::min_value()
            } else {
                value
            }
        }
        Bound::Unbounded => value,
    };

    value
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

pub trait AttributeAccessor: Send + Sync + 'static {
    type Property: Num + PartialOrd + Copy + Clone + Display + Debug + Send + Sync;

    fn current_value(&self, entity: &AttributesRef) -> Result<Self::Property, AttributeError>;
    fn set_current_value(
        &self,
        value: Self::Property,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError>;
    fn base_value(&self, entity: &AttributesRef) -> Result<Self::Property, AttributeError>;
    fn set_base_value(
        &self,
        value: Self::Property,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError>;
    fn name(&self) -> &str;
    fn attribute_type_id(&self) -> AttributeTypeId;
}

#[derive(TypePath, Deref, DerefMut)]
pub struct BoxAttributeAccessor<P: Num>(pub Box<dyn AttributeAccessor<Property = P>>);

impl<P: Num> BoxAttributeAccessor<P> {
    pub fn new<T: Attribute<Property = P>>(evaluator: AttributeExtractor<T>) -> Self {
        Self(Box::new(evaluator))
    }
}

impl<P: Num> Debug for BoxAttributeAccessor<P> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!();
        //f.debug_tuple("BoxExtractor").field(&self.0.name()).finish()
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
    type Property = T::Property;

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

pub fn on_add_attribute<T: Attribute>(trigger: Trigger<OnInsert, T>, mut commands: Commands) {
    commands
        .entity(trigger.target())
        .trigger(NotifyDirtyNode::<T>::default());
}

pub fn on_change_notify_attribute_dependencies<T: Attribute>(
    query: Query<(&T, &AttributeDependencies<T>), Changed<T>>,
    mut commands: Commands,
) {
    for (attribute, dependencies) in query.iter() {
        let unique_entities: HashSet<Entity> = dependencies.iter().collect();
        let notify_targets: Vec<Entity> = unique_entities.into_iter().collect();

        debug!(
            "Attribute<{}> changed. Notify: {:?} ",
            pretty_type_name::<T>(),
            notify_targets
        );
        commands.trigger_targets(
            NotifyAttributeDependencyChanged::<T> {
                base_value: attribute.base_value(),
                current_value: attribute.current_value(),
                phantom_data: Default::default(),
            },
            notify_targets,
        );
    }
}

pub fn on_change_notify_attribute_parents<T: Attribute>(
    query: Query<Entity, Changed<T>>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        debug!(
            "Attribute<{}> changed. Notify parent chain.",
            pretty_type_name::<T>(),
        );
        commands
            .entity(entity)
            .trigger(NotifyDirtyNode::<T>::default());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ReflectAccessAttribute;

    attribute!(TestAttribute, u32);

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
    }
}
