use bevy::prelude::*;
use avian3d::prelude::Rotation;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{CarState, PlayerCar};
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
    _car_state: Res<CarState>,
    _car_query: Query<&Rotation, With<PlayerCar>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let w = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    let a = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let s = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
    let d = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    let sp = keys.pressed(KeyCode::Space);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    let speed_kmh = telemetry.speed_history.last().copied().unwrap_or(0.0) * 3.6;
    let turn_deg = telemetry.steer_history.last().copied().unwrap_or(0.0).to_degrees();

    egui::Area::new("bottom_left_keys".into())
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            let key_w = 30.0;
            let gap = 4.0;
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);

            ui.horizontal(|ui| {
                draw_key(ui, "", false, key_w);
                draw_key(ui, "W", w, key_w);
                draw_key(ui, "", false, key_w);
            });
            ui.horizontal(|ui| {
                draw_key(ui, "A", a, key_w);
                draw_key(ui, "S", s, key_w);
                draw_key(ui, "D", d, key_w);
            });
            ui.horizontal(|ui| {
                draw_key(ui, "Shift", shift, key_w * 2.0);
                draw_key(ui, "Space", sp, key_w * 2.0);
            });
        });

    egui::Area::new("bottom_right_stats".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(egui::Color32::from_black_alpha(140))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.0}", speed_kmh))
                                .size(20.0)
                                .color(egui::Color32::from_rgb(100, 200, 255))
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("km/h")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(140, 140, 140)),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("{:.0}°", turn_deg.abs()))
                                .size(20.0)
                                .color(egui::Color32::from_rgb(255, 180, 60))
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(if turn_deg > 2.0 { "LEFT" } else if turn_deg < -2.0 { "RIGHT" } else { "CENTER" })
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 140)),
                        );
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
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, bg);
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0), fg);
}