use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy::window::PresentMode;
use bevy_dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use death_by_attributes::abilities::{GameAbilityBuilder, GameAbilityContainer};
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::effects::{Effect, EffectBuilder, EffectPeriodicTimer};
use std::any::TypeId;

use bevy::ecs::relationship::Relationship;
use bevy::platform::collections::HashMap;
use bevy::ui::debug::print_ui_layout_tree;
use death_by_attributes::modifiers::{ModifierOf, Modifiers};
use death_by_attributes::{ActorEntityMut, DeathByAttributesPlugin, OnBaseValueChanged, attribute};
use rand::{Rng, rng};
use std::time::{Duration, Instant};

attribute!(Strength);
attribute!(Agility);

attribute!(Health);
attribute!(MaxHealth);
attribute!(HealthRegen);

attribute!(Mana);
attribute!(ManaRegen);

attribute!(AttackPower);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins) /*.set(LogPlugin {
            filter: "error,death_by_attributes=info".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }))*/
        .add_plugins(DeathByAttributesPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        //.add_plugins(WorldInspectorPlugin::new())
        /*.add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig::default(),
        })*/
        .add_systems(Startup, (setup_window, setup, setup_ui))
        .add_systems(Update, do_gameplay_stuff)
        .add_systems(
            Update,
            (display_attribute)
                .chain()
                .run_if(on_timer(Duration::from_millis(16))),
        )
        .add_systems(PreUpdate, inputs)
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .register_type::<Modifiers>()
        .register_type::<ModifierOf>()
        .register_type::<EffectPeriodicTimer>()
        .add_observer(clamp_health)
        .run();
}

#[derive(Component)]
struct UiFireballText;

#[derive(Component)]
struct Fireball;

fn setup_window(mut query: Query<&mut Window>) {
    for mut window in query.iter_mut() {
        window.present_mode = PresentMode::Immediate;
    }
}

fn setup(mut commands: Commands) {
    let mut rng = rand::rng();
    let mut ability_component = GameAbilityContainer::default();
    ability_component.grant_ability(
        "fireball".to_string(),
        GameAbilityBuilder::default()
            .with_cooldown(0.100)
            .with_cost::<Health>(-12.0)
            .with_activation(|_: ActorEntityMut, mut commands: Commands| {
                info!("fireball!");
                commands.spawn(Fireball);
            })
            .build(),
    );

    let player_entity = commands
        .spawn((
            Player,
            ability_component,
            Health::new(100.0),
            MaxHealth::new(100.0),
            HealthRegen::new(8.0),
            Mana::new(1000.0),
            ManaRegen::new(12.0),
            Strength::new(8.0),
            Agility::new(12.0),
        ))
        .insert(Name::new("Player"))
        .id();

    info!("Created Player entity {}", player_entity);

    // Effect 1 - Passive Max Health Boost
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_continuous_application()
        //.mutate_by_scalar::<MaxHealth>(10.0, Additive)
        //.mutate_by_scalar::<MaxHealth>(0.10, Multiplicative)
        .apply(&mut commands);

    // Effect 2 - Periodic Health Regen
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        //.mutate_by_scalar::<Health>(5.0, Additive)
        .apply(&mut commands);

    // Effect 3 - Instant
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_instant_application()
        //.mutate_by_scalar::<Health>(-35.0, Additive)
        .apply(&mut commands);

    // Effect 4
    /*AttributeBuilder::<AttackPower>::new(player_entity)
    .mutate_by_attribute::<Strength>(1.0, Additive)
    .mutate_by_attribute::<Health>(1.0, Additive)
    .build(&mut commands);*/

    /*for _ in 0..1000 {
        let effect_entity = commands.spawn_empty().id();
        let npc_entity = commands
            .spawn((
                Health::new(100.0),
                MaxHealth::new(1000.0),
                HealthRegen::new(2.0),
                Mana::new(1000.0),
            ))
            .id();

        for _ in 0..50 {
            EffectBuilder::new(npc_entity, effect_entity)
                .with_duration(rng.random_range(10.0..30.0))
                .with_periodic_application(rng.random_range(1.0..2.0))
                .mutate_by_scalar::<Health>(rng.random_range(1.0..20.0), Additive)
                .apply(&mut commands);
        }
    }*/
}

fn clamp_health(trigger: Trigger<OnBaseValueChanged>, mut query: Query<(&mut Health, &MaxHealth)>) {
    if let Ok((mut health, max_health)) = query.get_mut(trigger.target()) {
        let clamped_value = health.base_value().clamp(0.0, max_health.current_value());
        health.set_base_value(clamped_value);
    } else {
        println!("Incorrect entity target in clamp_health.")
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerHealthMarker;

#[derive(Component)]
struct EntityHealthMarker;

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(15.), Val::Px(400.)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((Text::new("Health"), PlayerHealthMarker));
            builder.spawn((Text::new("Entity Health"), EntityHealthMarker));
        });
}

fn display_attribute(
    q_player: Query<(&Health, &MaxHealth, &AttackPower), With<Player>>,
    mut q_health: Query<&mut Text, With<PlayerHealthMarker>>,
) {
    for (health, max_health, attack) in q_player.iter() {
        if let Ok(mut text) = q_health.single_mut() {
            text.0 = format!(
                "Values: Current [Base]
                Health: {:.1} [{:.1}]
                Max Health: {:.1} [{:.1}]
                {:?}",
                health.current_value(),
                health.base_value(),
                max_health.current_value(),
                max_health.base_value(),
                attack,
            );
        }
    }
}

fn inputs(
    mut q_player: Query<(ActorEntityMut, &mut GameAbilityContainer), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    commands: Commands,
) {
    if let Ok((entity_mut, mut abilities)) = q_player.single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            /*abilities
            .get_abilities_mut()
            .get_mut("fireball")
            .unwrap()
            .try_activate(entity_mut, commands);*/
        }
    }
}

fn do_gameplay_stuff() {
    std::thread::sleep(Duration::from_millis(10));
}
