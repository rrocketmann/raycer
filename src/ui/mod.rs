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

    egui::Area::new("bottom_left_keys".into())
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            let key_w = 32.0;
            let key_h = 32.0;
            let gap = 3.0;
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);

            ui.horizontal(|ui| {
                ui.add_space(key_w + gap);
                draw_key(ui, "W", w, key_w, key_h);
            });
            ui.horizontal(|ui| {
                draw_key(ui, "A", a, key_w, key_h);
                draw_key(ui, "S", s, key_w, key_h);
                draw_key(ui, "D", d, key_w, key_h);
            });
            ui.horizontal(|ui| {
                draw_key(ui, "Shift", shift, key_w * 2.0, key_h);
                draw_key(ui, "Space", sp, key_w * 2.0, key_h);
            });
        });

    egui::Area::new("bottom_right_speed".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::BLACK)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 120, 120)))
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}", speed_kmh as i32))
                                .size(32.0)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("km/h")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                        );
                    });
                });
        });
}

fn draw_key(ui: &mut egui::Ui, label: &str, pressed: bool, width: f32, height: f32) {
    let bg = if pressed {
        egui::Color32::from_rgb(100, 200, 255)
    } else {
        egui::Color32::from_rgb(40, 40, 40)
    };
    let fg = if pressed {
        egui::Color32::BLACK
    } else {
        egui::Color32::from_rgb(220, 220, 220)
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), fg);
}