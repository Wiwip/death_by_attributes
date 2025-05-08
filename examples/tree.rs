use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::prelude::*;
use root_attribute::actors::ActorBuilder;
use root_attribute::attributes::AttributeComponent;
use root_attribute::effects::{Effect, EffectBuilder};
use root_attribute::modifiers::ModType::Additive;
use root_attribute::modifiers::scalar::Modifier;
use root_attribute::modifiers::{EffectOf, ModAggregator, ModifierOf};
use root_attribute::systems::{
    flag_dirty_modifier_nodes, pretty_print_tree_system, update_attribute_tree_system,
};
use root_attribute::{Actor, DeathByAttributesPlugin, attribute};

#[derive(Component)]
struct Mark;

attribute!(Health);

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (
                flag_dirty_modifier_nodes::<Health>,
                pretty_print_tree_system::<Health>,
                update_attribute_tree_system::<Health>,
                pretty_print_tree_system::<Health>,
            )
                .chain(),
        )
        .add_systems(Update, modify);

    app.update();
    println!("------------------------------ [Hello, world!]------------------");
    app.update();
}

fn setup(mut commands: Commands) {
    let player = commands.spawn_empty().id();
    ActorBuilder::new(player)
        .with_attribute::<Health>(0.0)
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
    let mod4 = commands
        .spawn((
            EffectOf(mod3),
            Name::new("M4"),
            Modifier::<Health>::new(1.0, Additive),
            ModAggregator::<Health>::default(),
        ))
        .id();
    let effect = commands.spawn_empty().id();
    EffectBuilder::new(mod4, effect)
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

fn modify(mut query: Query<&mut Modifier<Health>, With<Mark>>) {
    let mut modifier = query.single_mut().unwrap();
    modifier.value = ModAggregator::additive(120.0);
}
