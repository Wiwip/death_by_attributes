use crate::attributes::Value;
use crate::attributes::{Attribute, AttributeExtractor, BoxAttributeAccessor};
use crate::graph::NodeType;
use crate::inspector::pretty_type_name;
use crate::math::AbsDiff;
use crate::modifier::calculator::{AttributeCalculator, ModOp};
use crate::modifier::{Modifier, ModifierMarker};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::{ApplyAttributeModifierEvent, AttributeTypeId, EffectSource, EffectTarget};
use crate::{AttributesMut, AttributesRef};
use bevy::prelude::*;
use petgraph::algo::has_path_connecting;
use std::any::type_name;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    #[reflect(ignore)]
    pub value_source: Value<T::Property>,
    pub who: Who,
    pub operation: ModOp,
    pub scaling: f64,
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(value: Value<T::Property>, modifier: ModOp, who: Who, scaling: f64) -> Self {
        Self {
            value_source: value,
            who,
            operation: modifier,
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
        let scaling = if self.scaling.fract() == 0.0 {
            format!("{}", "")
        } else {
            format!("[*{:.2}]", self.scaling)
        };

        write!(
            f,
            "Mod<{}>({}{}{}, {})",
            pretty_type_name::<T>(),
            self.operation,
            self.value_source,
            scaling,
            self.who,
        )
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
                    value_source: self.value_source.clone(),
                    who: self.who,
                    operation: self.operation,
                    scaling: self.scaling,
                },
                Name::new(format!("Mod<{}> ({:?})", pretty_type_name::<T>(), self.who)),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut AttributesMut) -> bool {
        // Measure the modifier
        let new_val = match actor_entity.get::<T>() {
            None => panic!("Could not find attribute {}", type_name::<T>()),
            Some(attribute) => {
                let entity = actor_entity.as_readonly();

                let calculator = AttributeCalculator::<T>::convert(self, &entity);
                let new_val = calculator.eval(attribute.base_value());
                new_val
            }
        };

        // Apply the modifier
        if let Some(mut attribute) = actor_entity.get_mut::<T>() {
            // Ensure that the modifier meaningfully changed the value before we trigger the event.
            let has_changed = new_val.are_different(attribute.current_value());
            if has_changed {
                attribute.set_base_value(new_val);
            }
            has_changed
        } else {
            panic!("Could not find attribute {}", type_name::<T>());
        }
    }

    fn write_event(&self, target: Entity, commands: &mut Commands) {
        commands.send_event(ApplyAttributeModifierEvent::<T> {
            target,
            modifier: self.clone(),
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
