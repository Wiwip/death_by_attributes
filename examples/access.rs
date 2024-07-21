

use bevy::prelude::*;

#[derive(Resource)]
struct Bar;

fn sys(_: Query<EntityRef>, _: ResMut<Bar>) {}

fn main() {
    App::new()
        .insert_resource(Bar)
        .add_systems(Update, sys)
        .run();
}