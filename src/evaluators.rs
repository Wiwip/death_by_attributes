use crate::attributes::AttributeAccessorMut;
use crate::mutator::{ModAggregator, ModType};
use crate::{AttributeEntityMut, Editable};
use bevy::animation::AnimationEvaluationError;
use bevy::prelude::Reflect;

pub trait Evaluator: Clone {
    fn get_magnitude(
        &self,
        entity: &mut AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError>;
    fn get_aggregator(
        &self,
        entity: &mut AttributeEntityMut,
    ) -> Result<ModAggregator, AnimationEvaluationError>;
}

#[derive(Reflect, Clone, Debug)]
pub struct FixedEvaluator {
    magnitude: f32,
    mod_type: ModType,
}

impl FixedEvaluator {
    pub fn new(magnitude: f32, mod_type: ModType) -> Self {
        Self {
            magnitude,
            mod_type,
        }
    }
}

impl Evaluator for FixedEvaluator {
    fn get_magnitude(
        &self,
        entity: &mut AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError> {
        Ok(self.magnitude)
    }

    fn get_aggregator(
        &self,
        entity: &mut AttributeEntityMut,
    ) -> Result<ModAggregator, AnimationEvaluationError> {
        Ok(match self.mod_type {
            ModType::Additive => ModAggregator::additive(self.magnitude),
            ModType::Multiplicative => ModAggregator::multiplicative(self.magnitude),
            ModType::Overrule => ModAggregator::overrule(self.magnitude),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MetaEvaluator<P>
where
    P: AttributeAccessorMut,
{
    attribute: P,
    scale: f32,
    mod_type: ModType,
}

impl<P> MetaEvaluator<P>
where
    P: AttributeAccessorMut,
{
    pub fn new(attribute: P, scale: f32, mod_type: ModType) -> Self {
        Self {
            attribute,
            scale,
            mod_type,
        }
    }
}

impl<P> Evaluator for MetaEvaluator<P>
where
    P: AttributeAccessorMut + Clone,
{
    fn get_magnitude(
        &self,
        entity: &mut AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError> {
        match self.attribute.get_mut(entity) {
            Ok(attribute) => Ok(attribute.get_current_value()),
            Err(err) => Err(err),
        }
    }

    fn get_aggregator(
        &self,
        entity: &mut AttributeEntityMut,
    ) -> Result<ModAggregator, AnimationEvaluationError> {
        let actual_magnitude = match self.get_magnitude(entity) {
            Ok(magnitude) => magnitude * self.scale,
            Err(err) => return Err(err),
        };
        Ok(match self.mod_type {
            ModType::Additive => ModAggregator::additive(actual_magnitude),
            ModType::Multiplicative => ModAggregator::multiplicative(actual_magnitude),
            ModType::Overrule => ModAggregator::overrule(actual_magnitude),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttributeDef;
    use crate::GameAbilityContainer;
    use crate::attribute;
    use crate::attributes::AttributeMut;
    use crate::mutator::EvaluateMutator;

    use crate::mutator::ModType::Additive;
    use crate::mutator::Mutator;
    use crate::*;
    use bevy::ecs::system::RunSystemOnce;

    attribute!(Health);
    attribute!(HealthRegen);

    #[test]
    fn test_meta_attribute_world() {
        let mut world = World::default();
        let id = world.spawn((Health::new(0.0), HealthRegen::new(10.0))).id();

        let health = AttributeMut::new_unchecked(|c: &mut Health| &mut c.attribute);
        let health_regen = AttributeMut::new_unchecked(|c: &mut HealthRegen| &mut c.attribute);
        let eval = MetaEvaluator::new(health_regen, 0.42, Additive);
        let mutator = Mutator::new(health, eval);

        let health = world.get::<Health>(id).unwrap();
        println!("{}", health.base_value);
        assert_eq!(health.base_value, 0.0);
        let _ = world.run_system_once(execute_basic);
        let health = world.get::<Health>(id).unwrap();
        println!("{}", health.base_value);
        assert_eq!(health.base_value, 999.0);

        let mut world = World::default();
        let id = world.spawn((Health::new(0.0), HealthRegen::new(10.0))).id();

        let health = world.get::<Health>(id).unwrap();
        println!("{}", health.base_value);
        assert_eq!(health.base_value, 0.0);
        let _ = world.run_system_once(execute_meta);
        let health = world.get::<Health>(id).unwrap();
        println!("{}", health.base_value);
        assert_eq!(health.base_value, 10.0);

        fn execute_meta(mut query: Query<AttributeEntityMut>) {
            for mut entity in query.iter_mut() {
                let health = attribute_mut!(Health);
                let health_regen = attribute_mut!(HealthRegen);
                let eval = MetaEvaluator::new(health_regen, 0.42, Additive);
                let mutator = Mutator::new(health, eval);

                let _ = mutator.apply(&mut entity);
            }
        }

        fn execute_basic(mut query: Query<AttributeEntityMut>) {
            for mut entity in query.iter_mut() {
                let health = attribute_mut!(Health);
                let eval = FixedEvaluator::new(999.0, Additive);
                let mutator = Mutator::new(health, eval);

                let _ = mutator.apply(&mut entity);
            }
        }
    }
}
