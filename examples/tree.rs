use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::prelude::*;
use ptree::{TreeBuilder, print_tree};
use root_attribute::actors::ActorBuilder;
use root_attribute::attributes::AttributeComponent;
use root_attribute::effects::{EffectBuilder, EffectOf, Effects};
use root_attribute::modifiers::ModType::Additive;
use root_attribute::modifiers::{ModAggregator, Modifier};
use root_attribute::systems::{flag_dirty_modifier_nodes, update_effect_tree_system};
use root_attribute::{Actor, DeathByAttributesPlugin, OnModifierApplied, attribute};

#[derive(Component)]
struct Mark;

attribute!(Health);
attribute!(DamageReduction);
attribute!(Damage);

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (
                flag_dirty_modifier_nodes::<Health>,
                print_effect_hierarchy_system::<Health>,
                update_effect_tree_system::<Health>,
                print_effect_hierarchy_system::<Health>,
            )
                .chain(),
        )
        .add_systems(Update, modify)
        .add_systems(
            PostUpdate,
            (apply_damage, print_effect_hierarchy_system::<Health>).chain(),
        )
        .add_event::<DamageEvent>();

    app.update();
    println!("------------------------------ [Hello, world!] ------------------------------");
    app.update();
}

fn modify(query: Query<Entity, With<Actor>>, mut event_writer: EventWriter<DamageEvent>) {
    for entity in query.iter() {
        event_writer.write(DamageEvent {
            source: entity,
            target: entity,
        });
    }
}

fn setup(mut commands: Commands) {
    let player = commands.spawn_empty().id();
    ActorBuilder::new(player)
        .with::<Health>(0.0)
        .with::<Damage>(100.0)
        .with::<DamageReduction>(0.25)
        .with_component(Name::new("Player"))
        .commit(&mut commands);

    let mod1 = commands
        .spawn((
            EffectOf(player),
            Name::new("M1"),
            Modifier::<Health>::new(1.0, Additive),
            ModAggregator::<Health>::default(),
        ))
        .id();
    let mod2 = commands
        .spawn((
            EffectOf(mod1),
            Name::new("M2"),
            Modifier::<Health>::new(1.0, Additive),
            ModAggregator::<Health>::default(),
        ))
        .id();
    let mod3 = commands
        .spawn((
            EffectOf(mod2),
            Name::new("M3"),
            Modifier::<Health>::new(1.0, Additive),
            ModAggregator::<Health>::default(),
        ))
        .id();

    let effect = commands.spawn_empty().id();
    EffectBuilder::new(mod3, effect)
        .with_permanent_duration()
        .with_continuous_application()
        .with_name("Effect X".into())
        .modify_by_scalar::<Health>(1.0, Additive)
        .commit(&mut commands);

    commands.spawn((
        EffectOf(effect),
        Mark,
        Name::new("M5"),
        Modifier::<Health>::new(10.0, Additive),
        ModAggregator::<Health>::default(),
    ));
}

#[derive(Event)]
struct DamageEvent {
    source: Entity,
    target: Entity,
}

fn apply_damage(
    mut event_reader: EventReader<DamageEvent>,
    source: Query<&Damage>,
    target: Query<&DamageReduction>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let Ok(damage) = source.get(ev.source) else {
            continue;
        };
        let Ok(reduction) = target.get(ev.target) else {
            continue;
        };
        let received_damage = damage.current_value() * (1.0 - reduction.current_value());
        let send_event = OnModifierApplied::<Health> {
            phantom_data: Default::default(),
            value: ModAggregator::additive(-received_damage),
        };
        println!("Mod triggered: {} on {}", send_event.value, ev.target);
        commands.trigger_targets(send_event, ev.target);
    }
}

pub fn print_effect_hierarchy_system<T: Component + AttributeComponent>(
    actors: Query<Entity, With<Actor>>,
    descendants: Query<&Effects>,
    entities: Query<(
        &Name,
        Option<&T>,
        Option<&Modifier<T>>,
        Option<&ModAggregator<T>>,
    )>,
) {
    let mut builder = TreeBuilder::new("Actor-Attribute Tree".into());
    for actor in actors.iter() {
        print_effect_hierarchy(actor, &mut builder, descendants, entities);
    }
    let tree = builder.build();
    let _ = print_tree(&tree);
}

pub fn print_effect_hierarchy<T: Component + AttributeComponent>(
    current_entity: Entity,
    builder: &mut TreeBuilder,
    descendants: Query<&Effects>,
    entities: Query<(
        &Name,
        Option<&T>,
        Option<&Modifier<T>>,
        Option<&ModAggregator<T>>,
    )>,
) {
    let Ok((name, attribute, modifier, aggregator)) = entities.get(current_entity) else {
        return;
    };
    let attribute = if let Some(attribute) = attribute {
        format!("[{}/{}]", attribute.base_value(), attribute.current_value())
    } else {
        "".to_string()
    };
    let modifier = if let Some(modifier) = modifier {
        format!("{}", modifier.value)
    } else {
        "".to_string()
    };
    let aggregator = if let Some(aggregator) = aggregator {
        format!("{}", aggregator)
    } else {
        "".to_string()
    };

    let tree_item = format!(
        "{} [{name}] A-{} M-{} Ag-{} ",
        current_entity, attribute, modifier, aggregator
    );

    // Iterate recursively on all the childrens
    if let Ok(childrens) = descendants.get(current_entity) {
        builder.begin_child(tree_item);
        for child in childrens.iter() {
            print_effect_hierarchy::<T>(child, builder, descendants, entities);
        }
        builder.end_child();
    } else {
        builder.add_empty_child(tree_item);
    }
}
