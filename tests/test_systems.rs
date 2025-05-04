use bevy::app::App;
use bevy::asset::io::ErasedAssetWriter;
use bevy::prelude::{ChildOf, Component};
use bevy::prelude::{Deref, Update};
use bevy::prelude::{DerefMut, IntoScheduleConfigs};
use bevy::prelude::{Reflect, Res, World};
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::attributes::AttributeDef;
use death_by_attributes::effects::{
    Effect, EffectDuration, EffectPeriodicTimer, GameEffectDuration, MutationAggregatorCache,
};
use death_by_attributes::evaluators::FixedEvaluator;
use death_by_attributes::mutator::ModType::Additive;
use death_by_attributes::mutator::{EvaluateMutator, Mutator, StoredMutator};
use death_by_attributes::systems::{
    on_effect_removed, update_base_values, update_current_values,
};
use death_by_attributes::{AttributeUpdate, attribute};
use std::time::Duration;

attribute!(TestA);

#[test]
fn test_update_base_values() {
    let mut app = App::new();
    app.add_systems(Update, update_base_values);
    app.insert_resource(MutationAggregatorCache::default());

    let entity = app.world_mut().spawn(TestA::new(0.0)).id();

    let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
    let effect = Effect {
        modifiers: vec![StoredMutator(Box::new(mutator))],
    };

    let mut timer = EffectPeriodicTimer::new(1.0);
    timer.0.tick(Duration::from_secs(10));

    app.world_mut()
        .spawn((ChildOf(entity), effect, EffectDuration::new(100.0), timer));

    // Check that the dirty bool is properly set on the current value when the base value changes
    let res = app
        .world()
        .get_resource::<MutationAggregatorCache>()
        .unwrap();
    {
        let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
        let dirty = res
            .is_current_value_dirty(entity, mutator.target())
            .unwrap();

        assert_eq!(true, dirty);
    }

    app.update();

    let value = app.world().get::<TestA>(entity).unwrap();
    assert_eq!(42.0, value.base_value);

    app.update();

    let value = app.world().get::<TestA>(entity).unwrap();
    assert_eq!(84.0, value.base_value);
}

#[test]
fn check_mutation_cache() {
    let mut app = App::new();
    app.insert_resource(MutationAggregatorCache::default());
    app.add_systems(Update, update_current_values);
    app.add_observer(on_effect_removed);

    let entity = app.world_mut().spawn(TestA::new(0.0)).id();

    let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
    let effect = Effect {
        modifiers: vec![StoredMutator(Box::new(mutator))],
    };

    // We spawn an effect targeting our entity and verify that the cache is updated
    let effect_entity = app
        .world_mut()
        .spawn((ChildOf(entity), effect, EffectDuration::new(100.0)))
        .id();

    {
        let res = app
            .world()
            .get_resource::<MutationAggregatorCache>()
            .unwrap();
        let type_map = res.evaluators.get(&entity).unwrap();
        let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
        let (_, stored_aggregator, _, _) = type_map.get(&mutator.target()).unwrap();

        let dirty = res
            .is_current_value_dirty(entity, mutator.target())
            .unwrap();

        assert_eq!(42.0, stored_aggregator.additive);
        assert_eq!(true, dirty);
    }

    app.update();

    {
        let res = app
            .world()
            .get_resource::<MutationAggregatorCache>()
            .unwrap();
        let type_map = res.evaluators.get(&entity).unwrap();
        let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
        let (_, stored_aggregator, _, _) = type_map.get(&mutator.target()).unwrap();

        let dirty = res
            .is_current_value_dirty(entity, mutator.target())
            .unwrap();

        assert_eq!(42.0, stored_aggregator.additive);
        assert_eq!(false, dirty);
    }

    // Despawn the effect and check that the cache is updated
    app.world_mut().despawn(effect_entity);

    {
        let res = app
            .world()
            .get_resource::<MutationAggregatorCache>()
            .unwrap();
        let type_map = res.evaluators.get(&entity).unwrap();
        let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
        let (_, stored_aggregator, _, _) = type_map.get(&mutator.target()).unwrap();

        let dirty = res
            .is_current_value_dirty(entity, mutator.target())
            .unwrap();

        assert_eq!(0.0, stored_aggregator.additive);
        assert_eq!(true, dirty);
    }

    app.update();

    let res = app
        .world()
        .get_resource::<MutationAggregatorCache>()
        .unwrap();
    {
        let type_map = res.evaluators.get(&entity).unwrap();
        let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
        let (_, stored_aggregator, _, _) = type_map.get(&mutator.target()).unwrap();

        let dirty = res
            .is_current_value_dirty(entity, mutator.target())
            .unwrap();

        assert_eq!(0.0, stored_aggregator.additive);
        assert_eq!(false, dirty);
    }

    app.update();

    let res = app
        .world()
        .get_resource::<MutationAggregatorCache>()
        .unwrap();
    {
        let type_map = res.evaluators.get(&entity).unwrap();
        let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
        let (_, stored_aggregator, _, _) = type_map.get(&mutator.target()).unwrap();

        let dirty = res
            .is_current_value_dirty(entity, mutator.target())
            .unwrap();

        assert_eq!(0.0, stored_aggregator.additive);
        assert_eq!(false, dirty);
    }
}

#[test]
fn test_update_current_values() {
    let mut app = App::new();
    app.add_systems(Update, update_current_values);
    app.insert_resource(MutationAggregatorCache::default());
    app.add_observer(on_effect_removed);

    let entity = app.world_mut().spawn(TestA::new(0.0)).id();

    let mutator = Mutator::new::<TestA>(FixedEvaluator::new(42.0, Additive));
    let effect = Effect {
        modifiers: vec![StoredMutator(Box::new(mutator))],
    };

    let effect_entity = app
        .world_mut()
        .spawn((ChildOf(entity), effect, EffectDuration::new(100.0)))
        .id();

    app.update();

    let value = app.world().get::<TestA>(entity).unwrap();
    assert_eq!(0.0, value.base_value);
    assert_eq!(42.0, value.current_value);

    app.update();

    // Despawn the effect and verify the attribute values
    app.world_mut().despawn(effect_entity);

    app.update();

    let value = app.world().get::<TestA>(entity).unwrap();
    assert_eq!(0.0, value.base_value);
    assert_eq!(0.0, value.current_value);
}
