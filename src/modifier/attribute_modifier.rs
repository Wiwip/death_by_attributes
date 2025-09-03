use crate::attributes::AccessAttribute;
use crate::attributes::{Attribute, AttributeExtractor, BoxAttributeAccessor};
use crate::graph::NodeType;
use crate::inspector::pretty_type_name;
use crate::modifier::calculator::{AttributeCalculator, Mod};
use crate::modifier::{Modifier, ModifierMarker};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::{ApplyAttributeModifierEvent, AttributeTypeId, EffectSource, EffectTarget};
use crate::{AttributesMut, AttributesRef};
use bevy::prelude::*;
use serde::Serialize;
use std::any::type_name;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Component, Copy, Clone, Debug, Reflect, Serialize)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    pub who: Who,
    #[reflect(ignore)]
    pub modifier: Mod<T::Property>,
    pub scaling: T::Property,
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(modifier: Mod<T::Property>, who: Who, scaling: T::Property) -> Self {
        Self {
            who,
            modifier,
            scaling,
        }
    }

    pub fn as_accessor(&self) -> BoxAttributeAccessor<T::Property> {
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

impl<T> Modifier for AttributeModifier<T>
where
    T: Attribute,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity {
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

            //let has_changed = new_val.abs_diff(attribute.base_value()) > 0;
            //if has_changed {
                attribute.set_base_value(new_val);
                true
            //} else {
            //    false
            //}
        } else {
            panic!("Could not find attribute {}", type_name::<T>());
        }
    }

    fn write_event(&self, target: Entity, commands: &mut Commands) {
        commands.send_event(ApplyAttributeModifierEvent::<T> {
            target,
            modifier: self.modifier,
            attribute: BoxAttributeAccessor::new(AttributeExtractor::<T>::new()),
        });
    }

    fn who(&self) -> Who {
        self.who
    }

    fn attribute_type_id(&self) -> AttributeTypeId {
        T::attribute_type_id()
    }
}
