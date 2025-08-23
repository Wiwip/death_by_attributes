mod builder;

use bevy::app::{App, Plugin};

/// Triggers are essentially automated abilities.
/// An ability or effect is automatically applied whenever the conditions of the trigger are met
pub struct TriggerPlugin;

impl Plugin for TriggerPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}
