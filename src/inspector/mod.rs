mod debug_overlay;
pub mod test;

use crate::inspector::debug_overlay::ui_for_entities_expanded_filtered;
use crate::inspector::test::{explore_actors_system, setup_debug_overlay};
use bevy::app::MainSchedulePlugin;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::schedule::BoxedCondition;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_egui::egui::Align2;
use bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, egui};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui::bevy_inspector::Filter;
use std::marker::PhantomData;
use std::sync::Mutex;
use std::time::Duration;

/// Plugin displaying an egui window for all entities matching the filter `F`.
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_inspector_egui::quick::FilterQueryInspectorPlugin;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(FilterQueryInspectorPlugin::<With<Transform>>::default())
///         .run();
/// }
/// ```
pub struct ActorInspectorPlugin<F> {
    condition: Mutex<Option<BoxedCondition>>,
    marker: PhantomData<fn() -> F>,
}

impl<F> Default for ActorInspectorPlugin<F> {
    fn default() -> Self {
        Self {
            condition: Mutex::new(None),
            marker: PhantomData,
        }
    }
}
impl<A> ActorInspectorPlugin<A> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Only show the UI of the specified condition is active
    pub fn run_if<M>(mut self, condition: impl Condition<M>) -> Self {
        let condition_system = IntoSystem::into_system(condition);
        self.condition = Mutex::new(Some(Box::new(condition_system) as BoxedCondition));
        self
    }
}

impl<F: 'static> Plugin for ActorInspectorPlugin<F>
where
    F: QueryFilter,
{
    fn build(&self, app: &mut App) {
        /*check_plugins(app, "FilterQueryInspectorPlugin");

        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }

        let condition: Option<Box<dyn ReadOnlySystem<In = (), Out = bool>>> =
            self.condition.lock().unwrap().take();
        let mut system = entity_query_ui::<F>.into_configs();
        if let Some(condition) = condition {
            system.run_if_dyn(condition);
        }
        app.add_systems(EguiPrimaryContextPass, system);*/

        app.add_systems(Startup, setup_debug_overlay);
        app.add_systems(
            Update,
            explore_actors_system.run_if(on_timer(Duration::from_millis(32))),
        );
    }
}

const DEFAULT_SIZE: (f32, f32) = (320., 800.);

fn entity_query_ui<F: QueryFilter>(world: &mut World) {
    let egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world);

    let Ok(egui_context) = egui_context else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("MyWindow") //pretty_type_name::<F>())
        .default_size(DEFAULT_SIZE)
        .anchor(Align2::RIGHT_TOP, (-5.0, 5.0))
        .pivot(Align2::RIGHT_TOP)
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui_for_entities_expanded_filtered(world, ui, &Filter::<F>::all());
                ui.allocate_space(ui.available_size());
            });
        });
}

fn check_plugins(app: &App, name: &str) {
    if !app.is_plugin_added::<MainSchedulePlugin>() {
        panic!(
            r#"`{name}` should be added after the default plugins:
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins({name}::default())
            "#,
        );
    }

    if !app.is_plugin_added::<EguiPlugin>() {
        panic!(
            r#"`{name}` needs to be added after `EguiPlugin`:
        .add_plugins(EguiPlugin::default())
        .add_plugins({name}::default())
            "#,
        );
    }
}

pub fn pretty_type_name<T>() -> String {
    format!("{:?}", disqualified::ShortName::of::<T>())
}
