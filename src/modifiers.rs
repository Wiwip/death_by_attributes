use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::ops::Add;

use bevy::prelude::Component;
use Modifier::Meta;

use crate::attributes::GameAttributeMarker;
use crate::modifiers::Modifier::Scalar;

#[derive(Debug, Clone)]
pub enum Modifier {
    Scalar(ScalarModifier),
    Meta(MetaModifier),
}

impl Modifier {
    pub fn get_attribute_id(&self) -> TypeId {
        match self {
            Scalar(item) => item.target_attribute,
            Meta(item) => item.target_attribute,
        }
    }

    pub fn get_type(&self) -> &ModifierType {
        match self {
            Scalar(item) => &item.mod_type,
            Meta(item) => &item.mod_type,
        }
    }
}

#[derive(Clone)]
pub struct ScalarModifier {
    pub target_attribute: TypeId,
    pub magnitude: f32,
    pub mod_type: ModifierType,
}

impl ScalarModifier {
    pub fn additive<M: Component + GameAttributeMarker>(magnitude: f32) -> ScalarModifier {
        ScalarModifier {
            target_attribute: TypeId::of::<M>(),
            magnitude,
            mod_type: ModifierType::Additive,
        }
    }

    pub fn multi<M: Component + GameAttributeMarker>(magnitude: f32) -> ScalarModifier {
        ScalarModifier {
            target_attribute: TypeId::of::<M>(),
            magnitude,
            mod_type: ModifierType::Multiplicative,
        }
    }

    pub fn overrule<M: Component + GameAttributeMarker>(magnitude: f32) -> ScalarModifier {
        ScalarModifier {
            target_attribute: TypeId::of::<M>(),
            magnitude,
            mod_type: ModifierType::Overrule,
        }
    }
}

impl Debug for ScalarModifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.mod_type {
            ModifierType::Additive => {
                write!(f, "Add:{:.1}", self.magnitude)
            }
            ModifierType::Multiplicative => {
                write!(f, "Mul:{:.1}", self.magnitude)
            }
            ModifierType::Overrule => {
                write!(f, "Over:{:.1}", self.magnitude)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct MetaModifier {
    pub target_attribute: TypeId,
    pub magnitude_attribute: TypeId,
    pub mod_type: ModifierType,
}

impl Debug for MetaModifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MetaMod")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ModifierType {
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Default)]
pub struct ModifierAggregator {
    pub additive: f32,
    pub multiplicative: f32,
    pub veto: Option<f32>,
}

impl ModifierAggregator {
    pub fn get_current_value(&self, base_value: f32) -> f32 {
        match self.veto {
            None => (base_value + self.additive) * (1.0 + self.multiplicative),
            Some(value) => value,
        }
    }
}

impl From<&ScalarModifier> for ModifierAggregator {
    fn from(value: &ScalarModifier) -> Self {
        let mut aggregator = ModifierAggregator::default();
        match value.mod_type {
            ModifierType::Additive => aggregator.additive += value.magnitude,
            ModifierType::Multiplicative => aggregator.multiplicative += value.magnitude,
            ModifierType::Overrule => aggregator.veto = Some(value.magnitude),
        }
        aggregator
    }
}

impl Add for &ModifierAggregator {
    type Output = ModifierAggregator;

    fn add(self, rhs: Self) -> Self::Output {
        ModifierAggregator {
            additive: self.additive + rhs.additive,
            multiplicative: self.additive + rhs.additive,
            veto: self.veto.or(rhs.veto),
        }
    }
}

impl Sum for ModifierAggregator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                additive: 0.0,
                multiplicative: 0.0,
                veto: None,
            },
            |a, b| Self {
                additive: a.additive + b.additive,
                multiplicative: a.multiplicative + b.multiplicative,
                veto: a.veto.or(b.veto),
            },
        )
    }
}
