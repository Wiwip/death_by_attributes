use crate::AttributesMut;
use crate::attributes::BoxAttributeAccessor;
use crate::prelude::{Attribute, AttributeCalculator, Mod};
use bevy::prelude::*;
use fixed::traits::Fixed;

#[derive(Event)]
pub struct ApplyAttributeModifierEvent<T: Attribute> {
    pub target: Entity,
    pub modifier: Mod<T::Property>,
    pub attribute: BoxAttributeAccessor<T>,
}

pub fn apply_modifier_events<T: Attribute>(
    mut event_reader: EventReader<ApplyAttributeModifierEvent<T>>,
    mut attributes: Query<AttributesMut>,
) {
    for ev in event_reader.read() {
        let result = apply_modifier(&ev, &mut attributes);
        match result {
            Ok(_) => {}
            Err(e) => {
                error!("Error applying modifier: {}", e);
            }
        }
    }
}

pub fn apply_modifier<T: Attribute>(
    ev: &ApplyAttributeModifierEvent<T>,
    attributes: &mut Query<AttributesMut>,
) -> Result<(), BevyError> {
    let attributes_ref = attributes.get(ev.target)?;
    let base_value = ev.attribute.base_value(&attributes_ref)?;
    let calculator = AttributeCalculator::<T>::from(ev.modifier);
    let new_base_value = calculator.eval(base_value);
        
    let has_changed = new_base_value.abs_diff(base_value) > 0;
    if has_changed {
        let mut attributes_mut = attributes.get_mut(ev.target)?;
        ev.attribute
            .set_base_value(new_base_value, &mut attributes_mut)?;
    }
    Ok(())
}
