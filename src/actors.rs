use crate::attributes::AttributeComponent;
use crate::systems::{
    flag_dirty_modifier_nodes, tick_effects_periodic_timer, trigger_instant_effect_applied,
    trigger_periodic_effect, update_attribute_tree_system,
};
use crate::{Actor, Dirty, ObserverMarker, RegisteredSystemCache};
use bevy::app::{PostUpdate, PreUpdate};
use bevy::ecs::component::Mutable;
use bevy::log::debug;
use bevy::prelude::{
    Command, Commands, Component, Entity, IntoScheduleConfigs, IntoSystemSet, World,
};
use std::any::type_name;
use std::marker::PhantomData;

pub struct ActorBuilder<'a> {
    pub(crate) entity: Entity,
    commands: &'a mut Commands<'a, 'a>,
}

impl<'a> ActorBuilder<'a> {
    pub fn new(commands: &'a mut Commands<'a, 'a>) -> ActorBuilder<'a> {
        let mut entity_command = commands.spawn_empty();
        entity_command.insert(Actor);
        Self {
            entity: entity_command.id(),
            commands,
        }
    }

    pub fn from(commands: &'a mut Commands<'a, 'a>, entity: Entity) -> ActorBuilder<'a> {
        commands.entity(entity).insert(Actor);
        Self { entity, commands }
    }

    pub fn with_attribute<T>(self, value: f32) -> ActorBuilder<'a>
    where
        T: Component<Mutability = Mutable> + AttributeComponent,
    {
        // Ensures that the systems related to this attribute exist in the schedule
        self.commands.queue(AttributeCommand::<T> {
            phantom: Default::default(),
        });

        // Inserts an attribute T on the entity
        self.commands.entity(self.entity).insert(T::new(value));
        // Flags it as dirty so it will be updated automatically
        self.commands
            .entity(self.entity)
            .insert(Dirty::<T>::default());
        self
    }

    pub fn commit(self) -> Entity {
        self.commands.entity(self.entity).id()
    }
}

struct AttributeCommand<T> {
    phantom: PhantomData<T>,
}

impl<T: Component<Mutability = Mutable> + AttributeComponent> Command for AttributeCommand<T> {
    fn apply(self, world: &mut World) -> () {
        // Systems cannot be added multiple time. We use a resource as a 'marker'.
        if !world.contains_resource::<RegisteredSystemCache<T>>() {
            world.schedule_scope(PreUpdate, |_, schedule| {
                schedule.add_systems(
                    update_attribute_tree_system::<T>.after(trigger_periodic_effect::<T>),
                );
                schedule
                    .add_systems(trigger_periodic_effect::<T>.after(tick_effects_periodic_timer));
            });
            world.schedule_scope(PostUpdate, |_, schedule| {
                //schedule.add_systems(flag_dirty_attribute_nodes::<T>);
                schedule.add_systems(flag_dirty_modifier_nodes::<T>);
                //schedule.add_systems(on_instant_effect_applied::<T>.before(despawn_instant_effect));
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
