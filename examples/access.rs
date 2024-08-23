use bevy::prelude::*;

#[derive(Resource)]
struct Bar;

fn sys_ref(_: Query<EntityRef>, _: ResMut<Bar>) {}

fn sys_mut(_: Query<EntityMut>, _: Res<Bar>) {}

fn main() {
    App::new()
        .insert_resource(Bar)
        .add_systems(Update, sys_ref)
        .add_systems(Update, sys_mut)
        .run();
}
