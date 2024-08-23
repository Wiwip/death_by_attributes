use crate::abilities::GameAbilityComponent;
use crate::attributes::{GameAttribute, GameAttributeMarker};
use crate::effect::GameEffectContainer;
use crate::modifiers::{MetaModifier, ScalarModifier};
use bevy::ecs::component::Components;
use bevy::ecs::system::SystemParam;
use bevy::log::warn_once;
use bevy::prelude::{
    AppTypeRegistry, Component, EntityMut, EntityRef, FromWorld, GetField, Res, Resource, World,
};
use bevy::reflect::{ReflectFromPtr, ReflectMut, ReflectRef, TypeRegistry, TypeRegistryArc};
use std::any::TypeId;
use std::sync::Arc;

#[derive(SystemParam)]
pub struct GameAttributeContextMut<'w> {
    pub components: &'w Components,
    pub type_registry: Res<'w, AppTypeRegistry>,
}

impl GameAttributeContextMut<'_> {
    pub fn get_mut_by_id<'a>(
        &'a self,
        entity_mut: &'a EntityMut,
        type_id: TypeId,
    ) -> Option<&mut GameAttribute> {
        let Some(component_id) = self.components.get_id(type_id) else {
            warn_once!("The requested type_id is not part of the components.");
            return None;
        };

        let Some(ptr) = entity_mut.get_by_id(component_id) else {
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

    pub fn get_by_id<'a>(
        &'a self,
        entity_mut: &'a EntityRef,
        type_id: TypeId,
    ) -> Option<&GameAttribute> {
        let Some(component_id) = self.components.get_id(type_id) else {
            warn_once!("The requested type_id is not part of the components.");
            return None;
        };

        let Some(ptr) = entity_mut.get_by_id(component_id) else {
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

    pub fn get_effect_container<'a>(
        &'a self,
        entity_mut: &'a EntityMut,
    ) -> Option<&GameEffectContainer> {
        entity_mut.get::<GameEffectContainer>()
    }

    pub fn get_ability_container<'a>(
        &'a self,
        entity_mut: &'a EntityMut,
    ) -> Option<&GameAbilityComponent> {
        entity_mut.get::<GameAbilityComponent>()
    }

    pub fn get_mut<'a, T: Component + GameAttributeMarker>(
        &'a self,
        entity_mut: &'a EntityMut,
    ) -> Option<&mut GameAttribute> {
        self.get_mut_by_id(entity_mut, TypeId::of::<T>())
    }

    pub fn get<'a, T: Component + GameAttributeMarker>(
        &'a self,
        entity_mut: &'a EntityRef,
    ) -> Option<&GameAttribute> {
        self.get_by_id(entity_mut, TypeId::of::<T>())
    }

    pub fn convert_modifier(
        &self,
        entity_ref: &EntityRef,
        meta: &MetaModifier,
    ) -> Option<ScalarModifier> {
        if let Some(attribute) = self.get_by_id(entity_ref, meta.magnitude_attribute) {
            return Some(ScalarModifier {
                target_attribute: meta.target_attribute,
                magnitude: attribute.current_value,
                mod_type: meta.mod_type,
            });
        }
        None
    }
}

/*
impl<'a> From<&'a GameAttributeContextMut<'a>> for GameAttributeContext<'a> {
    fn from(value: &'a GameAttributeContextMut<'a>) -> Self {
        GameAttributeContext {
            components: value.components,
            type_registry: ,
        }
    }
}


#[derive(SystemParam)]
pub struct GameAttributeContext<'a> {
    pub components: &'a Components,
    pub type_registry: TypeRegistryArc,
}

impl GameAttributeContext<'_> {
    pub fn get_by_id(&self, entity_ref: EntityRef, type_id: TypeId) -> Option<&GameAttribute> {
        let Some(component_id) = self.components.get_id(type_id) else {
            warn_once!("The requested type_id is not part of the components.");
            return None;
        };

        let Some(ptr) = entity_ref.get_by_id(component_id) else {
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

    pub fn get_game_effect_container(&self, entity_ref: EntityRef) -> Option<&GameEffectContainer> {
        entity_ref.get::<GameEffectContainer>()
    }

    pub fn get<T: Component + GameAttributeMarker>(&self, entity_ref: EntityRef) -> Option<&GameAttribute> {
        self.get_attribute(entity_ref, TypeId::of::<T>())
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
*/
