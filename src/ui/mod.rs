use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{Car, PlayerCar};
use crate::car::Telemetry;
use crate::track::MinimapImage;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, egui_panel);
    }
}

fn egui_panel(
    mut contexts: EguiContexts,
    minimap: Res<MinimapImage>,
    telemetry: Res<Telemetry>,
    car_query: Query<(&Car, &Transform), With<PlayerCar>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let minimap_texture_id = contexts.add_image(bevy_egui::EguiTextureHandle::Strong(minimap.0.clone()));
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let w = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    let a = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let s = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
    let d = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    let sp = keys.pressed(KeyCode::Space);

    let car_yaw = car_query.iter().next().map(|(c, _)| c.yaw).unwrap_or(0.0);

    egui::SidePanel::right("telemetry")
        .min_width(220.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);

            ui.heading("Speed");
            ui.add_space(2.0);
            let speed_kmh = telemetry.speed_history.last().copied().unwrap_or(0.0) * 3.6;
            ui.label(format!("{:.0} km/h", speed_kmh));
            ui.add_space(2.0);
            let (speed_min, speed_max) = telemetry.speed_history.iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            draw_graph(ui, &telemetry.speed_history, speed_min, speed_max, egui::Color32::from_rgb(100, 200, 255));

            ui.separator();
            ui.heading("Steering");
            ui.add_space(2.0);
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
            let (steer_min, steer_max) = telemetry.steer_history.iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            draw_graph(ui, &telemetry.steer_history, steer_min, steer_max, egui::Color32::from_rgb(255, 180, 60));

            ui.separator();
            ui.heading("Heading");
            ui.add_space(2.0);
            draw_compass(ui, car_yaw);

            ui.separator();
            ui.heading("Map");
            ui.add_space(2.0);
            let minimap_size = 150.0;
            ui.image(egui::load::SizedTexture::new(minimap_texture_id, egui::vec2(minimap_size, minimap_size)));

            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                draw_key(ui, "W", w);
                draw_key(ui, "A", a);
                draw_key(ui, "S", s);
                draw_key(ui, "D", d);
                ui.add_space(4.0);
                draw_key(ui, "\u{2423}", sp);
            });
        });
}

fn draw_key(ui: &mut egui::Ui, label: &str, pressed: bool) {
    let bg = if pressed {
        egui::Color32::from_rgb(100, 200, 255)
    } else {
        egui::Color32::from_rgb(50, 50, 50)
    };
    let fg = if pressed {
        egui::Color32::BLACK
    } else {
        egui::Color32::from_rgb(200, 200, 200)
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), fg);
}

fn draw_compass(ui: &mut egui::Ui, yaw: f32) {
    let size = 80.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let radius = size * 0.4;

    painter.circle_filled(center, radius, egui::Color32::from_rgb(30, 30, 30));
    painter.circle_stroke(center, radius, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)));

    let dirs = [("N", -std::f32::consts::FRAC_PI_2), ("E", 0.0), ("S", std::f32::consts::FRAC_PI_2), ("W", std::f32::consts::PI)];
    for (label, angle) in dirs {
        let pos = center + radius * egui::Vec2::new(angle.cos(), angle.sin()) * 0.78;
        painter.text(pos, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(10.0), egui::Color32::from_rgb(160, 160, 160));
    }

    let arrow_len = radius * 0.65;
    let tip = center + arrow_len * egui::Vec2::new(yaw.cos(), yaw.sin());
    let back = center - arrow_len * 0.3 * egui::Vec2::new(yaw.cos(), yaw.sin());
    painter.line_segment([tip, back], egui::Stroke::new(2.5, egui::Color32::from_rgb(120, 255, 120)));

    let perp = egui::Vec2::new(-yaw.sin(), yaw.cos());
    let left = center + arrow_len * 0.15 * perp;
    let right = center - arrow_len * 0.15 * perp;
    painter.line_segment([tip, left], egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 255, 120)));
    painter.line_segment([tip, right], egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 255, 120)));
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

    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

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
            let x = rect.min.x + (i as f32 / (data.len() - 1) as f32) * rect.width();
            let y = rect.max.y - ((v - min_val) / range) * rect.height();
            egui::pos2(x, y.clamp(rect.min.y, rect.max.y))
        })
        .collect();

    if !points.is_empty() {
        for p in &points {
            painter.circle_filled(*p, 1.5, color);
        }
    }

    let _ = response;
}