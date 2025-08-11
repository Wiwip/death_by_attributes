use crate::attributes::Attribute;
use crate::inspector::pretty_type_name;
use crate::modifier::calculator::{AttributeCalculator, Mod};
use crate::modifier::{ModifierMarker, Mutator};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::{EffectTarget, OnEffectStatusChangeEvent};
use crate::{AttributesMut, AttributesRef, Dirty};
use bevy::prelude::*;
use std::any::{TypeId, type_name};
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;
use crate::graph::AttributeTypeId;

#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    pub who: Who,
    pub modifier: Mod,
    #[reflect(ignore)]
    marker: PhantomData<T>,
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(modifier: Mod, who: Who) -> Self {
        Self {
            who,
            modifier,
            marker: Default::default(),
        }
    }
}


impl<T> Display for AttributeModifier<T>
where
    T: Attribute,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mod<{}>({:.1})", type_name::<T>(), self.modifier)
    }
}

impl<T> Mutator for AttributeModifier<T>
where
    T: Attribute,
{
    fn spawn(&self, commands: &mut Commands, actor_entity: AttributesRef) -> Entity {
        debug!(
            "[{}] Added Mod<{}> [{}]",
            actor_entity.id(),
            pretty_type_name::<T>(),
            self.modifier,
        );

        /*let mut observer = Observer::new(
            |trigger: Trigger<OnEffectStatusChangeEvent>,
             query: Query<&EffectTarget>,
             mut commands: Commands| {
                debug!(
                    "Observer[{}] -> Target[{}] change for {}",
                    trigger.observer(),
                    trigger.target(),
                    pretty_type_name::<T>()
                );
                let parent = query.get(trigger.observer()).unwrap();

                // Marks dirty the actor, the effect, and the modifier.
                commands
                    .entity(trigger.target())
                    .insert(Dirty::<T>::default());
                commands.entity(parent.0).insert(Dirty::<T>::default());
                commands
                    .entity(trigger.observer())
                    .insert(Dirty::<T>::default());
            },
        );
        observer.watch_entity(actor_entity.id());*/

        commands
            .spawn((
                AttributeModifier::<T> {
                    who: self.who,
                    modifier: self.modifier,
                    marker: Default::default(),
                },
                //observer,
                Name::new(format!("Mod<{}> ({:?})", pretty_type_name::<T>(), self.who)),
            ))
            .id()
    }

    fn apply(&self, actor_entity: &mut AttributesMut) -> bool {
        if let Some(mut attribute) = actor_entity.get_mut::<T>() {
            let calculator = AttributeCalculator::from(self.modifier);
            let new_val = calculator.eval(attribute.base_value());
            // Ensure that the modifier meaningfully changed the value before we trigger the event.
            if (new_val - &attribute.base_value()).abs() > f64::EPSILON {
                attribute.set_base_value(new_val);
                true
            } else {
                false
            }
        } else {
            panic!("Could not find attribute {}", type_name::<T>());
        }
    }

    fn who(&self) -> Who {
        self.who
    }

    fn modifier(&self) -> Mod {
        self.modifier
    }

    fn attribute_type_id(&self) -> AttributeTypeId {
        T::attribute_type_id()
    }
}
