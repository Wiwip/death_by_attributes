use crate::modifiers::{EffectOf, ModAggregator, Modifier, ModifierOf};
use crate::{Dirty, OnAttributeValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::log::debug;
use bevy::prelude::{Commands, Component, Entity, Name, Observer, Query, Trigger, World};
use std::any::type_name;
use std::marker::PhantomData;

pub trait AttributeComponent {
    fn new(value: f32) -> Self;
    fn base_value(&self) -> f32;
    fn set_base_value(&mut self, value: f32);
    fn current_value(&self) -> f32;
    fn set_current_value(&mut self, value: f32);
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(bevy::prelude::Component, Default, Clone, bevy::prelude::Reflect, Debug)]
        #[require($crate::abilities::GameAbilityContainer, $crate::modifiers::ModAggregator<$StructName>)]
        pub struct $StructName {
            base_value: f32,
            current_value: f32,
        }

        impl $crate::attributes::AttributeComponent for $StructName {
            fn new(value: f32) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
            fn base_value(&self) -> f32 {
                self.base_value
            }
            fn set_base_value(&mut self, value: f32) {
                self.base_value = value;
            }
            fn current_value(&self) -> f32 {
                self.current_value
            }
            fn set_current_value(&mut self, value: f32) {
                self.current_value = value;
            }
        }
    };
}

pub struct AttributeBuilder<A> {
    actor_entity: Entity,
    queue: CommandQueue,
    _marker: PhantomData<A>,
}

impl<C> AttributeBuilder<C>
where
    C: AttributeComponent + Component<Mutability = Mutable>,
{
    pub fn new(actor_entity: Entity) -> Self {
        Self {
            actor_entity,
            queue: Default::default(),
            _marker: Default::default(),
        }
    }

    pub fn by_ref<S>(mut self, scaling_factor: f32) -> Self
    where
        S: AttributeComponent + Component<Mutability = Mutable>,
    {
        let mut observer = Observer::new(
            move |trigger: Trigger<OnAttributeValueChanged>,
                  source_attribute: Query<&S>,
                  mut modifiers: Query<&mut Modifier<C>>| {
                let Ok(source_attribute) = source_attribute.get(trigger.target()) else {
                    debug!(
                        "Could not find source attribute [{}]. Is the attribute added to the actor already?",
                        type_name::<S>()
                    );
                    return;
                };
                let Ok(modifier) = modifiers.get(trigger.observer()) else {
                    return;
                };
                if modifier.value.additive != source_attribute.current_value() * scaling_factor {
                    // Ensures we only deref_mut if the value actually changes
                    let Ok(mut modifier) = modifiers.get_mut(trigger.observer()) else {
                        return;
                    };
                    modifier.value.additive = source_attribute.current_value() * scaling_factor;
                }
            },
        );
        observer.watch_entity(self.actor_entity);
        self.queue.push(move |world: &mut World| {
            // Spawn the observer-modifier
            world.spawn((
                observer,
                Name::new("Derived Attributes"),
                Modifier::<C>::default(),
                ModAggregator::<C>::default(),
                ModifierOf(self.actor_entity),
                EffectOf(self.actor_entity),
                Dirty::<C>::default(),
            ));
            // Inserts the attribute on the actor
            world
                .entity_mut(self.actor_entity)
                .insert((C::new(0.0), ModAggregator::<C>::default()));
        });
        self.queue.push(move |world: &mut World| {
            world.trigger_targets(OnAttributeValueChanged, self.actor_entity);
        });
        self
    }

    pub fn commit(mut self, commands: &mut Commands) {
        commands.append(&mut self.queue);
    }
}
