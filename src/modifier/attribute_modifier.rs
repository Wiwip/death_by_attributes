use crate::attributes::AccessAttribute;
use crate::attributes::{Attribute, AttributeExtractor, BoxAttributeAccessor};
use crate::graph::{AttributeTypeId, NodeType};
use crate::inspector::pretty_type_name;
use crate::modifier::calculator::{AttributeCalculator, Mod};
use crate::modifier::{ModifierMarker, Mutator};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::{EffectSource, EffectTarget};
use crate::{AttributesMut, AttributesRef};
use bevy::prelude::*;
use fixed::traits::Fixed;
use std::any::type_name;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    pub who: Who,
    #[reflect(ignore)]
    pub modifier: Mod<T::Property>,
    pub scaling: f64,
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(modifier: Mod<T::Property>, who: Who, scaling: f64) -> Self {
        Self {
            who,
            modifier,
            scaling,
        }
    }

    pub fn as_accessor(&self) -> BoxAttributeAccessor<T> {
        BoxAttributeAccessor::new(AttributeExtractor::<T>::new())
    }
}

impl<T> Display for AttributeModifier<T>
where
    T: Attribute,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mod<{}>({:.1})", pretty_type_name::<T>(), self.modifier)
    }
}

impl<T> Mutator for AttributeModifier<T>
where
    T: Attribute,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity {
        /*debug!(
            "[{}] Added Mod<{}> [{}]",
            actor_entity.id(),
            pretty_type_name::<T>(),
            self.modifier,
        );*/

        commands
            .spawn((
                NodeType::Modifier,
                EffectSource(actor_entity.id()),
                EffectTarget(actor_entity.id()),
                AttributeModifier::<T> {
                    who: self.who,
                    modifier: self.modifier,
                    scaling: self.scaling,
                },
                Name::new(format!("Mod<{}> ({:?})", pretty_type_name::<T>(), self.who)),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut AttributesMut) -> bool {
        if let Some(mut attribute) = actor_entity.get_mut::<T>() {
            println!(
                "Directly applying modifier {} to {}",
                self.modifier,
                attribute.name()
            );
            let calculator = AttributeCalculator::<T>::from(self.modifier);
            let new_val = calculator.eval(attribute.base_value());
            // Ensure that the modifier meaningfully changed the value before we trigger the event.

            let has_changed = new_val.abs_diff(attribute.base_value()) > 0;
            if has_changed {
                attribute.set_base_value(new_val);
                true
            } else {
                false
            }
        } else {
            panic!("Could not find attribute {}", type_name::<T>());
        }
    }

    fn who(&self) -> Who {
        self.who
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
