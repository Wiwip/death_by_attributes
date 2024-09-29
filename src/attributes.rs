use bevy::prelude::Reflect;

pub trait GameAttributeMarker {}

#[derive(Reflect)]
pub struct GameAttribute {
    pub base_value: f32,
    pub current_value: f32,
}

#[macro_export]
macro_rules! easy_attribute {
    ( $StructName:ident) => {
        #[derive(Component, Attribute, Reflect, Deref, DerefMut)]
        pub struct $StructName {
            pub value: GameAttribute,
        }
    };
}
