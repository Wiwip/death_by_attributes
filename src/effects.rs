use crate::attributes::{AttributeAccessorRef, AttributeMut, GameAttribute};
use crate::{AttributeEvaluationError, attribute_mut, attributes};

use crate::attributes::{AttributeAccessorMut, AttributeDef};
use crate::effects::GameEffectDuration::{Instant, Permanent};
use crate::effects::GameEffectPeriod::Periodic;

use crate::evaluators::FixedEvaluator;
use crate::mutator::ModType::{Additive, Multiplicative};
use crate::mutator::{MutatorWrapper, EvaluateMutator, ModType, Mutator};
use bevy::prelude::TimerMode::Once;
use bevy::prelude::*;
use bevy::time::TimerMode::Repeating;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::time::Duration;

pub type Modifiers = Vec<MutatorWrapper>;

#[derive(Default, Reflect, Clone)]
pub struct GameEffect {
    #[reflect(ignore, clone)]
    pub modifiers: Modifiers,
    pub periodic_application: Option<GameEffectPeriod>,
    pub duration: GameEffectDuration,
}

#[derive(Default, Component)]
pub struct GameEffectContainer {
    pub effects: Vec<GameEffect>,
}

impl GameEffectContainer {
    pub fn add_effect(&mut self, effect: &GameEffect) {
        self.effects.push(effect.clone());
    }

    pub fn remove_expired_effects(&mut self) {
        self.effects.retain(|effect| match &effect.duration {
            GameEffectDuration::Duration(duration) => !duration.finished(),
            _ => true,
        });
    }
}

impl fmt::Display for GameEffectContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Applied Modifiers: ")?;
        for effect in self.effects.iter() {
            write!(f, "\n   {:?}", effect)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct GameEffectBuilder {
    effect: GameEffect,
}

impl GameEffectBuilder {
    pub fn new() -> GameEffectDurationBuilder {
        GameEffectDurationBuilder::default()
    }

    pub fn with_additive_modifier(
        mut self,
        magnitude: f32,
        attribute_ref: impl AttributeAccessorMut + Clone,
    ) -> Self {
        let evaluator = FixedEvaluator::new(magnitude,Additive);
        let modifier = MutatorWrapper::new(Mutator::new(attribute_ref, evaluator));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn with_multiplicative_modifier(
        mut self,
        magnitude: f32,
        attribute_ref: impl AttributeAccessorMut + Clone,
    ) -> Self {
        let evaluator = FixedEvaluator::new(magnitude,Multiplicative);
        let modifier = MutatorWrapper::new(Mutator::new(attribute_ref, evaluator));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn with_meta_modifier(
        mut self,
        value: f32,
        dest: impl AttributeAccessorMut + Clone,
        source: impl AttributeAccessorRef + Clone,
    ) -> Self {
        //let evaluator = MetaModEvaluator::new(value);
        //let modifier = AttributeModVariable::new(MetaMod::new(dest, source, evaluator));
        //self.effect.modifiers.push(modifier);
        self
    }

    pub fn build(self) -> GameEffect {
        self.effect
    }
}

#[derive(Default)]
pub struct GameEffectDurationBuilder;

impl GameEffectDurationBuilder {
    pub fn with_instant_application(self) -> GameEffectBuilder {
        GameEffectBuilder {
            effect: GameEffect {
                duration: Instant,
                ..default()
            },
        }
    }
    pub fn with_duration(self, seconds: f32) -> GameEffectPeriodBuilder {
        GameEffectPeriodBuilder {
            duration: Some(seconds),
        }
    }
    pub fn with_permanent_duration(self) -> GameEffectPeriodBuilder {
        GameEffectPeriodBuilder::default()
    }
}

#[derive(Default)]
pub struct GameEffectPeriodBuilder {
    duration: Option<f32>,
}

impl GameEffectPeriodBuilder {
    pub fn with_periodic_application(self, seconds: f32) -> GameEffectBuilder {
        let duration = match self.duration {
            None => Permanent,
            Some(t) => GameEffectDuration::Duration(Timer::from_seconds(t, Once)),
        };

        GameEffectBuilder {
            effect: GameEffect {
                periodic_application: Some(Periodic(Timer::from_seconds(seconds, Repeating))),
                duration,
                ..default()
            },
        }
    }
    pub fn with_continuous_application(self) -> GameEffectBuilder {
        let duration = match self.duration {
            None => Permanent,
            Some(t) => GameEffectDuration::Duration(Timer::from_seconds(t, Once)),
        };

        GameEffectBuilder {
            effect: GameEffect {
                periodic_application: None,
                duration,
                ..default()
            },
        }
    }
}

impl GameEffect {
    pub fn builder() -> GameEffectBuilder {
        GameEffectBuilder::default()
    }

    #[inline]
    pub fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    #[inline]
    pub fn modifiers_mut(&mut self) -> &mut Modifiers {
        &mut self.modifiers
    }

    pub fn add_modifier(&mut self, modifier: impl EvaluateMutator) {
        self.modifiers.push(MutatorWrapper::new(modifier));
    }

    pub fn tick_effect(&mut self, elapsed_time: Duration) {
        if let Some(period) = &mut self.periodic_application {
            match period {
                GameEffectPeriod::Realtime => { /* Nothing to do here! */ }
                Periodic(timer) => {
                    timer.tick(elapsed_time);
                }
            }
        }

        match &mut self.duration {
            Instant => {
                error!("Instant effects shouldn't be ticked.")
            }
            GameEffectDuration::Duration(effect_timer) => {
                effect_timer.tick(elapsed_time);
            }
            Permanent => { /* Nothing to do */ }
        }
    }
}

impl Debug for GameEffect {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GE D:{:?} A:{:?}",
            self.duration, self.periodic_application
        )
    }
}

impl fmt::Display for GameEffect {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GE D:{:?} A:{:?}",
            self.duration, self.periodic_application
        )
    }
}

/// A [`GameEffectEvent`] permits the application of ['GameEffect'] through the bevy event system.
///
#[derive(Event)]
pub struct GameEffectEvent {
    pub entity: Entity,
    pub effect: GameEffect,
}

#[derive(Default, Clone, Reflect)]
pub enum GameEffectDuration {
    #[default]
    Instant,
    Duration(Timer),
    Permanent,
}

impl Debug for GameEffectDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Instant => {
                write!(f, "-")
            }
            GameEffectDuration::Duration(timer) => {
                write!(f, "{:.1}", timer.remaining_secs())
            }
            Permanent => {
                write!(f, "Inf")
            }
        }
    }
}

#[derive(Default, Debug, Clone, Reflect)]
pub enum GameEffectPeriod {
    #[default]
    Realtime,
    Periodic(Timer),
}
