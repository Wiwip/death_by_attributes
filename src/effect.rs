use crate::attributes::{GameAttribute, GameAttributeContext, GameAttributeMarker};
use crate::modifiers::{MetaModifier, Modifier, ModifierType, ScalarModifier};
use bevy::prelude::*;
use std::any::{TypeId};
use std::fmt::{Debug, Formatter};
use std::ops::{Mul};
use std::sync::{Mutex};
use std::time::Duration;

#[derive(Default, Component)]
pub struct GameEffectContainer {
    pub effects: Mutex<Vec<GameEffect>>,
}

impl GameEffectContainer {
    pub fn add_effect(&self, effect: &GameEffect) {
        self.effects.try_lock().unwrap().push(effect.clone());
    }

    pub fn remove_expired_effects(&mut self) {
        self.effects
            .try_lock()
            .unwrap()
            .retain(|effect| match &effect.duration {
                GameEffectDuration::Duration(duration) => !duration.finished(),
                _ => true,
            });
    }
}

pub fn apply_instant_effect(context: &GameAttributeContext, effect: &GameEffect) {
    for modifier in &effect.modifiers {
        apply_instant_modifier(context, modifier);
    }
}

pub fn apply_instant_modifier(context: &GameAttributeContext, modifier: &Modifier) {
    let target_attribute_option = context.get_attribute_mut(modifier.get_attribute_id());

    if let Some(target_attribute) = target_attribute_option {
        match modifier {
            Modifier::Scalar(scalar_mod) => apply_scalar_modifier(target_attribute, scalar_mod),
            Modifier::Meta(meta_mod) => {
                let scalar_mod_option = context.convert_modifier(meta_mod);
                if let Some(scalar_mod) = scalar_mod_option {
                    apply_scalar_modifier(target_attribute, &scalar_mod);
                }
            }
        }
    }
}

fn apply_scalar_modifier(attribute: &mut GameAttribute, scalar_mod: &ScalarModifier) {
    match scalar_mod.mod_type {
        ModifierType::Additive => {
            attribute.base_value += scalar_mod.magnitude;
            attribute.current_value = attribute.base_value;
        }
        ModifierType::Multiplicative => {
            attribute.base_value *= scalar_mod.magnitude;
            attribute.current_value = attribute.base_value;
        }
        ModifierType::Overrule => {
            attribute.base_value = scalar_mod.magnitude;
            attribute.current_value = attribute.base_value;
        }
    }
}

pub fn apply_realtime_effect(
    context: &GameAttributeContext,
    effect: &GameEffect,
    elapsed_time: f32,
) {
    for modifier in &effect.modifiers {
        apply_realtime_modifier(context, modifier, elapsed_time);
    }
}

pub fn apply_realtime_modifier(
    context: &GameAttributeContext,
    modifier: &Modifier,
    elapsed_time: f32,
) {
    let target_attribute_option = context.get_attribute_mut(modifier.get_attribute_id());

    if let Some(target_attribute) = target_attribute_option {
        match modifier {
            Modifier::Scalar(scalar_mod) => {
                apply_scalar_realtime_modifier(target_attribute, scalar_mod, elapsed_time)
            }
            Modifier::Meta(meta_mod) => {
                let scalar_mod_option = context.convert_modifier(meta_mod);
                if let Some(scalar_mod) = scalar_mod_option {
                    apply_scalar_realtime_modifier(target_attribute, &scalar_mod, elapsed_time);
                }
            }
        }
    }
}

fn apply_scalar_realtime_modifier(
    attribute: &mut GameAttribute,
    scalar_mod: &ScalarModifier,
    elapsed_time: f32,
) {
    match scalar_mod.mod_type {
        ModifierType::Additive => {
            attribute.base_value += scalar_mod.magnitude * elapsed_time;
            attribute.current_value = attribute.base_value;
        }
        ModifierType::Multiplicative => { /* A realtime multiplicative bonus doesn't make sense */ }
        ModifierType::Overrule => {
            attribute.base_value = scalar_mod.magnitude;
            attribute.current_value = attribute.base_value;
        }
    }
}

#[derive(Default)]
pub struct GameEffectBuilder {
    effect: GameEffect,
}

impl GameEffectBuilder {
    pub fn new() -> GameEffectBuilder {
        GameEffectBuilder::default()
    }

    pub fn with_scalar_modifier(mut self, modifier: ScalarModifier) -> Self {
        self.effect.modifiers.push(Modifier::Scalar(modifier));
        return self;
    }

    pub fn with_meta_modifier<
        T: Component + GameAttributeMarker,
        M: Component + GameAttributeMarker,
    >(
        mut self,
        style: ModifierType,
    ) -> Self {
        self.effect.modifiers.push(Modifier::Meta {
            0: MetaModifier {
                target_attribute: TypeId::of::<T>(),
                magnitude_attribute: TypeId::of::<M>(),
                mod_type: style,
            },
        });
        return self;
    }

    pub fn with_realtime_application(mut self) -> Self {
        self.effect.periodic_application = Some(GameEffectPeriod::Realtime);
        return self;
    }

    pub fn with_periodic_application(mut self, seconds: f32) -> Self {
        self.effect.periodic_application = Some(GameEffectPeriod::Periodic(Timer::from_seconds(
            seconds,
            TimerMode::Repeating,
        )));
        return self;
    }

    pub fn with_duration(mut self, seconds: f32) -> Self {
        let timer = Timer::from_seconds(seconds, TimerMode::Once);
        self.effect.duration = GameEffectDuration::Duration(timer);
        return self;
    }

    pub fn with_permanent_duration(mut self) -> Self {
        self.effect.duration = GameEffectDuration::Permanent;
        return self;
    }

    pub fn build(self) -> GameEffect {
        self.effect
    }
}

/// A [`GameEffect`] contains a collection of modifiers to be applied to an [`GameEffectContainer`]
///
/// By default, game effects are instant. Their duration can be modified by using an [`GameEffectDuration`]
///
#[derive(Default, Clone)]
pub struct GameEffect {
    pub modifiers: Vec<Modifier>,
    pub periodic_application: Option<GameEffectPeriod>,
    pub duration: GameEffectDuration,
}

impl GameEffect {
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
            "GE M:[{:?}] D:{:?} A:{:?}",
            self.modifiers, self.duration, self.periodic_application
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

#[derive(Default, Clone)]
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

#[derive(Default, Debug, Clone)]
pub enum GameEffectPeriod {
    #[default]
    Realtime,
    Periodic(Timer),
}
