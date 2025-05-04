use attributes_macro::Attribute;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use criterion::BatchSize::SmallInput;
use criterion::*;
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::attributes::AttributeDef;
use death_by_attributes::effects::{
    Effect, EffectPeriodicTimer, EffectBuilder, GameEffectPeriod,
};
use death_by_attributes::evaluators::FixedEvaluator;
use death_by_attributes::mutator::ModType::Additive;
use death_by_attributes::mutator::{Mutator, StoredMutator};
use death_by_attributes::systems::{
    on_instant_effect_added, tick_effects_duration_timer, tick_effects_periodic_timer,
    update_base_values,
};
use death_by_attributes::{AttributeEntityMut, attribute};
use rand::{Rng, rng};
use std::time::{Duration, Instant};

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

attribute!(Health);
attribute!(HealthRegen);
attribute!(Mana);
attribute!(ManaRegen);

fn populate_world(mut command: Commands) {
    let mut rng = rand::rng();
    for _ in 0..10000 {
        let id = command
            .spawn((Health::new(0.0), Mana::default(), ManaRegen::default()))
            .id();

        for _ in 0..50 {
            let mutator = Mutator::new::<Health>(FixedEvaluator::new(1.0, Additive));

            let effect = Effect {
                modifiers: vec![StoredMutator(Box::new(mutator))],
            };

            let mut periodic_application = EffectPeriodicTimer::new(1.0);
            periodic_application
                .0
                .tick(Duration::from_millis(rng.random_range(0..2000)));

            command.spawn((ChildOf(id), effect, periodic_application));
        }
    }
}

fn populate_instant_effects(mut command: Commands) {
    let mut rng = rand::rng();
    for _ in 0..1000 {
        let id = command
            .spawn((Health::new(0.0), Mana::default(), ManaRegen::default()))
            .id();

        for _ in 0..5 {
            let mutator = Mutator::new::<Health>(FixedEvaluator::new(
                rng.random_range(-10.0..15.0),
                Additive,
            ));

            let effect = Effect {
                modifiers: vec![StoredMutator(Box::new(mutator))],
            };

            command.spawn((ChildOf(id), effect));
        }
    }
}

fn bench_on_instant_effect_added() -> App {
    let mut app = App::new();
    app.add_observer(on_instant_effect_added);
    app.add_systems(Update, populate_instant_effects);
    app
}

fn bench_update_base_values() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, populate_world);
    app.add_systems(Update, tick_effects_periodic_timer);
    app
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut world = World::new();

    c.bench_function("on_instant_effect_added", |b| {
        let mut app = bench_on_instant_effect_added();
        b.iter(|| {
            app.update();
        })
    });

    c.bench_function("update_base_values", |b| {
        let mut app = bench_update_base_values();
        app.update();

        b.iter(|| app.world_mut().run_system_once(update_base_values))
    });

    c.bench_function("tick_effects_duration_timer", |b| {
        b.iter(|| world.run_system_once(tick_effects_duration_timer))
    });

    c.bench_function("tick_effects_periodic_timer", |b| {
        b.iter(|| world.run_system_once(tick_effects_periodic_timer))
    });
}
