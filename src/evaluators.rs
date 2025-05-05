use crate::OnCurrentValueChanged;
use crate::attributes::AttributeComponent;
use crate::mutator::{ModAggregator, ModType, Mutator};
use bevy::ecs::component::Mutable;
use bevy::log::info;
use bevy::prelude::{Component, Observer, Query, Reflect, Trigger};
use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub trait MutatorEvaluator: Debug + Send + Sync + 'static {
    fn get_magnitude(&self) -> f32;
    fn set_magnitude(&mut self, magnitude: f32);
    fn get_aggregator(&self) -> ModAggregator;
    fn get_observer(&self) -> Option<Observer>;
}

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

    fn get_observer(&self) -> Option<Observer> {
        // No need for an observer on fixed mutators
        None
    }
}

/// Retrieves the value of an [AttributeDef][`crate::attributes::AttributeDef`] on an Attribute [`Component`]
/// to determine the magnitude of the evaluated mutator.
///
#[derive(Reflect)]
pub struct MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    magnitude: Option<f32>,
    scale: f32,
    _phantom: PhantomData<A>,
    mod_type: ModType,
}

impl<A> MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    pub fn new(scale: f32, mod_type: ModType) -> Self {
        Self {
            magnitude: None,
            mod_type,
            _phantom: Default::default(),
            scale,
        }
    }
}

impl<A> Clone for MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    fn clone(&self) -> Self {
        Self {
            magnitude: self.magnitude,
            scale: self.scale,
            _phantom: Default::default(),
            mod_type: self.mod_type,
        }
    }
}

impl<A> Debug for MetaEvaluator<A>
where
    A: AttributeComponent + Component<Mutability = Mutable>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaEvaluator")
            .field("target", &type_name::<A>())
            .field("magnitude", &self.magnitude)
            .field("scale", &self.scale)
            .field("mod_type", &self.mod_type)
            .finish()
    }
}

impl<A> MutatorEvaluator for MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    fn get_magnitude(&self) -> f32 {
        let magnitude = self.magnitude.expect("No magnitude set for evaluator yet.");
        magnitude * self.scale
    }

    fn set_magnitude(&mut self, magnitude: f32) {
        self.magnitude = Some(magnitude)
    }

    fn get_aggregator(&self) -> ModAggregator {
        match self.mod_type {
            ModType::Additive => ModAggregator::additive(self.get_magnitude()),
            ModType::Multiplicative => ModAggregator::multiplicative(self.get_magnitude()),
            ModType::Overrule => ModAggregator::overrule(self.get_magnitude()),
        }
    }

    fn get_observer(&self) -> Option<Observer> {
        Some(Observer::new(meta_mutator_update::<A>))
    }
}

fn meta_mutator_update<T>(
    trigger: Trigger<OnCurrentValueChanged>,
    attributes: Query<&T>,
    mut mutators: Query<&mut Mutator>,
) where
    T: Component<Mutability = Mutable> + AttributeComponent, // The target attribute
{
    let actor_entity = trigger.target();
    let mutator_entity = trigger.observer();

    let Ok(attribute) = attributes.get(actor_entity) else {
        return;
    };
    let Ok(mut mutator) = mutators.get_mut(mutator_entity) else {
        return;
    };

    let new_val = attribute.get_current_value();
    mutator.0.set_magnitude(new_val);

    info!(
        "MetaMutator: [{}] changed to [{}]",
        type_name::<T>(),
        new_val,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameAbilityContainer;
    use crate::attribute;
    use crate::attributes::AttributeComponent;
    use crate::effects::EffectBuilder;
    use crate::mutator::ModType::Additive;
    use crate::mutator::{EvaluateMutator, Mutator, MutatorHelper};
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
