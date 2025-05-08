use crate::ModifierOf;
use crate::attributes::AttributeComponent;
use crate::{OnValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::Name;
use bevy::prelude::TimerMode::{Once, Repeating};
use bevy::prelude::*;
use std::fmt::{Debug, Formatter};
use crate::modifiers::{EffectOf, ModAggregator, ModType, Modifier, ModifierCommand};

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
    period: Option<EffectPeriodicTimer>,
}

impl EffectBuilder {
    pub fn new(actor_entity: Entity, effect_entity: Entity) -> GameEffectDurationBuilder {
        debug!("Spawned effect entity {}", effect_entity);
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
        let command = ModifierCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            modifier: Modifier::<T>::new(magnitude, mod_type),
            observer: None,
        };
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.effect_entity)
                .insert(ModAggregator::<T>::default());
        });
        self.queue.push(command);
        self
    }

    /// Spawns an observer watching the actor's attributes on the modifier entity. 
    /// When OnValueChanged is triggered, it takes the current value of the attribute, 
    /// it applies the scaling factor and updates the modifier's value to the new value.  
    pub fn modify_by_ref<T, S>(mut self, scaling_factor: f32) -> Self
    where
        T: AttributeComponent + Component<Mutability = Mutable>,
        S: AttributeComponent + Component<Mutability = Mutable>,
    {
        let mut observer = Observer::new(
            move |trigger: Trigger<OnValueChanged>,
                  mut modifiers: Query<&mut Modifier<T>>,
                  attributes: Query<&S>| {
                let Ok(attribute) = attributes.get(trigger.target()) else {
                    return;
                };
                let Ok(mut modifier) = modifiers.get_mut(trigger.observer()) else {
                    return;
                };
                modifier.value.additive = scaling_factor * attribute.current_value();
            },
        );
        observer.watch_entity(self.actor_entity);
        self.queue.push(ModifierCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            modifier: Modifier::<T>::default(),
            observer: Some(observer),
        });
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.effect_entity).insert(Name::new(name));
        });
        self
    }

    pub fn with_component(mut self, component: impl Bundle) -> Self {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.actor_entity).insert(component);
        });
        self
    }

    pub fn with_custom_calculations(mut self, command: impl Command) -> Self {
        
        self
    }
    
    pub fn commit(mut self, commands: &mut Commands) -> Entity {
        commands.queue(EffectCommand {
            effect_entity: self.effect_entity,
            actor_entity: self.actor_entity,
            duration: self.duration,
            period: self.period,
            effect: self.effect,
        });
        commands.append(&mut self.queue);
        self.effect_entity
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
    effect_entity: Entity,
    actor_entity: Entity,
    duration: Option<EffectDuration>,
    period: Option<EffectPeriodicTimer>,
    effect: Effect,
}

impl Command for EffectCommand {
    fn apply(self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.effect_entity);
        let mut entity_mut = world.entity_mut(self.effect_entity);
        entity_mut.insert((ModifierOf(self.actor_entity), self.effect));
        if let Some(duration) = self.duration {
            entity_mut.insert(duration);
        }
        if !entity_mut.contains::<Name>() {
            entity_mut.insert(Name::new("Effect"));
        }
        // Do not attach periodic event to the "Effect" hierarchy as it will passively affect the tree
        // Add it to the Modifier hierarchy so it can modify the attributes of the targeted entity
        match self.period {
            None => entity_mut.insert(EffectOf(self.actor_entity)),
            Some(period) => entity_mut.insert(period),
        };
    }
}

pub struct GameEffectPeriodBuilder {
    effect_builder: EffectBuilder,
}

impl GameEffectPeriodBuilder {
    pub fn with_periodic_application(mut self, seconds: f32) -> EffectBuilder {
        self.effect_builder.period =
            Some(EffectPeriodicTimer(Timer::from_seconds(seconds, Repeating)));
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
