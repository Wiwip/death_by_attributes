use crate::inspector::pretty_type_name;
use crate::prelude::Attribute;
use bevy::prelude::*;
use serde::Serialize;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul, Sub};
use num_traits::{AsPrimitive, FromPrimitive, Num, One, SaturatingMul, Zero};

#[derive(Debug, Clone, Copy, Reflect, Serialize)]
pub enum Mod<M> {
    Set(M),
    Add(M),
    Sub(M),
    Increase(M),
    More(M),
    //Less(f64),
}

impl<M> Mod<M>
where
    M: Num + Copy + Clone,
{
    pub fn value_mut(&mut self) -> &mut M {
        match self {
            Mod::Set(value) => value,
            Mod::Add(value) => value,
            Mod::Sub(value) => value,
            Mod::Increase(value) => value,
            Mod::More(value) => value,
            //Mod::Less(value) => value,
        }
    }

    pub fn value(&self) -> M {
        match self {
            Mod::Set(value) => *value,
            Mod::Add(value) => *value,
            Mod::Sub(value) => *value,
            Mod::Increase(value) => *value,
            Mod::More(value) => *value,
            //Mod::Less(value) => value,
        }
    }
}

impl<M> Mod<M>
where
    M: Num + Copy + Clone + 'static,
{
    pub fn set<T: Num + AsPrimitive<M> + Copy>(value: T) -> Self {
        Self::Set(value.as_())
    }

    pub fn add<T: Num + AsPrimitive<M> + Copy>(value: T) -> Self {
        Self::Add(value.as_())
    }

    pub fn sub<T: Num + AsPrimitive<M> + Copy>(value: T) -> Self {
        Self::Sub(value.as_())
    }

    pub fn increase<T: Num + AsPrimitive<M> + Copy>(value: T) -> Self {
        Self::Increase(value.as_())
    }

    pub fn more<T: Num + AsPrimitive<M> + Copy>(value: T) -> Self {
        Self::More(value.as_())
    }
}

impl<M> Default for Mod<M>
where
    M: Num + Copy + Clone,
{
    fn default() -> Self {
        Self::Add(M::zero())
    }
}

impl<M> Mul<M> for Mod<M>
where
    M: Num + Copy + Clone,
{
    type Output = Mod<M>;

    fn mul(self, rhs: M) -> Self::Output {
        match self {
            Mod::Set(value) => Mod::Set(value * rhs),
            Mod::Add(value) => Mod::Add(value * rhs),
            Mod::Sub(value) => Mod::Add(value * rhs),
            Mod::Increase(value) => Mod::Increase(value * rhs),
            Mod::More(value) => Mod::More(value * rhs),
            //Mod::Less(value) => Mod::Less(value * rhs),
        }
    }
}

impl<M> Display for Mod<M>
where
    M: Num + FromPrimitive + Display + Debug + Copy + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mod::Set(value) => write!(f, "{:.1}", value),
            Mod::Add(value) => {
                write!(f, "+{:.1}", value)
            }
            Mod::Sub(value) => {
                write!(f, "-{:.1}", value)
            }
            Mod::Increase(value) => write!(f, "{:.1}%", value.mul(M::from_u32(100).unwrap())),
            Mod::More(value) => write!(f, "{:.1}%", value.mul(M::from_u32(100).unwrap())),
            //Mod::Less(value) => write!(f, "{:.1}%", value * 100.0),
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
    pub(crate) increase: T::Property,
    pub(crate) more: T::Property,
}

impl<T: Attribute> AttributeCalculator<T> {
    pub fn eval(&self, base_value: T::Property) -> T::Property {
        if let Some(set) = self.set {
            return set;
        }

        // Step 1 - Additions
        let addition_result = base_value + self.additive;

        // Step 2 - Substraction
        let subtraction_result = addition_result - self.subtractive;

        // Step 3 - Additive Multiplication
        // Clamp self.increase to prevent negative increase to attributes
        let clamped_increase = if self.increase < T::Property::zero() {
            T::Property::zero()
        } else {
            self.increase
        };
        let add_multi_result = subtraction_result * (T::Property::one() + clamped_increase);

        // Step 4 - More multipliers
        let result = add_multi_result * self.more;

        result
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
}

impl<T: Attribute> Default for AttributeCalculator<T> {
    fn default() -> Self {
        Self {
            set: None,
            additive: T::Property::from_u8(0).unwrap(),
            subtractive: T::Property::from_u8(0).unwrap(),
            increase: T::Property::from_u8(0).unwrap(),
            more: T::Property::from_u8(1).unwrap(),
        }
    }
}

impl<T: Attribute> From<Mod<T::Property>> for AttributeCalculator<T> {
    fn from(modifier: Mod<T::Property>) -> Self {
        match modifier {
            Mod::Set(value) => Self {
                set: Some(value),
                ..default()
            },
            Mod::Add(value) => Self {
                additive: value,
                ..default()
            },
            Mod::Sub(value) => Self {
                subtractive: value,
                ..default()
            },
            Mod::Increase(value) => Self {
                increase: value,
                ..default()
            },
            Mod::More(value) => Self {
                more: value,
                ..default()
            },
        }
    }
}

impl<T: Attribute> From<&Vec<Mod<T::Property>>> for AttributeCalculator<T> {
    fn from(modifiers: &Vec<Mod<T::Property>>) -> Self {
        let set: Vec<&T::Property> = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::Set(value) => Some(value),
                _ => None,
            })
            .collect();

        if set.len() > 0 {
            return Self {
                set: Some(*set[0]),
                ..default()
            };
        }

        let additive: T::Property = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::Add(value) => Some(*value),
                _ => None,
            })
            .sum();

        let subtractive: T::Property = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::Sub(value) => Some(*value),
                _ => None,
            })
            .sum();

        let increased: T::Property = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::Increase(value) => Some(*value),
                _ => None,
            })
            .sum();

        let more: T::Property = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::More(value) => Some(*value),
                _ => None,
            })
            .fold(T::Property::one(), |acc, x| {
                acc * (T::Property::one() + x)
            });

        Self {
            set: None,
            additive,
            subtractive,
            increase: increased,
            more,
        }
    }
}
