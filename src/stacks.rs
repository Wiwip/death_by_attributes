use bevy::prelude::*;
use crate::effects::{Effect, EffectDuration};

pub enum EffectStackingPolicy {
    None, // Each effect is independently added to the entity
    Add {
        count: u32,
        max_stack: u32,
    },
    Override, // The effect overrides previous applications
}

#[derive(Component, Reflect)]
pub struct Stacks(pub u32);

impl Default for Stacks {
    fn default() -> Self {
        Self(1) // By default, a new effect has 1 stack
    }
}

/// Applies the appropriate stacking policy to an effect
pub(crate) fn apply_stacking_policy(
    policy: &EffectStackingPolicy,
    effect_entity: Entity,
    stacks: &mut Query<&mut Stacks, With<Effect>>,
    durations: &mut Query<&mut EffectDuration, With<Effect>>,
) {
    match policy {
        EffectStackingPolicy::Add { count, max_stack } => {
            // Apply additive stacking, increasing stack count up to max
            if let Ok(mut stack_count) = stacks.get_mut(effect_entity) {
                stack_count.0 += count;
                stack_count.0 = stack_count.0.clamp(1, *max_stack);
            } else {
                error!("Failed to find Stacks component for entity: {:?}", effect_entity);
            }
        }
        EffectStackingPolicy::Override => {
            // Reset duration for overridden effects
            if let Ok(mut duration) = durations.get_mut(effect_entity) {
                duration.0.reset();
            } else {
                error!("Failed to find EffectDuration component for entity: {:?}", effect_entity);
            }
        }
        EffectStackingPolicy::None => {
            error!(
                "Effect stacking should not be triggered for effect entity {:?} with incompatible policy (None)",
                effect_entity
            );
        }
    }
}
