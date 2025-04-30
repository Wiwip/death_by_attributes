use crate::modifiers::AttributeRef;
use crate::{AttributeEvaluationError, attribute_field, attributes};

use crate::attributes::{AttributeDef, EditableAttribute};
use crate::evaluators::{AttributeModEvaluator, BoxAttributeModEvaluator};
use crate::effects::GameEffectDuration::{Instant, Permanent};
use crate::modifiers::ModType::{Additive, Multiplicative};
use crate::modifiers::{AttributeMod, BoxEditableAttribute, ModType};
use bevy::prelude::*;
use bevy::reflect::Typed;
use std::any::TypeId;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;
use std::time::Duration;
use bevy::prelude::TimerMode::Once;
use bevy::time::TimerMode::Repeating;
use crate::effects::GameEffectPeriod::Periodic;

pub type Modifiers = Vec<BoxAttributeModEvaluator>;

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
        attribute_ref: impl EditableAttribute + Clone,
    ) -> Self {
        let modifier =
            BoxAttributeModEvaluator::new(AttributeMod::new(attribute_ref, value, Additive));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn with_multiplicative_modifier(
        mut self,
        value: f32,
        attribute_ref: impl EditableAttribute + Clone,
    ) -> Self {
        let modifier =
            BoxAttributeModEvaluator::new(AttributeMod::new(attribute_ref, value, Multiplicative));
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
            Some(t) => GameEffectDuration::Duration(Timer::from_seconds(t, Once))
        };

        GameEffectBuilder {
            effect: GameEffect {
                periodic_application: Some(Periodic(Timer::from_seconds(
                    seconds,
                    Repeating,
                ))),
                duration,
                ..default()
            },
        }
    }
    pub fn with_continuous_application(self) -> GameEffectBuilder {
        let duration = match self.duration {
            None => Permanent,
            Some(t) => GameEffectDuration::Duration(Timer::from_seconds(t, Once))
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

/*#[derive(Asset, Reflect, Clone, Default)]
#[reflect(Clone, Default)]
pub struct GameEffect {
    // This field is ignored by reflection because AnimationCurves can contain things that are not reflect-able
    #[reflect(ignore, clone)]
    pub modifiers: Modifiers,
    pub periodic_application: Option<GameEffectPeriod>,
    pub duration: GameEffectDuration,
}

*/

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

    pub fn add_modifier(&mut self, modifier: impl EditableAttribute) {
        // Update the duration of the animation by this curve duration if it's longer
        //self.modifiers.push(Modifier::new(curve));
    }

    pub fn builder() -> GameEffectBuilder {
        GameEffectBuilder::default()
    }

    pub fn tick_effect(&mut self, elapsed_time: Duration) {
        if let Some(period) = &mut self.periodic_application {
            match period {
                GameEffectPeriod::Realtime => { /* Nothing to do here! */ }
                GameEffectPeriod::Periodic(timer) => {
                    timer.tick(elapsed_time);
                }
            }
        }

        match &mut self.duration {
            GameEffectDuration::Instant => {
                error!("Instant effects shouldn't be ticked.")
            }
            GameEffectDuration::Duration(effect_timer) => {
                effect_timer.tick(elapsed_time);
            }
            GameEffectDuration::Permanent => { /* Nothing to do */ }
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
            GameEffectDuration::Instant => {
                write!(f, "-")
            }
            GameEffectDuration::Duration(timer) => {
                write!(f, "{:.1}", timer.remaining_secs())
            }
            GameEffectDuration::Permanent => {
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
