use crate::abilities::GameAbilityComponent;
use crate::attributes::{GameAttribute, GameAttributeMarker};
use crate::effect::GameEffectContainer;
use crate::modifiers::{MetaModifier, ScalarModifier};
use bevy::ecs::component::Components;
use bevy::ecs::system::SystemParam;
use bevy::log::warn_once;
use bevy::prelude::{
    AppTypeRegistry, Commands, Component, EntityMut, EntityRef, FromWorld, GetField, Mut, Res,
    Resource, World,
};
use bevy::reflect::{ReflectFromPtr, ReflectMut, ReflectRef, TypeRegistry, TypeRegistryArc};
use std::any::TypeId;
use std::sync::{Arc, RwLock};

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

        let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
        let data = unsafe { reflect_from_ptr.as_reflect_mut(ptr.assert_unique()) };

        if let ReflectMut::Struct(value) = data.reflect_mut() {
            let attr = value.get_field_mut::<GameAttribute>("value")?;
            Some(attr)
        } else {
            None
        }
    }

    pub fn get_by_id<'a>(
        &'a self,
        entity_mut: &'a EntityMut,
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

    pub fn get_ability_container_mut<'a>(
        &'a self,
        entity_mut: &'a EntityMut<'_>,
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
        entity_mut: &'a EntityMut,
    ) -> Option<&GameAttribute> {
        self.get_by_id(entity_mut, TypeId::of::<T>())
    }

    pub fn convert_modifier(
        &self,
        entity_mut: &EntityMut,
        meta: &MetaModifier,
    ) -> Option<ScalarModifier> {
        if let Some(attribute) = self.get_by_id(entity_mut, meta.magnitude_attribute) {
            return Some(ScalarModifier {
                target_attribute: meta.target_attribute,
                magnitude: attribute.current_value,
                mod_type: meta.mod_type,
            });
        }
        None
    }

    pub fn try_activate(&self, entity_mut: EntityMut, name: String, commands: Commands) {
        let Some(mut gec) = self.get_ability_container_mut(&entity_mut) else {
            return;
        };

        if let Some(ability) = gec.abilities.get(&name) {
            ability.try_activate(self, &entity_mut, commands);
        }
    }
}
