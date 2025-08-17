use crate::effect::AttributeDependencies;
use crate::graph::AttributeTypeId;
use crate::inspector::pretty_type_name;
use crate::prelude::{AttributeCalculator, AttributeCalculatorCached};
use crate::systems::{NotifyAttributeChanged, NotifyDirtyNode};
use crate::{AttributeError, AttributesMut, AttributesRef, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, Typed};
use bevy_inspector_egui::__macro_exports::bevy_reflect::ReflectRemote;
use fixed::prelude::{LossyInto, ToFixed};
use fixed::traits::Fixed;
use fixed::types::{I16F16, I32F32, U16F16, U32F0, U32F32, U64F0, U8F0};
use std::any::TypeId;
use std::collections::{Bound, HashSet};
use std::fmt::Display;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::RangeBounds;
use fixed::types::extra::U8;

pub trait Attribute:
    Component<Mutability = Mutable> + Copy + Clone + Reflect + Debug + TypePath + GetTypeRegistration
{
    type Property: Fixed
        + LossyInto<f64>
        + PartialOrd
        + Copy
        + Clone
        + Display
        + Debug
        + Send
        + Sync;

    fn new<T: ToFixed + Copy>(value: T) -> Self;
    fn base_value(&self) -> Self::Property;
    fn set_base_value(&mut self, value: Self::Property);
    fn current_value(&self) -> Self::Property;
    fn set_current_value(&mut self, value: Self::Property);
    fn attribute_type_id() -> AttributeTypeId;
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident ) => {
        $crate::attribute!($StructName, U16F16);
    };
    ( $StructName:ident, f32 ) => {
        $crate::attribute!($StructName, $crate::attributes::U16F16);
    };
    ( $StructName:ident, u32 ) => {
        $crate::attribute!($StructName, $crate::attributes::U32F0);
    };
    ( $StructName:ident, u64 ) => {
        $crate::attribute!($StructName, $crate::attributes::U64F0);
    };

    ( $StructName:ident, $ValueType:ty ) => {
        ::paste::paste! {
            #[derive(bevy::prelude::Component, Clone, Copy, bevy::prelude::Reflect, Debug)]
            #[reflect(AccessAttribute)]
            pub struct $StructName {
                #[reflect(remote = [<$ValueType Proxy>])]
                base_value: $ValueType,
                #[reflect(remote = [<$ValueType Proxy>])]
                current_value: $ValueType,
            }
        }

        impl $crate::attributes::Attribute for $StructName {
            type Property = $ValueType;

            fn new<T: ToFixed + Copy>(value: T) -> Self {
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
            fn attribute_type_id() -> $crate::graph::AttributeTypeId {
                $crate::graph::AttributeTypeId::of::<Self>()
            }
        }
    };
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
        let a = self.attribute.base_value();

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

pub(crate) fn clamp_attributes_observer<A: Attribute>(
    trigger: Trigger<OnAttributeValueChanged<A>>,
    mut query: Query<(&mut A, &Clamp<A>)>,
) {
    let Ok((mut attribute, clamp)) = query.get_mut(trigger.target()) else {
        return;
    };

    match clamp.bounds.0 {
        Bound::Included(min) => {
            if attribute.base_value() < min {
                attribute.set_base_value(min);
            }
        }
        Bound::Excluded(min) => {
            if attribute.base_value() <= min {
                attribute.set_base_value(min);
            }
        }
        Bound::Unbounded => {}
    }

    match clamp.bounds.1 {
        Bound::Included(max) => {
            if attribute.base_value() > max {
                attribute.set_base_value(max);
            }
        }
        Bound::Excluded(max) => {
            if attribute.base_value() >= max {
                attribute.set_base_value(max);
            }
        }
        Bound::Unbounded => {}
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
        self.base_value().lossy_into()
    }
    fn access_current_value(&self) -> f64 {
        self.current_value().lossy_into()
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

pub trait AttributeAccessor<N>: Send + Sync + 'static {
    fn current_value(&self, entity: &AttributesRef) -> Result<N, AttributeError>;
    fn set_current_value(&self, value: N, entity: &mut AttributesMut)
    -> Result<(), AttributeError>;
    fn base_value(&self, entity: &AttributesRef) -> Result<N, AttributeError>;
    fn set_base_value(&self, value: N, entity: &mut AttributesMut) -> Result<(), AttributeError>;
    fn name(&self) -> &str;
    fn attribute_type_id(&self) -> AttributeTypeId;
}

#[derive(TypePath, Deref, DerefMut)]
pub struct BoxAttributeAccessor<T: Attribute>(pub Box<dyn AttributeAccessor<T::Property>>);

impl<T: Attribute> BoxAttributeAccessor<T> {
    pub fn new(evaluator: AttributeExtractor<T>) -> Self {
        Self(Box::new(evaluator))
    }
}

impl<T: Attribute> std::fmt::Debug for BoxAttributeAccessor<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl<T: Attribute> AttributeAccessor<T::Property> for AttributeExtractor<T> {
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
            NotifyAttributeChanged::<T> {
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
    ($proxy_name:ident, $remote_type:ty, $bits_type:ty) => {
        #[derive(Reflect, Debug)]
        pub struct $proxy_name($bits_type);

        impl ReflectRemote for $proxy_name {
            type Remote = $remote_type;

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

impl_reflect_remote_fixed!(U8F0Proxy, U8F0, u8);

// Generate proxy for I16F16 (backed by i32)
impl_reflect_remote_fixed!(I16F16Proxy, I16F16, i32);

// Generate proxy for U16F16 (backed by u32)
impl_reflect_remote_fixed!(U16F16Proxy, U16F16, u32);

// Generate proxy for I32F32 (backed by i64)
impl_reflect_remote_fixed!(I32F32Proxy, I32F32, i64);

// Generate proxy for U32F0 (backed by u32)
impl_reflect_remote_fixed!(U32F0Proxy, U32F0, u32);
impl_reflect_remote_fixed!(U32F32Proxy, U32F32, u64);

impl_reflect_remote_fixed!(U64F0Proxy, U64F0, u64);
