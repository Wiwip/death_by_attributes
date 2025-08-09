use crate::attributes::Attribute;
use crate::modifier::calculator::AttributeCalculator;
use crate::modifier::{Mutator, Who};
use crate::prelude::*;
use crate::{AttributesMut, AttributesRef, OnAttributeValueChanged};
use bevy::log::debug;
use bevy::prelude::{Commands, Entity, Name, Observer, Query, Reflect, Trigger};
use std::any::type_name;
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, Reflect)]
pub struct DerivedModifier<T, S> {
    #[reflect(ignore)]
    _target: PhantomData<T>,
    #[reflect(ignore)]
    _source: PhantomData<S>,
    pub who: Who,
    pub modifier: Mod,
    pub scaling_factor: f64,
}

impl<T, S> DerivedModifier<T, S> {
    pub fn new(modifier: Mod, scaling_factor: f64, who: Who) -> Self {
        Self {
            _target: Default::default(),
            _source: Default::default(),
            who,
            scaling_factor,
            modifier,
        }
    }
}

impl<T, S> Mutator for DerivedModifier<T, S>
where
    T: Attribute,
    S: Attribute,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity {
        debug!(
            "Added modifier<{}> [{}] to {}",
            type_name::<T>(),
            type_name::<S>(),
            actor_entity.id()
        );
        let scaling_factor = self.scaling_factor;

        let mut observer = Observer::new(
            // When the source attribute changes, update the modifier of the target attribute.
            move |trigger: Trigger<OnAttributeValueChanged<S>>,
                  attributes: Query<&S>,
                  mut modifiers: Query<&mut AttributeModifier<T>>| {
                let Ok(attribute) = attributes.get(trigger.target()) else {
                    return;
                };
                let Ok(mut modifier) = modifiers.get_mut(trigger.observer()) else {
                    return;
                };

                let value_mut = modifier.modifier.value_mut();
                *value_mut = scaling_factor * attribute.current_value(); // modify by scaling factor
            },
        );
        observer.watch_entity(actor_entity.id());

        let Some(attribute_value) = actor_entity.get::<S>() else {
            panic!(
                "Could not find attribute {} on {}",
                type_name::<S>(),
                actor_entity.id(),
            );
        };
        let value = attribute_value.current_value() * self.scaling_factor;
        let scaled_modifier = self.modifier * value;

        commands
            .spawn((
                Name::new(format!("{}", type_name::<T>())),
                observer,
                AttributeModifier::<T>::new(scaled_modifier, self.who),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut AttributesMut) -> bool {
        let Some(origin_value) = actor_entity.get::<S>() else {
            panic!("Should have found source attribute");
        };
        let value = origin_value.current_value() * self.scaling_factor;

        if let Some(mut target_attribute) = actor_entity.get_mut::<T>() {
            let scaled_modifier = self.modifier * value;
            let calculator = AttributeCalculator::from(scaled_modifier);
            let new_val = calculator.eval(target_attribute.base_value());

            if (new_val - target_attribute.base_value()).abs() > f64::EPSILON {
                target_attribute.set_base_value(new_val);
                true
            } else {
                false
            }
        } else {
            panic!("Could not find target attribute {}", type_name::<T>());
        }
    }

    fn who(&self) -> Who {
        self.who
    }
}
