use crate::Dirty;
use crate::attributes::AttributeComponent;
use crate::modifiers::{ModAggregator, ModType, Modifier, ModifierCommand};
use crate::{ActorEntityMut, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::Name;
use bevy::prelude::TimerMode::{Once, Repeating};
use bevy::prelude::*;
use std::any::type_name;
use std::fmt::{Debug, Formatter};

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Effects)]
pub struct EffectOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EffectOf, linked_spawn)]
pub struct Effects(Vec<Entity>);

#[derive(Component, Debug, Default)]
pub struct Effect;

#[derive(Component, Debug)]
pub struct EffectSource(pub Entity);

#[derive(Component, Debug)]
pub struct EffectTarget(pub Entity);

pub trait EffectCondition: Default {
    fn can_apply(&self, actor: &ActorEntityMut) -> bool;
}

#[derive(Component, Reflect, Deref, DerefMut, Clone)]
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
        magnitude: f64,
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
    pub fn modify_by_ref<T, S>(mut self, scaling_factor: f64) -> Self
    where
        T: AttributeComponent + Component<Mutability = Mutable>,
        S: AttributeComponent + Component<Mutability = Mutable>,
    {
        let mut observer = Observer::new(
            // When the source attribute changes, update the modifier of the target attribute.
            move |trigger: Trigger<OnAttributeValueChanged<S>>,
                  attributes: Query<&S>,
                  mut modifiers: Query<&mut Modifier<T>>| {
                debug!(
                    "Observer <{}->{}> triggered: {} -> {}",
                    type_name::<S>(),
                    type_name::<T>(),
                    trigger.target(),
                    trigger.observer()
                );
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

        let duration = self.duration.clone();
        let period = self.period.clone();

        self.queue.push(move |world: &mut World| {
            let entity = world.spawn_empty().id();
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.insert((
                observer,
                Name::new(format!("{}", type_name::<T>())),
                Modifier::<T>::default(),
                ModAggregator::<T>::default(),
                Dirty::<T>::default(),
            ));
            match duration {
                // The effect is instant. It must modify parents.
                None => {
                    entity_mut.insert(EffectOf(self.effect_entity));
                }
                Some(_) => match period {
                    // The effect has a duration but doesn't tick. Permanent buff.
                    None => {
                        entity_mut.insert(EffectOf(self.effect_entity));
                    }
                    // The effect is a periodic event with a duration but no permanent stats buff.
                    Some(_) => {
                        entity_mut.insert(EffectOf(self.effect_entity));
                    }
                },
            };

            world
                .entity_mut(self.effect_entity)
                .insert(ModAggregator::<T>::default());
        });
        
        self.queue.push(move |world: &mut World| {
            world.trigger_targets(OnAttributeValueChanged::<S>::default(), self.actor_entity);
        });
        self
    }

    pub fn with_trigger<E: Event, B: Bundle, M>(
        mut self,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> Self {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.effect_entity).observe(observer);
        });
        self
    }

    pub fn with_conditions<C: Component + EffectCondition>(mut self) -> Self {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.effect_entity).insert(C::default());
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
        entity_mut.insert((EffectOf(self.actor_entity), self.effect));
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

#[derive(Component, Clone)]
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
