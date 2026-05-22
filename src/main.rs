use bevy::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
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
        .insert_resource(Gravity(Vec3::NEG_Y * 15.0))
        .insert_resource(SubstepCount(12))
        .insert_resource(SolverConfig {
            contact_damping_ratio: 15.0,
            max_overlap_solve_speed: 8.0,
            ..default()
        })
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins((car::CarPlugin, track::TrackPlugin, ui::UiPlugin))
        .run();
}
