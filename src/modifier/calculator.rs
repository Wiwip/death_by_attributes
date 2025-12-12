use crate::AttributesRef;
use crate::math::SaturatingAttributes;
use crate::prelude::{Attribute, AttributeModifier};
use bevy::prelude::*;
use num_traits::{AsPrimitive, Bounded, FromPrimitive, Zero};
use serde::Serialize;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, Copy, Reflect, Serialize)]
pub enum ModOp {
    Set,
    Add,
    Sub,
    Increase,
    More,
    //Less(f64),
}

impl Display for ModOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModOp::Set => write!(f, "="),
            ModOp::Add => write!(f, "+"),
            ModOp::Sub => write!(f, "-"),
            ModOp::Increase => write!(f, "+*"),
            ModOp::More => write!(f, "*"),
        }
    }
}

#[derive(Component, Clone, Copy, Reflect, Debug)]
pub struct AttributeCalculatorCached<T: Attribute> {
    #[reflect(ignore)]
    pub calculator: AttributeCalculator<T>,
}

impl<T: Attribute> Default for AttributeCalculatorCached<T> {
    fn default() -> Self {
        Self {
            calculator: AttributeCalculator::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct AttributeCalculator<T: Attribute> {
    pub(crate) set: Option<T::Property>,
    pub(crate) additive: T::Property,
    pub(crate) subtractive: T::Property,
    pub(crate) increase: f64,
    pub(crate) more: f64,
}

impl<T: Attribute> AttributeCalculator<T> {
    pub fn eval(&self, base_value: T::Property) -> T::Property {
        if let Some(set) = self.set {
            return set;
        }

        // Step 1 - Additions
        let addition_result: T::Property = base_value.saturating_add(self.additive);

        // Step 2 - Substraction
        let subtraction_result: f64 = addition_result.saturating_sub(self.subtractive).as_();

        // Step 3 - Additive Multiplication
        // Clamp self.increase to prevent negative increase to attributes
        let clamped_increase = self.increase.max(0.0);
        let add_multi_result = subtraction_result * (1.0 + clamped_increase);

        // Step 4 - More multipliers
        let result = add_multi_result * self.more;

        // Step 5 - Clamp the result to property's min/max values
        let min = T::Property::min_value();
        let max = T::Property::max_value();

        let min_f: f64 = min.as_();
        let max_f: f64 = max.as_();

        let clamped_value = result.clamp(min_f, max_f);
        T::Property::from_f64(clamped_value).unwrap()
    }

    pub fn combine(self, other: AttributeCalculator<T>) -> AttributeCalculator<T> {
        // If either has a set value, the last one wins (or you could define other logic)
        let set = self.set.or(other.set);

        // Combine additive values
        let additive = self.additive + other.additive;
        let subtractive = self.subtractive + other.subtractive;

        // Combine increased values (they stack additively)
        let increased = self.increase + other.increase;

        // Combine more values (they stack multiplicatively)
        let more = self.more * other.more;

        AttributeCalculator::<T> {
            set,
            additive,
            subtractive,
            increase: increased,
            more,
        }
    }

    /// Combines another AttributeCalculator into this one in-place.
    /// - set: Uses this calculator's set value if present, otherwise uses other's
    /// - additive: Adds other's additive value to this one
    /// - increased: Adds other's increased value to this one
    /// - more: Multiplies this calculator's more value by other's
    pub fn combine_in_place(&mut self, other: &AttributeCalculator<T>) {
        self.set = self.set.or(other.set);
        self.additive += other.additive;
        self.subtractive += other.subtractive;
        self.increase += other.increase;
        self.more *= other.more;
    }

    pub fn convert(modifier: &AttributeModifier<T>, attributes_ref: &AttributesRef) -> Self {
        match modifier.operation {
            ModOp::Set => Self {
                set: Some(modifier.value_source.value(attributes_ref).unwrap()),
                ..default()
            },
            ModOp::Add => Self {
                additive: modifier.value_source.value(attributes_ref).unwrap(),
                ..default()
            },
            ModOp::Sub => Self {
                subtractive: modifier.value_source.value(attributes_ref).unwrap(),
                ..default()
            },
            ModOp::Increase => Self {
                increase: modifier.value_source.value(attributes_ref).unwrap().as_(),
                ..default()
            },
            ModOp::More => Self {
                more: modifier.value_source.value(attributes_ref).unwrap().as_(),
                ..default()
            },
        }
    }
}

impl<T: Attribute> Default for AttributeCalculator<T> {
    fn default() -> Self {
        Self {
            set: None,
            additive: T::Property::zero(),
            subtractive: T::Property::zero(),
            increase: 0.0,
            more: 1.0,
        }
    }
}
