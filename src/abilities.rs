use crate::attributes::AttributeAccessorMut;
use crate::effects::GameEffect;
use crate::{AttributeEntityMut, AttributeEntityRef};

use crate::modifiers::ModType::Additive;
use crate::modifiers::{AttributeModVariable, AttributeModifier, ModEvaluator};
use bevy::platform::collections::HashMap;
use bevy::prelude::ops::abs;
use bevy::prelude::*;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub type AbilityActivationFn = fn(&mut AttributeEntityMut, Commands);

#[derive(Component, Default)]
pub struct GameAbilityContainer {
    abilities: RwLock<HashMap<String, GameAbility>>,
}

impl GameAbilityContainer {
    pub fn grant_ability(&mut self, name: String, ability: GameAbility) {
        self.abilities.write().unwrap().insert(name, ability);
    }

    pub fn get_abilities_mut(&mut self) -> RwLockWriteGuard<'_, HashMap<String, GameAbility>> {
        self.abilities.write().unwrap()
    }

    pub fn get_abilities(&self) -> RwLockReadGuard<'_, HashMap<String, GameAbility>> {
        self.abilities.read().unwrap()
    }
}

#[derive(Default)]
pub struct GameAbilityBuilder {
    ability: GameAbility,
}

impl GameAbilityBuilder {
    pub fn with_effect(mut self, effect: GameEffect, who: GameEffectTarget) -> Self {
        self.ability.applied_effects.push((who, effect));
        self
    }

    pub fn with_cost(
        mut self,
        cost: f32,
        attribute_ref: impl AttributeAccessorMut + Clone,
    ) -> Self {
        let evaluator = ModEvaluator::new(cost, Additive);
        self.ability.cost = Some(AttributeModVariable::new(AttributeModifier::new(
            attribute_ref,
            evaluator,
        )));
        self
    }

    pub fn with_cooldown(mut self, seconds: f32) -> Self {
        self.ability.cooldown = Timer::from_seconds(seconds, TimerMode::Once);
        self
    }

    pub fn with_activation(mut self, function: AbilityActivationFn) -> Self {
        self.ability.ability_activation = Some(function);
        self
    }

    pub fn build(self) -> GameAbility {
        self.ability
    }
}

pub enum GameEffectTarget {
    Caller,
    Target,
}

#[derive(Default)]
pub struct GameAbility {
    pub applied_effects: Vec<(GameEffectTarget, GameEffect)>,
    pub cost: Option<AttributeModVariable>,
    pub cooldown: Timer,
    pub ability_activation: Option<AbilityActivationFn>,
}

impl GameAbility {
    pub fn try_activate(&mut self, entity_mut: &mut AttributeEntityMut, commands: Commands) {
        if self.can_activate(entity_mut) {
            self.commit_cost(entity_mut);

            if let Some(activation_function) = self.ability_activation {
                activation_function(entity_mut, commands);
            }
        }
    }

    pub fn can_activate(&self, entity_ref: &mut AttributeEntityMut) -> bool {
        // Check cooldown first. If ability is still on cooldown, we cannot activate it yet.
        if !self.cooldown.finished() {
            return false;
        }

        // If there's no cost, the ability is free and usable.
        let Some(modifier) = &self.cost else {
            return true;
        };

        let current_value = modifier.0.get_current_value(entity_ref);
        let cost_magnitude = modifier.0.get_magnitude();

        f32::abs(cost_magnitude) <= current_value
    }

    pub fn commit_cost(&mut self, entity_mut: &mut AttributeEntityMut) {
        if let Some(modifier) = &self.cost {
            modifier.0.apply(entity_mut).expect("couldn't commit");
        }

        self.cooldown.reset();
    }
}
