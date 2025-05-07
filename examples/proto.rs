#[warn(unused_imports)]
use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use death_by_attributes::actors::ActorBuilder;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::effects::EffectBuilder;
use death_by_attributes::modifiers::{EffectOf};
use death_by_attributes::modifiers::mutator::ModType::{Additive, Multiplicative};
use death_by_attributes::modifiers::mutator::Modifier;
use death_by_attributes::systems::{
    flag_dirty_modifier_nodes, pretty_print_tree, update_attribute_tree_system,
};
use death_by_attributes::{
    Actor, DeathByAttributesPlugin, Dirty, OnAttributeChanged, OnCurrentValueChanged, attribute,
};
use rand::Rng;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

attribute!(Health);
attribute!(MaxHealth);
attribute!(AttackPower);
attribute!(Strength);
attribute!(Agility);

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_plugins(LogPlugin {
            level: bevy::log::Level::DEBUG,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            pretty_print_tree::<Health>.run_if(on_timer(Duration::from_millis(1000))),
        )
        .run();
}

fn modify_tree(
    mut query: Query<Entity, With<Modifier<Health>>>,
    mut modifiers: Query<&mut Modifier<Health>>,
) {
    let mut rng = rand::rng();

    for entity in query.iter_mut() {
        if rng.random_range(0.0..100.0) < 25.0 {
            println!("Changed {entity}");
            if let Ok(mut modifier) = modifiers.get_mut(entity) {
                modifier.value = 2.0; //rng.random_range(0.0..10.0);
            }
        }
    }
}

fn setup(mut commands: Commands) {
    for _ in 0..20000 {
        let player = ActorBuilder::new(&mut commands.reborrow())
            .with_attribute::<Health>(0.0)
            .with_attribute::<MaxHealth>(0.0)
            .commit();

        commands
            .entity(player)
            .insert(Name::new("Player"))
            .with_related_entities::<EffectOf>(|commands| {
                commands
                    .spawn((
                        Name::new("E1"),
                        Dirty::<Health>::default(),
                        Health::new(0.0),
                    ))
                    .with_related_entities::<EffectOf>(|commands| {
                        commands.spawn((
                            Name::new("E1-M1"),
                            Dirty::<Health>::default(),
                            Modifier::<Health>::new(2.0),
                        ));
                        commands.spawn((
                            Name::new("E1-M2"),
                            Dirty::<Health>::default(),
                            Modifier::<Health>::new(6.0),
                        ));
                        commands.spawn((
                            Name::new("E1-M3"),
                            Dirty::<Health>::default(),
                            Modifier::<Health>::new(11.0),
                        ));
                        commands
                            .spawn((
                                Name::new("E1-E1"),
                                Dirty::<Health>::default(),
                                Health::new(0.0),
                            ))
                            .with_related_entities::<EffectOf>(|commands| {
                                commands.spawn((
                                    Name::new("E1-E1-M1"),
                                    Dirty::<Health>::default(),
                                    Modifier::<Health>::new(1.0),
                                ));
                                commands.spawn((
                                    Name::new("E1-E1-M2"),
                                    Dirty::<Health>::default(),
                                    Modifier::<Health>::new(1.0),
                                ));
                            });
                    });
                commands.spawn((
                    Name::new("E2"),
                    Dirty::<Health>::default(),
                    Modifier::<Health>::new(100.0),
                ));
                commands
                    .spawn((
                        Name::new("E3"),
                        Dirty::<Health>::default(),
                        Health::new(0.0),
                    ))
                    .with_related_entities::<EffectOf>(|commands| {
                        commands.spawn((
                            Name::new("E3-M1"),
                            Dirty::<Health>::default(),
                            Modifier::<Health>::new(1.0),
                        ));
                        commands.spawn((
                            Name::new("E3-M2"),
                            Dirty::<Health>::default(),
                            Modifier::<Health>::new(1.0),
                        ));
                    });
            });

        // Effect 1 - Passive Max Health Boost
        let effect_entity = commands.spawn_empty().id();
        EffectBuilder::new(player, effect_entity)
            .with_permanent_duration()
            .with_continuous_application()
            .modify_by_scalar::<Health>(9.0, Additive)
            .modify_by_scalar::<Health>(0.10, Multiplicative)
            .apply(&mut commands);

        // Effect 2 - Periodic Health Regen
        let effect_entity = commands.spawn_empty().id();
        EffectBuilder::new(player, effect_entity)
            .with_permanent_duration()
            .with_periodic_application(1.0)
            .modify_by_scalar::<Health>(5.0, Additive)
            .modify_by_scalar::<MaxHealth>(5.0, Additive)
            .apply(&mut commands);

        // Effect 3 - Instant
        let effect_entity = commands.spawn_empty().id();
        EffectBuilder::new(player, effect_entity)
            .with_instant_application()
            .modify_by_scalar::<Health>(10.0, Additive)
            .apply(&mut commands);
        println!("effect_entity: {}", effect_entity);
        
        // Effect 3 - Instant
        let effect_entity = commands.spawn_empty().id();
        EffectBuilder::new(player, effect_entity)
            .with_instant_application()
            .modify_by_scalar::<Health>(-35.0, Additive)
            .apply(&mut commands);
        println!("effect_entity: {}", effect_entity);
    }
}
