use bevy::prelude::*;

mod car;
mod gym;
mod track;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "raycer".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((car::CarPlugin, track::TrackPlugin, gym::GymPlugin, ui::UiPlugin))
        .run();
}