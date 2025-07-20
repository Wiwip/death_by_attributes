use crate::attributes::{
    AttributeClamp, Attribute, attribute_clamp_system, update_max_clamp_values,
};
use crate::modifiers::ModAggregator;
use crate::systems::{flag_dirty_attribute, flag_dirty_modifier, tick_effects_periodic_timer, apply_periodic_effect, update_effect_tree_system, apply_modifier_on_trigger};
use crate::{
    Actor, ApplyModifier, OnAttributeValueChanged, OnBaseValueChange, RegisteredPhantomSystem,
};
use bevy::app::{PostUpdate, PreUpdate};
use bevy::ecs::component::Mutable;
use bevy::ecs::event::EventRegistry;
use bevy::ecs::world::CommandQueue;
use bevy::log::debug;
use bevy::prelude::{
    Bundle, Command, Commands, Component, Entity, Events, IntoScheduleConfigs, Query, Trigger,
    World,
};
use std::any::type_name;
use std::marker::PhantomData;

pub struct ActorBuilder {
    entity: Entity,
    queue: CommandQueue,
}

impl ActorBuilder {
    pub fn new(actor: Entity) -> ActorBuilder {
        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.entity_mut(actor).insert(Actor);
        });
        Self {
            entity: actor,
            queue,
        }
    }

    pub fn with<T>(mut self, value: f64) -> ActorBuilder
    where
        T: Component<Mutability = Mutable> + Attribute,
    {
        // Ensures that the systems related to this attribute exist in the schedule
        self.queue.push(AttributeInitCommand::<T> {
            phantom: Default::default(),
        });
        // Inserts the actual T attribute on the entity
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.entity)
                .insert((T::new(value), ModAggregator::<T>::default()));

            // TODO: Should probably be a global observer
            world
                .entity_mut(self.entity)
                .observe(apply_modifier_on_trigger::<T>);
        });
        self
    }

    pub fn clamp<T>(mut self, clamp: AttributeClamp<T>) -> ActorBuilder
    where
        T: Component<Mutability = Mutable> + Attribute,
    {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.entity).insert(clamp);
        });
        self
    }

    pub fn clamp_max<T, C>(mut self, min: f64) -> ActorBuilder
    where
        T: Component<Mutability = Mutable> + Attribute,
        C: Component<Mutability = Mutable> + Attribute,
    {
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.entity)
                .insert(AttributeClamp::<T>::MinMax(min, f64::MAX))
                .observe(update_max_clamp_values::<T, C>);
            world.trigger_targets(OnAttributeValueChanged::<T>::default(), self.entity);
        });
        self
    }

    pub fn with_command(mut self, command: impl Command) -> ActorBuilder {
        self.queue.push(command);
        self
    }

    pub fn with_component(mut self, component: impl Bundle) -> ActorBuilder {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.entity).insert(component);
        });
        self
    }

    pub fn commit(mut self, commands: &mut Commands) {
        commands.append(&mut self.queue);
    }
}


struct AttributeInitCommand<T> {
    phantom: PhantomData<T>,
}

impl<T: Component<Mutability = Mutable> + Attribute> Command for AttributeInitCommand<T> {
    fn apply(self, world: &mut World) -> () {
        // Systems cannot be added multiple time. We use a resource as a 'marker'.
        if !world.contains_resource::<RegisteredPhantomSystem<T>>() {
            debug!("Registered Systems for: {}.", type_name::<T>());
            world.init_resource::<RegisteredPhantomSystem<T>>();
            if !world.contains_resource::<Events<OnBaseValueChange<T>>>() {
                EventRegistry::register_event::<OnBaseValueChange<T>>(world);
            }
            world.schedule_scope(PreUpdate, |_, schedule| {
                schedule.add_systems(
                    update_effect_tree_system::<T>.after(apply_periodic_effect::<T>),
                );
                schedule
                    .add_systems(apply_periodic_effect::<T>.after(tick_effects_periodic_timer));
            });
            world.schedule_scope(PostUpdate, |_, schedule| {
                schedule.add_systems(flag_dirty_attribute::<T>);
                schedule.add_systems(flag_dirty_modifier::<T>);
                schedule.add_systems(attribute_clamp_system::<T>);
            });
        }
    }
}
