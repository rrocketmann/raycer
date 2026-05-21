use bevy::prelude::*;
use avian3d::prelude::*;
use bevy::time::Fixed;

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
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 120.0))
        .insert_resource(SubstepCount(12))
        .insert_resource(Gravity(Vec3::NEG_Y * 9.81))
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins((car::CarPlugin, track::TrackPlugin, ui::UiPlugin))
        .run();
}