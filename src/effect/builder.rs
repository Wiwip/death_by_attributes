use crate::assets::EffectDef;
use crate::attributes::Attribute;
use crate::condition::{AttributeCondition, BoxCondition};
use crate::effect::application::EffectApplicationPolicy;
use crate::effect::{EffectExecution, EffectStackingPolicy};
use crate::modifier::{ModifierFn, Mutator, Who};
use crate::prelude::{AttributeCalculatorCached, AttributeModifier, DerivedModifier, Mod};
use bevy::ecs::component::Mutable;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::{Bundle, Component, Entity, EntityCommands, Event, Name};
use std::ops::RangeBounds;

pub struct EffectBuilder {
    effect_entity_commands: Vec<Box<ModifierFn>>,
    effects: Vec<Box<dyn Mutator>>,
    custom_execution: Option<Box<dyn EffectExecution>>,
    modifiers: Vec<Box<dyn Mutator>>,
    application: EffectApplicationPolicy,
    conditions: Vec<BoxCondition>,
    stacking_policy: EffectStackingPolicy,
}

impl EffectBuilder {
    fn new(timing: EffectApplicationPolicy) -> Self {
        Self {
            effect_entity_commands: vec![],
            effects: vec![],
            custom_execution: None,
            modifiers: vec![],
            application: timing,
            conditions: vec![],
            stacking_policy: EffectStackingPolicy::None,
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

    pub fn modify<T: Attribute + Component<Mutability = Mutable>>(
        mut self,
        modifier: Mod,
        who: Who,
    ) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |entity_mut: &mut EntityCommands, _: Entity| {
                entity_mut.insert(AttributeCalculatorCached::<T>::default());
            },
        ));

        self.modifiers
            .push(Box::new(AttributeModifier::<T>::new(modifier, who)));
        self
    }

    /// Spawns an observer watching the actor's attributes on the modifier entity.
    /// When OnValueChanged is triggered, it takes the current value of the attribute,
    /// it applies the scaling factor and updates the modifier's value to the new value.
    pub fn modify_by_ref<T, S>(
        mut self,
        scaling_factor: f64,
        modifier: Mod,
        mod_target: Who,
    ) -> Self
    where
        T: Attribute,
        S: Attribute,
    {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(AttributeCalculatorCached::<T>::default());
            },
        ));

        self.modifiers.push(Box::new(DerivedModifier::<T, S>::new(
            modifier,
            scaling_factor,
            mod_target,
        )));
        self
    }

    /*pub fn with_trigger<E: Event, B: Bundle, M>(
        mut self,
        _observer: impl IntoObserverSystem<E, B, M>,
    ) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |_effect_entity: &mut EntityCommands, _: Entity| {
                //effect_entity.insert(Condition::<T>::default());
            },
        ));
        //self
        todo!()
    }*/

    pub fn when_condition(mut self, condition: impl crate::condition::Condition + 'static) -> Self {
        self.conditions.push(BoxCondition::new(condition));
        self
    }

    pub fn when_source_attribute<T: Attribute + Component<Mutability = Mutable>>(
        mut self,
        range: impl RangeBounds<f64> + Send + Sync + 'static,
    ) -> Self {
        let predicate = AttributeCondition::new::<T>(range, Who::Source);
        self.conditions.push(BoxCondition::new(predicate));
        self
    }

    pub fn when_target_attribute<T: Attribute + Component<Mutability = Mutable>>(
        mut self,
        range: impl RangeBounds<f64> + Send + Sync + 'static,
    ) -> Self {
        let predicate = AttributeCondition::new::<T>(range, Who::Target);
        self.conditions.push(BoxCondition::new(predicate));
        self
    }

    /*pub fn when_predicate(mut self, condition: impl crate::conditions::Condition + 'static) -> Self {
        //self.conditions.push(ErasedCondition::new(condition));
        self
    }

    pub fn with_tag_requirement<T: AttributeComponent + Component<Mutability = Mutable>>(
        mut self,
        condition_check: fn(f64) -> bool,
    ) -> Self {
        self.effects.push(Box::new(Condition::<T> {
            _target: Default::default(),
            condition_fn: condition_check,
        }));
        self
    }

    pub fn with_predicate<T: AttributeComponent + Component<Mutability = Mutable>>(
        mut self,
        condition_check: fn(f64) -> bool,
    ) -> Self {


        self.effects.push(Box::new(Condition::<T> {
            _target: Default::default(),
            condition_fn: condition_check,
        }));
        self
    }*/

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
        }
    }
}
