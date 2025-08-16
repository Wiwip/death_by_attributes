use crate::{AttributesMut};
use crate::assets::EffectDef;
use crate::effect::stacks::NotifyAddStackEvent;
use crate::effect::timing::{EffectDuration, EffectTicker};
use crate::effect::{
    AppliedEffects, Effect, CalculationContext, CaptureContext, EffectStackingPolicy,
    EffectTargeting,
};
use crate::graph::NodeType;
use crate::modifier::{Mutator, Who};
use crate::prelude::{EffectInactive, EffectSource, EffectTarget, EffectIntensity, Attribute, ApplyAttributeModifierEvent};
use bevy::asset::{Assets, Handle};
use bevy::log::debug;
use bevy::prelude::*;
use std::cmp::PartialEq;

/// Describes how the effect is applied to entities
#[derive(Debug, Clone, Reflect, PartialEq)]
pub enum EffectApplicationPolicy {
    /// Applied once immediately
    Instant,

    /// Applied once and persists forever
    Permanent,

    /// Applied once, persists for a duration, then removed
    Temporary { duration: Timer },

    /// Applied repeatedly at intervals, forever
    Periodic { interval: Timer },

    /// Applied repeatedly at intervals for a limited time
    PeriodicTemporary { interval: Timer, duration: Timer },
}

impl EffectApplicationPolicy {
    // Constructor methods
    pub fn instant() -> Self {
        Self::Instant
    }

    pub fn permanent() -> Self {
        Self::Permanent
    }

    pub fn for_seconds(duration: f32) -> Self {
        Self::Temporary {
            duration: Timer::from_seconds(duration, TimerMode::Once),
        }
    }

    pub fn every_seconds(interval: f32) -> Self {
        Self::Periodic {
            interval: Timer::from_seconds(interval, TimerMode::Repeating),
        }
    }

    pub fn every_seconds_for_duration(interval: f32, duration: f32) -> Self {
        Self::PeriodicTemporary {
            interval: Timer::from_seconds(interval, TimerMode::Repeating),
            duration: Timer::from_seconds(duration, TimerMode::Once),
        }
    }

    // State checking methods
    pub fn is_expired(&self) -> bool {
        match self {
            Self::Instant => true,
            Self::Permanent | Self::Periodic { .. } => false,
            Self::Temporary { duration } => duration.finished(),
            Self::PeriodicTemporary { duration, .. } => duration.finished(),
        }
    }

    pub fn is_periodic(&self) -> bool {
        match self {
            Self::Instant | Self::Permanent | Self::Temporary { .. } => false,
            Self::Periodic { .. } | Self::PeriodicTemporary { .. } => true,
        }
    }

    pub fn should_apply_now(&self) -> bool {
        match self {
            Self::Instant => true,                             // Apply once on creation
            Self::Permanent | Self::Temporary { .. } => false, // Applied through aggregator systems
            Self::Periodic { interval } | Self::PeriodicTemporary { interval, .. } => {
                interval.just_finished()
            }
        }
    }

    /*pub fn tick(&mut self, delta: std::time::Duration) {
        match self {
            Self::Temporary { duration } => {
                duration.tick(delta);
            }
            Self::Periodic { interval } => {
                interval.tick(delta);
            }
            Self::PeriodicTemporary { interval, duration } => {
                interval.tick(delta);
                duration.tick(delta);
            }
            _ => {} // Instant and Permanent don't need ticking
        };
    }*/

    pub fn reset(&mut self) {
        match self {
            Self::Temporary { duration } | Self::PeriodicTemporary { duration, .. } => {
                duration.reset();
            }
            _ => {}
        }
    }

    pub fn to_bundles(&self) -> (Option<impl Bundle>, Option<impl Bundle>) {
        let duration = match self {
            EffectApplicationPolicy::Temporary { duration } => Some(EffectDuration::new(duration)),
            EffectApplicationPolicy::PeriodicTemporary { duration, .. } => {
                Some(EffectDuration::new(duration))
            }
            _ => None,
        };

        let period = match self {
            EffectApplicationPolicy::Periodic { interval } => Some(EffectTicker::new(interval)),
            EffectApplicationPolicy::PeriodicTemporary { interval, .. } => {
                Some(EffectTicker::new(interval))
            }
            _ => None,
        };

        (duration, period)
    }
}

#[derive(Event)]
pub struct ApplyEffectEvent {
    pub targeting: EffectTargeting,
    pub handle: Handle<EffectDef>,
}

impl ApplyEffectEvent {
    fn apply_instant_effect(
        &self,
        mut actors: &mut Query<(Option<&AppliedEffects>, AttributesMut), Without<Effect>>,
        mut apply_modifier_writer: EventWriter<ApplyAttributeModifierEvent>,
        effect: &EffectDef,
    ) -> Result<(), BevyError> {
        debug!("Applying instant effect to {}", self.targeting.target());

        if let Some(execution) = &effect.custom_execution {
            // Captures variables and forwards the captured values to the capture context
            let mut capture_context = CaptureContext::from(&self.targeting, &mut actors);
            execution.capture_attributes(&mut capture_context)?;
            // Then execute the effect using the captured attributes
            let mut execution_context = CalculationContext::new(capture_context);
            execution.execute_effect(&mut execution_context)?;

            // Apply the collected modifiers
            let modifiers = execution_context.into_modifiers();
            self.apply_modifiers(&mut actors, &mut modifiers.iter(), &mut apply_modifier_writer);
        };

        self.apply_modifiers(&mut actors, &mut effect.modifiers.iter(), &mut apply_modifier_writer);

        Ok(())
    }

    fn apply_modifiers<'a, I>(
        &self,
        actors: &mut Query<(Option<&AppliedEffects>, AttributesMut), Without<Effect>>,
        modifiers: &mut I,
        apply_modifier_writer: &mut EventWriter<ApplyAttributeModifierEvent>
    ) where
        I: Iterator<Item = &'a Box<dyn Mutator>>,
    {
        for modifier in modifiers {
            match modifier.who() {
                Who::Target => {
                    let (_, target) = actors.get(self.targeting.target()).unwrap();
                    apply_modifier_writer.write(ApplyAttributeModifierEvent {
                        target: target.id(),
                        modifier: modifier.modifier(),
                        attribute: modifier.as_accessor(),
                    });
                }
                Who::Source => {
                    let (_, source) = actors.get(self.targeting.source()).unwrap();
                    apply_modifier_writer.write(ApplyAttributeModifierEvent {
                        target: source.id(),
                        modifier: modifier.modifier(),
                        attribute: modifier.as_accessor(),
                    });
                }
                Who::Effect => {
                    todo!()
                }
            }
        }
    }

    fn spawn_persistent_effect(
        &self,
        mut commands: &mut Commands,
        effect: &EffectDef,
        actors: &mut Query<(Option<&AppliedEffects>, AttributesMut), Without<Effect>>,
        effects: &mut Query<&Effect>,
        add_stack_event: &mut EventWriter<NotifyAddStackEvent>,
    ) -> Result<(), BevyError> {
        debug!("Applying duration effect to {}", self.targeting.target());

        // We want to know whether an effect with the same handle already exists on the actor
        let (optional_effects, _) = actors.get_mut(self.targeting.target())?;
        let effects_on_actor = match optional_effects {
            None => {
                vec![]
            }
            Some(effects_on_actor) => {
                let effects = effects_on_actor.iter().filter_map(|effect_entity| {
                    let other_effect = effects.get(effect_entity).unwrap();
                    if other_effect.0.id() == self.handle.id() {
                        Some(effect_entity)
                    } else {
                        None
                    }
                });
                effects.collect::<Vec<_>>()
            }
        };

        match effect.stacking_policy {
            EffectStackingPolicy::None => {
                // Continue spawning effect
                debug!("Stacking policy is None");
            }
            EffectStackingPolicy::Add { .. } | EffectStackingPolicy::RefreshDuration => {
                debug!("Stacking policy is Add or Override");
                if effects_on_actor.len() > 0 {
                    debug!("Effect already exists on actor. Adding stacks per definition.");
                    add_stack_event.write(NotifyAddStackEvent {
                        effect_entity: *effects_on_actor.first().unwrap(),
                        handle: self.handle.clone(),
                    });
                    return Ok(());
                } else {
                    debug!("Effect does not exist on actor. Creating new effect instance.");
                }
            }
        }

        let mut effect_commands = commands.spawn_empty();
        let effect_entity = effect_commands.id();
        for effect_fn in &effect.effect_fn {
            effect_fn(&mut effect_commands, self.targeting.target());
        }

        // Spawns the effect entity
        effect_commands.insert((
            NodeType::Effect,
            EffectTarget(self.targeting.target()),
            EffectSource(self.targeting.source()),
            Effect(self.handle.clone()),
        ));

        // Converts the policy to components that can be added to the entity
        let (duration, ticker) = effect.application.to_bundles();
        if let Some(duration) = duration {
            effect_commands.insert(duration);
        }
        if let Some(ticker) = ticker {
            effect_commands.insert(ticker);
        }
        if let Some(intensity) = effect.intensity {
            effect_commands.insert(EffectIntensity::new(intensity));
        }

        // Prepare entity commands
        for effect_mod in &effect.effect_modifiers {
            let (_, target) = actors.get_mut(self.targeting.target())?;
            effect_mod.spawn(&mut commands, target.as_readonly());
        }

        // Spawn effect modifiers
        effect
            .modifiers
            .iter()
            .for_each(|modifier| match modifier.who() {
                Who::Target => {
                    let (_, target) = actors.get_mut(self.targeting.target()).unwrap();
                    let mod_entity = modifier.spawn(commands, target.as_readonly());
                    commands
                        .entity(mod_entity)
                        .insert(EffectTarget(effect_entity));
                }
                Who::Source => {
                    let (_, source) = actors.get_mut(self.targeting.source()).unwrap();
                    let mod_entity = modifier.spawn(commands, source.as_readonly());
                    commands
                        .entity(mod_entity)
                        .insert(EffectTarget(effect_entity));
                }
                Who::Effect => todo!(),
            });

        Ok(())
    }
}

pub(crate) fn apply_effect_event_observer(
    trigger: Trigger<ApplyEffectEvent>,
    mut actors: Query<(Option<&AppliedEffects>, AttributesMut), Without<Effect>>,
    mut effects: Query<&Effect>,
    effect_assets: Res<Assets<EffectDef>>,
    apply_modifier_writer: EventWriter<ApplyAttributeModifierEvent>,
    mut event_writer: EventWriter<NotifyAddStackEvent>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let effect = effect_assets
        .get(&trigger.handle)
        .ok_or("No effect asset.")?;

    if effect.application.should_apply_now() {
        trigger.apply_instant_effect(&mut actors, apply_modifier_writer, effect)?;
    }

    if effect.application != EffectApplicationPolicy::Instant {
        trigger.spawn_persistent_effect(
            &mut commands,
            effect,
            &mut actors,
            &mut effects,
            &mut event_writer,
        )?;
    }

    Ok(())
}
