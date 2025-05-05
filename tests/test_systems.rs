use bevy::app::App;
use bevy::prelude::Reflect;
use bevy::prelude::{Component, Update};
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::effects::{EffectBuilder, EffectPeriodicTimer};
use death_by_attributes::evaluators::FixedEvaluator;
use death_by_attributes::mutator::ModType::Additive;
use death_by_attributes::mutator::{ModAggregator, Mutator, MutatorHelper};
use death_by_attributes::systems::{
    on_attribute_mutation_changed, on_base_value_changed, on_duration_effect_applied,
    on_duration_effect_removed, trigger_periodic_effects,
};
use death_by_attributes::{CachedMutations, attribute};
use std::time::Duration;

attribute!(TestA);

#[test]
fn test_update_base_values() {
    let mut app = App::new();
    app.add_systems(Update, trigger_periodic_effects);
    app.add_observer(on_duration_effect_applied);
    app.add_observer(on_base_value_changed);
    app.insert_resource(CachedMutations::default());

    let player = app.world_mut().spawn(TestA::new(0.0)).id();
    let effect = app.world_mut().spawn_empty().id();

    EffectBuilder::new(player, effect)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        .mutate_by_scalar::<TestA>(42.0, Additive)
        .apply(&mut app.world_mut().commands());

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(0.0, value.base_value);

    app.world_mut().flush();

    let mut timer = app
        .world_mut()
        .get_mut::<EffectPeriodicTimer>(effect)
        .unwrap();
    timer.0.tick(Duration::from_secs(10));

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(0.0, value.base_value);

    app.update();

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(42.0, value.base_value);

    app.update();

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(84.0, value.base_value);
}

#[test]
fn check_mutation_cache() {
    let mut app = App::new();
    app.insert_resource(CachedMutations::default());
    app.add_observer(on_base_value_changed);
    app.add_observer(on_duration_effect_applied);
    app.add_observer(on_duration_effect_removed);

    let player = app.world_mut().spawn(TestA::new(0.0)).id();
    let effect = app.world_mut().spawn_empty().id();

    let mutator = MutatorHelper::new::<TestA>(FixedEvaluator::new(42.0, Additive));
    let mutator = Mutator::new(mutator);

    EffectBuilder::new(player, effect)
        .with_permanent_duration()
        .with_continuous_application()
        .mutate_by_scalar::<TestA>(42.0, Additive)
        .apply(&mut app.world_mut().commands());

    app.update();

    {
        let mut res = app
            .world_mut()
            .get_resource_mut::<CachedMutations>()
            .unwrap();
        let type_map = res.evaluators.entry(player).or_default();
        let (_, stored_aggregator) = type_map
            .entry(mutator.target())
            .or_insert((mutator.clone(), ModAggregator::default()));

        assert_eq!(42.0, stored_aggregator.additive);
    }

    app.update();

    {
        let res = app.world().get_resource::<CachedMutations>().unwrap();
        let type_map = res.evaluators.get(&player).unwrap();
        let (_, stored_aggregator) = type_map.get(&mutator.target()).unwrap();

        assert_eq!(42.0, stored_aggregator.additive);
    }

    // Despawn the effect and check that the cache is updated
    app.world_mut().despawn(effect);

    {
        let res = app.world().get_resource::<CachedMutations>().unwrap();
        let type_map = res.evaluators.get(&player).unwrap();
        let (_, stored_aggregator) = type_map.get(&mutator.target()).unwrap();

        assert_eq!(0.0, stored_aggregator.additive);
    }
}

#[test]
fn test_update_current_values() {
    let mut app = App::new();
    app.add_observer(on_base_value_changed);
    app.add_observer(on_duration_effect_applied);
    app.add_observer(on_duration_effect_removed);
    app.add_observer(on_attribute_mutation_changed);
    app.insert_resource(CachedMutations::default());

    let player = app.world_mut().spawn(TestA::new(0.0)).id();
    let effect = app.world_mut().spawn_empty().id();

    EffectBuilder::new(player, effect)
        .with_permanent_duration()
        .with_continuous_application()
        .mutate_by_scalar::<TestA>(42.0, Additive)
        .apply(&mut app.world_mut().commands());

    app.update();

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(0.0, value.base_value);
    assert_eq!(42.0, value.current_value);

    // Despawn the effect and confirm the attribute values
    app.world_mut().despawn(effect);

    app.update();

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(0.0, value.base_value);
    assert_eq!(0.0, value.current_value);
}
