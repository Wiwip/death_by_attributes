use attributes_macro::Attribute;
use bevy::ecs::component::{ComponentHooks, StorageType};
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use bevy::text::Text;
use bevy::time::common_conditions::on_timer;
use death_by_attributes::abilities::{GameAbilityBuilder, GameAbilityComponent};
use death_by_attributes::attributes::GameAttribute;
use death_by_attributes::attributes::GameAttributeMarker;
use death_by_attributes::effect::{GameEffectBuilder, GameEffectContainer, GameEffectEvent};
use death_by_attributes::events::CurrentValueUpdateTrigger;
use death_by_attributes::modifiers::{ModifierType, ScalarModifier};
use death_by_attributes::systems::{
    handle_apply_effect_events, tick_active_effects, update_attribute_base_value,
    update_attribute_current_value,
};
use death_by_attributes::{
    BaseValueUpdate, CurrentValueUpdate, DeathByAttributesPlugin,
};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_systems(Startup, (register_types, setup, setup_ui))
        .add_systems(
            Update,
            display_attribute.run_if(on_timer(Duration::from_millis(16))),
        )
        .add_systems(Update, inputs)
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        //.add_systems(PostBaseValueUpdate, on_health_changed)
        .observe(clamp_health)
        .run();
}

fn register_types(type_registry: ResMut<AppTypeRegistry>) {
    type_registry.write().register::<Health>();
    type_registry.write().register::<HealthCap>();
    type_registry.write().register::<HealthRegen>();
    type_registry.write().register::<Mana>();
}

fn setup(mut commands: Commands, mut event_writer: EventWriter<GameEffectEvent>) {
    let mut ability_component = GameAbilityComponent::default();
    ability_component.grant_ability(
        "test".to_string(),
        GameAbilityBuilder::default()
            .with_cooldown(1.0)
            .with_cost::<Mana>(12.0)
            .build(),
    );

    let id = commands
        .spawn((
            Player,
            ability_component,
            GameEffectContainer::default(),
            Health::new(100.0),
            HealthCap::new(100.0),
            HealthRegen::new(8.0),
            Mana::new(100.0),
        ))
        .id();

    let health_regen = GameEffectBuilder::new()
        .with_permanent_duration()
        .with_realtime_application()
        .with_meta_modifier::<Health, HealthRegen>(ModifierType::Additive)
        .build();

    event_writer.send(GameEffectEvent {
        entity: id,
        effect: health_regen,
    });

    let event_effect = GameEffectBuilder::new()
        .with_permanent_duration()
        .with_scalar_modifier(ScalarModifier::additive::<HealthCap>(20.0))
        .build();

    event_writer.send(GameEffectEvent {
        entity: id,
        effect: event_effect,
    });
}

fn clamp_health(
    trigger: Trigger<CurrentValueUpdateTrigger>,
    mut query: Query<(&mut Health, &HealthCap)>,
) {
    let (mut health, max_health) = query.get_mut(trigger.entity()).unwrap();
    health.base_value = health.base_value.clamp(0.0, max_health.value.current_value);
    health.current_value = health.base_value;
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct UiHealthText;

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let section = TextSection::new(
        "",
        TextStyle {
            font_size: 18.0,
            ..default()
        },
    );

    let text_bundle = TextBundle::from_sections([
        section.clone(),
        section.clone(),
        section.clone(),
        section.clone(),
        section.clone(),
    ]);
    commands.spawn((UiHealthText, text_bundle));
}

fn display_attribute(
    q_player: Query<
        (
            &Health,
            Option<&HealthCap>,
            Option<&HealthRegen>,
            Option<&Mana>,
            &GameEffectContainer,
        ),
        With<Player>,
    >,
    mut q_ui: Query<&mut Text, With<UiHealthText>>,
) {
    for (health, health_cap, health_regen, mana, gec) in q_player.iter() {
        for mut ui in q_ui.iter_mut() {
            ui.sections[0].value = format!(
                "Health: {:.1} [{:.1}]",
                health.value.current_value, health.value.base_value
            );
            if let Some(health_cap) = health_cap {
                ui.sections[1].value = format!(
                    "\nMaxHealth: {:.1} [{:.1}]",
                    health_cap.value.current_value, health_cap.value.base_value
                );
            }
            if let Some(health_regen) = health_regen {
                ui.sections[2].value = format!(
                    "\nHealth Regen: {:.1} [{:.1}]",
                    health_regen.value.current_value, health_regen.value.base_value
                );
            }
            if let Some(mana) = mana {
                ui.sections[3].value = format!(
                    "\nMana: {:.1} [{:.1}]",
                    mana.value.current_value, mana.value.base_value
                );
            }
            ui.sections[4].value = format!("\n{:?}", gec.effects);
        }
    }
}

#[derive(Component, Attribute, Reflect, Deref, DerefMut)]
pub struct Health {
    pub value: GameAttribute,
}

#[derive(Component, Attribute, Reflect, Deref, DerefMut)]
pub struct HealthCap {
    pub value: GameAttribute,
}

#[derive(Component, Attribute, Reflect, Deref, DerefMut)]
pub struct HealthRegen {
    pub value: GameAttribute,
}

#[derive(Component, Attribute, Reflect, Deref, DerefMut)]
pub struct Mana {
    pub value: GameAttribute,
}

pub fn inputs(
    query: Query<(&GameAbilityComponent), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(player) = query.get_single() {
        if keys.just_pressed(KeyCode::Space) {
            println!("try_activate");
            player.try_activate("test".to_string());
        }
    }
}
