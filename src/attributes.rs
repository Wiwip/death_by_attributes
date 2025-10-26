use crate::condition::{convert_bounds, multiply_bounds};
use crate::effect::AttributeDependencies;
use crate::inspector::pretty_type_name;
use crate::math::AbsDiff;
use crate::prelude::*;
use crate::systems::{MarkNodeDirty, NotifyAttributeDependencyChanged};
use crate::{AttributeError, AttributesMut, AttributesRef, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, Typed};
use num_traits::NumCast;
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
    Self: Component<Mutability = Mutable> + Copy + Clone + Debug + Display,
    Self: Reflect + TypePath + GetTypeRegistration,
    Self: Serialize,
{
    type Property: Num
        + NumOps
        + NumCast
        + NumAssign
        + NumAssignOps
        + Sum
        + Bounded
        + AbsDiff
        + PartialOrd
        + FromPrimitive
        + AsPrimitive<f64>
        + FromReflect
        + GetTypeRegistration
        + Typed
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
    fn value() -> Value<Self::Property> {
        Value::new::<Self>()
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
        let new_val = calculator.eval(self.attribute.base_value());

        let has_changed = new_val.are_different(self.attribute.current_value());
        if has_changed {
            self.attribute.set_current_value(new_val);
        }
        has_changed
    }

    pub fn update_attribute_from_cache(&mut self) -> bool {
        let new_val = self
            .calculator_cache
            .calculator
            .eval(self.attribute.base_value());

        let has_changed = new_val.are_different(self.attribute.current_value());
        if has_changed {
            self.attribute.set_current_value(new_val);
        }
        has_changed
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

pub trait ValueSource: Send + Sync + 'static {
    type Output: Num;

    fn value(&self, entity: &AttributesRef) -> Result<Self::Output, AttributeError>;
    fn describe(&self) -> String;

    fn clone_value(&self) -> Box<dyn ValueSource<Output = Self::Output>>;
}

/// A ['Value'] refers to an Attribute value.
/// It can be a literal value, or a reference to an Attribute.
#[derive(Deref, DerefMut)]
pub struct Value<P: Num>(Box<dyn ValueSource<Output = P>>);

impl<P: Num + Display + Debug + Copy + Clone + Send + Sync + 'static> Value<P> {
    pub fn new<T: Attribute<Property = P>>() -> Self {
        Self(Box::new(AttributeValue::<T> {
            value: P::zero(),
            phantom_data: Default::default(),
        }))
    }

    pub fn lit(value: P) -> Self {
        Self(Box::new(Lit(value)))
    }
}

impl<P: Num + Display + Debug + Copy + Clone + Send + Sync + 'static> Default for Value<P> {
    fn default() -> Self {
        Value::lit(P::zero())
    }
}

impl<P: Num + 'static> Clone for Value<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone_value())
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

/// An ['AttributeValue'] is a dynamic reference to an Attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttributeValue<T: Attribute> {
    value: T::Property,
    phantom_data: PhantomData<T>,
}

impl<T: Attribute> ValueSource for AttributeValue<T> {
    type Output = T::Property;

    fn value(&self, entity: &AttributesRef) -> Result<Self::Output, AttributeError> {
        Ok(entity
            .get::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .current_value())
    }

    fn describe(&self) -> String {
        format!("{}", pretty_type_name::<T>())
    }

    fn clone_value(&self) -> Box<dyn ValueSource<Output = Self::Output>> {
        Box::new(AttributeValue::<T> {
            value: self.value,
            phantom_data: Default::default(),
        })
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit<P: Num>(P);

impl<P: Num + Display + Debug + Copy + Clone + Send + Sync + 'static> ValueSource for Lit<P> {
    type Output = P;

    fn value(&self, _: &AttributesRef) -> Result<Self::Output, AttributeError> {
        Ok(self.0)
    }

    fn describe(&self) -> String {
        format!("{}", self.0)
    }

    fn clone_value(&self) -> Box<dyn ValueSource<Output = Self::Output>> {
        Box::new(Lit(self.0))
    }
}

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
        entity: trigger.target(),
        phantom_data: Default::default(),
    });
}

pub fn on_change_notify_attribute_dependencies<T: Attribute>(
    query: Query<(&T, &AttributeDependencies<T>), Changed<T>>,
    mut commands: Commands,
) {
    for (attribute, dependencies) in query.iter() {
        let unique_entities: HashSet<Entity> = dependencies.iter().collect();
        let notify_targets: Vec<Entity> = unique_entities.into_iter().collect();

        /*debug!(
            "Attribute<{}> changed. Notify: {:?} ",
            pretty_type_name::<T>(),
            notify_targets
        );*/

        notify_targets.iter().for_each(|target| {
            commands.trigger(NotifyAttributeDependencyChanged::<T> {
                entity: *target,
                base_value: attribute.base_value(),
                current_value: attribute.current_value(),
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
        /*debug!(
            "Attribute<{}> changed. Notify parent chain.",
            pretty_type_name::<T>(),
        );*/
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
