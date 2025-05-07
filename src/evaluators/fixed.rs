use crate::attributes::AttributeComponent;
use crate::evaluators::MutatorEvaluator;
use bevy::ecs::component::Mutable;
use bevy::prelude::{Component, Event, Observer, Reflect};
use std::fmt::{Debug, Display, Formatter};
/*
/// A data type that returns a float value when evaluated.
///
/// Usually used by [Mutator][`crate::mutator::Mutator`].
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

impl Display for FixedEvaluator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", &self.magnitude, &self.mod_type)
    }
}

impl MutatorEvaluator for FixedEvaluator {
    fn get_magnitude(&self) -> f32 {
        self.magnitude
    }

    fn set_magnitude(&mut self, magnitude: f32) {
        self.magnitude = magnitude;
    }

    fn get_aggregator(&self) -> ModAggregator {
        match self.mod_type {
            ModType::Additive => ModAggregator::additive(self.magnitude),
            ModType::Multiplicative => ModAggregator::multiplicative(self.magnitude),
            ModType::Overrule => ModAggregator::overrule(self.magnitude),
        }
    }

    fn get_observer<O: Event, T: Component<Mutability = Mutable> + AttributeComponent>(
        &self,
    ) -> Option<Observer> {
        // No need for an observer on fixed mutators
        None
    }
}
*/

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModAggregator;
    use crate::attribute;
    use crate::attributes::AttributeComponent;
    use crate::effects::EffectBuilder;
    use crate::mutators::EvaluateMutator;
    use crate::mutators::mutator::ModType::Additive;
    use crate::mutators::mutator::MutatorHelper;
    use crate::*;
    use bevy::ecs::system::RunSystemOnce;

    attribute!(Health);
    attribute!(HealthRegen);

    static MUTATOR_VALUE: f32 = 42.0;

    #[test]
    fn test_fixed_evaluator_mutators() {
        let mut world = World::default();
        let id = world.spawn(Health::new(0.0)).id();

        let health = world.get::<Health>(id).unwrap();
        assert_eq!(health.base_value, 0.0);

        let _ = world.run_system_once(apply_mutator);

        let health = world.get::<Health>(id).unwrap();
        assert_eq!(health.base_value, MUTATOR_VALUE);

        fn apply_mutator(mut query: Query<ActorEntityMut>) {
            let entity = query.single_mut().unwrap();
            let mutator =
                MutatorHelper::new::<Health>(FixedEvaluator::new(MUTATOR_VALUE, Additive));
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

    #[test]
    fn test_meta_attribute() {
        const INIT_HEALTH_VALUE: f32 = 0.0;
        const NEW_HEALTH_VALUE: f32 = 10.0;

        let mut app = App::new();
        app.add_observer(on_attribute_mutation_changed);

        let effect = app.world_mut().spawn_empty().id();
        let player = app
            .world_mut()
            .spawn((Health::new(0.0), HealthRegen::new(INIT_HEALTH_VALUE)))
            .id();

        // Make the effect to be applied
        EffectBuilder::new(player, effect)
            .with_permanent_duration()
            .with_continuous_application()
            .mutate_by_attribute::<Health, HealthRegen>(1.0, Additive)
            .apply(&mut app.world_mut().commands());

        app.world_mut().flush();

        // Update the value of an attribute and notify of its change

        let mut health_regen = app.world_mut().get_mut::<HealthRegen>(player).unwrap();
        health_regen.base_value = NEW_HEALTH_VALUE;
        health_regen.current_value = NEW_HEALTH_VALUE;

        app.world_mut()
            .trigger_targets(OnCurrentValueChanged, player);

        app.update();

        // Check that the value of the mutator is now increased to the value of HealthRegen
        let mut mutators = app.world_mut().query::<&Mutator>();
        let query = mutators.query(app.world_mut());
        let mutator = query.single().unwrap();
        println!("{:?}", mutator);
        assert_eq!(NEW_HEALTH_VALUE, mutator.get_magnitude());
    }
}
*/
