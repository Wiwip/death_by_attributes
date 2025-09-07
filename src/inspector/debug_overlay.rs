use crate::actors::Actor;
use crate::attributes::ReflectAccessAttribute;
use crate::effect::Stacks;
use crate::inspector::pretty_type_name_str;
use crate::modifier::{ModifierMarker, ReflectAccessModifier};
use crate::prelude::{AppliedEffects, Attribute, Effect};
use bevy::ecs::component::{ComponentId, Components};
use bevy::prelude::*;
use bevy::reflect::ReflectFromPtr;
use bevy_inspector_egui::restricted_world_view::Error;
use ptree::{TreeBuilder, write_tree};
use std::any::TypeId;

#[derive(Component, Copy, Clone)]
pub struct DebugOverlayMarker;

pub fn setup_debug_overlay(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            margin: UiRect::axes(Val::Px(5.), Val::Px(5.)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((
                Text::new("Debug Overlay"),
                TextFont {
                    font: asset_server.load("fonts/JetBrainsMono-Regular.ttf"),
                    font_size: 12.0,
                    ..default()
                },
                DebugOverlayMarker,
            ));
        });
}

pub fn explore_actors_system(
    actors: Query<(EntityRef, Option<&AppliedEffects>), (With<Actor>, With<DebugOverlayMarker>)>,
    effects: Query<(&Effect, &Stacks, Option<&Name>, Option<&AppliedEffects>)>,
    modifiers: Query<EntityRef, With<ModifierMarker>>,
    type_registry: Res<AppTypeRegistry>,
    world_components: &Components,
    mut debug_overlay: Query<
        &mut Text,
        (
            With<DebugOverlayMarker>,
            Without<Actor>,
            Without<ModifierMarker>,
        ),
    >,
) {
    let mut builder = TreeBuilder::new("Actor Tree".into());

    for (actor_ref, actor_effects) in actors.iter() {
        builder.begin_child(format!("Actor {}", actor_ref.id()));

        let mut actor_components = get_components_sorted(world_components, actor_ref);

        list_attributes(
            &mut builder,
            &type_registry,
            actor_ref,
            &mut actor_components,
        );

        if let Some(actor_effects) = actor_effects {
            list_effects(
                &mut builder,
                actor_effects,
                effects,
                modifiers,
                &type_registry,
                &world_components,
            );
        }

        builder.end_child();
    }

    let tree = builder.build();
    if let Ok(mut text) = debug_overlay.single_mut() {
        let mut w = Vec::new();
        let _ = write_tree(&tree, &mut w);
        text.0 = String::from_utf8(w).unwrap();
    }
}

fn list_attributes(
    builder: &mut TreeBuilder,
    type_registry: &AppTypeRegistry,
    actor_ref: EntityRef,
    actor_components: &mut Vec<(String, ComponentId, Option<TypeId>, usize)>,
) {
    builder.begin_child("Attributes".to_string());
    // List attributes for printing
    for (_, component_id, type_id, _) in actor_components.iter() {
        let Some(type_id) = type_id else {
            continue;
        };
        let Ok(ptr) = actor_ref.get_by_id(*component_id) else {
            continue;
        };

        let registry = type_registry.read();
        let reflect_attribute = registry.get_type_data::<ReflectAccessAttribute>(*type_id);
        let Some(reflect_access_attribute) = reflect_attribute else {
            continue;
        };

        let registration = registry
            .get(*type_id)
            .ok_or(Error::NoTypeRegistration(*type_id))
            .unwrap();
        let reflect_from_ptr = registration
            .data::<ReflectFromPtr>()
            .ok_or(Error::NoTypeData(*type_id, "ReflectFromPtr"))
            .unwrap();

        let value = unsafe { reflect_from_ptr.as_reflect(ptr) };
        let Some(attribute) = reflect_access_attribute.get(value) else {
            continue;
        };

        builder
            .begin_child(format!(
                "{}: {:.1}",
                attribute.name(),
                attribute.access_current_value()
            ))
            .end_child();
    }
    builder.end_child();
}

fn list_effects(
    mut builder: &mut TreeBuilder,
    actor_effects: &AppliedEffects,
    effect_query: Query<(&Effect, &Stacks, Option<&Name>, Option<&AppliedEffects>)>,
    modifier_query: Query<EntityRef, With<ModifierMarker>>,
    type_registry: &AppTypeRegistry,
    world_components: &Components,
) {
    builder.begin_child("Effects".to_string());
    for effect_entity in actor_effects.iter() {
        let Ok((_, stacks, name, modifiers)) = effect_query.get(effect_entity) else {
            continue;
        };
        let name = match name {
            None => "Effect",
            Some(name) => name,
        };

        builder.begin_child(format!(
            "[{effect_entity}] {name} [{}]",
            stacks.current_value()
        ));

        let Some(modifiers) = modifiers else {
            continue;
        };
        for modifier in modifiers.iter() {
            let Ok(modifier_ref) = modifier_query.get(modifier) else {
                continue;
            };

            let mut modifier_components = get_components_sorted(world_components, modifier_ref);

            list_modifiers_for_effect(
                &mut builder,
                type_registry,
                modifier_ref,
                &mut modifier_components,
            );
        }
        builder.end_child();
    }
    builder.end_child();
}

fn get_components_sorted(
    world_components: &Components,
    modifier_ref: EntityRef,
) -> Vec<(String, ComponentId, Option<TypeId>, usize)> {
    let archetype = modifier_ref.archetype();
    let mut modifier_components: Vec<_> = archetype
        .components()
        .map(|component_id| {
            let info = world_components.get_info(component_id).unwrap();
            let name = pretty_type_name_str(info.name());

            (name, component_id, info.type_id(), info.layout().size())
        })
        .collect();
    modifier_components.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    modifier_components
}

fn list_modifiers_for_effect(
    builder: &mut TreeBuilder,
    type_registry: &AppTypeRegistry,
    modifier_ref: EntityRef,
    modifier_components: &mut Vec<(String, ComponentId, Option<TypeId>, usize)>,
) {
    // List attributes for printing
    for (_name, component_id, type_id, _) in modifier_components.iter() {
        let Some(type_id) = type_id else {
            continue;
        };
        let Ok(ptr) = modifier_ref.get_by_id(*component_id) else {
            continue;
        };

        let registry = type_registry.read();
        let reflect_attribute = registry.get_type_data::<ReflectAccessModifier>(*type_id);
        let Some(reflect_access_modifier) = reflect_attribute else {
            continue;
        };

        let registration = registry
            .get(*type_id)
            .ok_or(Error::NoTypeRegistration(*type_id))
            .unwrap();
        let reflect_from_ptr = registration
            .data::<ReflectFromPtr>()
            .ok_or(Error::NoTypeData(*type_id, "ReflectFromPtr"))
            .unwrap();

        // SAFETY: Confirm assumptions here.
        let value = unsafe { reflect_from_ptr.as_reflect(ptr) };
        let Some(modifier) = reflect_access_modifier.get(value) else {
            continue;
        };

        builder
            .begin_child(format!(
                "[{}] {}",
                modifier_ref.id(),
                modifier.describe(),
            ))
            .end_child();
    }
}
