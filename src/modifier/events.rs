use crate::AttributesMut;
use crate::inspector::pretty_type_name;
use crate::math::AbsDiff;
use crate::modifier::calculator::AttributeCalculator;
use crate::prelude::*;
use crate::systems::MarkNodeDirty;
use bevy::prelude::*;

#[derive(Message)]
pub struct ApplyAttributeModifierMessage<T: Attribute> {
    pub target: Entity,
    pub modifier: AttributeModifier<T>,
}

pub fn apply_modifier_events<T: Attribute>(
    mut event_reader: MessageReader<ApplyAttributeModifierMessage<T>>,
    mut attributes: Query<AttributesMut>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let has_changed = apply_modifier(&ev, &mut attributes).unwrap_or(false);
        
        if has_changed {
            commands.trigger(MarkNodeDirty::<T> {
                entity: ev.target,
                phantom_data: Default::default(),
            });
        }
    }
}

pub fn apply_modifier<T: Attribute>(
    ev: &ApplyAttributeModifierMessage<T>,
    attributes: &mut Query<AttributesMut>,
) -> Result<bool, BevyError> {
    let attributes_ref = attributes.get(ev.target)?;

    let base_value = attributes_ref
        .get::<T>()
        .ok_or(format!(
            "Could not find attribute {} on entity {}.",
            pretty_type_name::<T>(),
            ev.target
        ))?
        .current_value();

    let Ok(calculator) = AttributeCalculator::<T>::convert(&ev.modifier, &attributes_ref) else {
        return Err(format!("Could not convert modifier {} to calculator.", ev.modifier).into());
    };
    let new_base_value = calculator.eval(base_value);

    let has_changed = new_base_value.are_different(base_value);
    if has_changed {
        let mut attributes_mut = attributes.get_mut(ev.target)?;

        let mut attribute = attributes_mut.get_mut::<T>().ok_or(format!(
            "Could not find attribute {} on entity {}.",
            pretty_type_name::<T>(),
            ev.target
        ))?;

        attribute.set_base_value(new_base_value);
    }
    Ok(has_changed)
}
