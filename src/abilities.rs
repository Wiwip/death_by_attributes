use crate::attributes::GameAttributeMarker;
use crate::context::GameAttributeContextMut;
use crate::effect::{apply_instant_modifier, GameEffect};
use crate::modifiers::Modifier::Scalar;
use crate::modifiers::{Modifier, ScalarModifier};
use bevy::prelude::*;
use bevy::utils::HashMap;

///
///
///
#[derive(Component, Default)]
pub struct GameAbilityComponent {
    abilities: HashMap<String, GameAbility>,
}

impl GameAbilityComponent {
    pub fn try_activate(
        &self,
        context: &GameAttributeContextMut,
        entity_mut: &EntityMut,
        name: String,
    ) {
        if let Some(ability) = self.abilities.get(&name) {
            ability.try_activate(&context, entity_mut);
        }
    }

    pub fn grant_ability(&mut self, name: String, ability: GameAbility) {
        self.abilities.insert(name, ability);
    }
}

#[derive(Default)]
pub struct GameAbilityBuilder {
    ability: GameAbility,
}

impl GameAbilityBuilder {
    pub fn with_effect(mut self, effect: GameEffect) -> Self {
        self.ability.applied_effects.push(effect);
        self
    }

    pub fn with_cost<T: Component + GameAttributeMarker>(mut self, cost: f32) -> Self {
        self.ability.cost = Some(Scalar(ScalarModifier::additive::<T>(cost)));
        self
    }

    pub fn with_cooldown(mut self, seconds: f32) -> Self {
        self.ability.cooldown = Timer::from_seconds(seconds, TimerMode::Once);
        self
    }

    pub fn with_activation(mut self, function: fn()) -> Self {
        self.ability.ability_activation = Some(function);
        self
    }

    pub fn build(self) -> GameAbility {
        self.ability
    }
}

#[derive(Default)]
pub struct GameAbility {
    pub applied_effects: Vec<GameEffect>,
    pub cost: Option<Modifier>,
    pub cooldown: Timer,
    pub ability_activation: Option<fn()>,
}

impl GameAbility {
    pub fn try_activate(&self, context: &GameAttributeContextMut, entity_mut: &EntityMut) {
        if self.can_activate(context, entity_mut) {
            self.commit_cost(context, entity_mut);

            if let Some(activation_function) = self.ability_activation {
                activation_function();
            }
        }
    }

    pub fn can_activate(&self, context: &GameAttributeContextMut, entity_mut: &EntityMut) -> bool {
        // Check cooldown first. If ability is still on cooldown, cannot activate yet.
        if !self.cooldown.finished() {
            // return false;
        }

        // If there's no cost, the ability is free and usable.
        let Some(modifier) = &self.cost else {
            return true;
        };

        let binding = entity_mut.as_readonly();
        let attr_opt = context.get_by_id(&binding, modifier.get_attribute_id());
        let Some(attr) = attr_opt else {
            return false;
        };

        let cost_mod = match modifier {
            Scalar(scalar) => scalar,
            Modifier::Meta(meta) => &context.convert_modifier(&binding, meta).unwrap(),
        };

        f32::abs(cost_mod.magnitude) <= attr.current_value
    }

    pub fn commit_cost(&self, context: &GameAttributeContextMut, entity_mut: &EntityMut) {
        if let Some(modifier) = &self.cost {
            apply_instant_modifier(context, entity_mut, &modifier);
        }
    }
}

pub trait AbilityTask {}
