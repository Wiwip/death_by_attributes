use crate::graph::AttributeTypeId;
use crate::inspector::pretty_type_name;
use crate::prelude::{AttributeCalculator, AttributeCalculatorCached};
use crate::{AttributeError, AttributesMut, AttributesRef, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use std::any::TypeId;
use std::collections::{Bound, HashSet};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{RangeBounds};
use crate::effect::AttributeDependencies;
use crate::systems::{NotifyAttributeChanged, NotifyDirtyNode};

pub trait Attribute:
    Component<Mutability = Mutable> + Clone + Reflect + TypePath + GetTypeRegistration
{
    fn new(value: f64) -> Self;

    fn base_value(&self) -> f64;
    fn set_base_value(&mut self, value: f64);
    fn current_value(&self) -> f64;
    fn set_current_value(&mut self, value: f64);

    fn attribute_type_id() -> AttributeTypeId;
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(bevy::prelude::Component, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[reflect(AccessAttribute)]
        pub struct $StructName {
            base_value: f64,
            current_value: f64,
        }

        impl $crate::attributes::Attribute for $StructName {
            fn new(value: f64) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
            fn base_value(&self) -> f64 {
                self.base_value
            }
            fn set_base_value(&mut self, value: f64) {
                self.base_value = value;
            }
            fn current_value(&self) -> f64 {
                self.current_value
            }
            fn set_current_value(&mut self, value: f64) {
                self.current_value = value;
            }
            fn attribute_type_id() -> $crate::graph::AttributeTypeId {
                $crate::graph::AttributeTypeId::of::<Self>()
            }
        }
    };

        ( $StructName:ident, $($RequiredType:ty),+ $(,)? ) => {
        #[derive(bevy::prelude::Component, Default, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[reflect(AccessAttribute)]
        #[require($crate::prelude::ModAggregator<$StructName>, $($RequiredType),+)]
        pub struct $StructName {
            base_value: f64,
            current_value: f64,
        }

        impl $crate::attributes:AttributeComponentt for $StructName {
            fn new(value: f64) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
            fn base_value(&self) -> f64 {
                self.base_value
            }
            fn set_base_value(&mut self, value: f64) {
                self.base_value = value;
            }
            fn current_value(&self) -> f64 {
                self.current_value
            }
            fn set_current_value(&mut self, value: f64) {
                self.current_value = value;
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
    pub fn update_attribute(&mut self, calculator: &AttributeCalculator) -> bool {
        let new_val = calculator.eval(self.attribute.base_value());

        let is_notable_update = (new_val - &self.attribute.current_value()).abs() > f64::EPSILON;
        if is_notable_update {
            self.attribute.set_current_value(new_val);
        }

        is_notable_update
    }

    pub fn update_attribute_from_cache(&mut self) -> bool {
        let new_val = self
            .calculator_cache
            .calculator
            .eval(self.attribute.base_value());

        let is_notable_update = (new_val - &self.attribute.current_value()).abs() > f64::EPSILON;
        if is_notable_update {
            self.attribute.set_current_value(new_val);
        }

        is_notable_update
    }
}

#[derive(Component, Clone)]
pub struct Clamp<A> {
    bounds: (Bound<f64>, Bound<f64>),
    phantom_data: PhantomData<A>,
}

impl<A: Attribute> Clamp<A> {
    pub fn new(range: impl RangeBounds<f64> + Send + Sync + 'static) -> Self {
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
                attribute.set_base_value(min + f64::EPSILON);
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
                attribute.set_base_value(max - f64::EPSILON);
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
        self.base_value()
    }
    fn access_current_value(&self) -> f64 {
        self.current_value()
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

pub trait AttributeAccessor: Send + Sync + 'static {
    fn current_value(&self, entity: &AttributesRef) -> Result<f64, AttributeError>;
    fn set_current_value(
        &self,
        value: f64,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError>;
    fn base_value(&self, entity: &AttributesRef) -> Result<f64, AttributeError>;
    fn set_base_value(&self, value: f64, entity: &mut AttributesMut) -> Result<(), AttributeError>;
    fn name(&self) -> &str;
    fn attribute_type_id(&self) -> AttributeTypeId;
}

#[derive(TypePath, Deref, DerefMut)]
pub struct BoxAttributeAccessor(pub Box<dyn AttributeAccessor>);

impl BoxAttributeAccessor {
    pub fn new<T: AttributeAccessor + 'static>(evaluator: T) -> Self {
        Self(Box::new(evaluator))
    }
}

impl std::fmt::Debug for BoxAttributeAccessor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoxExtractor").field(&self.0.name()).finish()
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

impl<A: Attribute> AttributeAccessor for AttributeExtractor<A> {
    fn current_value(&self, entity: &AttributesRef) -> Result<f64, AttributeError> {
        Ok(entity
            .get::<A>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<A>()))?
            .current_value())
    }

    fn set_current_value(
        &self,
        value: f64,
        entity: &mut AttributesMut,
    ) -> Result<(), AttributeError> {
        entity
            .get_mut::<A>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<A>()))?
            .set_current_value(value);
        Ok(())
    }

    fn base_value(&self, entity: &AttributesRef) -> Result<f64, AttributeError> {
        Ok(entity
            .get::<A>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<A>()))?
            .base_value())
    }

    fn set_base_value(&self, value: f64, entity: &mut AttributesMut) -> Result<(), AttributeError> {
        entity
            .get_mut::<A>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<A>()))?
            .set_base_value(value);
        Ok(())
    }

    fn name(&self) -> &'static str {
        A::type_path()
    }

    fn attribute_type_id(&self) -> AttributeTypeId {
        A::attribute_type_id()
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
        commands.entity(entity).trigger(
            NotifyDirtyNode::<T>::default(),
        );
    }
}