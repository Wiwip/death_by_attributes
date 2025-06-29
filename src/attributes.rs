use crate::effects::EffectOf;
use crate::modifiers::{ModAggregator, Modifier};
use crate::{Dirty, OnAttributeValueChanged, OnModifierApplied};
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::log::debug;
use bevy::prelude::{Commands, Component, Entity, Name, Observer, Query, Trigger, World};
use std::any::type_name;
use std::marker::PhantomData;
use std::ops::DerefMut;

pub trait AttributeComponent {
    fn new(value: f64) -> Self;
    fn base_value(&self) -> f64;
    fn set_base_value(&mut self, value: f64);
    fn current_value(&self) -> f64;
    fn set_current_value(&mut self, value: f64);
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident) => {
        #[derive(bevy::prelude::Component, Default, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[require($crate::modifiers::ModAggregator<$StructName>)]
        pub struct $StructName {
            base_value: f64,
            current_value: f64,
        }

        impl $crate::attributes::AttributeComponent for $StructName {
            fn new(value: f64) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
            fn base_value(&self) -> f64 {
                self.base_value
            }
            fn set_base_value(&mut self, value: f64) {
                self.base_value = value;
            }
            fn current_value(&self) -> f64 {
                self.current_value
            }
            fn set_current_value(&mut self, value: f64) {
                self.current_value = value;
            }
        }
    };

        ( $StructName:ident, $($RequiredType:ty),+ $(,)? ) => {
        #[derive(bevy::prelude::Component, Default, Clone, Copy, bevy::prelude::Reflect, Debug)]
        #[require($crate::abilities::GameAbilityContainer, $crate::modifiers::ModAggregator<$StructName>, $($RequiredType),+)]
        pub struct $StructName {
            base_value: f64,
            current_value: f64,
        }

        impl $crate::attributes::AttributeComponent for $StructName {
            fn new(value: f64) -> Self {
                Self {
                    base_value: value,
                    current_value: value,
                }
            }
            fn base_value(&self) -> f64 {
                self.base_value
            }
            fn set_base_value(&mut self, value: f64) {
                self.base_value = value;
            }
            fn current_value(&self) -> f64 {
                self.current_value
            }
            fn set_current_value(&mut self, value: f64) {
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

    pub fn by_ref<S>(mut self, scaling_factor: f64) -> Self
    where
        S: AttributeComponent + Component<Mutability = Mutable>,
    {
        let mut observer = Observer::new(
            move |trigger: Trigger<OnAttributeValueChanged<S>>,
                  source_attribute: Query<&S>,
                  mut modifiers: Query<&mut Modifier<C>>,
                  //mut commands: Commands
        | {
                let Ok(source_attribute) = source_attribute.get(trigger.target()) else {
                    debug!(
                        "Could not find source attribute [{}]. Is the attribute added to the actor already?",
                        type_name::<S>()
                    );
                    return;
                };
                let Ok(modifier) = modifiers.get(trigger.observer()) else {
                    debug!("Could not find modifier");
                    return;
                };

                let new_value = source_attribute.current_value() * scaling_factor;
                if (modifier.value.additive - new_value).abs() > f64::EPSILON {
                    // Ensures we only deref_mut if the value actually changes
                    let Ok(mut modifier) = modifiers.get_mut(trigger.observer()) else {
                        return;
                    };
                    modifier.value.additive = source_attribute.current_value() * scaling_factor;
                    //commands.entity(trigger.target()).insert(Dirty::<C>::default());
                }
            },
        );
        observer.watch_entity(self.actor_entity);
        self.queue.push(move |world: &mut World| {
            // Spawn the observer-modifier
            world.spawn((
                observer,
                Name::new(format!("Derived Attributes ({})", type_name::<C>())),
                Modifier::<C>::default(),
                ModAggregator::<C>::default(),
                Dirty::<C>::default(),
                EffectOf(self.actor_entity),
            ));
            // Inserts the attribute on the actor
            world
                .entity_mut(self.actor_entity)
                .insert((/*C::new(0.0),*/ModAggregator::<C>::default()));
        });
        self.queue.push(move |world: &mut World| {
            world.trigger_targets(OnAttributeValueChanged::<C>::default(), self.actor_entity);
        });
        self
    }

    pub fn commit(mut self, commands: &mut Commands) {
        commands.append(&mut self.queue);
    }
}

#[derive(Component)]
pub enum AttributeClamp<A> {
    Phantom(PhantomData<A>),
    Min(f64),
    Max(f64),
    MinMax(f64, f64),
}

pub(crate) fn attribute_clamp_system<A: Component<Mutability = Mutable> + AttributeComponent>(
    mut query: Query<(&mut A, &AttributeClamp<A>)>,
) {
    for (mut attribute, clamp) in query.iter_mut() {
        match clamp {
            AttributeClamp::Min(min) => {
                let new_base = attribute.base_value().min(*min);
                attribute.set_base_value(new_base);

                let new_current = attribute.current_value().min(*min);
                attribute.set_current_value(new_current);
            }
            AttributeClamp::Max(max) => {
                let new_base = attribute.base_value().min(*max);
                attribute.set_base_value(new_base);

                let new_current = attribute.current_value().min(*max);
                attribute.set_current_value(new_current);
            }
            AttributeClamp::MinMax(min, max) => {
                let new_base = attribute.base_value().clamp(*min, *max);
                attribute.set_base_value(new_base);

                let new_current = attribute.current_value().clamp(*min, *max);
                attribute.set_current_value(new_current);
            }
            _ => {}
        }
    }
}

pub(crate) fn update_max_clamp_values<T, C>(
    trigger: Trigger<OnAttributeValueChanged<T>>,
    attribute: Query<&C>,
    mut query: Query<&mut AttributeClamp<T>>,
) where
    T: Component<Mutability = Mutable> + AttributeComponent,
    C: Component<Mutability = Mutable> + AttributeComponent,
{
    let Ok(mut clamp) = query.get_mut(trigger.target()) else {
        return;
    };
    let Ok(attribute) = attribute.get(trigger.target()) else {
        return;
    };
    match clamp.deref_mut() {
        AttributeClamp::Min(_) => {}
        AttributeClamp::Max(max) => *max = attribute.current_value(),
        AttributeClamp::MinMax(_, max) => *max = attribute.current_value(),
        _ => {}
    }
}
