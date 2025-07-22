use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use ptree::TreeBuilder;
use root_attribute::abilities::{AbilityBuilder, TryActivateAbility};
use root_attribute::actors::ActorBuilder;
use root_attribute::assets::GameEffect;
use root_attribute::attributes::Attribute;
use root_attribute::attributes::ReflectAccessAttribute;
use root_attribute::context::EffectContext;
use root_attribute::effects::{
    EffectBuilder, EffectPeriodicTimer, EffectSource, EffectSources, EffectTarget, EffectTargetedBy,
};
use root_attribute::inspector::ActorInspectorPlugin;
use root_attribute::modifiers::ModType::{Additive, Multiplicative};
use root_attribute::modifiers::{AttributeModifier, ModAggregator, ModTarget};
use root_attribute::stacks::{EffectStackingPolicy, Stacks};
use root_attribute::{attribute, ActorEntityMut, AttributesPlugin};
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
        .add_plugins(AttributesPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(ActorInspectorPlugin)
        .add_systems(
            Startup,
            (setup_effects, setup_window, setup, setup_camera).chain(),
        )
        .add_systems(Update, do_gameplay_stuff)
        //.add_systems(Update, display_tree)
        //.add_systems(Update, pretty_print_tree_system)
        //.add_systems(Update, display_modifier_tree::<Health>)
        .add_systems(PreUpdate, inputs)
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .add_observer(damage_event_calculations)
        .register_type::<EffectPeriodicTimer>()
        .register_type::<Health>()
        .register_type::<AttackPower>()
        .register_type::<Mana>()
        .register_type::<ManaPool>()
        .register_type::<AttributeModifier<AttackPower>>()
        .register_type::<AttributeModifier<MagicPower>>()
        .register_type::<EffectSources>()
        .register_type::<EffectTargetedBy>()
        .register_type::<Stacks>()
        .register_type::<EffectSource>()
        .register_type::<EffectTarget>()
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

#[derive(Resource)]
struct FireballAbility(Entity);

#[derive(Resource)]
struct EffectsDatabase {
    ap_buff: Handle<GameEffect>,
    mp_buff: Handle<GameEffect>,
    hp_buff: Handle<GameEffect>,
    hp_regen: Handle<GameEffect>,
}

fn setup_effects(mut effects: ResMut<Assets<GameEffect>>, mut commands: Commands) {
    // Attack Power effect
    let ap_buff = effects.add(
        EffectBuilder::new()
            .with_permanent_duration()
            .with_continuous_application()
            .with_name("AttackPower Buff".into())
            .modify_by_ref::<AttackPower, Health>(0.10, Additive, ModTarget::Target)
            .modify_by_ref::<Intelligence, Health>(0.10, Additive, ModTarget::Target)
            .build(),
    );

    // Magic Power effect
    let mp_buff = effects.add(
        EffectBuilder::new()
            .with_permanent_duration()
            .with_continuous_application()
            .with_name("MagicPower Buff".into())
            .modify_by_scalar::<MagicPower>(10.0, Additive, ModTarget::Target)
            .with_condition::<Health>(20.0..100.0)
            .with_stacking_policy(EffectStackingPolicy::Add {
                count: 1,
                max_stack: 10,
            })
            .build(),
    );

    // Effect 1 - Passive Max Health Boost
    let hp_buff = effects.add(
        EffectBuilder::new()
            .with_permanent_duration()
            .with_continuous_application()
            .with_name("MaxHealth Increase".into())
            .modify_by_scalar::<MaxHealth>(0.10, Multiplicative, ModTarget::Target)
            .with_stacking_policy(EffectStackingPolicy::Override)
            .build(),
    );

    // Effect 2 - Periodic Health Regen
    let hp_regen = effects.add(
        EffectBuilder::new()
            .with_permanent_duration()
            .with_periodic_application(1.0)
            .with_name("Health Regen".into())
            .modify_by_scalar::<Health>(1.0, Additive, ModTarget::Target)
            .modify_by_ref::<Health, HealthRegen>(1.0, Additive, ModTarget::Target)
            .build(),
    );

    commands.insert_resource(EffectsDatabase {
        ap_buff,
        mp_buff,
        hp_buff,
        hp_regen,
    });
}

fn setup(mut commands: Commands, mut ctx: EffectContext, efx: Res<EffectsDatabase>) {
    let _rng = rand::rng();
    let player_entity = commands.spawn_empty().id();
    let ability = commands.spawn_empty().id();

    commands.insert_resource(FireballAbility(ability));

    AbilityBuilder::new(ability, player_entity)
        .with_activation(|_: ActorEntityMut, _: Commands| {
            info!("fireball!");
        })
        .with_cooldown(1.0)
        .with_cost::<Mana>(12.0)
        .build(&mut commands);

    ActorBuilder::new(player_entity)
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
        .with_component((Name::new("Player"), Player))
        .commit(&mut commands);

    ctx.apply_effect_to_self(player_entity, efx.ap_buff.clone());
    ctx.apply_effect_to_self(player_entity, efx.hp_buff.clone());
    ctx.apply_effect_to_self(player_entity, efx.hp_regen.clone());

    // Should have two stacks
    ctx.apply_effect_to_self(player_entity, efx.mp_buff.clone());
    ctx.apply_effect_to_self(player_entity, efx.mp_buff.clone());
}

#[derive(Component)]
struct Player;

#[derive(Component)]
pub struct EntityHealthMarker;
#[derive(Component)]
pub struct ModifierTree;

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn inputs(
    mut players: Query<Entity, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    ability: Res<FireballAbility>,
    mut commands: Commands,
) {
    if let Ok(player_entity) = players.single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            commands.trigger_targets(TryActivateAbility, ability.0);
            commands.trigger_targets(DamageEvent { damage: 10.0 }, player_entity);
        }
    }
}

fn do_gameplay_stuff() {
    std::thread::sleep(Duration::from_millis(12));
}

/// Prints the hierarchy of modifiers in a tree structure for a given `Entity` and its descendants.
///
/// This function recursively traverses through an entity and its descendants to construct a
/// string representation of the modifiers associated with each entity. The result is added to a given
/// `TreeBuilder` instance to visualize the hierarchy.
///
/// # Type Parameters
/// - `T`: A type that implements both `Component` and `AttributeComponent`. Represents the specific
///   type of modifier being used.
///
/// # Notes
/// - This function will gracefully return if it fails to fetch the required data (e.g., if an entity doesn't exist).
/// - For entities without modifiers or aggregators, the corresponding strings will be empty.
///
/// # Examples
/// ```rust
/// let mut builder = TreeBuilder::new();
/// let current_entity = some_entity_id;
/// print_modifier_hierarchy::<MyComponent>(
///     current_entity,
///     &mut builder,
///     descendants_query,
///     entities_query,
/// );
/// println!("{}", builder.build());
/// ```
pub fn print_modifier_hierarchy<T: Component + Attribute>(
    current_entity: Entity,
    builder: &mut TreeBuilder,
    descendants: Query<&EffectTargetedBy>,
    entities: Query<(
        &Name,
        Option<&AttributeModifier<T>>,
        Option<&ModAggregator<T>>,
    )>,
) {
    let Ok((name, modifier, aggregator)) = entities.get(current_entity) else {
        return;
    };
    let modifier = if let Some(modifier) = modifier {
        format!("Mod:{}", modifier.aggregator)
    } else {
        "".to_string()
    };
    let aggregator = if let Some(aggregator) = aggregator {
        format!("{}", aggregator)
    } else {
        "".to_string()
    };

    let tree_item = format!("{} [{name}] {} {} ", current_entity, modifier, aggregator);

    // Iterate recursively on all the childrens
    if let Ok(childrens) = descendants.get(current_entity) {
        builder.begin_child(tree_item);
        for child in childrens.iter() {
            print_modifier_hierarchy::<T>(child, builder, descendants, entities);
        }
        builder.end_child();
    } else {
        builder.add_empty_child(tree_item);
    }
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
