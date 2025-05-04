use bevy::prelude::*;
use std::fmt::{Debug, Formatter};

pub trait AttributeComponent {
    fn get_attribute_mut(&mut self) -> &mut AttributeDef;
    fn get_attribute(&self) -> &AttributeDef;
}

#[derive(Default, Reflect, Debug, Clone)]
pub struct AttributeDef {
    pub base_value: f32,
    pub current_value: f32,
}

impl std::fmt::Display for AttributeDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Base: {}, Current: {}",
            self.base_value, self.current_value
        )
    }
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(Component, Default, Clone, Reflect, Deref, DerefMut, Debug)]
        #[require(GameAbilityContainer)]
        pub struct $StructName {
            pub attribute: AttributeDef,
        }

        impl AttributeComponent for $StructName {
            fn get_attribute_mut(&mut self) -> &mut AttributeDef {
                &mut self.attribute
            }
            fn get_attribute(&self) -> &AttributeDef {
                &self.attribute
            }
        }

        impl $StructName {
            pub fn new(value: f32) -> Self {
                Self {
                    attribute: AttributeDef {
                        base_value: value,
                        current_value: value,
                    },
                }
            }
        }
    };

    ( $StructName:ident, $($RequireStruct:ident),* ) => {
        #[derive(Component, Attribute, Default, Clone, Reflect, Deref, DerefMut, Debug)]
        #[require(GameAbilityContainer)]
        #[require($($RequireStruct),*)]
        pub struct $StructName {
            pub attribute: AttributeDef,
        }

        impl AttributeComponent for $StructName {
            fn get_attribute_mut(&mut self) -> &mut AttributeDef {
                &mut self.attribute
            }
            fn get_attribute(&self) -> &AttributeDef {
                &self.attribute
            }
        }

        impl $StructName {
            pub fn new(value: f32) -> Self {
                Self {
                    attribute: AttributeDef {
                        base_value: value,
                        current_value: value,
                    },
                }
            }
        }
    };
}
