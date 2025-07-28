use crate::assets::EffectDef;
use crate::attributes::Attribute;
use crate::effects::{Effect, EffectInactive, EffectSource, EffectTarget};
use crate::evaluator::{AttributeExtractor, BoxExtractor};
use crate::stacks::Stacks;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

pub trait Condition: Send + Sync + 'static {
    fn evaluate(&self, context: &ConditionContext) -> bool;
}

pub struct BoxCondition(Box<dyn Condition>);

impl BoxCondition {
    pub fn new<C: Condition + 'static>(condition: C) -> Self {
        Self(Box::new(condition))
    }
}

pub struct ConditionContext<'a> {
    pub target_actor: &'a EntityRef<'a>,
    pub source_actor: &'a EntityRef<'a>,
    pub owner: &'a EntityRef<'a>,
}

pub enum Who {
    Target,
    Source,
    Owner,
}

impl Who {
    /// Resolves the `Who` variant to a specific entity from the context.
    pub fn get_entity<'a>(&self, context: &'a ConditionContext<'a>) -> &'a EntityRef<'a> {
        match self {
            Who::Target => context.target_actor,
            Who::Source => context.source_actor,
            Who::Owner => context.owner,
        }
    }
}

#[derive(TypePath)]
pub struct AttributeCondition {
    target: Who,
    extractor: BoxExtractor,
    bounds: (Bound<f64>, Bound<f64>),
}

impl AttributeCondition {
    pub fn new<'a, A: Attribute>(
        range: impl RangeBounds<f64> + Send + Sync + 'static,
        mod_target: Who,
    ) -> Self {
        Self {
            target: mod_target,
            extractor: BoxExtractor::new(AttributeExtractor::<A>::new()),
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
        }
    }

    pub fn target<A: Attribute>(range: impl RangeBounds<f64> + Send + Sync + 'static) -> Self {
        Self::new::<A>(range, Who::Target)
    }

    pub fn source<A: Attribute>(range: impl RangeBounds<f64> + Send + Sync + 'static) -> Self {
        Self::new::<A>(range, Who::Source)
    }
}

impl Condition for AttributeCondition {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        let entity = self.target.get_entity(context);

        match self.extractor.0.extract_value(entity) {
            Ok(value) => self.bounds.contains(&value),
            Err(e) => {
                error!("Error evaluating attribute condition: {}", e);
                false
            }
        }
    }
}

impl std::fmt::Display for AttributeCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Clone)]
pub struct StackCondition {
    pub bounds: (Bound<u32>, Bound<u32>),
}

impl StackCondition {
    pub fn new(range: impl RangeBounds<u32> + Send + Sync + 'static) -> Self {
        Self {
            bounds: (range.start_bound().cloned(), range.end_bound().cloned()),
        }
    }
}

impl std::fmt::Display for StackCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "StackCondition with bounds: {:?}", self.bounds)
    }
}

impl Condition for StackCondition {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        match context.owner.get::<Stacks>() {
            Some(value) => self.bounds.contains(&value.0),
            None => {
                error!(
                    "Effect {}: StackCondition requires a Stacks component.",
                    context.owner.id()
                );
                false
            }
        }
    }
}

pub struct And<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for And<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn evaluate(&self, value: &ConditionContext) -> bool {
        self.c1.evaluate(value) && self.c2.evaluate(value)
    }
}

pub struct Or<C1, C2> {
    c1: C1,
    c2: C2,
}

impl<C1, C2> Condition for Or<C1, C2>
where
    C1: Condition,
    C2: Condition,
{
    fn evaluate(&self, context: &ConditionContext) -> bool {
        self.c1.evaluate(context) || self.c2.evaluate(context)
    }
}

pub struct Not<C>(C);

impl<C: Condition> Condition for Not<C> {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        !self.0.evaluate(context)
    }
}

/// A condition that wraps a closure or function pointer.
///
/// This allows for creating custom, inline condition logic without needing
/// to define a new struct for every case.
pub struct FunctionCondition<F>
where
    F: Fn(&ConditionContext) -> bool + Send + Sync + 'static,
{
    f: F,
}

impl<F> FunctionCondition<F>
where
    F: Fn(&ConditionContext) -> bool + Send + Sync + 'static,
{
    /// Creates a new `FunctionCondition` from a closure.
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Condition for FunctionCondition<F>
where
    F: Fn(&ConditionContext) -> bool + Send + Sync + 'static,
{
    /// Evaluates the condition by calling the wrapped function.
    fn evaluate(&self, context: &ConditionContext) -> bool {
        (self.f)(context)
    }
}

pub struct TagCondition<C: Component> {
    target: Who,
    phantom_data: PhantomData<C>,
}

impl<C: Component> Condition for TagCondition<C> {
    fn evaluate(&self, context: &ConditionContext) -> bool {
        self.target.get_entity(context).contains::<C>()
    }
}

pub trait ConditionExt: Condition + Sized {
    fn and<C: Condition>(self, other: C) -> And<Self, C> {
        And {
            c1: self,
            c2: other,
        }
    }

    fn or<C: Condition>(self, other: C) -> Or<Self, C> {
        Or {
            c1: self,
            c2: other,
        }
    }

    fn not(self) -> Not<Self> {
        Not(self)
    }
}

impl<T: Condition> ConditionExt for T {}

pub(crate) fn evaluate_effect_conditions(
    mut query: Query<(
        EntityRef,
        &Effect,
        &EffectSource,
        &EffectTarget,
        Option<&EffectInactive>,
    )>,
    parents: Query<EntityRef>,
    effects: Res<Assets<EffectDef>>,
    mut commands: Commands,
) {
    for (effect_entity_ref, effect, source, target, status) in query.iter_mut() {
        let effect_entity = effect_entity_ref.id();
        let Ok(target_actor_ref) = parents.get(source.get()) else {
            error!(
                "Effect {} has no parent entity {}.",
                effect_entity_ref.id(),
                target.get()
            );
            continue;
        };
        let Ok(source_actor_ref) = parents.get(target.0) else {
            error!(
                "Effect {} has no target entity {}.",
                effect_entity_ref.id(),
                target.get()
            );
            continue;
        };

        let Some(effect) = effects.get(&effect.0) else {
            error!("Effect {} has no effect definition.", effect_entity_ref.id());
            continue;
        };

        let context = ConditionContext {
            target_actor: &target_actor_ref,
            source_actor: &source_actor_ref,
            owner: &effect_entity_ref,
        };

        // Determines whether the effect should activate
        let should_be_active = effect
            .conditions
            .iter()
            .all(|condition| condition.0.evaluate(&context));

        let is_inactive = status.is_some();
        if should_be_active && is_inactive {
            // Effect was inactive and its conditions are now met, so activate it.
            println!("Effect {effect_entity} is now active.");
            commands.entity(effect_entity).remove::<EffectInactive>();
        } else if !should_be_active && !is_inactive {
            // Effect was active and its conditions are no longer met, so deactivate it.
            println!("Effect {effect_entity} is now inactive.");
            commands.entity(effect_entity).insert(EffectInactive);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Resource)]
    struct EffectDatabase {
        effect_a: Handle<EffectDef>,
    }
}
