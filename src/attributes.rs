use crate::OnAttributeValueChanged;
use bevy::ecs::component::Mutable;
use bevy::prelude::{Component, Query, Trigger};
use std::marker::PhantomData;
use std::ops::DerefMut;

pub trait Attribute {
    fn new(value: f64) -> Self;
    fn base_value(&self) -> f64;
    fn set_base_value(&mut self, value: f64);
    fn current_value(&self) -> f64;
    fn set_current_value(&mut self, value: f64);
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(bevy::prelude::Component, Default, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[require($crate::modifiers::ModAggregator<$StructName>)]
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
        }
    };

        ( $StructName:ident, $($RequiredType:ty),+ $(,)? ) => {
        #[derive(bevy::prelude::Component, Default, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[require($crate::modifiers::ModAggregator<$StructName>, $($RequiredType),+)]
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

#[derive(Component)]
pub enum AttributeClamp<A> {
    Phantom(PhantomData<A>),
    Min(f64),
    Max(f64),
    MinMax(f64, f64),
}

pub(crate) fn attribute_clamp_system<A: Component<Mutability = Mutable> + Attribute>(
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
