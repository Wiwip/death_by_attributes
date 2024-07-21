use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::attributes::GameAttributeMarker;

use crate::effect::GameEffect;
use crate::modifiers::{Modifier, ScalarModifier};
use crate::modifiers::Modifier::Scalar;


///
///
///
#[derive(Component, Default)]
pub struct GameAbilityComponent {
    abilities: HashMap<String, GameAbility>,
}

impl GameAbilityComponent {
    pub fn try_activate(&self, name: String) {

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

    pub fn build(self) -> GameAbility {
        self.ability
    }
}

#[derive(Default)]
pub struct GameAbility {
    pub applied_effects: Vec<GameEffect>,
    pub cost: Option<Modifier>,
    pub cooldown: Timer,
}


impl GameAbility {
    pub fn try_activate(&self) {}

    pub fn can_activate(&self, ) -> bool {
        false
    }

    pub fn commit_cost(&self) {}
}

