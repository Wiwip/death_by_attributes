use crate::ability::AbilityActivationFn;
use crate::condition::{BoxCondition, CustomExecution, StoredExecution};
use crate::effect::EffectExecution;
use crate::effect::EffectStackingPolicy;
use crate::modifier::{Modifier, ModifierFn};
use crate::mutator::EntityActions;
use crate::prelude::EffectApplicationPolicy;
use bevy::prelude::*;

#[derive(Asset, TypePath)] //, Serialize)]
pub struct ActorDef {
    pub name: String,
    pub description: String,
    pub mutators: Vec<EntityActions>,
    pub abilities: Vec<Handle<AbilityDef>>,
    pub effects: Vec<Handle<EffectDef>>,
}

#[derive(Asset, TypePath)]
pub struct EffectDef {
    pub effect_fn: Vec<Box<ModifierFn>>,
    pub effect_modifiers: Vec<Box<dyn Modifier>>,
    pub execution: Option<Box<dyn EffectExecution>>,
    pub modifiers: Vec<Box<dyn Modifier>>,
    pub application: EffectApplicationPolicy,
    pub conditions: Vec<BoxCondition>,
    pub stacking_policy: EffectStackingPolicy,
    pub intensity: Option<f32>,
}

#[derive(Asset, TypePath)]
pub struct AbilityDef {
    pub name: String,
    pub description: String,
    pub executions: Vec<StoredExecution>,
    pub mutators: Vec<EntityActions>,
    pub cost: Vec<BoxCondition>,
    pub condition: Vec<BoxCondition>,
    pub cost_effects: Vec<Box<dyn Modifier>>,
    pub activation_fn: AbilityActivationFn,
}
