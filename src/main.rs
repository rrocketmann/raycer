mod car;
mod gym;
mod physics;
mod telemetry;
mod track;
mod ui;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            car::CarPlugin,
            track::TrackPlugin,
            physics::PhysicsPlugin,
            gym::GymPlugin,
            ui::UiPlugin,
            telemetry::TelemetryPlugin,
        ))
        .run();
}
