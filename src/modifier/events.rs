use crate::inspector::pretty_type_name;
use crate::math::AbsDiff;
use crate::modifier::calculator::AttributeCalculator;
use crate::prelude::*;
use crate::systems::MarkNodeDirty;
use crate::{AppAttributeBindings, AttributesMut};
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use crate::context::BevyContext;

#[derive(Message)]
pub struct ApplyAttributeModifierMessage<T: Attribute> {
    pub source_entity: Entity,
    pub target_entity: Entity,
    pub effect_entity: Entity,
    pub modifier: Modifier<T>,
}

pub fn apply_modifier_events<T: Attribute>(
    mut event_reader: MessageReader<ApplyAttributeModifierMessage<T>>,
    mut attributes: Query<AttributesMut>,
    mut commands: Commands,
    type_registry: Res<AppTypeRegistry>,
    type_bindings: Res<AppAttributeBindings>,
) {
    for ev in event_reader.read() {
        let has_changed = apply_modifier(
            &ev,
            &mut attributes,
            type_registry.0.clone(),
            type_bindings.clone(),
        )
        .unwrap_or(false);

        if has_changed {
            commands.trigger(MarkNodeDirty::<T> {
                entity: ev.target_entity,
                phantom_data: Default::default(),
            });
        }
    }
}

pub fn apply_modifier<T: Attribute>(
    trigger: &ApplyAttributeModifierMessage<T>,
    attributes: &mut Query<AttributesMut>,
    type_registry: TypeRegistryArc,
    type_bindings: AppAttributeBindings,
) -> Result<bool, BevyError> {
    let query = [trigger.source_entity, trigger.target_entity];
    let [source, target] = attributes.get_many(query)?;

    let base_value = target
        .get::<T>()
        .ok_or(format!(
            "Could not find attribute {} on entity {}.",
            pretty_type_name::<T>(),
            trigger.target_entity
        ))?
        .base_value();

    // We update the modifier's internal value before applying it.
    let context = BevyContext {
        source_actor: &source,
        target_actor: &target, // Needs to be fixed.
        owner: &source,
        type_registry: type_registry.clone(),
        type_bindings: type_bindings.clone(),
    };
    let mut modifier = trigger.modifier.clone();
    modifier.update_value(&context);

    // Apply the modifier
    let Ok(calculator) = AttributeCalculator::<T>::convert(&modifier) else {
        return Err(format!(
            "Could not convert modifier {} to calculator.",
            modifier,
        )
        .into());
    };
    let new_base_value = calculator.eval(base_value);

    let has_changed = new_base_value.are_different(base_value);
    if has_changed {
        let mut attributes_mut = attributes.get_mut(trigger.target_entity)?;

        let mut attribute = attributes_mut.get_mut::<T>().ok_or(format!(
            "Could not find attribute {} on entity {}.",
            pretty_type_name::<T>(),
            trigger.target_entity
        ))?;

        attribute.set_base_value(new_base_value);
    }
    Ok(has_changed)
}
