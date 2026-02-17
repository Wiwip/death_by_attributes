use crate::effect::{EffectApplicationPolicy, EffectStackingPolicy};
use crate::modifier::{Modifier, ModifierFn};
use crate::mutator::EntityActions;
use bevy::prelude::*;
use express_it::logic::BoolExpr;
use std::collections::VecDeque;
use express_it::frame::LazyPlan;

#[derive(Asset, TypePath)]
pub struct ActorDef {
    pub name: String,
    pub description: String,
    pub builder_actions: VecDeque<EntityActions>,
    pub abilities: Vec<Handle<AbilityDef>>,
    pub effects: Vec<Handle<EffectDef>>,
}

#[derive(Asset, TypePath)]
pub struct EffectDef {
    pub application_policy: EffectApplicationPolicy,
    pub stacking_policy: EffectStackingPolicy,

    pub effect_fn: Vec<Box<ModifierFn>>,
    pub modifiers: Vec<Box<dyn Modifier>>,

    pub attach_conditions: Vec<BoolExpr>,
    pub activate_conditions: Vec<BoolExpr>,

    pub on_actor_triggers: Vec<EntityActions>,
    pub on_effect_triggers: Vec<EntityActions>,
}

#[derive(Asset, TypePath)]
pub struct AbilityDef {
    pub name: String,
    pub description: String,

    pub mutators: Vec<EntityActions>,
    pub observers: Vec<EntityActions>,
    pub cost: Vec<BoolExpr>,
    pub execution_conditions: Vec<BoolExpr>,
    pub cost_modifiers: Vec<Box<dyn Modifier>>,

    pub on_execute: Vec<LazyPlan>,
}
