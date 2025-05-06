use crate::mutators::mutator::ModAggregator;
use bevy::prelude::{Component, Event, Observer};
use std::fmt::{Debug, Display};
use bevy::ecs::component::Mutable;
use crate::attributes::AttributeComponent;

pub mod meta;
pub mod fixed;

pub trait MutatorEvaluator: Debug + Display + Send + Sync + 'static {
    fn get_magnitude(&self) -> f32;
    fn set_magnitude(&mut self, magnitude: f32);
    fn get_aggregator(&self) -> ModAggregator;
    fn get_observer<O: Event, T: Component<Mutability = Mutable> + AttributeComponent>(&self) -> Option<Observer>;
}
