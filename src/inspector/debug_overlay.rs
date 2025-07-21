use crate::attributes::ReflectAccessAttribute;
use bevy::asset::UntypedAssetId;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::{AppTypeRegistry, Children, DetectChangesMut, Entity, World};
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use bevy_egui::egui::FontId;
use bevy_egui::egui::debug_text::print;
use bevy_inspector_egui::bevy_inspector::{EntityFilter, Filter, guess_entity_name};
use bevy_inspector_egui::egui_utils::layout_job;
use bevy_inspector_egui::reflect_inspector::{Context, InspectorUi};
use bevy_inspector_egui::restricted_world_view::{Error, ReflectBorrow, RestrictedWorldView};
use std::any::TypeId;
use std::borrow::Cow;

/// Display all entities matching the given [`EntityFilter`].
///
/// You can use the [`Filter`] type to specify both a static filter as a generic parameter (default is `Without<Parent>`),
/// and a word to match. [`Filter::from_ui`] will display a search box and fuzzy filter checkbox.
pub fn ui_for_entities_expanded_filtered<F>(world: &mut World, ui: &mut egui::Ui, filter: &F)
where
    F: EntityFilter,
{
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let mut root_entities = world.query_filtered::<Entity, F::StaticFilter>();
    let mut entities = root_entities.iter(world).collect::<Vec<_>>();

    filter.filter_entities(world, &mut entities);

    entities.sort();

    let id = egui::Id::new("world ui expanded");
    for entity in entities {
        let id = id.with(entity);

        let entity_name = guess_entity_name(world, entity);

        egui::CollapsingHeader::new(&entity_name)
            .id_salt(id)
            .default_open(true)
            .show(ui, |ui| {
                ui_for_entity_attributes(
                    &mut world.into(),
                    entity,
                    ui,
                    id,
                    &type_registry,
                );


            });
    }
}

/// Display the components of the given entity
pub(crate) fn ui_for_entity_attributes(
    world: &mut RestrictedWorldView<'_>,
    entity: Entity,
    ui: &mut egui::Ui,
    id: egui::Id,
    type_registry: &TypeRegistry,
) {
    let Ok(components) = components_of_entity(world, entity) else {
        entity_does_not_exist(ui, entity);
        return;
    };

    ui.heading("Attributes");
    ui.indent(id, |ui| {
        for (name, component_id, component_type_id, size) in components {
            let id = id.with(component_id);

            let header = egui::CollapsingHeader::new(&name).id_salt(id);

            let Some(component_type_id) = component_type_id else {
                header.show(ui, |ui| no_type_id(ui, &name));
                continue;
            };

            // create a context with access to the world except for the currently viewed component
            let (mut component_view, world) =
                world.split_off_component((entity, component_type_id));

            let value = match component_view.get_entity_component_reflect(
                entity,
                component_type_id,
                type_registry,
            ) {
                Ok(value) => value,
                Err(_) => {
                    continue;
                }
            };

            let reflect_attribute =
                type_registry.get_type_data::<ReflectAccessAttribute>(component_type_id);
            let Some(reflect_access_attribute) = reflect_attribute else {
                continue;
            };

            match value {
                ReflectBorrow::Mutable(mut value) => {
                    if let Some(attribute) = reflect_access_attribute.get(value.as_reflect()) {
                        let response = ui.label(format!(
                            "{}: {:.1}",
                            attribute.name(),
                            attribute.current_value()
                        ));
                        response.on_hover_ui(|ui| {
                            ui.label(format!("Base Value: {:.1}", attribute.base_value()));
                            ui.label(format!("Current Value: {:.1}", attribute.current_value()));
                        });
                    };
                }
                ReflectBorrow::Immutable(value) => {
                    if let Some(attribute) = reflect_access_attribute.get(value.as_reflect()) {
                        let response = ui.label(format!(
                            "{}: {:.1}",
                            attribute.name(),
                            attribute.current_value()
                        ));
                        response.on_hover_ui(|ui| {
                            ui.label(format!("Base Value: {:.1}", attribute.base_value()));
                            ui.label(format!("Current Value: {:.1}", attribute.current_value()));
                        });
                    };
                }
            };
        }
    });

    ui.reset_style();
}

fn components_of_entity(
    world: &mut RestrictedWorldView<'_>,
    entity: Entity,
) -> bevy::prelude::Result<Vec<(String, ComponentId, Option<TypeId>, usize)>> {
    let entity_ref = world.world().get_entity(entity)?;

    let archetype = entity_ref.archetype();
    let mut components: Vec<_> = archetype
        .components()
        .map(|component_id| {
            let info = world.world().components().get_info(component_id).unwrap();
            let name = pretty_type_name_str(info.name());

            (name, component_id, info.type_id(), info.layout().size())
        })
        .collect();
    components.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    Ok(components)
}

pub fn pretty_type_name<T>() -> String {
    format!("{:?}", disqualified::ShortName::of::<T>())
}
pub fn pretty_type_name_str(val: &str) -> String {
    format!("{:?}", disqualified::ShortName(val))
}

/*
 Imported temporarily
*/

pub fn show_error(error: Error, ui: &mut egui::Ui, name_of_type: &str) {
    match error {
        Error::NoAccessToResource(_) => no_access_resource(ui, name_of_type),
        Error::NoAccessToComponent((entity, _)) => no_access_component(ui, entity, name_of_type),
        Error::ComponentDoesNotExist((entity, _)) => {
            component_does_not_exist(ui, entity, name_of_type)
        }
        Error::ResourceDoesNotExist(_) => resource_does_not_exist(ui, name_of_type),
        Error::NoComponentId(_) => no_component_id(ui, name_of_type),
        Error::NoTypeRegistration(_) => {
            //not_in_type_registry(ui, name_of_type)
        }
        Error::NoTypeData(_, data) => no_type_data(ui, name_of_type, data),
    }
}

pub fn no_access_resource(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "No access to resource "),
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}
pub fn no_access_component(ui: &mut egui::Ui, entity: Entity, type_name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "No access to component "),
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " on entity "),
        (FontId::monospace(12.0), &format!("{entity:?}")),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

pub fn resource_does_not_exist(ui: &mut egui::Ui, name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Resource "),
        (FontId::monospace(12.0), name),
        (FontId::proportional(13.0), " does not exist in the world."),
    ]);

    ui.label(job);
}

pub fn component_does_not_exist(ui: &mut egui::Ui, entity: Entity, name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Component "),
        (FontId::monospace(12.0), name),
        (FontId::proportional(13.0), " does not exist on entity "),
        (FontId::monospace(12.0), &format!("{entity:?}")),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

pub fn no_component_id(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " has no associated "),
        (FontId::monospace(12.0), "ComponentId"),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

pub fn no_type_data(ui: &mut egui::Ui, type_name: &str, type_data: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " has no "),
        (FontId::monospace(12.0), type_data),
        (
            FontId::proportional(13.0),
            " type data, so it cannot be displayed",
        ),
    ]);

    ui.label(job);
}

pub fn entity_does_not_exist(ui: &mut egui::Ui, entity: Entity) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Entity "),
        (FontId::monospace(12.0), &format!("{entity:?}")),
        (FontId::proportional(13.0), " does not exist."),
    ]);

    ui.label(job);
}

pub fn no_world_in_context(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " needs the bevy world in the "),
        (FontId::monospace(12.0), "InspectorUi"),
        (
            FontId::proportional(13.0),
            " context to provide meaningful information.",
        ),
    ]);

    ui.label(job);
}

pub fn dead_asset_handle(ui: &mut egui::Ui, handle: UntypedAssetId) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Handle "),
        (FontId::monospace(12.0), &format!("{handle:?}")),
        (FontId::proportional(13.0), " points to no asset."),
    ]);

    ui.label(job);
}

pub fn state_does_not_exist(ui: &mut egui::Ui, name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "State "),
        (FontId::monospace(12.0), name),
        (
            FontId::proportional(13.0),
            " does not exist. Did you forget to call ",
        ),
        (
            FontId::monospace(12.0),
            &format!(".add_state::<{name}>(..)"),
        ),
        (FontId::proportional(13.0), "?"),
    ]);

    ui.label(job);
}

pub fn no_type_id(ui: &mut egui::Ui, component_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), component_name),
        (
            FontId::proportional(13.0),
            " is not backed by a rust type, so it cannot be displayed.",
        ),
    ]);

    ui.label(job);
}

pub fn name_of_type(type_id: TypeId, type_registry: &TypeRegistry) -> Cow<'_, str> {
    type_registry
        .get(type_id)
        .map(|registration| Cow::Borrowed(registration.type_info().type_path_table().short_path()))
        .unwrap_or_else(|| Cow::Owned(format!("{type_id:?}")))
}

pub fn reflect_value_no_impl(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " is "),
        (FontId::monospace(12.0), "#[reflect_value]"),
        (FontId::proportional(13.0), ", but has no "),
        (FontId::monospace(12.0), "InspectorEguiImpl"),
        (FontId::proportional(13.0), " registered in the "),
        (FontId::monospace(12.0), "TypeRegistry"),
        (FontId::proportional(13.0), " .\n"),
        (FontId::proportional(13.0), "Try calling "),
        (
            FontId::monospace(12.0),
            &format!(".register_type::<{}>", pretty_type_name_str(type_name)),
        ),
        (FontId::proportional(13.0), " or add the "),
        (FontId::monospace(12.0), "DefaultInspectorConfigPlugin"),
        (FontId::proportional(13.0), " for builtin types."),
    ]);

    ui.label(job);
}
pub fn no_default_value(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " has no "),
        (FontId::monospace(12.0), "ReflectDefault"),
        (
            FontId::proportional(13.0),
            " type data, so no value of it can be constructed.",
        ),
    ]);

    ui.label(job);
}

pub fn unconstructable_variant(
    ui: &mut egui::Ui,
    type_name: &str,
    variant: &str,
    unconstructable_field_types: &[&str],
) {
    let mut vec = Vec::with_capacity(2 + unconstructable_field_types.len() * 2 + 4);

    let qualified_variant = format!("{}::{}", pretty_type_name_str(type_name), variant);
    vec.extend([
        (FontId::monospace(12.0), qualified_variant.as_str()),
        (
            FontId::proportional(13.0),
            " has unconstructable fields.\nConsider adding ",
        ),
        (FontId::monospace(12.0), "#[reflect(Default)]"),
        (FontId::proportional(13.0), " to\n\n"),
    ]);
    vec.extend(unconstructable_field_types.iter().flat_map(|variant| {
        [
            (FontId::proportional(13.0), "- "),
            (FontId::monospace(12.0), *variant),
        ]
    }));

    let job = layout_job(&vec);

    ui.label(job);
}

pub fn not_in_type_registry(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " is not registered in the "),
        (FontId::monospace(12.0), "TypeRegistry"),
    ]);

    ui.label(job);
}

pub fn no_multiedit(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (
            FontId::proportional(13.0),
            " doesn't support multi-editing.",
        ),
    ]);

    ui.label(job);
}
