use crate::mutators::ModAggregator;

pub trait AttributeComponent {
    fn new(value: f32) -> Self;

    fn base_value(&self) -> f32;
    fn set_base_value(&mut self, value: f32);
    fn current_value(&self) -> f32;
    fn set_current_value(&mut self, value: f32);
    fn aggregator(&self) -> ModAggregator;
    fn aggregator_mut(&mut self) -> &mut ModAggregator;
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(bevy::prelude::Component, Default, Clone, bevy::prelude::Reflect, Debug)]
        #[require($crate::abilities::GameAbilityContainer)]
        pub struct $StructName {
            base_value: f32,
            current_value: f32,
            aggregator: $crate::mutators::ModAggregator,
        }

        impl $crate::attributes::AttributeComponent for $StructName  {
            fn new(value: f32) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                    aggregator: $crate::mutators::ModAggregator::default(),
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
            fn aggregator(&self) -> $crate::mutators::ModAggregator {
                self.aggregator
            }
            fn aggregator_mut(&mut self) -> &mut $crate::mutators::ModAggregator {
                &mut self.aggregator
            }
        }
    };
}
