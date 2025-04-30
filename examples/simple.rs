use attributes_macro::Attribute;
use bevy::ecs::component::{ComponentHooks, StorageType};
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use death_by_attributes::abilities::AbilityActivationFn;

use death_by_attributes::attributes::AttributeDef;

use death_by_attributes::effects::{
    GameEffect, GameEffectBuilder, GameEffectContainer, GameEffectEvent, GameEffectPeriod,
};

use death_by_attributes::evaluators::BoxAttributeModEvaluator;
use death_by_attributes::effects::GameEffectDuration::{Instant, Permanent};
use death_by_attributes::effects::GameEffectPeriod::Periodic;
use death_by_attributes::modifiers::ModType::{Additive, Multiplicative};
use death_by_attributes::modifiers::{AttributeMod, AttributeRef, BoxEditableAttribute, ModType};
use death_by_attributes::systems::{
    handle_apply_effect_events, tick_active_effects, update_attribute_base_value,
};
use death_by_attributes::{
    BaseValueUpdate, CurrentValueUpdate, CurrentValueUpdateTrigger, DeathByAttributesPlugin,
    attribute, attribute_field, modifiers,
};
use std::time::Duration;
use rand::{random, Rng};

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

fn setup(mut commands: Commands, mut event_writer: EventWriter<GameEffectEvent>) {
    //let mut ability_component = GameAbilityComponent::default();
    /*ability_component.grant_ability(
        "fireball".to_string(),
        GameAbilityBuilder::default()
            .with_cooldown(0.250)
            .with_cost::<Mana>(-12.0)
            .with_activation(|mut commands: Commands| {
                info!("fireball!");
                commands.spawn(Fireball);
            })
            .build(),
    );

    ability_component.grant_ability(
        "sprint".to_string(),
        GameAbilityBuilder::default()
            .with_cooldown(2.0)
            .with_effect(
                GameEffectBuilder::new()
                    .with_duration(4.0)
                    .with_scalar_modifier::<Health>(0.40, ModifierType::Multiplicative)
                    .build(),
                GameEffectTarget::OwnUnit,
            )
            .build(),
    );*/

    let entity = commands
        .spawn((
            Player,
            //ability_component,
            GameEffectContainer::default(),
            Health::new(100.0),
            HealthCap::new(1000.0),
            HealthRegen::new(8.0),
            Mana::new(1000.0),
        ))
        .id();


    let effect = GameEffectBuilder::new()
        .with_permanent_duration()
        .with_continuous_application()
        .with_additive_modifier(100.0, attribute_field!(Health))
        .with_multiplicative_modifier(0.1, attribute_field!(Health))
        .with_multiplicative_modifier(0.1, attribute_field!(HealthCap))
        .build();

    event_writer.write(GameEffectEvent { entity, effect });

    let mut rng = rand::thread_rng();

    for _ in 0..10 {
        let entity = commands
            .spawn((
                //ability_component,
                GameEffectContainer::default(),
                Health::new(100.0),
                HealthCap::new(1000000.0),
                HealthRegen::new(2.0),
                Mana::new(1000.0),
            ))
            .id();

        for _ in 0..50 {
            let effect = GameEffectBuilder::new()
                .with_duration(rng.gen_range(100.0..300.0))
                .with_periodic_application(rng.gen_range(20.0..100.0))
                .with_additive_modifier(3.0, attribute_field!(Health))
                .build();

            event_writer.write(GameEffectEvent { entity, effect });
        }
    }

    commands.spawn((
        //GameAbilityComponent::default(),
        GameEffectContainer::default(),
        Health::new(100.0),
        HealthCap::new(1000.0),
    ));
}

fn clamp_health(
    trigger: Trigger<CurrentValueUpdateTrigger>,
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
        if let Ok(mut text) = q_health.get_single_mut() {
            text.0 = format!(
                "Health: {:.1} [{:.1}]",
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
        if let Ok(mut text) = q_health.get_single_mut() {
            text.0 = format!(
                "Max Health: {:.1} [{:.1}]",
                max_health.current_value, max_health.base_value
            );
        }
    }
}

attribute!(Health);
attribute!(HealthCap);
attribute!(HealthRegen);
attribute!(Mana);

pub fn inputs(
    mut q_player: Query<EntityMut, With<Player>>,
    mut q_entities: Query<EntityMut, (Without<Player>, With<Health>)>,
    keys: Res<ButtonInput<KeyCode>>,
    commands: Commands,
) {
    if let Ok(player) = q_player.single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            if let Ok(entity) = q_entities.single_mut() {
                //context.try_activate(&player, "fireball".to_string(), commands);
            }
        }
    }
}
