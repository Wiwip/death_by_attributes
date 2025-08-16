use crate::prelude::Attribute;
use bevy::prelude::*;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::Mul;

#[derive(Debug, Clone, Copy, Reflect)]
pub enum Mod {
    Set(f64),
    Add(f64),
    Increase(f64),
    More(f64),
    //Less(f64),
}

impl Mod {
    pub fn value_mut(&mut self) -> &mut f64 {
        match self {
            Mod::Set(value) => value,
            Mod::Add(value) => value,
            Mod::Increase(value) => value,
            Mod::More(value) => value,
            //Mod::Less(value) => value,
        }
    }
    
    pub fn value(&self) -> f64 {
        match self {
            Mod::Set(value) => *value,
            Mod::Add(value) => *value,
            Mod::Increase(value) => *value,
            Mod::More(value) => *value,
            //Mod::Less(value) => value,
        }
    }
}

impl Default for Mod {
    fn default() -> Self {
        Self::Add(0.0)
    }
}

impl Mul<f64> for Mod {
    type Output = Mod;

    fn mul(self, rhs: f64) -> Self::Output {
        match self {
            Mod::Set(value) => Mod::Set(value * rhs),
            Mod::Add(value) => Mod::Add(value * rhs),
            Mod::Increase(value) => Mod::Increase(value * rhs),
            Mod::More(value) => Mod::More(value * rhs),
            //Mod::Less(value) => Mod::Less(value * rhs),
        }
    }
}

impl Display for Mod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mod::Set(value) => write!(f, "{:.1}", value),
            Mod::Add(value) => {
                if *value >= 0.0 {
                    write!(f, "+{:.1}", value)
                } else {
                    write!(f, "{:.1}", value)
                }
            }
            Mod::Increase(value) => write!(f, "{:.1}%", value * 100.0),
            Mod::More(value) => write!(f, "{:.1}%", value * 100.0),
            //Mod::Less(value) => write!(f, "{:.1}%", value * 100.0),
        }
    }
}

#[derive(Component, Clone, Copy, Reflect, Debug)]
pub struct AttributeCalculatorCached<T: Attribute> {
    pub calculator: AttributeCalculator,
    #[reflect(ignore)]
    marker: PhantomData<T>,
}

impl<T: Attribute> Default for AttributeCalculatorCached<T> {
    fn default() -> Self {
        Self {
            calculator: AttributeCalculator::default(),
            marker: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct AttributeCalculator {
    pub(crate) set: Option<f64>,
    pub(crate) additive: f64,
    pub(crate) increased: f64,
    pub(crate) more: f64,
}

impl AttributeCalculator {
    pub fn eval(&self, base_value: f64) -> f64 {
        if let Some(set) = self.set {
            return set;
        };

        (base_value + self.additive) * (1.0 + self.increased) * self.more
    }

    pub fn combine(self, other: AttributeCalculator) -> AttributeCalculator {
        // If either has a set value, the last one wins (or you could define other logic)
        let set = self.set.or(other.set);

        // Combine additive values
        let additive = self.additive + other.additive;

        // Combine increased values (they stack additively)
        let increased = self.increased + other.increased;

        // Combine more values (they stack multiplicatively)
        let more = self.more * other.more;

        AttributeCalculator {
            set,
            additive,
            increased,
            more,
        }
    }

    /// Combines another AttributeCalculator into this one in-place.
    /// - set: Uses this calculator's set value if present, otherwise uses other's
    /// - additive: Adds other's additive value to this one
    /// - increased: Adds other's increased value to this one
    /// - more: Multiplies this calculator's more value by other's
    pub fn combine_in_place(&mut self, other: &AttributeCalculator) {
        self.set = self.set.or(other.set);
        self.additive += other.additive;
        self.increased += other.increased;
        self.more *= other.more;
    }
}

impl Default for AttributeCalculator {
    fn default() -> Self {
        Self {
            set: None,
            additive: 0.0,
            increased: 0.0,
            more: 1.0,
        }
    }
}

impl From<Mod> for AttributeCalculator {
    fn from(modifier: Mod) -> Self {
        match modifier {
            Mod::Set(value) => Self {
                set: Some(value),
                ..default()
            },
            Mod::Add(value) => Self {
                additive: value,
                ..default()
            },
            Mod::Increase(value) => Self {
                increased: value,
                ..default()
            },
            Mod::More(value) => Self {
                more: value,
                ..default()
            },
        }
    }
}

impl From<&Vec<Mod>> for AttributeCalculator {
    fn from(modifiers: &Vec<Mod>) -> Self {
        let set: Vec<&f64> = modifiers
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

        let additive: f64 = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::Add(value) => Some(value),
                _ => None,
            })
            .sum();

        let increased: f64 = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::Increase(value) => Some(value),
                _ => None,
            })
            .sum();

        let more: f64 = modifiers
            .iter()
            .filter_map(|m| match m {
                Mod::More(value) => Some(value),
                _ => None,
            })
            .fold(1.0, |acc, x| acc * (1.0 + x));

        Self {
            set: None,
            additive,
            increased,
            more,
        }
    }
}
