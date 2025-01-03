use attributes_macro::Attribute;
use bevy::ecs::component::{ComponentHooks, StorageType};
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use death_by_attributes::abilities::{
    AbilityActivationFn, GameAbilityBuilder, GameAbilityComponent,
};
use death_by_attributes::attributes::GameAttribute;
use death_by_attributes::attributes::GameAttributeMarker;
use death_by_attributes::context::GameAttributeContextMut;
use death_by_attributes::effect::{GameEffectBuilder, GameEffectContainer, GameEffectEvent};
use death_by_attributes::events::CurrentValueUpdateTrigger;
use death_by_attributes::modifiers::{ModifierType, ScalarModifier};
use death_by_attributes::systems::{
    handle_apply_effect_events, tick_active_effects, update_attribute_base_value,
    update_attribute_current_value,
};
use death_by_attributes::{BaseValueUpdate, CurrentValueUpdate, DeathByAttributesPlugin};
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

fn register_types(type_registry: ResMut<AppTypeRegistry>) {
    type_registry.0.write().register::<Health>();
    type_registry.0.write().register::<HealthCap>();
    type_registry.0.write().register::<HealthRegen>();
    type_registry.0.write().register::<Mana>();
}

#[derive(Component)]
struct UiFireballText;

#[derive(Component)]
struct Fireball;

fn setup(mut commands: Commands, mut event_writer: EventWriter<GameEffectEvent>) {
    let mut ability_component = GameAbilityComponent::default();
    ability_component.grant_ability(
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
            )
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

    /*let health_regen = GameEffectBuilder::new()
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
        .with_scalar_modifier::<HealthCap>(20.0, ModifierType::Additive)
        .build();

    event_writer.send(GameEffectEvent {
        entity: id,
        effect: event_effect,
    });*/

    let damage_effect = GameEffectBuilder::new()
        .with_scalar_modifier::<Health>(-50.0, ModifierType::Additive)
        .build();

    event_writer.send(GameEffectEvent{
        entity: id,
        effect: damage_effect,
    });
}

fn clamp_health(
    trigger: Trigger<CurrentValueUpdateTrigger>,
    mut query: Query<(&mut Health, &HealthCap)>,
) {
    info!("ugh");
    let (mut health, max_health) = query.get_mut(trigger.entity()).unwrap();
    health.base_value = health.base_value.clamp(0.0, max_health.value.current_value);
    health.current_value = health.base_value;
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct HealthInterfaceMarker;

#[derive(Component)]
struct ManaInterfaceMarker;

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
            builder.spawn((Text::new("Health"), HealthInterfaceMarker));
            builder.spawn((Text::new("Mana"), ManaInterfaceMarker));
        });
}

fn display_attribute(
    q_player: Query<
        (
            &Health,
            Option<&HealthCap>,
            Option<&HealthRegen>,
            &Mana,
            &GameEffectContainer,
        ),
        With<Player>,
    >,
    mut q_health: Query<&mut Text, With<HealthInterfaceMarker>>,
    //mut q_mana: Query<&mut Text, With<ManaInterfaceMarker>>,
) {
    for (health, health_cap, health_regen, mana, gec) in q_player.iter() {
        if let Ok(mut text) = q_health.get_single_mut() {
            text.0 = format!(
                "Health: {:.1} [{:.1}]",
                health.value.current_value, health.value.base_value
            );
        }

        /*if let Ok(mut text) = q_mana.get_single_mut() {
            text.0 = format!(
                "\nMana: {:.1} [{:.1}]",
                mana.value.current_value, mana.value.base_value
            );
        }*/

        /*
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
        ui.sections[4].value = format!("\n{:}", gec);
        ui.sections[5].value = format!("\nFireball count: {:.1}", q_fireball.iter().count());*/
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
    mut query: Query<EntityMut, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut context: GameAttributeContextMut,
    commands: Commands,
) {
    if let Ok(player) = query.get_single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            context.try_activate(player, "fireball".to_string(), commands);
        }
    }
}
