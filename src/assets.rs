use crate::abilities::AbilityActivationFn;
use crate::conditions::BoxCondition;
use crate::effects::{EffectDurationPolicy, EffectPeriodicTimer};
use crate::modifiers::{ModifierFn, Mutator};
use crate::mutator::EntityMutator;
use crate::stacks::EffectStackingPolicy;
use crate::ActorEntityMut;
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
    pub modifiers: Vec<Box<dyn Mutator>>,
    pub duration: EffectDurationPolicy,
    pub period: Option<EffectPeriodicTimer>,
    pub conditions: Vec<BoxCondition>,
    pub stacking_policy: EffectStackingPolicy,
}

#[derive(Asset, TypePath)]
pub struct AbilityDef {
    pub name: String,
    pub description: String,
    pub mutators: Vec<EntityMutator>,
    pub cost_fn: Box<dyn Fn(&mut ActorEntityMut, bool) -> bool + Send + Sync>,
    pub activation_fn: AbilityActivationFn,
}
