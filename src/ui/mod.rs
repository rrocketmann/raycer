use bevy::prelude::*;
use avian3d::prelude::Rotation;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{CarSelection, CarState, CAR_DEFS, PlayerCar};
use crate::car::Telemetry;
use crate::blaster::{BlasterSelection, BLASTER_DEFS};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarDropdownOpen>()
            .init_resource::<BlasterDropdownOpen>()
            .add_systems(EguiPrimaryContextPass, egui_panel);
    }
}

#[derive(Resource, Default)]
struct CarDropdownOpen(bool);

#[derive(Resource, Default)]
struct BlasterDropdownOpen(bool);

fn egui_panel(
    mut contexts: EguiContexts,
    telemetry: Res<Telemetry>,
    _car_state: Res<CarState>,
    mut car_selection: ResMut<CarSelection>,
    mut dropdown: ResMut<CarDropdownOpen>,
    mut blaster_selection: ResMut<BlasterSelection>,
    mut blaster_dropdown: ResMut<BlasterDropdownOpen>,
    _car_query: Query<&Rotation, With<PlayerCar>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let w = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    let a = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let s_pressed = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
    let d = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    let sp = keys.pressed(KeyCode::Space);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let q = keys.pressed(KeyCode::KeyQ);
    let e = keys.pressed(KeyCode::KeyE);

    let speed_ms = telemetry.speed_history.last().copied().unwrap_or(0.0);

    let key_w = 28.0;
    let key_h = 28.0;
    let gap = 4.0;

    egui::Area::new("bottom_left_keys".into())
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
            ui.horizontal(|ui| {
                draw_key(ui, "Q", q, key_w, key_h);
                draw_key(ui, "W", w, key_w, key_h);
                draw_key(ui, "E", e, key_w, key_h);
            });
            ui.horizontal(|ui| {
                draw_key(ui, "A", a, key_w, key_h);
                draw_key(ui, "S", s_pressed, key_w, key_h);
                draw_key(ui, "D", d, key_w, key_h);
            });
            ui.horizontal(|ui| {
                draw_key(ui, "Shift", shift, key_w * 2.0, key_h);
                draw_key(ui, "Space", sp, key_w * 2.0, key_h);
            });
        });

    let current_name = CAR_DEFS[car_selection.index].name;
    let car_selector_width = 130.0;
    let item_height = 20.0;
    let max_visible = 8;
    let total = CAR_DEFS.len() as f32;

    let dropdown_height = if dropdown.0 {
        (total * item_height).min(max_visible as f32 * item_height)
    } else {
        0.0
    };

    egui::Area::new("car_selector".into())
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0 - dropdown_height))
        .show(ctx, |ui| {
            let btn_text = current_name.to_string();

            let btn_resp = ui.add_sized(
                [car_selector_width, 22.0],
                egui::Button::new(
                    egui::RichText::new(btn_text).size(12.0).color(egui::Color32::from_rgb(220, 220, 220)),
                ),
            );

            if btn_resp.clicked() {
                dropdown.0 = !dropdown.0;
            }
        });

    if dropdown.0 {
        egui::Area::new("car_dropdown".into())
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0 - 22.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(max_visible as f32 * item_height)
                    .id_salt("car_list_scroll")
                    .show(ui, |ui| {
                        ui.set_width(car_selector_width);
                        for (i, def) in CAR_DEFS.iter().enumerate() {
                            let selected = i == car_selection.index;
                            let (bg, fg) = if selected {
                                (egui::Color32::from_rgb(100, 200, 255), egui::Color32::BLACK)
                            } else {
                                (egui::Color32::from_rgb(35, 35, 35), egui::Color32::from_rgb(200, 200, 200))
                            };
                            let item_resp = ui.add_sized(
                                [car_selector_width, item_height],
                                egui::Button::new(
                                    egui::RichText::new(def.name).size(12.0).color(fg),
                                ).fill(bg).stroke(egui::Stroke::NONE),
                            );
                            if item_resp.clicked() && i != car_selection.index {
                                car_selection.index = i;
                                car_selection.pending_change = true;
                                dropdown.0 = false;
                            }
                        }
                    });
            });
    }

    let blaster_name = BLASTER_DEFS[blaster_selection.index].name;
    let blaster_select_width = 130.0;
    let blaster_item_h = 20.0;
    let blaster_total = BLASTER_DEFS.len() as f32;
    let blaster_drop_h = if blaster_dropdown.0 {
        (blaster_total * blaster_item_h).min(8.0 * blaster_item_h)
    } else {
        0.0
    };

    let blaster_x = 136.0;

    egui::Area::new("blaster_selector".into())
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(blaster_x, -12.0 - blaster_drop_h))
        .show(ctx, |ui| {
            let b_resp = ui.add_sized(
                [blaster_select_width, 22.0],
                egui::Button::new(
                    egui::RichText::new(blaster_name).size(12.0).color(egui::Color32::from_rgb(220, 220, 220)),
                ),
            );
            if b_resp.clicked() {
                blaster_dropdown.0 = !blaster_dropdown.0;
            }
        });

    if blaster_dropdown.0 {
        egui::Area::new("blaster_dropdown".into())
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(blaster_x, -12.0 - 22.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(8.0 * blaster_item_h)
                    .id_salt("blaster_list_scroll")
                    .show(ui, |ui| {
                        ui.set_width(blaster_select_width);
                        for (i, def) in BLASTER_DEFS.iter().enumerate() {
                            let selected = i == blaster_selection.index;
                            let (bg, fg) = if selected {
                                (egui::Color32::from_rgb(100, 200, 255), egui::Color32::BLACK)
                            } else {
                                (egui::Color32::from_rgb(35, 35, 35), egui::Color32::from_rgb(200, 200, 200))
                            };
                            let item_resp = ui.add_sized(
                                [blaster_select_width, blaster_item_h],
                                egui::Button::new(
                                    egui::RichText::new(def.name).size(12.0).color(fg),
                                ).fill(bg).stroke(egui::Stroke::NONE),
                            );
                            if item_resp.clicked() && i != blaster_selection.index {
                                blaster_selection.index = i;
                                blaster_selection.pending_change = true;
                                blaster_dropdown.0 = false;
                            }
                        }
                    });
            });
    }

    egui::Area::new("bottom_right_speed".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(40, 40, 40))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 120, 120)))
                .corner_radius(2.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}", speed_ms as i32))
                                .size(14.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                        );
                        ui.label(
                            egui::RichText::new("m/s")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                        );
                    });
                });
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        dropdown.0 = false;
        blaster_dropdown.0 = false;
    }
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
    let radius = 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, radius, bg);
    painter.rect_stroke(rect, radius, egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), fg);
}