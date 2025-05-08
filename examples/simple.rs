use bevy::ecs::relationship::Relationship;
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::log::LogPlugin;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy::ui::debug::print_ui_layout_tree;
use bevy::window::PresentMode;
use bevy_dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use ptree::{TreeBuilder, print_tree, write_tree};
use rand::{Rng, rng};
use root_attribute::abilities::{GameAbilityBuilder, GameAbilityContainer};
use root_attribute::actors::ActorBuilder;
use root_attribute::attributes::{AttributeBuilder, AttributeComponent};
use root_attribute::effects::{Effect, EffectBuilder, EffectPeriodicTimer};
use root_attribute::modifiers::ModType::{Additive, Multiplicative};
use root_attribute::modifiers::{Effects, ModAggregator, Modifier, ModifierOf, Modifiers};
use root_attribute::systems::recursive_pretty_print;
use root_attribute::{
    Actor, ActorEntityMut, DeathByAttributesPlugin, OnAttributeValueChanged, attribute,
};
use std::time::Duration;

attribute!(Strength);
attribute!(Agility);

attribute!(Health);
attribute!(MaxHealth);
attribute!(HealthRegen);

attribute!(Mana);
attribute!(ManaPool);
attribute!(ManaRegen);

attribute!(AttackPower);
attribute!(MagicPower);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "error,root_attribute=debug".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }))
        .add_plugins(DeathByAttributesPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, (setup_window, setup, setup_ui))
        .add_systems(Update, do_gameplay_stuff)
        .add_systems(
            Update,
            display_attribute.run_if(on_timer(Duration::from_millis(32))),
        )
        .add_systems(Update, display_tree)
        /*.add_systems(
            Update,
            agg.run_if(on_timer(Duration::from_millis(32))),
        )*/
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
        .register_type::<Health>()
        .register_type::<AttackPower>()
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

fn agg(query: Query<(Entity, &Name, &ModAggregator<AttackPower>)>) {
    for (entity, name, aggregator) in query.iter() {
        print!("[{} / {name} / {}] ", entity, aggregator.additive);
    }
    println!("---")
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
    let player_entity = commands.spawn_empty().id();

    ActorBuilder::new(player_entity)
        .with::<Strength>(12.0)
        .with::<Agility>(7.0)
        .with::<Health>(100.0)
        .with::<MaxHealth>(100.0)
        .with::<HealthRegen>(2.0)
        .with::<Mana>(100.0)
        .with::<ManaPool>(100.0)
        .with::<ManaRegen>(8.0)
        .with::<MagicPower>(0.0)
        .with::<AttackPower>(0.0)
        .with_component((Name::new("Player"), Player))
        .commit(&mut commands);

    AttributeBuilder::<AttackPower>::new(player_entity)
        .by_ref::<Health>(0.10)
        .by_ref::<MaxHealth>(0.25)
        .commit(&mut commands);
    AttributeBuilder::<MagicPower>::new(player_entity)
        .by_ref::<MaxHealth>(1.0)
        .by_ref::<AttackPower>(1.0)
        .commit(&mut commands);

    // Effect 1 - Passive Max Health Boost
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_continuous_application()
        .modify_by_scalar::<MaxHealth>(100.0, Additive)
        .modify_by_scalar::<MaxHealth>(0.10, Multiplicative)
        .commit(&mut commands);

    // Effect 2 - Periodic Health Regen
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        .modify_by_scalar::<AttackPower>(1.0, Additive)
        .modify_by_scalar::<Health>(5.0, Additive)
        .modify_by_scalar::<Mana>(5.0, Additive)
        .commit(&mut commands);

    /*
        // Effect 3 - Instant
        EffectBuilder::new(player_entity, &mut commands.reborrow())
            .with_instant_application()
            .modify_by_scalar::<Health>(-35.0, Additive)
            .commit();
    */
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

fn clamp_health(
    trigger: Trigger<OnAttributeValueChanged>,
    mut query: Query<(&mut Health, &MaxHealth)>,
) {
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
            builder.spawn((Text::new(""), EntityHealthMarker));
        });
}

fn display_attribute(
    q_player: Query<(&Health, &MaxHealth, &AttackPower, &MagicPower), With<Player>>,
    mut q_health: Query<&mut Text, With<PlayerHealthMarker>>,
) {
    for (health, max_health, ap, mp) in q_player.iter() {
        if let Ok(mut text) = q_health.single_mut() {
            text.0 = format!(
                "Values: Current [Base]
Health: {:.1} [{:.1}]
Max Health: {:.1} [{:.1}]
AP: {:.1} [{:.1}]
MP: {:.1} [{:.1}]",
                health.current_value(),
                health.base_value(),
                max_health.current_value(),
                max_health.base_value(),
                ap.current_value(),
                ap.base_value(),
                mp.current_value(),
                mp.base_value(),
            );
        }
    }
}

pub fn display_tree(
    actors: Query<Entity, With<Actor>>,
    descendants: Query<&Effects>,
    entities: Query<&Name>,
    mut text: Query<&mut Text, With<EntityHealthMarker>>,
) {
    let mut builder = TreeBuilder::new("Actor-Attribute Tree".into());
    for actor in actors.iter() {
        recursive_pretty_print(actor, &mut builder, descendants, entities);
    }
    let tree = builder.build();
    if let Ok(mut text) = text.single_mut() {
        let mut w = Vec::new();
        let _ = write_tree(&tree, &mut w);
        text.0 = String::from_utf8(w).unwrap();
    }
}

fn inputs(
    mut q_player: Query<(Entity), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if let Ok((player_entity)) = q_player.single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            let effect_entity = commands.spawn_empty().id();

            EffectBuilder::new(player_entity, effect_entity)
                .with_instant_application()
                .modify_by_scalar::<Health>(-18.0, Additive)
                .commit(&mut commands);

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
