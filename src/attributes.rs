use crate::graph::AttributeTypeId;
use crate::inspector::pretty_type_name;
use crate::prelude::{AttributeCalculator, AttributeCalculatorCached};
use crate::OnAttributeValueChanged;
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use std::marker::PhantomData;
use std::ops::DerefMut;

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
        #[derive(bevy::prelude::Component, Default, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[reflect(AccessAttribute)]
        //#[require($crate::prelude::ModAggregator<$StructName>)]
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
pub enum AttributeClamp<A> {
    Phantom(PhantomData<A>),
    Min(f64),
    Max(f64),
    MinMax(f64, f64),
}

pub(crate) fn clamp_attributes_system<A: Component<Mutability = Mutable> + Attribute>(
    mut query: Query<(&mut A, &AttributeClamp<A>)>,
) {
    for (mut attribute, clamp) in query.iter_mut() {
        match clamp {
            AttributeClamp::Min(min) => {
                let new_base = attribute.base_value().min(*min);
                attribute.set_base_value(new_base);

                let new_current = attribute.current_value().min(*min);
                attribute.set_current_value(new_current);
            }
            AttributeClamp::Max(max) => {
                let new_base = attribute.base_value().max(*max);
                attribute.set_base_value(new_base);

                let new_current = attribute.current_value().max(*max);
                attribute.set_current_value(new_current);
            }
            AttributeClamp::MinMax(min, max) => {
                let new_base = attribute.base_value().clamp(*min, *max);
                attribute.set_base_value(new_base);

                let new_current = attribute.current_value().clamp(*min, *max);
                attribute.set_current_value(new_current);
            }
            _ => {}
        }
    }
}

pub(crate) fn update_max_clamp_values<T, C>(
    trigger: Trigger<OnAttributeValueChanged<T>>,
    attribute: Query<&C>,
    mut query: Query<&mut AttributeClamp<T>>,
) where
    T: Component<Mutability = Mutable> + Attribute,
    C: Component<Mutability = Mutable> + Attribute,
{
    let Ok(mut clamp) = query.get_mut(trigger.target()) else {
        return;
    };
    let Ok(attribute) = attribute.get(trigger.target()) else {
        return;
    };
    match clamp.deref_mut() {
        AttributeClamp::Min(_) => {}
        AttributeClamp::Max(max) => *max = attribute.current_value(),
        AttributeClamp::MinMax(_, max) => *max = attribute.current_value(),
        _ => {}
    }
}

#[reflect_trait] // Generates a `ReflectMyTrait` type
pub trait AccessAttribute {
    fn base_value(&self) -> f64;
    fn current_value(&self) -> f64;
    fn name(&self) -> String;
}

impl<T> AccessAttribute for T
where
    T: Attribute,
{
    fn base_value(&self) -> f64 {
        self.base_value()
    }
    fn current_value(&self) -> f64 {
        self.current_value()
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}
