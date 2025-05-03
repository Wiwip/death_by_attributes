use crate::AttributeEntityMut;
use crate::attributes;
use crate::attributes::AttributeAccessorMut;
use crate::attributes::{AttributeAccessorRef, StoredAttribute};
use crate::effects::GameEffectDuration::{Instant, Permanent};
use crate::effects::GameEffectPeriod::Periodic;
use crate::evaluators::FixedEvaluator;
use crate::mutator::ModType::{Additive, Multiplicative};
use std::any::TypeId;
use std::collections::HashMap;

use crate::mutator::Mutator;
use crate::mutator::StoredMutator;
use crate::mutator::{ModAggregator, Modifiers};
use bevy::asset::uuid::Uuid;
use bevy::platform::hash::Hashed;
use bevy::prelude::TimerMode::Once;
use bevy::prelude::*;
use bevy::time::TimerMode::Repeating;
use bevy::utils::{PreHashMap, TypeIdMap};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub type EffectHandle = Uuid;

pub trait ApplicableEffect: Debug + Send + Sync + 'static {
    fn clone_value(&self) -> Box<dyn ApplicableEffect>;

    fn get_duration(&self) -> &GameEffectDuration;
    fn tick_effect(&mut self, elapsed_time: Duration);

    fn add_modifier(&mut self, modifier: StoredMutator);
    fn get_modifiers(&self) -> &Modifiers;

    fn get_id(&self) -> Uuid;
}

#[derive(Event)]
pub struct OnEffectAdded {
    pub effect: Effect,
}

#[derive(Event)]
pub struct OnEffectRemoved {
    pub effect: Effect,
}

#[derive(Component, Default, Clone)]
pub struct Effect {
    pub modifiers: Modifiers,
}
impl Effect {
    pub fn builder() -> GameEffectBuilder {
        GameEffectBuilder::default()
    }

    pub fn apply_effect(&self, entity: &mut AttributeEntityMut) {
        self.modifiers.iter().for_each(|modifier| {
            let _ = modifier.0.apply(entity);
        });
    }
}

#[derive(Default, Resource)]
pub struct EvalStruct {
    pub evaluators: HashMap<Entity, TypeIdMap<(StoredMutator, ModAggregator)>>,
    pub fast_evaluators: PreHashMap<Entity, TypeIdMap<(StoredMutator, ModAggregator)>>
}


#[derive(Component, Deref)]
//#[relationship(relationship_target = ActiveEffects)]
pub struct EffectTarget(pub Entity);

#[derive(Component, Deref)]
//#[relationship_target(relationship = EffectTarget)]
pub struct ActiveEffects(Vec<Entity>);


#[derive(Component, Deref, DerefMut)]
pub struct EffectDuration(pub Timer);

impl EffectDuration {
    pub fn new(seconds: f32) -> Self {
        Self (Timer::from_seconds(seconds, Once))
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct EffectPeriodicApplication(pub Timer);

impl EffectPeriodicApplication {
    pub fn new(seconds: f32) -> Self {
        Self (Timer::from_seconds(seconds, Repeating))
    }
}

/*
impl ApplicableEffect for Effect {
    fn clone_value(&self) -> Box<dyn ApplicableEffect> {
        Box::new(self.clone())
    }

    fn tick_effect(&mut self, elapsed_time: Duration) {
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

    fn get_modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    fn add_modifier(&mut self, modifier: StoredMutator) {
        self.modifiers.push(modifier);
    }

    fn get_id(&self) -> Uuid {
        self.id
    }
}
*/

/*
#[derive(Default, Component)]
pub struct GameEffectContainer {
    // The plain old list of effects currently applied to this entity
    pub effects: Vec<StoredEffect>,

    // Makes it easier to retrieve the modified attribute without reliance on the modifier themselves
    pub attribute_map: PreHashMap<(TypeId, usize), StoredAttribute>,

    // We don't want to recalculate the permanent modifiers every frame.
    pub cache: PreHashMap<(TypeId, usize), ModAggregator>,
}

impl GameEffectContainer {
    pub fn add_effect(&mut self, effect: impl ApplicableEffect) {
        for modifier in effect.get_modifiers() {
            let evaluator = modifier.get_stored_attribute().0.evaluator_id();
            let attribute = modifier.0.get_stored_attribute();
            self.attribute_map.insert(evaluator, attribute);
        }
        self.effects.push(StoredEffect::new(effect));
    }

    pub fn remove_expired_effects(&mut self) {
        /*self.effects.retain(|effect| match &effect.get_duration() {
            GameEffectDuration::Duration(duration) => !duration.finished(),
            _ => true,
        });*/
    }

    pub fn update_effect_duration(&mut self, elapsed_time: Duration) {

    }

    pub fn mark_attributes_dirty(&mut self) {}
}*/

/*impl fmt::Display for GameEffectContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Applied Modifiers: ")?;
        for effect in self.effects.iter() {
            write!(f, "\n   {:?}", effect)?;
        }
        Ok(())
    }
}*/

#[derive(Default)]
pub struct GameEffectBuilder {
    effect: Effect,
}

impl GameEffectBuilder {
    pub fn new() -> GameEffectDurationBuilder {
        GameEffectDurationBuilder::default()
    }

    pub fn with_additive_modifier(
        mut self,
        magnitude: f32,
        attribute_ref: impl AttributeAccessorMut + Clone + attributes::EvaluateAttribute,
    ) -> Self {
        let evaluator = FixedEvaluator::new(magnitude, Additive);
        let modifier = StoredMutator::new(Mutator::new(attribute_ref, evaluator));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn with_multiplicative_modifier(
        mut self,
        magnitude: f32,
        attribute_ref: impl AttributeAccessorMut + Clone + attributes::EvaluateAttribute,
    ) -> Self {
        let evaluator = FixedEvaluator::new(magnitude, Multiplicative);
        let modifier = StoredMutator::new(Mutator::new(attribute_ref, evaluator));
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

    pub fn build(self) -> Effect {
        self.effect
    }
}

#[derive(Default)]
pub struct GameEffectDurationBuilder;

impl GameEffectDurationBuilder {
    pub fn with_instant_application(self) -> GameEffectBuilder {
        GameEffectBuilder {
            /*effect: Effect {
                duration: Instant,
                ..default()
            },*/
            effect: Default::default(),
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
            effect: Effect {
                //periodic_application: Some(Periodic(Timer::from_seconds(seconds, Repeating))),
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
            effect: Effect {
                //periodic_application: None,

                ..default()
            },
        }
    }
}


/// A [`GameEffectEvent`] permits the application of ['GameEffect'] through the bevy event system.
///
#[derive(Event)]
pub struct GameEffectEvent {
    pub entity: Entity,
    pub effect: Effect,
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

#[derive(Debug, Deref, DerefMut, TypePath)]
pub struct StoredEffect {
    pub hash: Hashed<Uuid>,
    #[deref]
    pub effect: Box<dyn ApplicableEffect>,
}

impl Clone for StoredEffect {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            effect: ApplicableEffect::clone_value(&*self.effect),
        }
    }
}

impl StoredEffect {
    pub fn new(effect: impl ApplicableEffect) -> Self {
        Self {
            hash: Hashed::new(effect.get_id()),
            effect: Box::new(effect),
        }
    }
}
