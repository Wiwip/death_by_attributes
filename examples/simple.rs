use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use petgraph::Graph;
use petgraph::dot::Dot;
use ptree::TreeBuilder;
use root_attribute::ability::{AbilityBuilder, TargetData, TryActivateAbility};
use root_attribute::actors::ActorBuilder;
use root_attribute::assets::{AbilityDef, ActorDef, EffectDef};
use root_attribute::attributes::Attribute;
use root_attribute::attributes::ReflectAccessAttribute;
use root_attribute::condition::AttributeCondition;
use root_attribute::context::EffectContext;
use root_attribute::effect::{EffectStackingPolicy, Stacks};
use root_attribute::graph::EntityGraph;
use root_attribute::inspector::ActorInspectorPlugin;
use root_attribute::inspector::debug_overlay::DebugOverlayMarker;
use root_attribute::prelude::*;
use root_attribute::{AttributesMut, AttributesPlugin, attribute, init_attribute};
use std::fmt::Debug;
use std::time::Duration;

attribute!(Strength);
attribute!(Agility);
attribute!(Intelligence);

attribute!(Health);
attribute!(MaxHealth);
attribute!(HealthRegen);

attribute!(Mana);
attribute!(ManaPool);
attribute!(ManaRegen);

attribute!(AttackPower);
attribute!(Armour);
attribute!(MagicPower);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "error,root_attribute=debug".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }))
        .insert_resource(UiDebugOptions {
            //enabled: true,
            ..default()
        })
        .add_plugins((
            AttributesPlugin,
            init_attribute::<Strength>,
            init_attribute::<Agility>,
            init_attribute::<Intelligence>,
            init_attribute::<Health>,
            init_attribute::<MaxHealth>,
            init_attribute::<HealthRegen>,
            init_attribute::<Mana>,
            init_attribute::<ManaPool>,
            init_attribute::<ManaRegen>,
            init_attribute::<AttackPower>,
            init_attribute::<Armour>,
            init_attribute::<MagicPower>,
        ))
        .add_plugins(EguiPlugin::default())
        .add_plugins(DefaultInspectorConfigPlugin)
        //.add_plugins(WorldInspectorPlugin::default())
        .add_plugins(ActorInspectorPlugin)
        .add_systems(
            Startup,
            (
                setup_effects,
                setup_abilities,
                setup_window,
                setup_actor,
                setup_camera,
            )
                .chain(),
        )
        .add_systems(Update, do_gameplay_stuff)
        .add_systems(PreUpdate, inputs)
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .add_observer(damage_event_calculations)
        .register_type::<Health>()
        .register_type::<AttackPower>()
        .register_type::<Mana>()
        .register_type::<ManaPool>()
        .register_type::<EffectSources>()
        .register_type::<Effects>()
        .register_type::<Stacks>()
        .register_type::<EffectSource>()
        .register_type::<EffectTarget>()
        .run();
}

fn setup_window(mut query: Query<&mut Window>) {
    for mut window in query.iter_mut() {
        window.present_mode = PresentMode::Immediate;
    }
}

#[derive(Resource)]
struct AbilityDatabase {
    fireball: Handle<AbilityDef>,
    frostball: Handle<AbilityDef>,
}

#[derive(Resource)]
struct EffectsDatabase {
    ap_buff: Handle<EffectDef>,
    mp_buff: Handle<EffectDef>,
    hp_buff: Handle<EffectDef>,
    hp_regen: Handle<EffectDef>,
}

fn setup_effects(mut effects: ResMut<Assets<EffectDef>>, mut commands: Commands) {
    // Attack Power effect
    let ap_buff = effects.add(
        EffectBuilder::permanent()
            .name("AttackPower Buff".into())
            .modify_by_ref::<AttackPower, Health>(0.10, Mod::Add(1.0), Who::Target)
            .modify_by_ref::<Intelligence, Health>(0.25, Mod::Add(1.0), Who::Target)
            .build(),
    );

    let a = AttributeCondition::new::<Health>(..=100.0, Who::Source);

    // Magic Power effect
    let mp_buff = effects.add(
        EffectBuilder::permanent()
            .name("MagicPower Buff".into())
            .modify::<MagicPower>(Mod::Add(10.0), Who::Target)
            //.when_attribute::<Health>(..=100.0)
            .when_condition(a)
            .with_stacking_policy(EffectStackingPolicy::Add {
                count: 1,
                max_stack: 10,
            })
            .build(),
    );

    // Effect 1 - Passive Max Health Boost
    let hp_buff = effects.add(
        EffectBuilder::permanent()
            .name("MaxHealth Increase".into())
            .modify::<MaxHealth>(Mod::Inc(0.10), Who::Target)
            .with_stacking_policy(EffectStackingPolicy::RefreshDuration)
            .build(),
    );

    // Effect 2 - Periodic Health Regen
    let hp_regen = effects.add(
        EffectBuilder::every_seconds(1.0)
            .name("Health Regen".into())
            .modify::<Health>(Mod::Add(3.0), Who::Target)
            .modify_by_ref::<Health, HealthRegen>(1.0, Mod::Add(1.0), Who::Target)
            .build(),
    );

    commands.insert_resource(EffectsDatabase {
        ap_buff,
        mp_buff,
        hp_buff,
        hp_regen,
    });
}

#[derive(Component, Default)]
struct Fire;

#[derive(Component, Default)]
struct Frost;

fn setup_abilities(mut effects: ResMut<Assets<AbilityDef>>, mut commands: Commands) {
    let fireball = effects.add(
        AbilityBuilder::new()
            .with_name("Fireball".into())
            .with_activation(|entity: &mut AttributesMut, _: &mut Commands| {
                println!("Fireball! {}", entity.id());
            })
            .with_cooldown(1.0)
            .with_cost::<Mana>(12.0)
            .with_tag::<Fire>()
            .build(),
    );

    let frostball = effects.add(
        AbilityBuilder::new()
            .with_name("Frostball".into())
            .with_activation(|entity: &mut AttributesMut, _: &mut Commands| {
                println!("Frostball! {}", entity.id());
            })
            .with_cooldown(1.0)
            .with_cost::<Mana>(12.0)
            .with_tag::<Frost>()
            .build(),
    );
    commands.insert_resource(AbilityDatabase {
        fireball,
        frostball,
    });
}

fn setup_actor(
    mut ctx: EffectContext,
    mut actor_assets: ResMut<Assets<ActorDef>>,
    efx: Res<EffectsDatabase>,
    abilities: Res<AbilityDatabase>,
) {
    let _rng = rand::rng();

    let actor_template = actor_assets.add(
        ActorBuilder::new()
            .with_name("=== Player ===".into())
            .with::<Strength>(12.0)
            .with::<Agility>(7.0)
            .with::<Intelligence>(1.0)
            .with::<Health>(85.0)
            .clamp_max::<Health, MaxHealth>(0.0)
            .with::<MaxHealth>(1000.0)
            .with::<HealthRegen>(2.0)
            .with::<Mana>(100.0)
            .with::<ManaPool>(100.0)
            .with::<ManaRegen>(8.0)
            .with::<MagicPower>(1.0)
            .with::<AttackPower>(0.0)
            .with::<Armour>(0.10)
            .with_component((Player, DebugOverlayMarker))
            .grant_ability(&abilities.fireball)
            .grant_ability(&abilities.frostball)
            .build(),
    );

    let player_entity = ctx.spawn_actor(&actor_template).id();

    ctx.apply_effect_to_self(player_entity, &efx.ap_buff);
    ctx.apply_effect_to_self(player_entity, &efx.hp_buff);
    ctx.apply_effect_to_self(player_entity, &efx.hp_regen);

    // Should have two stacks
    ctx.apply_effect_to_self(player_entity, &efx.mp_buff);
    ctx.apply_effect_to_self(player_entity, &efx.mp_buff);
}

#[derive(Component, Copy, Clone)]
struct Player;

#[derive(Component)]
pub struct EntityHealthMarker;
#[derive(Component)]
pub struct ModifierTree;

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn inputs(
    mut players: Query<(Entity, &EntityGraph, &AttackPower), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if let Ok((player_entity, graph, attribute)) = players.single_mut() {
        if keys.just_pressed(KeyCode::KeyQ) {
            commands
                .entity(player_entity)
                .trigger(TryActivateAbility::by_tag::<Fire>(TargetData::Own));
        }
        if keys.just_pressed(KeyCode::KeyE) {
            commands
                .entity(player_entity)
                .trigger(TryActivateAbility::by_tag::<Frost>(TargetData::Own));
        }
        if keys.just_pressed(KeyCode::Backspace) {
            commands.trigger_targets(DamageEvent { damage: 10.0 }, player_entity);
        }
        if keys.just_pressed(KeyCode::KeyR) {
            println!("{:?}", Dot::new(&graph.graph));

            let cv = graph.calculate_attribute_value::<AttackPower>(attribute.current_value);
            println!("{:?}", cv);
        }
    }
}

fn do_gameplay_stuff() {
    std::thread::sleep(Duration::from_millis(12));
}

#[derive(Event)]
struct DamageEvent {
    damage: f64,
}

fn damage_event_calculations(
    trigger: Trigger<DamageEvent>,
    mut actors: Query<(&mut Health, &Armour)>,
) {
    let Ok((mut health, armour)) = actors.get_mut(trigger.target()) else {
        return;
    };

    let new_health = health.current_value() - trigger.damage * (1.0 - armour.current_value());
    health.set_base_value(new_health);
    debug!(
        "{} took {} damage",
        trigger.target(),
        trigger.damage * (1.0 - armour.current_value())
    );
}
