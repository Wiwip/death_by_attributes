use crate::attributes::Value;
use crate::attributes::{Attribute, AttributeExtractor, BoxAttributeAccessor};
use crate::graph::NodeType;
use crate::inspector::pretty_type_name;
use crate::math::AbsDiff;
use crate::modifier::calculator::{AttributeCalculator, ModOp};
use crate::modifier::{Modifier, ModifierMarker};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::{ApplyAttributeModifierMessage, EffectSource, EffectTarget};
use crate::systems::MarkNodeDirty;
use crate::{AttributesMut, AttributesRef, Spawnable};
use bevy::prelude::*;
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
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(value: Value<T::Property>, modifier: ModOp, who: Who) -> Self {
        Self {
            value_source: value,
            who,
            operation: modifier,
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
        write!(
            f,
            "Mod<{}>({}{}) {}",
            pretty_type_name::<T>(),
            self.operation,
            self.value_source,
            self.who,
        )
    }
}

impl<T: Attribute> Modifier for AttributeModifier<T> {
    fn apply_immediate(&self, actor_entity: &mut AttributesMut) -> bool {
        // Measure the modifier
        let new_val = match actor_entity.get::<T>() {
            None => panic!("Could not find attribute {}", type_name::<T>()),
            Some(attribute) => {
                let entity = actor_entity.as_readonly();

                let Ok(calculator) = AttributeCalculator::<T>::convert(self, &entity) else {
                    warn!("Could not convert modifier {} to calculator.", self);
                    return false;
                };
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

    fn apply_delayed(&self, target: Entity, commands: &mut Commands) {
        commands.write_message(ApplyAttributeModifierMessage::<T> {
            target,
            modifier: self.clone(),
            attribute: BoxAttributeAccessor::new(AttributeExtractor::<T>::new()),
        });
    }
}

impl<T: Attribute> Spawnable for AttributeModifier<T> {
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity {
        let mut entity_commands = commands.spawn((
            NodeType::Modifier,
            EffectSource(actor_entity.id()),
            EffectTarget(actor_entity.id()),
            AttributeModifier::<T> {
                value_source: self.value_source.clone(),
                who: self.who,
                operation: self.operation,
            },
            Name::new(format!("{}", self)),
        ));

        // This is a roundabout way to ensure that this attribute is marked as dirty when the dependency changes.
        let func = |entity: Entity, mut commands: Commands| {
            commands.trigger(MarkNodeDirty::<T> {
                entity,
                phantom_data: Default::default(),
            });
        };
        // This is fine because modifiers with no dependencies have an empty implementation.
        self.value_source
            .insert_dependency(actor_entity.id(), &mut entity_commands, func);

        entity_commands.id()
    }

    fn who(&self) -> Who {
        self.who
    }
}
