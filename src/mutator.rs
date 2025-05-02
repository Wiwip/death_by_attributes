use crate::attributes::{AttributeAccessorMut, AttributeDef};
use crate::evaluators::Evaluator;
use crate::{AttributeEntityMut, Editable};
use bevy::animation::AnimationEvaluationError;
use bevy::platform::hash::Hashed;
use bevy::prelude::*;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, AddAssign};

#[derive(Debug, TypePath)]
pub struct MutatorWrapper(pub Box<dyn EvaluateMutator>);

impl Clone for MutatorWrapper {
    fn clone(&self) -> Self {
        Self(EvaluateMutator::clone_value(&*self.0))
    }
}

impl MutatorWrapper {
    pub fn new(curve: impl EvaluateMutator) -> Self {
        Self(Box::new(curve))
    }
}

pub trait EvaluateMutator: Debug + Send + Sync + 'static {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn EvaluateMutator>;
    fn get_aggregator(&self, entity: &mut AttributeEntityMut) -> ModAggregator;
    fn get_magnitude(&self, entity: &mut AttributeEntityMut) -> f32;
    fn get_current_value(&self, entity: &mut AttributeEntityMut) -> f32;

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
pub struct Mutator<P, C> {
    pub attribute: P,
    pub evaluator: C,
}

impl<P, C> Mutator<P, C>
where
    P: AttributeAccessorMut + Clone,
    C: Evaluator,
{
    pub fn new(attribute: P, evaluator: C) -> Self {
        Self {
            attribute,
            evaluator,
        }
    }
}

impl<P, C> Clone for Mutator<P, C>
where
    P: Clone,
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            attribute: self.attribute.clone(),
            evaluator: self.evaluator.clone(),
        }
    }
}

impl<P, C> Debug for Mutator<P, C>
where
    C: 'static + Evaluator + Send + Sync,
    P: 'static + AttributeAccessorMut + Clone + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<P, C> EvaluateMutator for Mutator<P, C>
where
    P: AttributeAccessorMut + Clone + Send + Sync + 'static,
    C: Evaluator + Send + Sync + 'static,
{
    fn clone_value(&self) -> Box<dyn EvaluateMutator> {
        Box::new(self.clone())
    }

    fn get_aggregator(&self, entity: &mut AttributeEntityMut) -> ModAggregator {
        self.evaluator.get_aggregator(entity)
    }

    fn get_magnitude(&self, entity: &mut AttributeEntityMut) -> f32 {
        self.evaluator.get_magnitude(entity)
    }

    fn get_current_value(&self, entity: &mut AttributeEntityMut) -> f32 {
        self.attribute.get_mut(entity).unwrap().get_current_value()
    }

    fn apply(&self, entity: &mut AttributeEntityMut) -> Result<(), AnimationEvaluationError> {
        let magnitude = self.evaluator.get_magnitude(entity);
        let target_ref = self.attribute.get_mut(entity).unwrap();
        let attribute_def = target_ref
            .as_any_mut()
            .downcast_mut::<AttributeDef>()
            .unwrap();

        attribute_def.base_value += magnitude;
        Ok(())
    }

    fn apply_from_aggregator(
        &self,
        entity_mut: &mut AttributeEntityMut,
        aggregator: ModAggregator,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        let attribute_ref = self.attribute.get_mut(entity_mut)?;
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
        self.attribute.evaluator_id()
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
    pub fn additive(value: f32) -> Self {
        ModAggregator {
            additive: value,
            multiplicative: 0.0,
            overrule: None,
        }
    }
    pub fn multiplicative(value: f32) -> Self {
        ModAggregator {
            additive: 0.0,
            multiplicative: value,
            overrule: None,
        }
    }
    pub fn overrule(value: f32) -> Self {
        ModAggregator {
            additive: 0.0,
            multiplicative: 0.0,
            overrule: Some(value),
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
