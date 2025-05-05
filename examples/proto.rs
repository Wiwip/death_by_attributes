use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::prelude::Component;
use bevy::prelude::Reflect;
use bevy::prelude::*;
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attribute;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::effects::Effect;

attribute!(Health);

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(world: &mut World) {}

fn update(effects: Query<&Effect>) {}
