use crate::attributes::AttributeComponent;
use crate::systems::{
    flag_dirty_modifier_nodes, tick_effects_periodic_timer, trigger_instant_effect_applied,
    trigger_periodic_effect, update_effect_tree_system,
};
use crate::{Actor, ObserverMarker, RegisteredSystemCache};
use bevy::app::{PostUpdate, PreUpdate};
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::log::debug;
use bevy::prelude::{Bundle, Command, Commands, Component, Entity, IntoScheduleConfigs, World};
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

    pub fn with<T>(mut self, value: f32) -> ActorBuilder
    where
        T: Component<Mutability = Mutable> + AttributeComponent,
    {
        // Ensures that the systems related to this attribute exist in the schedule
        self.queue.push(AttributeInitCommand::<T> {
            phantom: Default::default(),
        });
        // Inserts the actual T attribute on the entity
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.entity).insert(T::new(value));
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

impl<T: Component<Mutability = Mutable> + AttributeComponent> Command for AttributeInitCommand<T> {
    fn apply(self, world: &mut World) -> () {
        // Systems cannot be added multiple time. We use a resource as a 'marker'.
        if !world.contains_resource::<RegisteredSystemCache<T>>() {
            debug!("Registered Systems for: {}.", type_name::<T>());
            world.schedule_scope(PreUpdate, |_, schedule| {
                schedule.add_systems(
                    update_effect_tree_system::<T>.after(trigger_periodic_effect::<T>),
                );
                schedule
                    .add_systems(trigger_periodic_effect::<T>.after(tick_effects_periodic_timer));
            });
            world.schedule_scope(PostUpdate, |_, schedule| {
                schedule.add_systems(flag_dirty_modifier_nodes::<T>);
            });
        }
        world.init_resource::<RegisteredSystemCache<T>>();

        // We ensure that only one observer exist for a specific attribute
        let mut query = world.query::<&ObserverMarker<T>>();
        if query.iter(world).count() == 0 {
            debug!(
                "Observer added for {} using `fn trigger_instant_effect_applied<T>`",
                type_name::<T>()
            );
            world
                .add_observer(trigger_instant_effect_applied::<T>)
                .insert(ObserverMarker::<T>::default());
        }
    }
}
