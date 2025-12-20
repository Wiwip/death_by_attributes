use crate::AttributesMut;
use crate::attributes::BoxAttributeAccessor;
use crate::math::AbsDiff;
use crate::prelude::{Attribute, AttributeCalculator, AttributeModifier};
use crate::systems::MarkNodeDirty;
use bevy::prelude::*;

#[derive(Message)]
pub struct ApplyAttributeModifierMessage<T: Attribute> {
    pub target: Entity,
    pub modifier: AttributeModifier<T>,
    pub attribute: BoxAttributeAccessor<T::Property>,
}

pub fn apply_modifier_events<T: Attribute>(
    mut event_reader: MessageReader<ApplyAttributeModifierMessage<T>>,
    mut attributes: Query<AttributesMut>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let result = apply_modifier(&ev, &mut attributes);

        match result {
            Ok(has_changed) => {
                if has_changed {
                    commands.trigger(MarkNodeDirty::<T> {
                        entity: ev.target,
                        phantom_data: Default::default(),
                    });
                }
            }
            Err(e) => {
                error!("Error applying modifier: {}", e);
            }
        }
    }
}

pub fn apply_modifier<T: Attribute>(
    ev: &ApplyAttributeModifierMessage<T>,
    attributes: &mut Query<AttributesMut>,
) -> Result<bool, BevyError> {
    let attributes_ref = attributes.get(ev.target)?;
    let base_value = ev.attribute.base_value(&attributes_ref)?;
    let Ok(calculator) = AttributeCalculator::<T>::convert(&ev.modifier, &attributes_ref) else {
        return Err(format!("Could not convert modifier {} to calculator.", ev.modifier).into());
    };
    let new_base_value = calculator.eval(base_value);

    let has_changed = new_base_value.are_different(base_value);
    if has_changed {
        let mut attributes_mut = attributes.get_mut(ev.target)?;
        ev.attribute
            .set_base_value(new_base_value, &mut attributes_mut)?;
    }
    Ok(has_changed)
}
