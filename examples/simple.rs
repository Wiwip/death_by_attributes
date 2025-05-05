

use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use death_by_attributes::abilities::{GameAbilityBuilder, GameAbilityContainer};
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::effects::{AffectedBy, EffectBuilder, EffectPeriodicTimer, EffectTarget};
use death_by_attributes::mutator::ModType::{Additive, Multiplicative};
use death_by_attributes::mutator::{EffectMutators, Mutating};
use death_by_attributes::{
    ActorEntityMut, DeathByAttributesPlugin, OnCurrentValueChanged, attribute,
};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(WorldInspectorPlugin::new())
        //.add_plugins(FrameTimeDiagnosticsPlugin::default())
        //.add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Startup, (setup_window, setup, setup_ui))
        .add_systems(Update, sleep)
        .add_systems(
            Update,
            (display_attribute, display_attribute_entity)
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
        .register_type::<AffectedBy>()
        .register_type::<EffectTarget>()
        .register_type::<EffectMutators>()
        .register_type::<Mutating>()
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
            MaxHealth::new(200.0),
            HealthRegen::new(8.0),
            Mana::new(1000.0),
            ManaRegen::new(12.0),
        ))
        .insert(Name::new("Player"))
        .id();

    info!("Created Player entity {}", player_entity);

    // Effect 1 - Passive Max Health Boost
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_continuous_application()
        .mutate_by_scalar::<MaxHealth>(10.0, Additive)
        .mutate_by_scalar::<MaxHealth>(0.10, Multiplicative)
        .apply(&mut commands);

    // Effect 2 - Periodic Health Regen
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        .mutate_by_scalar::<Health>(5.0, Additive)
        .apply(&mut commands);

    // Effect 3 - Instant
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_instant_application()
        .mutate_by_scalar::<Health>(-35.0, Additive)
        .apply(&mut commands);

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

fn clamp_health(
    trigger: Trigger<OnCurrentValueChanged>,
    mut query: Query<(&mut Health, &MaxHealth)>,
) {
    if let Ok((mut health, max_health)) = query.get_mut(trigger.target()) {
        health.base_value = health.base_value.clamp(0.0, max_health.get_current_value());
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
    q_player: Query<(&Health, &MaxHealth), With<Player>>,
    mut q_health: Query<&mut Text, With<PlayerHealthMarker>>,
) {
    for (health, max_health) in q_player.iter() {
        if let Ok(mut text) = q_health.single_mut() {
            text.0 = format!(
                "Values: Current [Base]\n\
                Health: {:.1} [{:.1}]\n\
                Max Health: {:.1} [{:.1}]",
                health.get_current_value(),
                health.get_base_value(),
                max_health.get_current_value(),
                max_health.get_base_value()
            );
        }
    }
}

fn display_attribute_entity(
    q_entity: Query<&MaxHealth, With<Player>>,
    mut q_health: Query<&mut Text, With<EntityHealthMarker>>,
) {
    for _max_health in q_entity.iter() {
        if let Ok(_text) = q_health.single_mut() {
            //text.0 = format!("Effects:",);
        }
    }
}

attribute!(Health);
attribute!(MaxHealth);
attribute!(HealthRegen);

attribute!(Mana);
attribute!(ManaRegen);

fn inputs(
    mut q_player: Query<(ActorEntityMut, &mut GameAbilityContainer), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    commands: Commands,
) {
    if let Ok((entity_mut, mut abilities)) = q_player.single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            abilities
                .get_abilities_mut()
                .get_mut("fireball")
                .unwrap()
                .try_activate(entity_mut, commands);
        }
    }
}

fn sleep() {
    std::thread::sleep(Duration::from_millis(5));
}
