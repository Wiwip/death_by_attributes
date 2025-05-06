#[warn(unused_imports)]
use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::mutators::Mutator;
use death_by_attributes::mutators::meta::AttributeBuilder;
use death_by_attributes::mutators::mutator::ModType::Additive;
use death_by_attributes::{
    DeathByAttributesPlugin, OnAttributeChanged, OnCurrentValueChanged, attribute,
};
use std::marker::PhantomData;

attribute!(Health);
attribute!(AttackPower);
attribute!(Strength);
attribute!(Agility);

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(DeathByAttributesPlugin)
        .add_plugins(LogPlugin {
            level: bevy::log::Level::DEBUG,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .add_systems(Update, print_attack_power)
        .add_systems(PostUpdate, exit)
        .run();
}

fn setup(mut commands: Commands) {
    let actor = commands
        .spawn((Health::new(100.0), Strength::new(8.0), Agility::new(12.0)))
        .id();

    AttributeBuilder::<AttackPower>::new(actor)
        .mutate_by_attribute::<Strength>(1.0, Additive)
        .mutate_by_attribute::<Agility>(0.25, Additive)
        .build(&mut commands);

    /*commands.trigger_targets(
        OnAttributeChanged::<AttackPower>::default(),
        actor,
    );*/
}

fn update(mut query: Query<&Mutator>) {
    for mutator in query.iter_mut() {
        //println!("{:#?}", mutator);
    }
}

fn print_attack_power(query: Query<&AttackPower>) {
    for power in query.iter() {
        println!("{:?}", power);
    }
}

fn exit(mut exit: EventWriter<AppExit>) {
    exit.write(AppExit::Success);
}
