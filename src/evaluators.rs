use std::fmt::Debug;
use crate::mutator::{ModAggregator, ModType};
use bevy::animation::AnimationEvaluationError;
use bevy::prelude::Reflect;

pub trait MutatorEvaluator: Debug + Send + Sync + 'static {
    fn get_magnitude(&self) -> Result<f32, AnimationEvaluationError>;
    fn get_aggregator(&self) -> Result<ModAggregator, AnimationEvaluationError>;
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

impl MutatorEvaluator for FixedEvaluator {
    fn get_magnitude(&self) -> Result<f32, AnimationEvaluationError> {
        Ok(self.magnitude)
    }

    fn get_aggregator(&self) -> Result<ModAggregator, AnimationEvaluationError> {
        Ok(match self.mod_type {
            ModType::Additive => ModAggregator::additive(self.magnitude),
            ModType::Multiplicative => ModAggregator::multiplicative(self.magnitude),
            ModType::Overrule => ModAggregator::overrule(self.magnitude),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttributeDef;
    use crate::GameAbilityContainer;
    use crate::attribute;
    use crate::attributes::AttributeComponent;
    use crate::mutator::ModType::Additive;
    use crate::mutator::{EvaluateMutator, Mutator};
    use crate::*;
    use bevy::ecs::system::RunSystemOnce;

    attribute!(Health);
    attribute!(HealthRegen);

    static MUTATOR_VALUE: f32 = 42.0;

    #[test]
    fn test_fixed_evaluator_mutators() {
        let mut world = World::default();
        let id = world.spawn((Health::new(0.0), HealthRegen::new(10.0))).id();

        let health = world.get::<Health>(id).unwrap();
        assert_eq!(health.base_value, 0.0);

        let _ = world.run_system_once(execute_basic);

        let health = world.get::<Health>(id).unwrap();
        assert_eq!(health.base_value, MUTATOR_VALUE);

        fn execute_basic(mut query: Query<AttributeEntityMut>) {
            let entity = query.single_mut().unwrap();
            let mutator = Mutator::new::<Health>(FixedEvaluator::new(MUTATOR_VALUE, Additive));
            mutator.apply_mutator(entity);
        }
    }

    #[test]
    fn test_aggregators() {
        const BASE_VALUE: f32 = 10.0;
        let aggregator = ModAggregator::additive(0.0);
        assert_eq!(BASE_VALUE, aggregator.evaluate(BASE_VALUE));

        let aggregator = ModAggregator::additive(10.0);
        assert_eq!(BASE_VALUE + 10.0, aggregator.evaluate(BASE_VALUE));

        let aggregator = ModAggregator::multiplicative(0.0);
        assert_eq!(BASE_VALUE, aggregator.evaluate(BASE_VALUE));

        let aggregator = ModAggregator::multiplicative(1.0);
        assert_eq!(2.0 * BASE_VALUE, aggregator.evaluate(BASE_VALUE));

        let aggregator = ModAggregator::overrule(42.0);
        assert_eq!(42.0, aggregator.evaluate(BASE_VALUE));

        let ag1 = ModAggregator::additive(10.0);
        let ag2 = ModAggregator::additive(20.0);
        assert_eq!(BASE_VALUE + 30.0, (ag1 + ag2).evaluate(BASE_VALUE));

        let ag1 = ModAggregator::additive(10.0);
        assert_eq!(BASE_VALUE, (ag1 + -ag1).evaluate(BASE_VALUE));
    }
}
