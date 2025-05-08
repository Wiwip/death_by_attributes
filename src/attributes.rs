pub trait AttributeComponent {
    fn new(value: f32) -> Self;
    fn base_value(&self) -> f32;
    fn set_base_value(&mut self, value: f32);
    fn current_value(&self) -> f32;
    fn set_current_value(&mut self, value: f32);
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(bevy::prelude::Component, Default, Clone, bevy::prelude::Reflect, Debug)]
        #[require($crate::abilities::GameAbilityContainer, $crate::modifiers::ModAggregator<$StructName>)]
        pub struct $StructName {
            base_value: f32,
            current_value: f32,
        }

        impl $crate::attributes::AttributeComponent for $StructName {
            fn new(value: f32) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
            fn base_value(&self) -> f32 {
                self.base_value
            }
            fn set_base_value(&mut self, value: f32) {
                self.base_value = value;
            }
            fn current_value(&self) -> f32 {
                self.current_value
            }
            fn set_current_value(&mut self, value: f32) {
                self.current_value = value;
            }
        }
    };
}
