use crate::abilities::GameAbilityComponent;
use crate::attributes::{GameAttribute, GameAttributeMarker};
use crate::effect::GameEffectContainer;
use crate::modifiers::{MetaModifier, ScalarModifier};
use bevy::ecs::component::Components;
use bevy::ecs::system::SystemParam;
use bevy::log::warn_once;
use bevy::prelude::Res;
use bevy::prelude::{AppTypeRegistry, Commands, Component, EntityMut, GetField, World};
use bevy::reflect::{ReflectFromPtr, ReflectMut, ReflectRef};
use std::any::TypeId;

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
        let Some(gec) = self.get_ability_container_mut(&entity_mut) else {
            return;
        };

        if let Some(ability) = gec.abilities.get(&name) {
            ability.try_activate(self, &entity_mut, commands);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use attributes_macro::Attribute;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    #[derive(Component, Attribute, Reflect, Deref, DerefMut)]
    #[reflect(Component)]
    pub struct SomeAttribute {
        pub value: GameAttribute,
    }

    fn spawn_test(mut commands: Commands) {
        commands.spawn(SomeAttribute::new(100.));
    }

    #[test]
    fn test_get_attribute_exist() {
        let mut world = World::default();

        // Must register the attribute manually
        let type_registry = AppTypeRegistry::default();
        type_registry.write().register::<SomeAttribute>();
        world.insert_resource(type_registry);

        fn get_attr_system(
            mut query: Query<EntityMut, With<SomeAttribute>>,
            context: GameAttributeContextMut,
        ) {
            assert_eq!(query.iter().len(), 1);

            for entity_mut in query.iter_mut() {
                context.get::<SomeAttribute>(&entity_mut).unwrap();
            }
        }

        world.run_system_once(spawn_test);
        world.run_system_once(get_attr_system);
    }
}
