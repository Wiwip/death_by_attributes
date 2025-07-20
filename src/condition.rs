use crate::assets::GameEffect;
use crate::attributes::Attribute;
use crate::effects::{Effect, EffectInactive, EffectOf};
use bevy::ecs::relationship::Relationship;
use bevy::ecs::world::EntityRefExcept;
use bevy::prelude::*;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::RangeBounds;

pub type EffectEntityRef<'w> = EntityRefExcept<'w, (EffectOf,)>;

#[derive(Component, Default)]
pub struct Conditions(pub Vec<ErasedCondition>);

pub trait Condition: Debug + Send + Sync + 'static {
    fn check(&self, entity: EffectEntityRef) -> Result<bool, BevyError>;
}

#[derive(Debug, TypePath)]
pub struct ErasedCondition(pub Box<dyn Condition>);

impl ErasedCondition {
    pub fn new(condition: impl Condition) -> Self {
        Self(Box::new(condition))
    }
}

#[derive(Clone)]
pub struct AttributeCondition<A, R> {
    phantom_data: PhantomData<A>,
    condition: R,
}

impl<A, R> AttributeCondition<A, R>
where
    A: Component + Attribute,
    R: RangeBounds<f64> + Send + Sync + 'static,
{
    pub fn new(condition: R) -> Self {
        Self {
            phantom_data: PhantomData,
            condition,
        }
    }
}

impl<A, R> Debug for AttributeCondition<A, R>
where
    A: Component + Attribute,
    R: RangeBounds<f64> + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeCondition")
            .field("condition", &self.condition)
            .finish()
    }
}

impl<A, R> Condition for AttributeCondition<A, R>
where
    A: Component + Attribute,
    R: RangeBounds<f64> + Send + Sync + Debug + 'static,
{
    fn check(&self, entity_ref: EffectEntityRef) -> Result<bool, BevyError> {
        let attribute = entity_ref.get::<A>().ok_or("Should have an attribute")?;
        let eval_result = self.condition.contains(&attribute.current_value());
        Ok(eval_result)
    }
}

pub(crate) fn evaluate_effect_conditions(
    mut query: Query<(Entity, &Effect, &EffectOf, Option<&EffectInactive>)>,
    parents: Query<EffectEntityRef>,
    effects: Res<Assets<GameEffect>>,
    mut commands: Commands,
) {
    for (effect_entity, effect, parent, status) in query.iter_mut() {
        let Ok(entity_ref) = parents.get(parent.get()) else {
            error!(
                "Effect {effect_entity} has no parent entity {}.",
                parent.get()
            );
            continue;
        };

        let effect = effects.get(&effect.0).unwrap();

        // If it returns, the effect is active
        let should_activate = effect.conditions.iter().all(|condition| {
            condition
                .0
                .check(entity_ref)
                .unwrap_or_else(|err| {
                    error!("Error checking condition: {err}");
                    false
                })
        });

        // Disable the effect if any of the conditions returns false
        match status {
            Some(_) => {
                // Effect is already inactive.
                if should_activate {
                    commands
                        .entity(effect_entity)
                        .try_remove::<EffectInactive>();
                }
            }
            None => {
                // Effect is active.
                if !should_activate {
                    commands.entity(effect_entity).try_insert(EffectInactive);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use std::ops::Range;
    use crate::actors::ActorBuilder;
    use crate::{attribute, AttributesPlugin};

    attribute!(TestAttribute);
    
}






















