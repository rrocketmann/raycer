use bevy::prelude::*;
use avian3d::prelude::*;

mod car;
mod track;
mod ui;

fn main() {
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "raycer".into(),
            canvas: Some("#raycer".into()),
            ..default()
        }),
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins((car::CarPlugin, track::TrackPlugin, ui::UiPlugin))
        .run();
}