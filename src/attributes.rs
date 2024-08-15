use crate::modifiers::{MetaModifier, ScalarModifier};
use bevy::ecs::component::{Components, Tick};
use bevy::prelude::*;
use bevy::reflect::{ReflectFromPtr, ReflectMut, ReflectRef, TypeRegistryArc};
use std::any::TypeId;
use crate::effect::{GameEffect, GameEffectContainer};

pub trait GameAttributeMarker {}

#[derive(Reflect)]
pub struct GameAttribute {
    pub base_value: f32,
    pub current_value: f32,
}

pub struct GameAttributeContextMut<'a> {
    pub entity_mut: EntityMut<'a>,
    pub components: &'a Components,
    pub type_registry: TypeRegistryArc,
}

impl GameAttributeContextMut<'_> {
    pub fn get_attribute_mut(&self, type_id: TypeId) -> Option<&mut GameAttribute> {
        let Some(component_id) = self.components.get_id(type_id) else {
            warn_once!("The requested type_id is not part of the components.");
            return None;
        };

        let Some(ptr) = self.entity_mut.get_by_id(component_id) else {
            return None;
        };

        let type_registry = self.type_registry.read();
        let reflect_data = type_registry
            .get(type_id)
            .unwrap_or_else(|| panic!("The type_id isn't registered."));

        // Yes. This is evil. Should use entity_mut, but necessary due to a bevy engine bug.
        let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
        let data = unsafe { reflect_from_ptr.as_reflect_mut(ptr.assert_unique()) };
        // Yes. This is evil. Should use entity_mut, but necessary due to a bevy engine bug.

        if let ReflectMut::Struct(value) = data.reflect_mut() {
            let attr = value.get_field_mut::<GameAttribute>("value").unwrap();
            Some(attr)
        } else {
            None
        }
    }

    pub fn get_attribute(&self, type_id: TypeId) -> Option<&GameAttribute> {
        let Some(component_id) = self.components.get_id(type_id) else {
            warn_once!("The requested type_id is not part of the components.");
            return None;
        };

        let Some(ptr) = self.entity_mut.get_by_id(component_id) else {
            return None;
        };

        let type_registry = self.type_registry.read();
        let reflect_data = type_registry
            .get(type_id)
            .unwrap_or_else(|| panic!("The type isn't registered."));

        let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>()?;
        let data = unsafe { reflect_from_ptr.as_reflect(ptr) };

        if let ReflectRef::Struct(value) = data.reflect_ref() {
            let attr = value.get_field::<GameAttribute>("value")?;
            Some(attr)
        } else {
            None
        }
    }

    pub fn get_game_effect_container(&self) -> Option<&GameEffectContainer> {
        self.entity_mut.get::<GameEffectContainer>()
    }

    pub fn get<T: Component + GameAttributeMarker>(&self) -> Option<&GameAttribute> {
        self.get_attribute(TypeId::of::<T>())
    }

    pub fn convert_modifier(&self, meta: &MetaModifier) -> Option<ScalarModifier> {
        if let Some(attribute) = self.get_attribute(meta.magnitude_attribute) {
            return Some(ScalarModifier {
                target_attribute: meta.target_attribute,
                magnitude: attribute.current_value,
                mod_type: meta.mod_type,
            });
        }
        None
    }
}
