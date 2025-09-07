use crate::AttributesRef;
use crate::condition::ConditionContext;
use crate::context::EffectContext;
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use crate::prelude::{Attribute, AttributeModifier};
use bevy::prelude::*;
use num_traits::{AsPrimitive, FromPrimitive, Num, NumCast, One, SaturatingMul, Zero};
use serde::Serialize;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Sum;
use std::ops::{Add, Mul, Sub};

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
            ModOp::Increase => write!(f, "*(+)"),
            ModOp::More => write!(f, "*"),
        }
    }
}
/*
impl<M> Mod<M>
where
    M: Num + NumCast + Copy + Clone + AsPrimitive<f64>,
{
    pub fn set_value(&mut self, value: f64) {
        let cast_value = M::from(value).unwrap();

        match self {
            Mod::Set(_) => {
                *self = Mod::Set(cast_value);
            }
            Mod::Add(_) => {
                *self = Mod::Add(cast_value);
            }
            Mod::Sub(_) => {
                *self = Mod::Sub(cast_value);
            }
            Mod::Increase(_) => {
                *self = Mod::Increase(value);
            }
            Mod::More(_) => {
                *self = Mod::More(value);
            }
        }
    }

    pub fn value(&self) -> f64 {
        match self {
            Mod::Set(value) => value.as_(),
            Mod::Add(value) => value.as_(),
            Mod::Sub(value) => value.as_(),
            Mod::Increase(value) => *value,
            Mod::More(value) => *value,
            //Mod::Less(value) => value,
        }
    }
}*/

/*impl<M> Mod<M>
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

    pub fn increase<T: Num + AsPrimitive<f64> + Copy>(value: T) -> Self {
        Self::Increase(value.as_())
    }

    pub fn more<T: Num + AsPrimitive<f64> + Copy>(value: T) -> Self {
        Self::More(value.as_())
    }
}*/

/*impl<M> Default for ModOp<M>
where
    M: Num + Copy + Clone,
{
    fn default() -> Self {
        Self::Add(M::zero())
    }
}

impl<M> Mul<M> for ModOp<M>
where
    M: Num + Copy + Clone + AsPrimitive<f64>,
{
    type Output = ModOp<M>;

    fn mul(self, rhs: M) -> Self::Output {
        match self {
            ModOp::Set(value) => ModOp::Set(value * rhs),
            ModOp::Add(value) => ModOp::Add(value * rhs),
            ModOp::Sub(value) => ModOp::Add(value * rhs),
            ModOp::Increase(value) => ModOp::Increase(value * rhs.as_()),
            ModOp::More(value) => ModOp::More(value * rhs.as_()),
            //Mod::Less(value) => Mod::Less(value * rhs),
        }
    }
}

impl<M> Display for ModOp<M>
where
    M: Num + FromPrimitive + Display + Debug + Copy + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModOp::Set(value) => write!(f, "{:.1}", value),
            ModOp::Add(value) => {
                write!(f, "+{:.1}", value)
            }
            ModOp::Sub(value) => {
                write!(f, "-{:.1}", value)
            }
            ModOp::Increase(value) => write!(f, "{:.1}%", value * 100.0),
            ModOp::More(value) => write!(f, "{:.1}%", value * 100.0),
            //Mod::Less(value) => write!(f, "{:.1}%", value * 100.0),
        }
    }
}*/

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
        let addition_result = base_value + self.additive;

        // Step 2 - Substraction
        let subtraction_result: f64 = (addition_result - self.subtractive).as_();

        // Step 3 - Additive Multiplication
        // Clamp self.increase to prevent negative increase to attributes
        let clamped_increase = if self.increase < 0.0 {
            0.0
        } else {
            self.increase
        };
        let add_multi_result = subtraction_result * (1.0 + clamped_increase);

        // Step 4 - More multipliers
        let result = add_multi_result * self.more;

        T::Property::from_f64(result).unwrap()
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
