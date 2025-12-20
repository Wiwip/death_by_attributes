use crate::attributes::{AttributeQueryData, IntoValue};
use crate::condition::{convert_bounds, multiply_bounds};
use crate::inspector::pretty_type_name;
use crate::prelude::*;
use crate::{AttributesRef, CurrentValueChanged};
use bevy::ecs::entity_disabling::Internal;
use bevy::prelude::*;
use num_traits::{AsPrimitive, Bounded, Num};
use std::collections::Bound;
use std::ops::RangeBounds;

#[derive(Component, Debug, Clone)]
pub struct Clamp<T: Attribute> {
    pub(crate) value: Value<T::Property>,
    pub(crate) limits: (Bound<T::Property>, Bound<T::Property>),
    pub(crate) bounds: (Bound<T::Property>, Bound<T::Property>),
}

impl<T> Clamp<T>
where
    T: Attribute,
    f64: AsPrimitive<T::Property>,
{
    pub fn new(
        value: impl IntoValue<Out = T::Property>,
        limits: impl RangeBounds<f64> + Send + Sync + Copy + 'static,
    ) -> Self {
        Self {
            value: value.into_value(),
            limits: convert_bounds::<f64, T>(limits),
            bounds: (Bound::Unbounded, Bound::Unbounded),
        }
    }
}

/// When the Source attribute changes, we update the bounds of the target attribute
pub fn observe_current_value_change_for_clamp_bounds<S: Attribute, T: Attribute>(
    trigger: On<CurrentValueChanged<S>>,
    mut set: ParamSet<(Query<AttributesRef>, Query<&mut Clamp<T>, Allow<Internal>>)>,
) {
    let source_value: T::Property = {
        let binding = set.p0();
        let attribute_ref = binding.get(trigger.entity).unwrap();

        let Some(clamp) = attribute_ref.get::<Clamp<T>>() else {
            warn!(
                "Entity has no Clamp<{}>: {}.",
                pretty_type_name::<T>(),
                trigger.entity
            );
            return;
        };

        let Ok(value_source) = clamp.value.value(&attribute_ref) else {
            warn!(
                "Error getting attribute value for clamp: {}.",
                trigger.entity
            );
            return;
        };
        value_source
    };

    let mut clamps = set.p1();
    let Ok(mut clamp) = clamps.get_mut(trigger.entity) else {
        warn!(
            "Clamp<{},{}> not found for clamp observer: {}.",
            "_", // TODO
            pretty_type_name::<T>(),
            trigger.observer()
        );
        return;
    };

    // Multiply the source value by the limit to get the derived limit
    let limit_bounds = multiply_bounds::<T>(clamp.limits, source_value);
    clamp.bounds = limit_bounds;
}

pub fn apply_clamps<T>(mut query: Query<(AttributeQueryData<T>, &Clamp<T>), Changed<T>>)
where
    T: Attribute,
{
    for (mut attribute_data, clamp) in query.iter_mut() {
        let clamp_value = bound_clamp(attribute_data.attribute.base_value(), clamp.bounds);
        attribute_data.attribute.set_base_value(clamp_value);

        attribute_data.update_attribute_from_cache();
    }
}

pub fn bound_clamp<V: Num + PartialOrd + Bounded + Copy>(
    value: V,
    clamp: impl RangeBounds<V>,
) -> V {
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
