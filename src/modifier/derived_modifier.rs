use crate::attributes::{Attribute, AttributeExtractor, BoxAttributeAccessor};
use crate::graph::{NodeType};
use crate::modifier::calculator::AttributeCalculator;
use crate::modifier::{Modifier, Who};
use crate::prelude::*;
use crate::systems::on_change_attribute_observer;
use crate::{AttributesMut, AttributesRef};
use bevy::log::debug;
use bevy::prelude::{Commands, Entity, Name, Observer, Reflect};
use fixed::prelude::{LossyFrom, LossyInto};
use fixed::traits::Fixed;
use std::any::type_name;
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, Reflect)]
pub struct DerivedModifier<S: Attribute, T: Attribute> {
    #[reflect(ignore)]
    _target: PhantomData<T>,
    #[reflect(ignore)]
    _source: PhantomData<S>,
    pub who: Who,
    pub modifier: Mod<T::Property>,
    pub scaling: f64,
}

impl<S, T> DerivedModifier<S, T>
where
    S: Attribute,
    T: Attribute,
{
    pub fn new(modifier: Mod<T::Property>, who: Who, scaling: f64) -> Self {
        Self {
            _target: Default::default(),
            _source: Default::default(),
            who,
            modifier,
            scaling,
        }
    }
}

impl<S, T> Modifier for DerivedModifier<S, T>
where
    S: Attribute,
    T: Attribute,
    T::Property: LossyFrom<S::Property>,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity {
        debug!(
            "Added modifier<{}> [{}] to {}",
            type_name::<T>(),
            type_name::<S>(),
            actor_entity.id()
        );
        let target_entity = actor_entity.id();
        commands
            .spawn((
                NodeType::Modifier,
                EffectSource(target_entity),
                EffectTarget(target_entity),
                Name::new(format!("{}", type_name::<T>())),
                AttributeModifier::<T>::new(self.modifier, self.who, self.scaling),
                AttributeDependency::<S>::new(target_entity),
                Observer::new(on_change_attribute_observer::<S, T>),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut AttributesMut) -> bool {
        let Some(origin_value) = actor_entity.get::<S>() else {
            panic!("Should have found source attribute");
        };
        let value = origin_value.current_value();

        if let Some(mut target_attribute) = actor_entity.get_mut::<T>() {
            let source_value = T::Property::lossy_from(value);
            let scaled_modifier = self.modifier * source_value;
            let calculator = AttributeCalculator::<T>::from(scaled_modifier);
            let new_val = calculator.eval(target_attribute.base_value());

            let is_value_changed = new_val.abs_diff(target_attribute.base_value()) > 0;
            if is_value_changed {
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

    fn write_event(&self, target: Entity, commands: &mut Commands) {
        commands.send_event(ApplyAttributeModifierEvent::<T> {
            target,
            modifier: self.modifier,
            attribute: BoxAttributeAccessor::new(AttributeExtractor::<T>::new()),
        });
    }
    
    /*fn modifier(&self) -> Mod<T::Property> {
        self.modifier
    }

    fn as_accessor(&self) -> BoxAttributeAccessor<T> {
        BoxAttributeAccessor::new(AttributeExtractor::<T>::new())
    }*/

    fn attribute_type_id(&self) -> AttributeTypeId {
        T::attribute_type_id()
    }
}
