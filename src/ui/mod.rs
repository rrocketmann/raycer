use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::Telemetry;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, egui_panel);
    }
}

fn egui_panel(mut contexts: EguiContexts, telemetry: Res<Telemetry>) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    egui::SidePanel::right("telemetry")
        .min_width(220.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);

            ui.heading("Speed");
            ui.add_space(4.0);
            let speed_kmh = telemetry.speed_history.last().copied().unwrap_or(0.0) * 3.6;
            ui.label(format!("{:.0} km/h", speed_kmh));
            ui.add_space(2.0);
            draw_graph(ui, &telemetry.speed_history, 0.0, 260.0, egui::Color32::from_rgb(100, 200, 255));

            ui.separator();
            ui.heading("Steering");
            ui.add_space(4.0);
            let angle_deg = telemetry.steer_history.last().copied().unwrap_or(0.0).to_degrees();
            let steer_label = if angle_deg.abs() < 2.0 {
                "Center"
            } else if angle_deg > 0.0 {
                "Left"
            } else {
                "Right"
            };
            ui.label(format!("{} ({:.1}\u{00b0})", steer_label, angle_deg));
            ui.add_space(2.0);
            draw_graph(ui, &telemetry.steer_history, -0.8, 0.8, egui::Color32::from_rgb(255, 180, 60));

            ui.separator();
            ui.heading("Yaw");
            ui.add_space(4.0);
            let yaw = telemetry.yaw_rate_history.last().copied().unwrap_or(0.0);
            ui.label(format!("{:.1}\u{00b0}", yaw.to_degrees()));
            ui.add_space(2.0);
            draw_graph(ui, &telemetry.yaw_rate_history, -3.14, 3.14, egui::Color32::from_rgb(120, 255, 120));
        });
}

fn draw_graph(ui: &mut egui::Ui, data: &[f32], min_val: f32, max_val: f32, color: egui::Color32) {
    let (rect, response) = ui.allocate_exact_size(
        egui::Vec2::new(ui.available_width(), 60.0),
        egui::Sense::hover(),
    );
    if !rect.is_positive() || data.len() < 2 {
        return;
    }
    let painter = ui.painter();
    let range = max_val - min_val;

    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 30));

    let zero_y = rect.max.y - ((0.0 - min_val) / range) * rect.height();
    if zero_y > rect.min.y && zero_y < rect.max.y {
        painter.line_segment(
            [egui::pos2(rect.min.x, zero_y), egui::pos2(rect.max.x, zero_y)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
        );
    }

    let points: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.min.x + (i as f32 / (data.len() - 1).max(1) as f32) * rect.width();
            let y = rect.max.y - ((v - min_val) / range) * rect.height();
            egui::pos2(x, y.clamp(rect.min.y, rect.max.y))
        })
        .collect();

    painter.line(points, egui::Stroke::new(1.5, color));

    let _ = response;
}