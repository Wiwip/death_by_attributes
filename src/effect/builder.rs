use crate::assets::EffectDef;
use crate::attributes::Attribute;
use crate::condition::{AttributeCondition, BoxCondition};
use crate::effect::EffectStackingPolicy;
use crate::effect::application::EffectApplicationPolicy;
use crate::expression::{Expr, ExprNode};
use crate::modifier::{ModOp, Who};
use crate::mutator::EntityActions;
use crate::prelude::*;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::{Bundle, Entity, EntityCommands, EntityEvent, Name};
use std::ops::RangeBounds;

pub struct EffectBuilder {
    def: EffectDef,
}

impl EffectBuilder {
    pub fn new(application: EffectApplicationPolicy) -> Self {
        Self {
            def: EffectDef {
                application_policy: application,
                stacking_policy: EffectStackingPolicy::None,
                effect_fn: vec![],
                modifiers: vec![],
                activate_conditions: vec![],
                attach_conditions: vec![],
                on_actor_triggers: vec![],
                on_effect_triggers: vec![],
            },
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

    pub fn every_second_permanently(interval: f32) -> Self {
        Self::new(EffectApplicationPolicy::every_seconds(interval))
    }

    pub fn every_second_for_duration(interval: f32, duration: f32) -> Self {
        Self::new(EffectApplicationPolicy::every_seconds_for_duration(
            interval, duration,
        ))
    }

    /// Modifies an attribute.
    ///
    /// A [Value](crate::attributes::Value) represents the value of the change to the attribute.
    /// It can be a literal [Lit](crate::attributes::Lit), an [AttributeValue](crate::attributes::AttributeValue), or anything implementing [ValueSource](crate::attributes::ValueSource)
    ///
    /// # Example
    /// ```
    /// # use vitality::prelude::*;
    /// attribute!(Health, u32);
    /// attribute!(Damage, u32);
    ///
    /// // A simple effect that increases the source's health by 100.
    /// let effect = EffectBuilder::new(EffectApplicationPolicy::Instant)
    ///     .modify::<Health>(100u32, ModOp::Add, Who::Source)
    ///     .build();
    ///
    /// let damage = EffectBuilder::instant()
    ///     .modify::<Health>(Damage::value(), ModOp::Sub, Who::Target)
    ///     .build();
    ///
    /// // Regen 2 health every 1.0 seconds for 12.0 seconds.
    /// let regen = EffectBuilder::every_second_for_duration(1.0, 12.0)
    ///     .modify::<Health>(2u32, ModOp::Add, Who::Source)
    ///     .build();
    /// ```
    pub fn modify<T: Attribute>(
        mut self,
        expr: impl Into<Expr<T::ExprType>>,
        modifier: ModOp,
        who: Who,
    ) -> Self {
        self.def
            .modifiers
            .push(Box::new(AttributeModifier::<T>::new(
                expr.into(),
                modifier,
                who,
            )));
        self
    }

    /// Attach the effect to the target entity only if the condition is met.
    ///
    /// # Examples
    ///
    /// ```
    /// # use vitality::prelude::*;
    /// attribute!(Health, f32);
    /// attribute!(Damage, f32);
    ///
    /// // A damage over time effect that has a 10% chance of applying every tick.
    /// EffectBuilder::every_second_for_duration(1.0, 12.0)
    ///     .modify::<Health>(Damage::value(), ModOp::Add, Who::Target)
    ///     .attach_if(ChanceCondition(0.10))
    ///     .build()
    /// ```
    pub fn attach_if(mut self, condition: impl Condition + 'static) -> Self {
        self.def
            .attach_conditions
            .push(BoxCondition::new(condition));
        self
    }

    pub fn activate_while(mut self, condition: impl Condition + 'static) -> Self {
        self.def
            .activate_conditions
            .push(BoxCondition::new(condition));
        self
    }

    pub fn add_effect_trigger<E: EntityEvent, B: Bundle, M>(
        mut self,
        observer: impl IntoObserverSystem<E, B, M> + Clone + Send + Sync + 'static,
    ) -> Self {
        self.def.on_effect_triggers.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.observe(observer.clone());
            },
        ));
        self
    }

    pub fn add_actor_trigger<E: EntityEvent, B: Bundle, M>(
        mut self,
        observer: impl IntoObserverSystem<E, B, M> + Clone + Send + Sync + 'static,
    ) -> Self {
        self.def.on_actor_triggers.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.observe(observer.clone());
            },
        ));
        self
    }

    pub fn when_source_attribute<T: Attribute>(
        mut self,
        range: impl RangeBounds<T::Property> + Send + Sync + 'static,
    ) -> Self {
        let predicate = AttributeCondition::<T>::new(range, Who::Source);
        self.def
            .activate_conditions
            .push(BoxCondition::new(predicate));
        self
    }

    pub fn when_target_attribute<T: Attribute>(
        mut self,
        range: impl RangeBounds<T::Property> + Send + Sync + 'static,
    ) -> Self {
        let predicate = AttributeCondition::<T>::new(range, Who::Target);
        self.def
            .activate_conditions
            .push(BoxCondition::new(predicate));
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.def.effect_fn.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(Name::new(name.clone()));
            },
        ));
        self
    }

    pub fn insert(mut self, bundle: impl Bundle + Copy) -> Self {
        self.def.effect_fn.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(bundle);
            },
        ));
        self
    }

    pub fn with_stacking_policy(mut self, policy: EffectStackingPolicy) -> Self {
        self.def.stacking_policy = policy;
        self
    }

    pub fn build(self) -> EffectDef {
        self.def
    }
}
