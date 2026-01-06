use crate::Spawnable;
use crate::condition::{GameplayContext, GameplayContextMut};
use crate::expression::Expr;
use crate::inspector::pretty_type_name;
use crate::math::AbsDiff;
use crate::modifier::calculator::{AttributeCalculator, ModOp};
use crate::modifier::events::ApplyAttributeModifierMessage;
use crate::modifier::{Modifier, ModifierMarker};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::*;
use bevy::prelude::*;
use std::any::type_name;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    #[reflect(ignore)]
    pub expression: Expr<T::Property>,
    pub who: Who,
    pub operation: ModOp,
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(value: Expr<T::Property>, modifier: ModOp, who: Who) -> Self {
        Self {
            expression: value,
            who,
            operation: modifier,
        }
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
            "_", //self.expression,
            self.who,
        )
    }
}

impl<T: Attribute> Modifier for AttributeModifier<T> {
    fn apply_immediate(&self, context: &mut GameplayContextMut) -> bool {
        let immutable_context = GameplayContext {
            source_actor: &context.attribute_ref(Who::Source),
            target_actor: &context.attribute_ref(Who::Target),
            owner: &context.attribute_ref(Who::Owner),
        };

        let Ok(calc) = AttributeCalculator::<T>::convert(self, &immutable_context) else {
            return false;
        };
        let Some(attribute) = context.attribute_ref(Who::Target).get::<T>() else {
            return false;
        };
        let new_val = calc.eval(attribute.base_value());

        let mut attributes_mut = context.attribute_mut(self.who);
        // Apply the modifier
        if let Some(mut attribute) = attributes_mut.get_mut::<T>() {
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

    fn apply_delayed(
        &self,
        source: Entity,
        target: Entity,
        effect: Entity,
        commands: &mut Commands,
    ) {
        commands.write_message(ApplyAttributeModifierMessage::<T> {
            source_entity: source,
            target_entity: target,
            effect_entity: effect,
            modifier: self.clone(),
        });
    }
}

impl<T: Attribute> Spawnable for AttributeModifier<T> {
    fn spawn(&self, commands: &mut EntityCommands) {
        commands.insert((
            AttributeModifier::<T> {
                expression: self.expression.clone(),
                who: self.who,
                operation: self.operation,
            },
            Name::new(format!("{}", self)),
        ));
    }

    fn who(&self) -> Who {
        self.who
    }
}
