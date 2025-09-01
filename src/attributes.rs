use crate::condition::{convert_bounds, multiply_bounds};
use crate::effect::AttributeDependencies;
use crate::inspector::pretty_type_name;
use crate::prelude::{AttributeCalculator, AttributeCalculatorCached};
use crate::systems::{NotifyAttributeDependencyChanged, NotifyDirtyNode};
use crate::{AttributeError, AttributesMut, AttributesRef, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use bevy_inspector_egui::__macro_exports::bevy_reflect::ReflectRemote;
use fixed::prelude::{LossyInto, ToFixed};
use fixed::traits::Fixed;
use fixed::types::{
    I0F8, I0F16, I8F8, I16F0, I16F16, I32F0, I32F32, I48F16, I64F0, U0F8, U0F16, U8F0, U8F8, U12F4,
    U16F0, U16F16, U24F8, U32F0, U32F32, U64F0,
};
use serde::Serialize;
use std::any::TypeId;
use std::collections::{Bound, HashSet};
use std::fmt::Display;
use std::fmt::{Debug, Formatter};
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::marker::PhantomData;
use std::ops::RangeBounds;

pub trait Attribute:
    Component<Mutability = Mutable>
    + Copy
    + Clone
    + Reflect
    + Debug
    + TypePath
    + GetTypeRegistration
    + Serialize
{
    type Property: Fixed
        + LossyInto<f64>
        + PartialOrd
        + Copy
        + Clone
        + Display
        + Debug
        + Send
        + Serialize
        + Sync;

    fn new<T: ToFixed + Copy>(value: T) -> Self;
    fn base_value(&self) -> Self::Property;
    fn set_base_value(&mut self, value: Self::Property);
    fn current_value(&self) -> Self::Property;
    fn set_current_value(&mut self, value: Self::Property);
    fn attribute_type_id() -> AttributeTypeId;
}

#[macro_export]
macro_rules! attribute_impl {
    ( $StructName:ident, $ValueType:path, $ValueTypeProxy:ident ) => {
        $crate::paste::paste! {
            #[derive(bevy::prelude::Component, Clone, Copy, bevy::prelude::Reflect, Debug)]
            #[derive(serde::Serialize, serde::Deserialize)]
            #[reflect(AccessAttribute)]
            pub struct $StructName {
                #[reflect(remote = $crate::attributes::[<$ValueTypeProxy Proxy>])]
                base_value: $ValueType,
                #[reflect(remote = $crate::attributes::[<$ValueTypeProxy Proxy>])]
                current_value: $ValueType,
            }
        }

        impl $crate::attributes::Attribute for $StructName {
            type Property = $ValueType;

            fn new<T: $crate::fixed::prelude::ToFixed + Copy>(value: T) -> Self {
                Self {
                    base_value: value.to_fixed(),
                    current_value: value.to_fixed(),
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
    ( $StructName:ident, $ValueType:ident ) => {
        $crate::paste::paste! {
            #[derive(bevy::prelude::Component, Clone, Copy, bevy::prelude::Reflect, Debug)]
            #[derive(serde::Serialize, serde::Deserialize)]
            #[reflect(AccessAttribute)]
            pub struct $StructName {
                #[reflect(remote = $crate::attributes::[<$ValueType Proxy>])]
                base_value: $ValueType,
                #[reflect(remote = $crate::attributes::[<$ValueType Proxy>])]
                current_value: $ValueType,
            }
        }

        impl $crate::attributes::Attribute for $StructName {
            type Property = $ValueType;

            fn new<T: $crate::fixed::prelude::ToFixed + Copy>(value: T) -> Self {
                Self {
                    base_value: value.to_fixed(),
                    current_value: value.to_fixed(),
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
        $crate::attribute_impl!($StructName, $crate::fixed::types::U24F8, U24F8);
    };
    ( $StructName:ident, f16 ) => {
        $crate::attribute_impl!($StructName, $crate::fixed::types::U12F4, U12F4);
    };
    ( $StructName:ident, f32 ) => {
        $crate::attribute_impl!($StructName, $crate::fixed::types::U24F8, U24F8);
    };
    ( $StructName:ident, u8 ) => {
        $crate::attribute_impl!($StructName, $crate::fixed::types::U8F0, U8F0);
    };
    ( $StructName:ident, u16 ) => {
        $crate::attribute_impl!($StructName, $crate::fixed::types::U16F0, U16F0);
    };
    ( $StructName:ident, u32 ) => {
        $crate::attribute_impl!($StructName, $crate::fixed::types::U32F0, U32F0);
    };
    ( $StructName:ident, u64 ) => {
        $crate::attribute_impl!($StructName, $crate::fixed::types::U64F0, U64F0);
    };
    ( $StructName:ident, $ValueType:ident  ) => {
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

        let has_changed = new_val.abs_diff(self.attribute.current_value()) > 0;
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

        let has_changed = new_val.abs_diff(self.attribute.current_value()) > 0;
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
{
    pub fn new(limits: impl RangeBounds<f64> + Send + Sync + Copy + 'static) -> Self {
        Self {
            limits: convert_bounds::<T, _>(limits),
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
    S::Property: LossyInto<T::Property>,
{
    let Ok((mut derived_clamp, source_attribute)) = query.get_mut(trigger.target()) else {
        return;
    };
    let source_value: T::Property = source_attribute.current_value().lossy_into();

    // Multiply the source value by the limit to get the derived limit
    let limit_bounds = multiply_bounds::<T>(derived_clamp.limits, source_value);
    derived_clamp.bounds = convert_bounds::<T, _>(limit_bounds);
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

fn bound_clamp<V: Fixed>(value: V, clamp: impl RangeBounds<V>) -> V {
    let value = match clamp.start_bound() {
        Bound::Included(&min) => value.max(min),
        Bound::Excluded(&min) => value.max(min + V::MIN),
        Bound::Unbounded => value,
    };

    let value = match clamp.end_bound() {
        Bound::Included(&max) => value.min(max),
        Bound::Excluded(&max) => value.min(max - V::MIN),
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
        self.base_value().lossy_into()
    }
    fn access_current_value(&self) -> f64 {
        self.current_value().lossy_into()
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

pub trait AttributeAccessor: Send + Sync + 'static {
    type Property: Fixed
        + LossyInto<f64>
        + PartialOrd
        + Copy
        + Clone
        + Display
        + Debug
        + Send
        + Sync;

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
pub struct BoxAttributeAccessor<P: Fixed>(pub Box<dyn AttributeAccessor<Property = P>>);

impl<P: Fixed> BoxAttributeAccessor<P> {
    pub fn new<T: Attribute<Property = P>>(evaluator: AttributeExtractor<T>) -> Self {
        Self(Box::new(evaluator))
    }
}

impl<P: Fixed> std::fmt::Debug for BoxAttributeAccessor<P> {
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

#[macro_export]
macro_rules! impl_reflect_remote_fixed {
    ($proxy_name:ident, $remote_type:ident, $bits_type:ty) => {
        #[derive(::bevy::reflect::Reflect, ::std::fmt::Debug)]
        pub struct $proxy_name($bits_type);

        impl ReflectRemote for $proxy_name {
            type Remote = ::fixed::types::$remote_type;

            fn as_remote(&self) -> &Self::Remote {
                // SAFETY: Fixed types are repr(transparent) over their bits type, so this cast is safe
                unsafe { std::mem::transmute(&self.0) }
            }

            fn as_remote_mut(&mut self) -> &mut Self::Remote {
                // SAFETY: Fixed types are repr(transparent) over their bits type, so this cast is safe
                unsafe { std::mem::transmute(&mut self.0) }
            }

            fn into_remote(self) -> Self::Remote {
                <$remote_type>::from_bits(self.0)
            }

            fn as_wrapper(remote: &Self::Remote) -> &Self {
                // SAFETY: Fixed types are repr(transparent) over their bits type, so this cast is safe
                unsafe { std::mem::transmute(remote) }
            }

            fn as_wrapper_mut(remote: &mut Self::Remote) -> &mut Self {
                // SAFETY: Fixed types are repr(transparent) over their bits type, so this cast is safe
                unsafe { std::mem::transmute(remote) }
            }

            fn into_wrapper(remote: Self::Remote) -> Self {
                Self(remote.to_bits())
            }
        }
    };
}

// 8-bit types
impl_reflect_remote_fixed!(U0F8Proxy, U0F8, u8);
impl_reflect_remote_fixed!(I0F8Proxy, I0F8, i8);
impl_reflect_remote_fixed!(U8F0Proxy, U8F0, u8);

// 16-bit types
impl_reflect_remote_fixed!(U0F16Proxy, U0F16, u16);
impl_reflect_remote_fixed!(I0F16Proxy, I0F16, i16);
impl_reflect_remote_fixed!(I8F8Proxy, I8F8, i16);
impl_reflect_remote_fixed!(U8F8Proxy, U8F8, u16);
impl_reflect_remote_fixed!(U12F4Proxy, U12F4, u16);
impl_reflect_remote_fixed!(I16F0Proxy, I16F0, i16);
impl_reflect_remote_fixed!(U16F0Proxy, U16F0, u16);

// 32-bit types
impl_reflect_remote_fixed!(I16F16Proxy, I16F16, i32);
impl_reflect_remote_fixed!(U16F16Proxy, U16F16, u32);
impl_reflect_remote_fixed!(U24F8Proxy, U24F8, u32);
impl_reflect_remote_fixed!(I32F0Proxy, I32F0, i32);
impl_reflect_remote_fixed!(U32F0Proxy, U32F0, u32);

// 64-bit types
impl_reflect_remote_fixed!(I32F32Proxy, I32F32, i64);
impl_reflect_remote_fixed!(I48F16Proxy, I48F16, i64);
impl_reflect_remote_fixed!(I64F0Proxy, I64F0, i64);
impl_reflect_remote_fixed!(U32F32Proxy, U32F32, u64);
impl_reflect_remote_fixed!(U64F0Proxy, U64F0, u64);

#[cfg(test)]
mod test {
    use super::*;
    use crate::ReflectAccessAttribute;

    attribute_impl!(TestAttribute, U32F0);

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
