use std::time::Instant;
use attributes_macro::Attribute;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use criterion::*;
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeDef;
use death_by_attributes::attributes::AttributeMut;
use death_by_attributes::effects::{GameEffectBuilder, GameEffectEvent, GameEffectPeriod};
use death_by_attributes::systems::{handle_apply_effect_events};
use death_by_attributes::{attribute, attribute_mut, AttributeEntityMut};
use rand::{Rng, rng};

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

attribute!(Health);
attribute!(HealthRegen);

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::rng();
    let mut world = World::default();
    for _ in 0..10000 {
        let id = world.spawn((Health::new(0.0), HealthRegen::new(10.0))).id();

        let effect = GameEffectBuilder::new()
            .with_duration(rng.random_range(100.0..300.0))
            .with_periodic_application(rng.random_range(20.0..100.0))
            .with_additive_modifier(rng.random_range(1.0..20.0), attribute_mut!(Health))
            .build();

        world.send_event(GameEffectEvent { entity: id, effect });
    }
    let _ = world.run_system_once(handle_apply_effect_events);

    c.bench_function("Direct Effect", |b| {
        b.iter(|| world.run_system_once(continuous_effect))
    });
    c.bench_function("Effect System", |b| {
        b.iter(|| world.run_system_once(update_attribute_base_value))
    });
}

fn continuous_effect(mut query: Query<(&mut Health, &HealthRegen)>, time: Res<Time>) {
    for (mut health, regen) in query.iter_mut() {
        health.base_value += regen.current_value * time.delta_secs();
    }
}

pub fn update_attribute_base_value(mut query: Query<(AttributeEntityMut)>) {
    for (mut entity_mut) in query.iter_mut() {
        for effect in &container.effects {
            if let Some(period) = &effect.periodic_application {
                match period {
                    GameEffectPeriod::Periodic(timer) => {
                        if timer.just_finished() {
                            for modifier in &effect.modifiers {
                                let _ = modifier.0.apply(&mut entity_mut);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}