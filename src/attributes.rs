
pub trait AttributeComponent {
    fn get_base_value(&self) -> f32;
    fn get_base_value_mut(&mut self) -> &mut f32;
    fn get_current_value(&self) -> f32;
    fn get_current_value_mut(&mut self) -> &mut f32;
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(Component, Default, Clone, Reflect, Debug)]
        #[require(GameAbilityContainer)]
        pub struct $StructName {
            pub base_value: f32,
            pub current_value: f32,
        }

        impl AttributeComponent for $StructName {
            fn get_base_value(&self) -> f32 {
                self.base_value
            }
            fn get_base_value_mut(&mut self) -> &mut f32 {
                &mut self.base_value
            }
            fn get_current_value(&self) -> f32 {
                self.current_value
            }
            fn get_current_value_mut(&mut self) -> &mut f32 {
                &mut self.current_value
            }
        }

        impl $StructName {
            pub fn new(value: f32) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
        }
    };
}
