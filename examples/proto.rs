#[warn(unused_imports)]
use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use rand::Rng;
use root_attribute::actors::ActorBuilder;
use root_attribute::attributes::AttributeComponent;
use root_attribute::effects::EffectBuilder;
use root_attribute::modifiers::EffectOf;
use root_attribute::modifiers::scalar::ModType::{Additive, Multiplicative};
use root_attribute::modifiers::scalar::Modifier;
use root_attribute::systems::{
    flag_dirty_modifier_nodes, pretty_print_tree_system, update_effect_tree_system,
};
use root_attribute::{
    Actor, DeathByAttributesPlugin, Dirty, OnAttributeChanged, OnCurrentValueChanged, attribute,
};
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
        /*.add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))*/
        .add_plugins(LogPlugin {
            level: bevy::log::Level::DEBUG,
            ..default()
        })
        .add_systems(Startup, setup)
        //.add_systems(Update, modify_tree)
        .add_systems(
            PostUpdate,
            pretty_print_tree_system::<Health>.run_if(on_timer(Duration::from_millis(1000))),
        )
        .run();
}

fn modify_tree(
    mut query: Query<Entity, With<Modifier<Health>>>,
    mut modifiers: Query<&mut Modifier<Health>>,
) {
    let mut rng = rand::rng();

    for entity in query.iter_mut() {
        if rng.random_range(0.0..100.0) < 0.001 {
            if let Ok(mut modifier) = modifiers.get_mut(entity) {
                modifier.value = rng.random_range(0.0..100.0);
            }
        }
    }
}

fn setup(mut commands: Commands) {
    for _ in 0..1 {
        let player = ActorBuilder::new(&mut commands.reborrow())
            .with::<Health>(0.0)
            .with::<MaxHealth>(0.0)
            .commit();

        commands.entity(player).insert(Name::new("Player"));
        //.with_related_entities::<EffectOf>(|commands| {

        /*
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
        });*/

        // Effect 1 - Passive Max Health Boost
        /*EffectBuilder::new(player, &mut commands.reborrow())
        .with_permanent_duration()
        .with_continuous_application()
        .with_name("Health Regen".into())
        .modify_by_scalar::<Health>(9.0, Additive)
        .modify_by_scalar::<Health>(0.10, Multiplicative)
        .commit();*/

        // Effect 2 - Periodic Health Regen
        EffectBuilder::new(player, &mut commands.reborrow())
            .with_permanent_duration()
            .with_periodic_application(1.0)
            .modify_by_scalar::<Health>(5.0, Additive)
            .modify_by_scalar::<MaxHealth>(5.0, Additive)
            .commit();
        /*
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
        println!("effect_entity: {}", effect_entity);*/
    }
}
