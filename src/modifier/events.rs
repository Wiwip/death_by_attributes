use crate::attributes::BoxAttributeAccessor;
use crate::prelude::{AttributeCalculator, Mod};
use crate::AttributesMut;
use bevy::prelude::*;

#[derive(Event)]
pub struct ApplyAttributeModifierEvent {
    pub target: Entity,
    pub modifier: Mod,
    pub attribute: BoxAttributeAccessor,
}

pub fn apply_modifier_events(
    mut event_reader: EventReader<ApplyAttributeModifierEvent>,
    mut attributes: Query<AttributesMut>,
) {
    for ev in event_reader.read() {
        let result = apply_modifier(&ev, &mut attributes);
        match result {
            Ok(_) => {
            }
            Err(e) => {
                error!("Error applying modifier: {}", e);
            }
        }
    }
}

pub fn apply_modifier(
    ev: &ApplyAttributeModifierEvent,
    attributes: &mut Query<AttributesMut>,
) -> Result<(), BevyError> {
    let attributes_ref = attributes.get(ev.target)?;
    let base_value = ev.attribute.base_value(&attributes_ref)?;
    let calculator = AttributeCalculator::from(ev.modifier);
    let new_base_value = calculator.eval(base_value);

    let is_value_changed = (new_base_value - base_value).abs() > f64::EPSILON;
    if is_value_changed {
        let mut attributes_mut = attributes.get_mut(ev.target)?;
        ev.attribute
            .set_base_value(new_base_value, &mut attributes_mut)?;
    }
    Ok(())
}
