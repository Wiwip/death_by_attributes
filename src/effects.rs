use crate::attributes::AttributeMut;
use crate::{AttributeEvaluationError, attribute_mut, attributes};

use crate::attributes::{AttributeDef, AttributeAccessorMut};
use crate::effects::GameEffectDuration::{Instant, Permanent};
use crate::effects::GameEffectPeriod::Periodic;

use crate::modifiers::ModType::{Additive, Multiplicative};
use crate::modifiers::{AttributeModifier, AttributeModVariable, ModType, ModEvaluator};
use bevy::prelude::TimerMode::Once;
use bevy::prelude::*;
use bevy::reflect::Typed;
use bevy::time::TimerMode::Repeating;
use std::any::TypeId;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;
use std::time::Duration;

pub type Modifiers = Vec<AttributeModVariable>;

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
        value: f32,
        attribute_ref: impl AttributeAccessorMut + Clone,
    ) -> Self {
        let evaluator = ModEvaluator::new(value, Additive);
        let modifier = AttributeModVariable::new(AttributeModifier::new(attribute_ref, evaluator));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn with_multiplicative_modifier(
        mut self,
        value: f32,
        attribute_ref: impl AttributeAccessorMut + Clone,
    ) -> Self {
        let evaluator = ModEvaluator::new(value, Multiplicative);
        let modifier = AttributeModVariable::new(AttributeModifier::new(attribute_ref, evaluator));
        self.effect.modifiers.push(modifier);
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
    #[inline]
    /// [`BoxedModifier`]s for each animation target. Indexed by the [`AnimationTargetId`].
    pub fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    #[inline]
    /// Get mutable references of [`BoxedModifier`]s for each animation target. Indexed by the [`AnimationTargetId`].
    pub fn modifiers_mut(&mut self) -> &mut Modifiers {
        &mut self.modifiers
    }

    pub fn add_modifier(&mut self, modifier: impl AttributeAccessorMut) {
        // Update the duration of the animation by this curve duration if it's longer
        //self.modifiers.push(GameAttribute::new(modifier));
    }

    pub fn builder() -> GameEffectBuilder {
        GameEffectBuilder::default()
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
