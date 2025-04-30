use crate::AttributeEntityMut;
use crate::attributes::{AttributeDef, EditableAttribute};
use crate::modifiers::{AttributeMod, ModAggregator, ModType};
use bevy::animation::AnimationEvaluationError;
use bevy::prelude::{
    AnimatableCurveEvaluator, AnimationCurveEvaluator, AnimationNodeIndex, EvaluatorId, Interval,
    Reflect, TypePath,
};
use bevy::reflect::TypeInfo;
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use bevy::platform::hash::Hashed;

#[derive(Debug, TypePath)]
pub struct BoxAttributeModEvaluator(pub Box<dyn AttributeModEvaluator>);

impl Clone for BoxAttributeModEvaluator {
    fn clone(&self) -> Self {
        Self(AttributeModEvaluator::clone_value(&*self.0))
    }
}

impl BoxAttributeModEvaluator {
    pub fn new(animation_curve: impl AttributeModEvaluator) -> Self {
        Self(Box::new(animation_curve))
    }
}

impl<P: Send + Sync + 'static> Debug for AttributeMod<P>
where
    P: Clone + EditableAttribute,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<P: Send + Sync + 'static> AttributeModEvaluator for AttributeMod<P>
where
    P: EditableAttribute + Clone,
{
    fn clone_value(&self) -> Box<dyn AttributeModEvaluator> {
        Box::new(self.clone())
    }

    fn aggregator(&self) -> ModAggregator {
        let mut aggregator = ModAggregator::default();
        match self.mod_type {
            ModType::Additive => aggregator.additive += self.magnitude,
            ModType::Multiplicative => aggregator.multiplicative += self.magnitude,
            ModType::Overrule => aggregator.overrule = Some(self.magnitude),
        }
        aggregator
    }

    fn apply_base(&self, proto: &mut AttributeEntityMut) -> Result<(), AnimationEvaluationError> {
        let a = self.attribute_ref.get_mut(proto)?;
        let def = a.as_any_mut().downcast_mut::<AttributeDef>().unwrap();

        match self.mod_type {
            ModType::Additive => def.base_value += self.magnitude,
            ModType::Multiplicative => def.base_value *= self.magnitude,
            ModType::Overrule => def.base_value = self.magnitude,
        }

        Ok(())
    }

    fn apply_current(&self, proto: &mut AttributeEntityMut) -> Result<(), AnimationEvaluationError> {
        let a = self.attribute_ref.get_mut(proto)?;
        let def = a.as_any_mut().downcast_mut::<AttributeDef>().unwrap();

        def.current_value = def.base_value;

        match self.mod_type {
            ModType::Additive => def.current_value += self.magnitude,
            ModType::Multiplicative => def.current_value *= self.magnitude,
            ModType::Overrule => def.current_value = self.magnitude,
        }

        Ok(())
    }

    fn commit(
        &self,
        proto: &mut AttributeEntityMut,
        aggregator: ModAggregator,
    ) -> Result<(), AnimationEvaluationError> {
        let a = self.attribute_ref.get_mut(proto)?;
        let def = a.as_any_mut().downcast_mut::<AttributeDef>().unwrap();

        match aggregator.overrule {
            None => {
                def.current_value =
                    (def.base_value + aggregator.additive) * (1.0 + aggregator.multiplicative)
            }
            Some(v) => def.current_value = v,
        }
        Ok(())
    }

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)> {
        self.attribute_ref.evaluator_id()
    }
}

pub trait AttributeModEvaluator: Debug + Send + Sync + 'static {
    /// Returns a boxed clone of this value.
    fn clone_value(&self) -> Box<dyn AttributeModEvaluator>;

    fn aggregator(&self) -> ModAggregator;

    fn apply_base(&self, proto: &mut AttributeEntityMut) -> Result<(), AnimationEvaluationError>;
    fn apply_current(&self, proto: &mut AttributeEntityMut) -> Result<(), AnimationEvaluationError>;

    fn commit(
        &self,
        proto: &mut AttributeEntityMut,
        aggregator: ModAggregator,
    ) -> Result<(), AnimationEvaluationError>;
    fn evaluator_id(&self) -> Hashed<(TypeId, usize)>;
}
