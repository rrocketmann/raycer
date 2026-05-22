use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{CarInput, CarState, Telemetry};
use crate::track::MinimapImage;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, egui_panel);
    }
}

fn egui_panel(
    mut contexts: EguiContexts,
    telemetry: Res<Telemetry>,
    car_input: Res<CarInput>,
    car_state: Res<CarState>,
    minimap_image: Res<MinimapImage>,
) {
    let minimap_tex = {
        contexts.add_image(bevy_egui::EguiTextureHandle::Strong(minimap_image.0.clone()));
        contexts.image_id(&minimap_image.0)
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let boosting = car_input.boosting;
    let braking = car_input.braking;
    let throttle = car_input.throttle;
    let steer = car_input.steer;

    egui::SidePanel::right("telemetry")
        .min_width(220.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);

            ui.heading("Speed");
            ui.add_space(2.0);
            let speed_kmh = telemetry.speed_history.last().copied().unwrap_or(0.0) * 3.6;
            ui.label(format!("{:.0} km/h", speed_kmh));
            if boosting {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "BOOST");
            }
            if car_state.skidding {
                ui.colored_label(egui::Color32::from_rgb(255, 200, 50), "SKID");
            }
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
            ui.heading("Map");
            ui.add_space(2.0);

            let size = 180.0;
            if let Some(texture_id) = minimap_tex {
                ui.image(egui::load::SizedTexture::new(texture_id, egui::vec2(size, size)));
            }

            ui.separator();
            ui.add_space(8.0);
            let key_width = 28.0;
            let key_spacing = 4.0;
            let shift_width = key_width * 2.0;
            let space_width = shift_width * 2.0;
            let row_indent = 8.0;
            let w_indent = row_indent + key_width + key_spacing;

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(key_spacing, key_spacing);
                ui.horizontal(|ui| {
                    ui.add_space(w_indent);
                    draw_key(ui, "W", throttle > 0.0, key_width);
                });
                ui.horizontal(|ui| {
                    ui.add_space(row_indent);
                    draw_key(ui, "A", steer > 0.0, key_width);
                    draw_key(ui, "S", throttle < 0.0, key_width);
                    draw_key(ui, "D", steer < 0.0, key_width);
                });
                ui.horizontal(|ui| {
                    draw_key(ui, "Shift", boosting, shift_width);
                });
                ui.horizontal(|ui| {
                    draw_key(ui, "Space", braking, space_width);
                });
            });
        });
}

fn draw_key(ui: &mut egui::Ui, label: &str, pressed: bool, width: f32) {
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
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), fg);
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
