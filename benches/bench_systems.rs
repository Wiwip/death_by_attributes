use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use criterion::*;
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::effects::EffectBuilder;
use death_by_attributes::mutator::ModType::Additive;
use death_by_attributes::systems::{
    on_instant_effect_applied, tick_effects_duration_timer, tick_effects_periodic_timer,
    trigger_periodic_effects,
};
use death_by_attributes::{CachedMutations, attribute};
use rand::Rng;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

attribute!(Health);
attribute!(HealthRegen);
attribute!(Mana);
attribute!(ManaRegen);

fn populate_world(mut command: Commands) {
    let mut rng = rand::rng();
    for _ in 0..1000 {
        let player = command
            .spawn((Health::new(0.0), Mana::default(), ManaRegen::default()))
            .id();

        let effect = command.spawn_empty().id();

        for _ in 0..50 {
            EffectBuilder::new(player, effect)
                .with_permanent_duration()
                .with_periodic_application(1.0)
                .mutate_by_scalar::<Health>(rng.random_range(0.0..42.0), Additive)
                .apply(&mut command);
        }
    }
}

fn populate_instant_effects(mut command: Commands) {
    let mut rng = rand::rng();
    for _ in 0..1000 {
        let player = command.spawn(Health::new(0.0)).id();
        let effect = command.spawn_empty().id();

        for _ in 0..50 {
            EffectBuilder::new(player, effect)
                .with_permanent_duration()
                .with_periodic_application(1.0)
                .mutate_by_scalar::<Health>(rng.random_range(0.0..42.0), Additive)
                .apply(&mut command);
        }
    }
}

fn bench_on_instant_effect_added() -> App {
    let mut app = App::new();
    app.insert_resource(CachedMutations::default());
    app.add_observer(on_instant_effect_applied);
    app.add_systems(Startup, populate_instant_effects);
    app
}

fn bench_update_base_values() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(CachedMutations::default());
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

        b.iter(|| app.world_mut().run_system_once(trigger_periodic_effects))
    });

    c.bench_function("tick_effects_duration_timer", |b| {
        b.iter(|| world.run_system_once(tick_effects_duration_timer))
    });

    c.bench_function("tick_effects_periodic_timer", |b| {
        b.iter(|| world.run_system_once(tick_effects_periodic_timer))
    });
}
