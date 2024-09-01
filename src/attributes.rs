use bevy::prelude::Reflect;

pub trait GameAttributeMarker {}

#[derive(Reflect)]
pub struct GameAttribute {
    pub base_value: f32,
    pub current_value: f32,
}
