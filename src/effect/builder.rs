use crate::assets::EffectDef;
use crate::attributes::Attribute;
use crate::condition::{AttributeCondition, BoxCondition};
use crate::effect::application::EffectApplicationPolicy;
use crate::effect::{EffectExecution, EffectStackingPolicy};
use crate::modifier::{ModifierFn, Modifier, Who};
use crate::prelude::{AttributeCalculatorCached, AttributeModifier, DerivedModifier, Mod};
use bevy::ecs::component::Mutable;
use bevy::prelude::{Bundle, Component, Entity, EntityCommands, Name};
use std::ops::RangeBounds;
use fixed::prelude::{LossyFrom, LossyInto};

pub struct EffectBuilder {
    effect_entity_commands: Vec<Box<ModifierFn>>,
    effects: Vec<Box<dyn Modifier>>,
    custom_execution: Option<Box<dyn EffectExecution>>,
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
            custom_execution: None,
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

    pub fn modify<T: Attribute>(
        mut self,
        modifier: Mod<T::Property>,
        who: Who,
    ) -> Self {
        self.modifiers
            .push(Box::new(AttributeModifier::<T>::new(modifier, who, 1.0)));
        self
    }

    /// Spawns an observer watching the actor's attributes on the modifier entity.
    /// When OnValueChanged is triggered, it takes the current value of the attribute,
    /// it applies the scaling factor and updates the modifier's value to the new value.
    pub fn modify_from<S, T>(
        mut self,
        modifier: Mod<T::Property>,
        mod_target: Who,
    ) -> Self
    where
        S: Attribute,
        T: Attribute,
        T::Property: LossyFrom<S::Property>,
    {
        self.modifiers.push(Box::new(DerivedModifier::<S, T>::new(
            modifier,
            mod_target,
            modifier.value().lossy_into(),
        )));
        self
    }

    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = Some(intensity);
        self
    }

    pub fn while_condition(mut self, condition: impl crate::condition::Condition + 'static) -> Self {
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

    pub fn with_execution_context(mut self, context: impl EffectExecution + 'static) -> Self {
        self.custom_execution = Some(Box::new(context));
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

    pub fn with_custom_execution(mut self, execution: impl EffectExecution + 'static) -> Self {
        self.custom_execution = Some(Box::new(execution));
        self
    }

    pub fn build(self) -> EffectDef {
        EffectDef {
            effect_fn: self.effect_entity_commands,
            effect_modifiers: self.effects,
            custom_execution: self.custom_execution,
            modifiers: self.modifiers,
            application: self.application,
            conditions: self.conditions,
            stacking_policy: self.stacking_policy,
            intensity: self.intensity,
        }
    }
}
