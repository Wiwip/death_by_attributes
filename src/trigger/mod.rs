mod builder;

use bevy::prelude::*;
use std::marker::PhantomData;

/// Triggers are essentially automated abilities.
/// An ability or effect is automatically applied whenever the conditions of the trigger are met.
/// So far my trigger ideas are:
/// - AbilityTrigger
/// - EffectTrigger
/// - TimedTrigger
pub struct TriggerPlugin;

impl Plugin for TriggerPlugin {
    fn build(&self, _app: &mut App) {
        //app.add_systems();
    }
}
