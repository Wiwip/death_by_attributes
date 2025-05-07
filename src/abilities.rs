use crate::effects::Effect;
use crate::{ActorEntityMut, OnBaseValueChanged};

use crate::attributes::AttributeComponent;
use bevy::ecs::component::Mutable;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub type AbilityActivationFn = fn(ActorEntityMut, Commands);

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
    pub fn with_effect(mut self, effect: Effect, who: GameEffectTarget) -> Self {
        self.ability.applied_effects.push((who, effect));
        self
    }

    pub fn with_cost<C: Component<Mutability = Mutable> + AttributeComponent>(
        mut self,
        cost: f32,
    ) -> Self {
        //let evaluator = FixedEvaluator::new(cost, Additive);
        //self.ability.cost = Some(Mutator::new(MutatorHelper::new::<C>(evaluator)));
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
    pub applied_effects: Vec<(GameEffectTarget, Effect)>,
    //pub cost: Option<Modifier>,
    pub cooldown: Timer,
    pub ability_activation: Option<AbilityActivationFn>,
}

impl GameAbility {
    /*pub fn try_activate(&mut self, mut entity_mut: ActorEntityMut, mut commands: Commands) {
        if self.can_activate(entity_mut.reborrow()) {
            self.commit_cost(entity_mut.reborrow());

            // Trigger update of the current value
            commands.trigger_targets(OnBaseValueChanged, entity_mut.id());
            //commands.trigger_targets(OnCurrentValueChanged, entity_mut.id());

            if let Some(activation_function) = self.ability_activation {
                activation_function(entity_mut, commands);
            }
        }
    }

    pub fn can_activate(&self, entity: ActorEntityMut) -> bool {
        // Check cooldown first. If ability is still on cooldown, we cannot activate it yet.
        if !self.cooldown.finished() {
            return false;
        }

        // If there's no cost, the ability is free and usable.
        let Some(modifier) = &self.cost else {
            return true;
        };

        let current_value = match modifier.0.get_current_value(entity) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let cost_magnitude = modifier.0.get_magnitude();

        f32::abs(cost_magnitude) <= current_value
    }

    pub fn commit_cost(&mut self, entity_mut: ActorEntityMut) {
        if let Some(mutator) = &self.cost {
            mutator.0.apply_mutator(entity_mut);
        }

        self.cooldown.reset();
    }*/
}
