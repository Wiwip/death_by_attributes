use crate::assets::EffectDef;
use crate::attributes::{Attribute, IntoValue};
use crate::condition::{AttributeCondition, BoxCondition};
use crate::effect::EffectStackingPolicy;
use crate::effect::application::EffectApplicationPolicy;
use crate::modifier::{Modifier, ModifierFn, Who};
use crate::mutator::EntityActions;
use crate::prelude::{AttributeModifier, ModOp};
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::{Bundle, Entity, EntityCommands, EntityEvent, Name};
use std::ops::RangeBounds;

pub struct EffectBuilder {
    effect_entity_commands: Vec<Box<ModifierFn>>,
    triggers: Vec<EntityActions>,
    effects: Vec<Box<dyn Modifier>>,
    modifiers: Vec<Box<dyn Modifier>>,
    application: EffectApplicationPolicy,
    application_conditions: Vec<BoxCondition>,
    conditions: Vec<BoxCondition>,
    stacking_policy: EffectStackingPolicy,
    intensity: Option<f32>,
}

impl EffectBuilder {
    pub fn new(application: EffectApplicationPolicy) -> Self {
        Self {
            effect_entity_commands: vec![],
            triggers: vec![],
            effects: vec![],
            modifiers: vec![],
            application,
            application_conditions: vec![],
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

    /// Modifies an attribute.
    ///
    /// A [Value](crate::attributes::Value) represents the magnitude of the change to the attribute.
    /// It can be a literal [Lit](crate::attributes::Lit), an [AttributeValue](crate::attributes::AttributeValue), or anything implementing [ValueSource](crate::attributes::ValueSource)
    ///
    /// # Example
    /// ```
    /// use root_attribute::prelude::*;
    /// attribute!(Health, u32);
    /// attribute!(Damage, u32);
    ///
    /// // A simple effect that increases the source's health by 100.
    /// let effect = EffectBuilder::new(EffectApplicationPolicy::Instant)
    ///     .modify::<Health>(100u32, ModOp::Add, Who::Source, 1.0)
    ///     .build();
    ///
    /// let damage = EffectBuilder::instant()
    ///     .modify::<Health>(Damage::value(), ModOp::Sub, Who::Target, 1.0)
    ///     .build();
    ///
    /// // Regen 2 health every 1.0 seconds for 12.0 seconds.
    /// let regen = EffectBuilder::every_seconds_for_duration(1.0, 12.0)
    ///     .modify::<Health>(2u32, ModOp::Add, Who::Source, 1.0)
    ///     .build();
    /// ```
    /// This function adds a new `AttributeModifier` to the `modifiers` collection and returns the updated object.
    pub fn modify<T: Attribute>(
        mut self,
        value: impl IntoValue<Out = T::Property> + 'static,
        modifier: ModOp,
        who: Who,
        scaling: f64,
    ) -> Self {
        self.modifiers.push(Box::new(AttributeModifier::<T>::new(
            value.into_value(),
            modifier,
            who,
            scaling,
        )));
        self
    }

    pub fn if_condition(mut self, condition: impl crate::condition::Condition + 'static) -> Self {
        self.application_conditions
            .push(BoxCondition::new(condition));
        self
    }

    pub fn add_trigger<E: EntityEvent, B: Bundle, M>(
        mut self,
        observer: impl IntoObserverSystem<E, B, M> + Clone + Send + Sync + 'static,
    ) -> Self {
        self.triggers.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.observe(observer.clone());
            },
        ));
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

    pub fn name(mut self, name: String) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(Name::new(name.clone()));
            },
        ));
        self
    }

    pub fn insert(mut self, bundle: impl Bundle + Copy) -> Self {
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
            triggers: self.triggers,
            effect_modifiers: self.effects,
            modifiers: self.modifiers,
            application: self.application,
            application_conditions: self.application_conditions,
            conditions: self.conditions,
            stacking_policy: self.stacking_policy,
            intensity: self.intensity,
        }
    }
}
