use crate::attributes::{
    AttributeClamp, AttributeComponent, attribute_clamp_system, update_max_clamp_values,
};
use crate::systems::{
    flag_dirty_attribute_nodes, flag_dirty_modifier_nodes, tick_effects_periodic_timer,
    trigger_instant_effect_applied, trigger_periodic_effect, update_effect_tree_system,
};
use crate::{
    Actor, ObserverMarker, OnAttributeValueChanged, OnBaseValueChange, OnModifierApplied,
    RegisteredSystemCache,
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
            world
                .entity_mut(self.entity)
                .observe(attribute_change_trigger::<T>);
        });
        self
    }

    pub fn clamp<T>(mut self, clamp: AttributeClamp<T>) -> ActorBuilder
    where
        T: Component<Mutability = Mutable> + AttributeComponent,
    {
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.entity).insert(clamp);
        });
        self
    }

    pub fn max<T, C>(mut self, min: f32) -> ActorBuilder
    where
        T: Component<Mutability = Mutable> + AttributeComponent,
        C: Component<Mutability = Mutable> + AttributeComponent,
    {
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.entity)
                .insert(AttributeClamp::<T>::MinMax(min, f32::MAX))
                .observe(update_max_clamp_values::<T, C>);
            world.trigger_targets(OnAttributeValueChanged, self.entity);
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

fn attribute_change_trigger<T: Component<Mutability = Mutable> + AttributeComponent>(
    trigger: Trigger<OnModifierApplied<T>>,
    mut query: Query<&mut T>,
) {
    let Ok(mut attribute) = query.get_mut(trigger.target()) else {
        return;
    };
    let new_val = trigger.value.evaluate(attribute.base_value());
    attribute.set_base_value(new_val);
}

struct AttributeInitCommand<T> {
    phantom: PhantomData<T>,
}

impl<T: Component<Mutability = Mutable> + AttributeComponent> Command for AttributeInitCommand<T> {
    fn apply(self, world: &mut World) -> () {
        // Systems cannot be added multiple time. We use a resource as a 'marker'.
        if !world.contains_resource::<RegisteredSystemCache<T>>() {
            debug!("Registered Systems for: {}.", type_name::<T>());
            world.init_resource::<RegisteredSystemCache<T>>();
            if !world.contains_resource::<Events<OnBaseValueChange<T>>>() {
                EventRegistry::register_event::<OnBaseValueChange<T>>(world);
            }
            world.schedule_scope(PreUpdate, |_, schedule| {
                schedule.add_systems(
                    update_effect_tree_system::<T>.after(trigger_periodic_effect::<T>),
                );
                schedule
                    .add_systems(trigger_periodic_effect::<T>.after(tick_effects_periodic_timer));
            });
            world.schedule_scope(PostUpdate, |_, schedule| {
                schedule.add_systems(flag_dirty_modifier_nodes::<T>);
                schedule.add_systems(flag_dirty_attribute_nodes::<T>);
                schedule.add_systems(attribute_clamp_system::<T>);
            });
        }
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
