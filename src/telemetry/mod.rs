use bevy::prelude::*;
use serde::Serialize;
use std::fs::File;
use std::io::Write;

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TelemetryRecorder>()
            .add_systems(Update, record_telemetry);
    }
}

#[derive(Resource, Default)]
pub struct TelemetryRecorder {
    pub recording: bool,
    pub data: Vec<TelemetryFrame>,
    pub file_path: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct TelemetryFrame {
    pub step: usize,
    pub episode: usize,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub speed: f32,
    pub reward: f32,
    pub action: [f32; 3],
}

pub fn record_telemetry(
    mut recorder: ResMut<TelemetryRecorder>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        recorder.recording = !recorder.recording;
        if recorder.recording {
            recorder.data.clear();
            println!("Telemetry recording started");
        } else {
            println!("Telemetry recording stopped");
        }
    }

    if keys.just_pressed(KeyCode::F2) {
        if let Some(path) = &recorder.file_path {
            save_telemetry(&recorder.data, path);
        } else {
            save_telemetry(&recorder.data, "telemetry.json");
        }
    }
}

fn save_telemetry(data: &[TelemetryFrame], path: &str) {
    if let Ok(mut file) = File::create(path) {
        for frame in data {
            if let Ok(json) = serde_json::to_string(frame) {
                let _ = writeln!(file, "{}", json);
            }
        }
        println!("Saved {} frames to {}", data.len(), path);
    }
}
