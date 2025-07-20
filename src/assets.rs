use bevy::prelude::*;
use crate::conditions::ErasedCondition;
use crate::effects::{EffectDurationPolicy, EffectPeriodicTimer};
use crate::modifiers::{Mutator, ModifierFn};
use crate::stacks::EffectStackingPolicy;

#[derive(Asset, TypePath)]
pub struct GameEffect {
    pub effect_fn: Vec<Box<ModifierFn>>,
    pub effect_modifiers: Vec<Box<dyn Mutator>>,
    pub modifiers: Vec<Box<dyn Mutator>>,
    pub duration: EffectDurationPolicy,
    pub period: Option<EffectPeriodicTimer>,
    pub conditions: Vec<ErasedCondition>,
    pub stacking_policy: EffectStackingPolicy
}

