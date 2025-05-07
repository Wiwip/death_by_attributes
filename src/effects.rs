use crate::attributes::AttributeComponent;

use crate::modifiers::mutator::{ModType, Modifier, MutatorCommand};
use crate::modifiers::{EffectOf, ModifierOf};
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::TimerMode::{Once, Repeating};
use bevy::prelude::*;
use std::fmt::{Debug, Formatter};

#[derive(Component, Debug, Default)]
pub struct Effect;

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct EffectPeriodicTimer(pub Timer);

impl EffectPeriodicTimer {
    pub fn new(seconds: f32) -> Self {
        Self(Timer::from_seconds(seconds, Repeating))
    }
}

pub struct EffectBuilder {
    pub(crate) actor_entity: Entity,
    pub(crate) effect_entity: Entity,
    pub(crate) effect: Effect,
    queue: CommandQueue,
    duration: Option<EffectDuration>,
    period: Option<Timer>,
}

impl EffectBuilder {
    pub fn new(actor_entity: Entity, effect_entity: Entity) -> GameEffectDurationBuilder {
        assert_ne!(actor_entity, effect_entity);
        info!("Created effect entity {}", effect_entity);
        GameEffectDurationBuilder {
            effect_builder: EffectBuilder {
                actor_entity,
                effect_entity,
                effect: Default::default(),
                queue: Default::default(),
                duration: None,
                period: None,
            },
        }
    }

    pub fn modify_by_scalar<T: AttributeComponent + Component<Mutability = Mutable>>(
        mut self,
        magnitude: f32,
        mod_type: ModType,
    ) -> Self {
        self.queue.push(MutatorCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            modifier: Modifier::<T>::new(magnitude),
        });
        self
    }

    pub fn modify_by_ref<S, D>(mut self, magnitude: f32, mod_type: ModType) -> Self
    where
        S: AttributeComponent + Component<Mutability = Mutable>,
        D: AttributeComponent + Component<Mutability = Mutable>,
    {
        todo!();
        /*self.queue.push(MutatorCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            mutator: MutatorHelper::new::<S>(MetaEvaluator::<D>::new(magnitude, mod_type)),
        });*/
        self
    }

    pub fn apply(self, commands: &mut Commands) {
        commands.queue(EffectCommand { builder: self });
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

pub(crate) struct EffectCommand {
    pub(crate) builder: EffectBuilder,
}

impl Command for EffectCommand {
    fn apply(mut self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.builder.effect_entity);
        {
            let mut entity_mut = world.entity_mut(self.builder.effect_entity);
            entity_mut.insert((
                Name::new("Effect"),
                ModifierOf(self.builder.actor_entity),
                self.builder.effect,
            ));

            if let Some(duration) = self.builder.duration {
                entity_mut.insert(duration);
            }
            if let Some(period) = self.builder.period {
                entity_mut.insert((
                    ModifierOf(self.builder.actor_entity),
                    EffectPeriodicTimer(period),
                ));
            } else {
                // Ensures that it can affect the hierarchy
                entity_mut.insert(EffectOf(self.builder.actor_entity));
            }
        }

        // Spawn mutators after the effects
        world.commands().append(&mut self.builder.queue);
        world.flush();
    }
}

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
