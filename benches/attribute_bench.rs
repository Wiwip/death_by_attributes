use attributes_macro::Attribute;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use criterion::{criterion_group, criterion_main, Criterion};
use death_by_attributes::attributes::GameAttribute;
use death_by_attributes::attributes::GameAttributeMarker;
use death_by_attributes::context::GameAttributeContextMut;
use std::hint::black_box;

#[derive(Component, Attribute, Reflect, Deref, DerefMut)]
pub struct SomeAttribute {
    pub value: GameAttribute,
}

fn get_attribute_direct_bench(mut query: Query<&mut SomeAttribute>) {
    for mut attr in query.iter_mut() {
        attr.base_value += 1.;
    }
}

fn get_attribute_reflect_bench(
    mut query: Query<EntityMut, With<SomeAttribute>>,
    mut context: GameAttributeContextMut,
) {
    for entity_mut in query.iter_mut() {
        let attr = context.get_mut::<SomeAttribute>(&entity_mut).unwrap();
        attr.base_value += 1.;
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut world = World::new();
    world.spawn_batch((0..1000).map(|_| SomeAttribute::new(100000.)));

    let mut schedule_direct = Schedule::default();
    schedule_direct.add_systems(get_attribute_direct_bench);
    c.bench_function("Get attribute (direct)", |b| {
        b.iter(|| schedule_direct.run(&mut world))
    });

    // Must register the attribute manually
    let type_registry = AppTypeRegistry::default();
    type_registry.write().register::<SomeAttribute>();
    world.insert_resource(type_registry);

    let mut schedule_reflect = Schedule::default();
    schedule_reflect.add_systems(get_attribute_reflect_bench);
    c.bench_function("Get attribute (reflect)", |b| {
        b.iter(|| schedule_reflect.run(&mut world))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
