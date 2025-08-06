use crate::AttributesRef;
use crate::assets::EffectDef;
use crate::condition::ConditionContext;
use crate::prelude::{Effect, EffectInactive, EffectSource, EffectTarget};
use bevy::asset::Assets;
use bevy::ecs::relationship::Relationship;
use bevy::log::error;
use bevy::prelude::{Commands, Query, Res};

pub fn evaluate_effect_conditions(
    mut query: Query<(
        AttributesRef,
        &Effect,
        &EffectSource,
        &EffectTarget,
        Option<&EffectInactive>,
    )>,
    parents: Query<AttributesRef>,
    effects: Res<Assets<EffectDef>>,
    mut commands: Commands,
) {
    for (effect_entity_ref, effect, source, target, status) in query.iter_mut() {
        let effect_entity = effect_entity_ref.id();
        let Ok(source_actor_ref) = parents.get(source.get()) else {
            error!(
                "Effect {} has no parent entity {}.",
                effect_entity_ref.id(),
                target.get()
            );
            continue;
        };
        let Ok(target_actor_ref) = parents.get(target.0) else {
            error!(
                "Effect {} has no target entity {}.",
                effect_entity_ref.id(),
                target.get()
            );
            continue;
        };

        let Some(effect) = effects.get(&effect.0) else {
            error!(
                "Effect {} has no effect definition.",
                effect_entity_ref.id()
            );
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
