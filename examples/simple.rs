use attributes_macro::Attribute;
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use death_by_attributes::abilities::{GameAbilityBuilder, GameAbilityContainer};
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::attributes::AttributeDef;
use death_by_attributes::effects::EffectBuilder;
use death_by_attributes::{
    AttributeEntityMut, CurrentValueChanged, DeathByAttributesPlugin, attribute,
};
use rand::Rng;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            display_attribute.run_if(on_timer(Duration::from_millis(16))),
        )
        .add_systems(
            Update,
            display_attribute_entity.run_if(on_timer(Duration::from_millis(16))),
        )
        .add_systems(PreUpdate, inputs)
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .add_observer(clamp_health)
        .run();
}

#[derive(Component)]
struct UiFireballText;

#[derive(Component)]
struct Fireball;

fn setup(mut commands: Commands) {
    let mut rng = rand::rng();
    let mut ability_component = GameAbilityContainer::default();
    ability_component.grant_ability(
        "fireball".to_string(),
        GameAbilityBuilder::default()
            .with_cooldown(0.100)
            .with_cost::<Health>(-12.0)
            .with_activation(|_: AttributeEntityMut, mut commands: Commands| {
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
            HealthCap::new(200.0),
            HealthRegen::new(8.0),
            Mana::new(1000.0),
            ManaRegen::new(12.0),
        ))
        .id();

    // Effect 1 - Passive Max Health Boost
    EffectBuilder::new(player_entity)
        .with_permanent_duration()
        .with_continuous_application()
        .with_additive_modifier::<HealthCap>(9.0)
        .with_multiplicative_modifier::<HealthCap>(0.1)
        .build(&mut commands);

    // Effect 2 - Periodic Health Regen
    EffectBuilder::new(player_entity)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        .with_additive_modifier::<Health>(5.0)
        .build(&mut commands);

    // Effect 3 - Instant
    EffectBuilder::new(player_entity)
        .with_instant_application()
        .with_additive_modifier::<Health>(-35.0)
        .build(&mut commands);

    for _ in 0..1000 {
        let npc_entity = commands
            .spawn((
                Health::new(100.0),
                HealthCap::new(1000.0),
                HealthRegen::new(2.0),
                Mana::new(1000.0),
            ))
            .id();

        for _ in 0..50 {
            EffectBuilder::new(npc_entity)
                .with_duration(rng.random_range(100.0..300.0))
                .with_periodic_application(rng.random_range(20.0..100.0))
                .with_additive_modifier::<Health>(rng.random_range(1.0..20.0))
                .build(&mut commands);
        }
    }
}

fn clamp_health(
    trigger: Trigger<CurrentValueChanged>,
    mut query: Query<(&mut Health, &HealthCap)>,
) {
    if let Ok((mut health, max_health)) = query.get_mut(trigger.target()) {
        health.base_value = health.base_value.clamp(0.0, max_health.current_value);
        health.current_value = health.base_value;
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
            margin: UiRect::axes(Val::Px(15.), Val::Px(5.)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((Text::new("Health"), PlayerHealthMarker));
            builder.spawn((Text::new("Entity Health"), EntityHealthMarker));
        });
}

fn display_attribute(
    q_player: Query<&Health, With<Player>>,
    mut q_health: Query<&mut Text, With<PlayerHealthMarker>>,
) {
    for (health) in q_player.iter() {
        if let Ok(mut text) = q_health.single_mut() {
            text.0 = format!(
                "Values: Current [Base]\n\
                Health: {:.1} [{:.1}]",
                health.current_value, health.base_value
            );
        }
    }
}

fn display_attribute_entity(
    q_entity: Query<&HealthCap, With<Player>>,
    mut q_health: Query<&mut Text, With<EntityHealthMarker>>,
) {
    for (max_health) in q_entity.iter() {
        if let Ok(mut text) = q_health.single_mut() {
            text.0 = format!(
                "Max Health: {:.1} [{:.1}]",
                max_health.current_value, max_health.base_value
            );
        }
    }
}

#[derive(Component, Attribute, Default, Clone, Reflect, Deref, DerefMut, Debug)]
#[require(GameAbilityContainer)]
pub struct Test {
    pub attribute: AttributeDef,
}

attribute!(Health);
attribute!(HealthCap);
attribute!(HealthRegen);

attribute!(Mana);
attribute!(ManaRegen);

pub fn inputs(
    mut q_player: Query<(AttributeEntityMut, &mut GameAbilityContainer), With<Player>>,
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
