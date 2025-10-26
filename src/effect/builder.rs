use crate::assets::EffectDef;
use crate::attributes::{Attribute, Value};
use crate::condition::{AttributeCondition, BoxCondition};
use crate::effect::application::EffectApplicationPolicy;
use crate::effect::{EffectExecution, EffectStackingPolicy, IntoEffectExecution};
use crate::modifier::{Modifier, ModifierFn, Who};
use crate::prelude::{AttributeModifier, ModOp, StoredExecution};
use bevy::prelude::{Bundle, Entity, EntityCommands, Name};
use std::ops::RangeBounds;

pub struct EffectBuilder {
    effect_entity_commands: Vec<Box<ModifierFn>>,
    effects: Vec<Box<dyn Modifier>>,
    executions: Vec<StoredExecution>,
    modifiers: Vec<Box<dyn Modifier>>,
    application: EffectApplicationPolicy,
    conditions: Vec<BoxCondition>,
    stacking_policy: EffectStackingPolicy,
    intensity: Option<f32>,
}

impl EffectBuilder {
    pub fn new(application: EffectApplicationPolicy) -> Self {
        Self {
            effect_entity_commands: vec![],
            effects: vec![],
            executions: vec![],
            modifiers: vec![],
            application,
            conditions: vec![],
            stacking_policy: EffectStackingPolicy::None,
            intensity: None,
        }
    }

    pub fn instant() -> Self {
        Self::new(EffectApplicationPolicy::Instant)
    }

    pub fn permanent() -> Self {
        Self::new(EffectApplicationPolicy::Permanent)
    }

    pub fn for_seconds(duration: f32) -> Self {
        Self::new(EffectApplicationPolicy::for_seconds(duration))
    }

    pub fn every_seconds(interval: f32) -> Self {
        Self::new(EffectApplicationPolicy::every_seconds(interval))
    }

    pub fn every_seconds_for_duration(interval: f32, duration: f32) -> Self {
        Self::new(EffectApplicationPolicy::every_seconds_for_duration(
            interval, duration,
        ))
    }

    /// Modifies an [Attribute]
    pub fn modify<T: Attribute>(
        mut self,
        value: Value<T::Property>,
        modifier: ModOp,
        who: Who,
        scaling: f64,
    ) -> Self {
        self.modifiers.push(Box::new(AttributeModifier::<T>::new(
            value, modifier, who, scaling,
        )));
        self
    }

    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = Some(intensity);
        self
    }

    pub fn while_condition(
        mut self,
        condition: impl crate::condition::Condition + 'static,
    ) -> Self {
        self.conditions.push(BoxCondition::new(condition));
        self
    }

    pub fn when_source_attribute<T: Attribute>(
        mut self,
        range: impl RangeBounds<T::Property> + Send + Sync + 'static,
    ) -> Self {
        let predicate = AttributeCondition::<T>::new(range, Who::Source);
        self.conditions.push(BoxCondition::new(predicate));
        self
    }

    pub fn when_target_attribute<T: Attribute>(
        mut self,
        range: impl RangeBounds<T::Property> + Send + Sync + 'static,
    ) -> Self {
        let predicate = AttributeCondition::<T>::new(range, Who::Target);
        self.conditions.push(BoxCondition::new(predicate));
        self
    }

    pub fn add_execution<I, S: for<'a> EffectExecution + 'static>(
        mut self,
        system: impl for<'a> IntoEffectExecution<'a, I, ExecFunction = S>,
    ) -> Self {
        self.executions.push(Box::new(system.into_condition()));
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(Name::new(name.clone()));
            },
        ));
        self
    }

    pub fn with_bundle(mut self, bundle: impl Bundle + Copy) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(bundle);
            },
        ));
        self
    }

    pub fn with_stacking_policy(mut self, policy: EffectStackingPolicy) -> Self {
        self.stacking_policy = policy;
        self
    }

    pub fn build(self) -> EffectDef {
        EffectDef {
            effect_fn: self.effect_entity_commands,
            effect_modifiers: self.effects,
            execution: self.executions,
            modifiers: self.modifiers,
            application: self.application,
            conditions: self.conditions,
            stacking_policy: self.stacking_policy,
            intensity: self.intensity,
        }
    }
}
