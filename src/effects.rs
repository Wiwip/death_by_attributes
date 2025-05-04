use crate::AttributeEntityMut;
use crate::attributes::AttributeComponent;
use crate::evaluators::FixedEvaluator;
use crate::mutator::ModType::{Additive, Multiplicative};
use crate::mutator::Mutator;
use crate::mutator::StoredMutator;
use crate::mutator::{ModAggregator, Mutators};
use bevy::ecs::component::Mutable;
use bevy::prelude::TimerMode::Once;
use bevy::prelude::*;
use bevy::time::TimerMode::Repeating;
use bevy::utils::TypeIdMap;
use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

#[derive(Component, Debug, Default)]
pub struct Effect {
    pub modifiers: Mutators,
}

impl Effect {
    pub fn apply_effect(&self, mut entity: AttributeEntityMut) {
        for modifier in self.modifiers.iter() {
            let _ = modifier.0.apply_mutator(entity.reborrow());
        }
    }
}

#[derive(Default, Resource)]
pub struct MutationAggregatorCache {
    pub evaluators: HashMap<Entity, TypeIdMap<(StoredMutator, ModAggregator, bool, bool)>>,
}

impl MutationAggregatorCache {
    pub fn is_base_value_dirty(&self, entity: Entity, type_id: TypeId) -> Result<bool, ()> {
        let Some(type_map) = self.evaluators.get(&entity) else {
            return Err(());
        };

        let Some((_, _, base_value_dirty, _)) = type_map.get(&type_id) else {
            return Err(());
        };

        Ok(*base_value_dirty)
    }

    pub fn is_current_value_dirty(&self, entity: Entity, type_id: TypeId) -> Result<bool, ()> {
        let Some(type_map) = self.evaluators.get(&entity) else {
            return Err(());
        };

        let Some((_, _, _, current_value_dirty)) = type_map.get(&type_id) else {
            return Err(());
        };

        Ok(*current_value_dirty)
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct EffectPeriodicTimer(pub Timer);

impl EffectPeriodicTimer {
    pub fn new(seconds: f32) -> Self {
        Self(Timer::from_seconds(seconds, Repeating))
    }
}

pub struct EffectBuilder {
    target: Entity,
    effect: Effect,
    duration: Option<EffectDuration>,
    period: Option<Timer>,
}

impl EffectBuilder {
    pub fn new(target: Entity) -> GameEffectDurationBuilder {
        GameEffectDurationBuilder {
            effect_builder: EffectBuilder {
                target,
                effect: Default::default(),
                duration: None,
                period: None,
            },
        }
    }

    pub fn with_additive_modifier<C: Component<Mutability = Mutable> + AttributeComponent>(
        mut self,
        magnitude: f32,
    ) -> Self {
        let evaluator = FixedEvaluator::new(magnitude, Additive);
        let modifier = StoredMutator::new(Mutator::new::<C>(evaluator));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn with_multiplicative_modifier<C: Component<Mutability = Mutable> + AttributeComponent>(
        mut self,
        magnitude: f32,
    ) -> Self {
        let evaluator = FixedEvaluator::new(magnitude, Multiplicative);
        let modifier = StoredMutator::new(Mutator::new::<C>(evaluator));
        self.effect.modifiers.push(modifier);
        self
    }

    pub fn build(self, commands: &mut Commands) {
        commands.queue(EffectCommand { builder: self });
    }

    pub fn build_deferred(self) -> EffectCommand {
        EffectCommand { builder: self }
    }
}

pub struct GameEffectDurationBuilder {
    effect_builder: EffectBuilder,
}

impl GameEffectDurationBuilder {
    pub fn with_instant_application(self) -> EffectBuilder {
        self.effect_builder
    }
    pub fn with_duration(mut self, seconds: f32) -> GameEffectPeriodBuilder {
        self.effect_builder.duration =
            Some(EffectDuration::Duration(Timer::from_seconds(seconds, Once)));
        GameEffectPeriodBuilder {
            effect_builder: self.effect_builder,
        }
    }
    pub fn with_permanent_duration(mut self) -> GameEffectPeriodBuilder {
        self.effect_builder.duration = Some(EffectDuration::Permanent);
        GameEffectPeriodBuilder {
            effect_builder: self.effect_builder,
        }
    }
}

pub struct EffectCommand {
    builder: EffectBuilder,
}

impl EffectCommand {
    pub fn get_effect(&self) -> &Effect {
        &self.builder.effect
    }
}

impl Command for EffectCommand {
    fn apply(self, world: &mut World) -> () {
        let mut entity_command = world.spawn_empty();

        if let Some(duration) = self.builder.duration {
            entity_command.insert(duration);
        }

        if let Some(period) = self.builder.period {
            entity_command.insert(EffectPeriodicTimer(period));
        }

        // This must be here. A bug in the ordering of commands makes the app crash
        entity_command.insert((ChildOf(self.builder.target), self.builder.effect));
    }
}

#[derive(Default, Debug, Clone, Reflect)]
pub struct GameEffectPeriod(pub Timer);

pub struct GameEffectPeriodBuilder {
    effect_builder: EffectBuilder,
}

impl GameEffectPeriodBuilder {
    pub fn with_periodic_application(mut self, seconds: f32) -> EffectBuilder {
        self.effect_builder.period = Some(Timer::from_seconds(seconds, Repeating));
        self.effect_builder
    }
    pub fn with_continuous_application(self) -> EffectBuilder {
        self.effect_builder
    }
}

#[derive(Component)]
pub enum EffectDuration {
    Permanent,
    Duration(Timer),
}

impl Debug for EffectDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectDuration::Duration(timer) => {
                write!(f, "{:.1}", timer.remaining_secs())
            }
            EffectDuration::Permanent => {
                write!(f, "Inf")
            }
        }
    }
}
