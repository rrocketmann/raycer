use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{Car, PlayerCar, MAP_HALF_SIZE};
use crate::car::Telemetry;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, egui_panel);
    }
}

fn egui_panel(
    mut contexts: EguiContexts,
    telemetry: Res<Telemetry>,
    car_query: Query<(&Car, &Transform), With<PlayerCar>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let w = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    let a = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let s = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
    let d = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    let sp = keys.pressed(KeyCode::Space);

    let car_x = car_query.iter().next().map(|(_, t)| t.translation.x).unwrap_or(0.0);
    let car_z = car_query.iter().next().map(|(_, t)| t.translation.z).unwrap_or(0.0);
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
            draw_graph(ui, &telemetry.speed_history, 0.0, 260.0, egui::Color32::from_rgb(100, 200, 255));

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
            draw_graph(ui, &telemetry.steer_history, -0.8, 0.8, egui::Color32::from_rgb(255, 180, 60));

            ui.separator();
            ui.heading("Heading");
            ui.add_space(2.0);
            draw_compass(ui, car_yaw);

            ui.separator();
            ui.heading("Map");
            ui.add_space(2.0);
            draw_minimap(ui, car_x, car_z, car_yaw);

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

fn draw_minimap(ui: &mut egui::Ui, car_x: f32, car_z: f32, car_yaw: f32) {
    let size = 150.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(45, 42, 35));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 75, 65)), egui::StrokeKind::Outside);

    let scale = size / (MAP_HALF_SIZE * 2.2);
    let cx = rect.center().x;
    let cy = rect.center().y;

    // Arena circle offset from car position (car is always at center)
    let arena_cx = cx - car_x * scale;
    let arena_cy = cy + car_z * scale;
    let border_r = MAP_HALF_SIZE * scale;
    painter.circle_stroke(egui::pos2(arena_cx, arena_cy), border_r, egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 80, 65)));

    // Car always at center, pointing up (yaw rotated so forward = up)
    let arrow_len = 10.0;
    let half_w = 4.0;
    let cos_y = car_yaw.cos();
    let sin_y = car_yaw.sin();
    let tip = egui::pos2(cx + arrow_len * cos_y, cy - arrow_len * sin_y);
    let bl = egui::pos2(cx - arrow_len * 0.5 * cos_y + half_w * sin_y, cy + arrow_len * 0.5 * sin_y + half_w * cos_y);
    let br = egui::pos2(cx - arrow_len * 0.5 * cos_y - half_w * sin_y, cy + arrow_len * 0.5 * sin_y - half_w * cos_y);
    painter.line_segment([tip, bl], egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 60)));
    painter.line_segment([tip, br], egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 60)));
    painter.line_segment([bl, br], egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 200, 60)));

    let _rect = rect;
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

    let points = smooth_path(data, rect, min_val, range);

    if points.len() >= 2 {
        let stroke = egui::Stroke::new(1.5, color);
        painter.add(egui::Shape::Path(egui::epaint::PathShape {
            points,
            closed: false,
            fill: egui::Color32::TRANSPARENT,
            stroke: stroke.into(),
        }));
    }

    let _ = response;
}

fn smooth_path(data: &[f32], rect: egui::Rect, min_val: f32, range: f32) -> Vec<egui::Pos2> {
    if data.len() < 2 {
        return Vec::new();
    }
    let raw: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.min.x + (i as f32 / (data.len() - 1) as f32) * rect.width();
            let y = rect.max.y - ((v - min_val) / range) * rect.height();
            egui::pos2(x, y.clamp(rect.min.y, rect.max.y))
        })
        .collect();

    catmull_rom(&raw, 4)
}

fn catmull_rom(points: &[egui::Pos2], subdivisions: usize) -> Vec<egui::Pos2> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(points.len() * subdivisions);
    for i in 0..points.len() - 1 {
        let p0 = if i == 0 { points[0] } else { points[i - 1] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() { points[i + 2] } else { p2 };
        for j in 0..subdivisions {
            let t = j as f32 / subdivisions as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let x = 0.5 * ((2.0 * p1.x)
                + (-p0.x + p2.x) * t
                + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
                + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3);
            let y = 0.5 * ((2.0 * p1.y)
                + (-p0.y + p2.y) * t
                + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
                + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3);
            result.push(egui::pos2(x, y));
        }
    }
    result.push(*points.last().unwrap());
    result
}