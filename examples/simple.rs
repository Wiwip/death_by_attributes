use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::log::LogPlugin;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;

use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use ptree::{write_tree, TreeBuilder};
use root_attribute::abilities::{AbilityBuilder, TryActivateAbility};
use root_attribute::actors::ActorBuilder;
use root_attribute::attributes::{AttributeBuilder, AttributeComponent};
use root_attribute::effects::{Effect, EffectBuilder, EffectOf, EffectPeriodicTimer, Effects};
use root_attribute::modifiers::ModType::{Additive, Multiplicative};
use root_attribute::modifiers::{ModAggregator, Modifier};
use root_attribute::systems::recursive_pretty_print;
use root_attribute::{attribute, Actor, ActorEntityMut, AttributesPlugin};
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
        .add_plugins(AttributesPlugin)
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
        .add_systems(Update, display_modifier_tree::<MagicPower>)
        .add_systems(PreUpdate, inputs)
        .add_systems(PostUpdate, display_components)
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .register_type::<Effects>()
        .register_type::<EffectPeriodicTimer>()
        .register_type::<Health>()
        .register_type::<AttackPower>()
        .register_type::<Mana>()
        .register_type::<ManaPool>()
        .register_type::<EffectOf>()
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

fn setup(mut commands: Commands) {
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
        .with::<Health>(100.0)
        .clamp_max::<Health, MaxHealth>(0.0)
        .with::<MaxHealth>(1000.0)
        .with::<HealthRegen>(2.0)
        .with::<Mana>(100.0)
        .with::<ManaPool>(100.0)
        .with::<ManaRegen>(8.0)
        .with::<MagicPower>(0.0)
        .with::<AttackPower>(0.0)
        .with_component((Name::new("Player"), Player))
        .commit(&mut commands);

    /*AttributeBuilder::<AttackPower>::new(player_entity)
        .by_ref::<Health>(0.10)
        .by_ref::<MaxHealth>(0.25)
        .commit(&mut commands);
    AttributeBuilder::<MagicPower>::new(player_entity)
        .by_ref::<ManaPool>(0.5)
        .by_ref::<MaxHealth>(0.01)
        .commit(&mut commands);*/

    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_continuous_application()
        .with_name("MagicPower Increase".into())
        .modify_by_ref::<MagicPower, ManaPool>(0.15)
        //.modify_by_scalar::<MaxHealth>(0.10, Multiplicative)
        .commit(&mut commands);

    // Effect 1 - Passive Max Health Boost
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_continuous_application()
        .with_name("MaxHealth Increase".into())
        .modify_by_scalar::<MaxHealth>(0.10, Multiplicative)
        .commit(&mut commands);

    // Effect 2 - Periodic Health Regen
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        .with_name("Health Regen".into())
        .modify_by_scalar::<Health>(1.0, Additive)
        .modify_by_ref::<Health, HealthRegen>(1.0)
        .commit(&mut commands);

    // Effect 3 - Instant Damage Taken
    let effect_entity = commands.spawn_empty().id();
    EffectBuilder::new(player_entity, effect_entity)
        .with_instant_application()
        .with_name("Damage Hit".into())
        .modify_by_scalar::<Health>(-35.0, Additive)
        .commit(&mut commands);
}

#[derive(Component)]
struct Player;
#[derive(Component)]
struct PlayerHealthMarker;
#[derive(Component)]
pub struct EntityHealthMarker;
#[derive(Component)]
pub struct ModifierTree;

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

    commands.spawn((
        ModifierTree,
        Text::new("Modifier Tree"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            right: Val::Px(30.0),
            ..default()
        },
    ));
}

fn display_attribute(
    q_player: Query<(&Health, &MaxHealth, &AttackPower, &MagicPower, &Mana, &ManaPool), With<Player>>,
    mut q_health: Query<&mut Text, With<PlayerHealthMarker>>,
) {
    for (health, max_health, ap, mp, mana, mana_pool) in q_player.iter() {
        if let Ok(mut text) = q_health.single_mut() {
            text.0 = format!(
                "Values: Current [Base]
Health: {:.1} [{:.1}]
Max Health: {:.1} [{:.1}]
AP: {:.1} [{:.1}]
MP: {:.1} [{:.1}]
Mana: {:.1} [{:.1}]
Mana Pool: {:.1} [{:.1}]",
                health.current_value(),
                health.base_value(),
                max_health.current_value(),
                max_health.base_value(),
                ap.current_value(),
                ap.base_value(),
                mp.current_value(),
                mp.base_value(),
                mana.current_value(),
                mana.base_value(),
                mana_pool.current_value(),
                mana_pool.base_value(),
            );
        }
    }
}

pub fn display_tree(
    actors: Query<Entity, With<Actor>>,
    effects: Query<&Effects>,
    entities: Query<&Name>,
    mut text: Query<&mut Text, With<EntityHealthMarker>>,
) {
    let mut builder = TreeBuilder::new("Effects Tree".into());
    for actor in actors.iter() {
        recursive_pretty_print(actor, &mut builder, effects, entities);
    }
    let tree = builder.build();
    if let Ok(mut text) = text.single_mut() {
        let mut w = Vec::new();
        let _ = write_tree(&tree, &mut w);
        text.0 = String::from_utf8(w).unwrap();
    }
}

pub fn display_modifier_tree<T: Component + AttributeComponent>(
    actors: Query<Entity, With<Actor>>,
    modifiers: Query<&Effects>,
    entities: Query<(&Name, Option<&Modifier<T>>, Option<&ModAggregator<T>>)>,
    mut text: Query<&mut Text, With<ModifierTree>>,
) {
    let mut builder = TreeBuilder::new("Modifiers Tree".into());
    for actor in actors.iter() {
        print_modifier_hierarchy(actor, &mut builder, modifiers, entities);
    }
    let tree = builder.build();
    if let Ok(mut text) = text.single_mut() {
        let mut w = Vec::new();
        let _ = write_tree(&tree, &mut w);
        text.0 = String::from_utf8(w).unwrap();
    }
}

pub fn display_components(
    // Get the world for introspection
    world: &World,
    // Get the entity ID of our player
    effects: Query<Entity, With<Effect>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    // We only run this once when Space is pressed
    if !input.just_pressed(KeyCode::KeyP) {
        return;
    }

    for effect in effects.iter() {
        let Ok(effect_ref) = world.get_entity(effect) else {
            continue;
        };

        let archetype = effect_ref.archetype();
        for component_id in archetype.components() {
            // Using the component_id, we can get ComponentInfo, which has the name
            if let Some(component_info) = world.components().get_info(component_id) {
                println!("  - {}", component_info.name());
            }
        }
    }
}

fn inputs(
    mut players: Query<Entity, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    ability: Res<FireballAbility>,
    mut commands: Commands,
) {
    if let Ok(player_entity) = players.single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            /*commands.trigger_targets(
                OnModifierApplied::<Health> {
                    phantom_data: Default::default(),
                    value: ModAggregator::<Health>::additive(-12.0),
                },
                player_entity,
            );*/

            commands.trigger_targets(TryActivateAbility, ability.0);
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
pub fn print_modifier_hierarchy<T: Component + AttributeComponent>(
    current_entity: Entity,
    builder: &mut TreeBuilder,
    descendants: Query<&Effects>,
    entities: Query<(&Name, Option<&Modifier<T>>, Option<&ModAggregator<T>>)>,
) {
    let Ok((name, modifier, aggregator)) = entities.get(current_entity) else {
        return;
    };
    let modifier = if let Some(modifier) = modifier {
        format!("Mod:{}", modifier.value)
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
