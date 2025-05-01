use crate::attributes::{AttributeAccessorMut, AttributeDef};
use std::any::TypeId;

use crate::{AttributeEntityMut, AttributeEntityRef, Editable};
use bevy::animation::AnimationEvaluationError;
use bevy::platform::hash::Hashed;
use bevy::prelude::*;
use bevy::reflect::Reflectable;
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, AddAssign};

#[derive(Debug, TypePath)]
pub struct AttributeModVariable(pub Box<dyn EvalModifier>);

impl Clone for AttributeModVariable {
    fn clone(&self) -> Self {
        Self(EvalModifier::clone_value(&*self.0))
    }
}

impl AttributeModVariable {
    pub fn new(animation_curve: impl EvalModifier) -> Self {
        Self(Box::new(animation_curve))
    }
}

pub trait EvalModifier: Debug + Send + Sync + 'static {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn EvalModifier>;

    fn get_aggregator(&self) -> ModAggregator;

    fn get_magnitude(&self) -> f32;

    fn get_current_value(&self, entity_ref: &mut AttributeEntityMut) -> f32;

    fn apply(
        &self,
        proto: &mut AttributeEntityMut,
    ) -> std::result::Result<(), AnimationEvaluationError>;

    fn apply_from_aggregator(
        &self,
        proto: &mut AttributeEntityMut,
        aggregator: ModAggregator,
    ) -> std::result::Result<(), AnimationEvaluationError>;

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)>;
}

#[derive(Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct AttributeModifier<P, C> {
    attribute_ref: P,
    evaluator: C,
}

impl<P, C> AttributeModifier<P, C>
where
    P: AttributeAccessorMut,
    C: ModifierCalculations<P::Property>,
{
    pub fn new(attribute_ref: P, evaluator: C) -> Self {
        Self {
            attribute_ref,
            evaluator,
        }
    }
}

impl<P, C> Clone for AttributeModifier<P, C>
where
    P: Clone,
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            attribute_ref: self.attribute_ref.clone(),
            evaluator: self.evaluator.clone(),
        }
    }
}

impl<P: Send + Sync + 'static, C> Debug for AttributeModifier<P, C>
where
    P: Clone + AttributeAccessorMut,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<P: Send + Sync + 'static, C> EvalModifier for AttributeModifier<P, C>
where
    P: AttributeAccessorMut + Clone,
    C: ModifierCalculations<P::Property>,
{
    fn clone_value(&self) -> Box<dyn EvalModifier> {
        Box::new(self.clone())
    }

    fn get_aggregator(&self) -> ModAggregator {
        self.evaluator.aggregator()
    }

    fn get_magnitude(&self) -> f32 {
        self.evaluator.get_magnitude()
    }

    fn get_current_value(&self, entity_mut: &mut AttributeEntityMut) -> f32 {
        self.attribute_ref
            .get_mut(entity_mut)
            .unwrap()
            .get_current_value()
    }

    fn apply(
        &self,
        entity_mut: &mut AttributeEntityMut,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        let attribute_ref = self.attribute_ref.get_mut(entity_mut)?;
        let attribute_def = attribute_ref
            .as_any_mut()
            .downcast_mut::<AttributeDef>()
            .unwrap();
        let aggregator = self.evaluator.aggregator();

        match aggregator.overrule {
            None => {
                attribute_def.base_value += aggregator.additive;
                attribute_def.base_value *= 1.0 + aggregator.multiplicative;
            }
            Some(v) => attribute_def.current_value = v,
        }
        Ok(())
    }

    fn apply_from_aggregator(
        &self,
        entity_mut: &mut AttributeEntityMut,
        aggregator: ModAggregator,
    ) -> Result<(), AnimationEvaluationError> {
        let attribute_ref = self.attribute_ref.get_mut(entity_mut)?;
        let attribute_def = attribute_ref
            .as_any_mut()
            .downcast_mut::<AttributeDef>()
            .unwrap();

        match aggregator.overrule {
            None => {
                attribute_def.current_value = (attribute_def.base_value + aggregator.additive)
                    * (1.0 + aggregator.multiplicative)
            }
            Some(v) => attribute_def.current_value = v,
        }
        Ok(())
    }

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)> {
        self.attribute_ref.evaluator_id()
    }
}

pub trait ModifierCalculations<T>: Debug + Clone + Reflectable {
    fn aggregator(&self) -> ModAggregator;
    fn get_magnitude(&self) -> f32;
}

#[derive(Default, Debug, Clone, Reflect)]
pub struct ModEvaluator {
    magnitude: f32,
    mod_type: ModType,
}

impl ModEvaluator {
    pub fn new(magnitude: f32, mod_type: ModType) -> Self {
        Self {
            magnitude,
            mod_type,
        }
    }
}

impl<T> ModifierCalculations<T> for ModEvaluator {
    fn aggregator(&self) -> ModAggregator {
        match self.mod_type {
            ModType::Additive => ModAggregator {
                additive: self.magnitude,
                multiplicative: 0.0,
                overrule: None,
            },
            ModType::Multiplicative => ModAggregator {
                additive: 0.0,
                multiplicative: self.magnitude,
                overrule: None,
            },
            ModType::Overrule => ModAggregator {
                additive: 0.0,
                multiplicative: 0.0,
                overrule: Some(self.magnitude),
            },
        }
    }

    fn get_magnitude(&self) -> f32 {
        self.magnitude
    }
}

#[derive(Default, Debug, Clone, Copy, Reflect)]
pub enum ModType {
    #[default]
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ModAggregator {
    pub additive: f32,
    pub multiplicative: f32,
    pub overrule: Option<f32>,
}

impl ModAggregator {
    pub fn get_current_value(&self, base_value: f32) -> f32 {
        match self.overrule {
            None => (base_value + self.additive) * (1.0 + self.multiplicative),
            Some(value) => value,
        }
    }
}

impl Add for &ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multiplicative: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl Add<ModAggregator> for &mut ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: ModAggregator) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multiplicative: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl AddAssign<ModAggregator> for &mut ModAggregator {
    fn add_assign(&mut self, rhs: ModAggregator) {
        self.additive += rhs.additive;
        self.multiplicative += rhs.multiplicative;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl Sum for ModAggregator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                additive: 0.0,
                multiplicative: 0.0,
                overrule: None,
            },
            |a, b| Self {
                additive: a.additive + b.additive,
                multiplicative: a.multiplicative + b.multiplicative,
                overrule: a.overrule.or(b.overrule),
            },
        )
    }
}
