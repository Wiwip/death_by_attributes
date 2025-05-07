use crate::attributes::AttributeComponent;
use bevy::prelude::Name;

use crate::Dirty;
use crate::modifiers::scalar::{ModType, Modifier, MutatorCommand};
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

pub struct EffectBuilder<'a> {
    commands: &'a mut Commands<'a, 'a>,
    pub(crate) actor_entity: Entity,
    pub(crate) effect_entity: Entity,
    pub(crate) effect: Effect,
    queue: CommandQueue,
    duration: Option<EffectDuration>,
    period: Option<EffectPeriodicTimer>,
}

impl<'a> EffectBuilder<'a> {
    pub fn new(
        actor_entity: Entity,
        commands: &'a mut Commands<'a, 'a>,
    ) -> GameEffectDurationBuilder<'a> {
        let effect_entity = commands.spawn_empty().id();
        debug!("Spawned effect entity {}", effect_entity);
        GameEffectDurationBuilder {
            effect_builder: EffectBuilder {
                commands,
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
        let command = MutatorCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            modifier: Modifier::<T>::new(magnitude),
        };

        self.queue.push(command);
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

    pub fn with_name(self, name: String) -> Self {
        self.commands
            .entity(self.effect_entity)
            .insert(Name::new(name));
        self
    }

    pub fn commit(mut self) {
        self.commands.queue(EffectCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            duration: self.duration,
            period: self.period,
            effect: self.effect,
        });
        self.commands.append(&mut self.queue);
    }
}

pub struct GameEffectDurationBuilder<'a> {
    effect_builder: EffectBuilder<'a>,
}

impl<'a> GameEffectDurationBuilder<'a> {
    pub fn with_instant_application(self) -> EffectBuilder<'a> {
        self.effect_builder
    }
    pub fn with_duration(mut self, seconds: f32) -> GameEffectPeriodBuilder<'a> {
        self.effect_builder.duration =
            Some(EffectDuration::Duration(Timer::from_seconds(seconds, Once)));
        GameEffectPeriodBuilder {
            effect_builder: self.effect_builder,
        }
    }
    pub fn with_permanent_duration(mut self) -> GameEffectPeriodBuilder<'a> {
        self.effect_builder.duration = Some(EffectDuration::Permanent);
        GameEffectPeriodBuilder {
            effect_builder: self.effect_builder,
        }
    }
}

pub(crate) struct EffectCommand {
    effect_entity: Entity,
    actor_entity: Entity,
    duration: Option<EffectDuration>,
    period: Option<EffectPeriodicTimer>,
    effect: Effect,
}

impl Command for EffectCommand {
    fn apply(self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.effect_entity);
        {
            let mut entity_mut = world.entity_mut(self.effect_entity);
            entity_mut.insert((
                Name::new("Effect"),
                ModifierOf(self.actor_entity),
                self.effect,
            ));

            if let Some(duration) = self.duration {
                entity_mut.insert(duration);
            }

            // Do not attach periodic event to the "Effect" hierarchy as it will passively affect the tree
            // Add it to the Modifier hierarchy so it can modify the attributes of the targeted actor
            match self.period {
                None => entity_mut.insert(EffectOf(self.actor_entity)),
                Some(period) => entity_mut.insert(period),
            };
        }
    }
}

pub struct GameEffectPeriodBuilder<'a> {
    effect_builder: EffectBuilder<'a>,
}

impl<'a> GameEffectPeriodBuilder<'a> {
    pub fn with_periodic_application(mut self, seconds: f32) -> EffectBuilder<'a> {
        self.effect_builder.period =
            Some(EffectPeriodicTimer(Timer::from_seconds(seconds, Repeating)));
        self.effect_builder
    }
    pub fn with_continuous_application(self) -> EffectBuilder<'a> {
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
