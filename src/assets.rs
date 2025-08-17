use crate::ability::AbilityActivationFn;
use crate::condition::BoxCondition;
use crate::effect::EffectExecution;
use crate::effect::EffectStackingPolicy;
use crate::modifier::{ModifierFn, Mutator};
use crate::mutator::EntityMutator;
use crate::prelude::EffectApplicationPolicy;
use bevy::prelude::*;

#[derive(Asset, TypePath)]
pub struct ActorDef {
    pub name: String,
    pub description: String,
    pub mutators: Vec<EntityMutator>,
    pub abilities: Vec<Handle<AbilityDef>>,
    pub effects: Vec<Handle<EffectDef>>,
}

#[derive(Asset, TypePath)]
pub struct EffectDef {
    pub effect_fn: Vec<Box<ModifierFn>>,
    pub effect_modifiers: Vec<Box<dyn Mutator>>,
    pub custom_execution: Option<Box<dyn EffectExecution>>,
    pub modifiers: Vec<Box<dyn Mutator>>,
    pub application: EffectApplicationPolicy,
    pub conditions: Vec<BoxCondition>,
    pub stacking_policy: EffectStackingPolicy,
    pub intensity: Option<f32>,
}

#[derive(Asset, TypePath)]
pub struct AbilityDef {
    pub name: String,
    pub description: String,
    pub mutators: Vec<EntityMutator>,
    pub cost: Vec<BoxCondition>,
    pub cost_effects: Vec<Box<dyn Mutator>>,
    pub activation_fn: AbilityActivationFn,
}
