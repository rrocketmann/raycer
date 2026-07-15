use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{CarSelection, Telemetry, CAR_DEFS};
use crate::blaster::{BlasterSelection, BLASTER_DEFS};
use crate::GameState;
use crate::AiEnemyCount;
use crate::RubberBullets;
use crate::MaxHealthPoints;
use crate::GameOutcome;
use crate::PendingState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, egui_panel);
    }
}

fn egui_panel(
    mut contexts: EguiContexts,
    telemetry: Res<Telemetry>,
    mut car_selection: ResMut<CarSelection>,
    mut blaster_selection: ResMut<BlasterSelection>,
    keys: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    mut ai_enemy_count: ResMut<AiEnemyCount>,
    mut rubber_bullets: ResMut<RubberBullets>,
    mut max_hp: ResMut<MaxHealthPoints>,
    outcome: Res<GameOutcome>,
    mut pending: ResMut<PendingState>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    match game_state.get() {
        GameState::Loading => {}
        GameState::PreGame => {
            pre_game_ui(ctx, &mut car_selection, &mut blaster_selection, &mut ai_enemy_count, &mut rubber_bullets, &mut max_hp, &mut *pending);
        }
        GameState::Playing => {
            playing_ui(ctx, &telemetry, &keys);
        }
        GameState::Eliminated => {
            death_ui(ctx, &outcome, &mut *pending);
        }
    }
}

fn name_box(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgba_unmultiplied(35, 35, 35, 180));
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(70, 70, 70, 180)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(14.0), egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220));
}

fn pre_game_ui(
    ctx: &egui::Context,
    car_selection: &mut CarSelection,
    blaster_selection: &mut BlasterSelection,
    ai_enemy_count: &mut AiEnemyCount,
    rubber_bullets: &mut RubberBullets,
    max_hp: &mut MaxHealthPoints,
    pending: &mut PendingState,
) {
    let panel_w = 260.0;
    let btn_size = 32.0;

    egui::CentralPanel::default()
        .frame(egui::Frame::new())
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.label(egui::RichText::new("R A Y C E R").size(56.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200)).strong());
                ui.add_space(60.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(panel_w, 330.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        // Car row
                        ui.label(egui::RichText::new("CAR").size(11.0).color(egui::Color32::from_rgba_unmultiplied(130, 130, 130, 180)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space((panel_w - btn_size * 2.0 - 100.0) / 2.0);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new("<").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if car_selection.random {
                                    car_selection.random = false;
                                    car_selection.index = CAR_DEFS.len() - 1;
                                } else if car_selection.index == 0 {
                                    car_selection.random = true;
                                } else {
                                    car_selection.index -= 1;
                                }
                                car_selection.pending_change = true;
                            }
                            name_box(ui, if car_selection.random { "RANDOM" } else { CAR_DEFS[car_selection.index].name });
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new(">").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if car_selection.random {
                                    car_selection.random = false;
                                    car_selection.index = 0;
                                } else if car_selection.index == CAR_DEFS.len() - 1 {
                                    car_selection.random = true;
                                } else {
                                    car_selection.index += 1;
                                }
                                car_selection.pending_change = true;
                            }
                        });

                        ui.add_space(20.0);

                        // Blaster row
                        ui.label(egui::RichText::new("BLASTER").size(11.0).color(egui::Color32::from_rgba_unmultiplied(130, 130, 130, 180)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space((panel_w - btn_size * 2.0 - 100.0) / 2.0);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new("<").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if blaster_selection.random {
                                    blaster_selection.random = false;
                                    blaster_selection.index = BLASTER_DEFS.len() - 1;
                                } else if blaster_selection.index == 0 {
                                    blaster_selection.random = true;
                                } else {
                                    blaster_selection.index -= 1;
                                }
                                blaster_selection.pending_change = true;
                            }
                            name_box(ui, if blaster_selection.random { "RANDOM" } else { BLASTER_DEFS[blaster_selection.index].name });
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new(">").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if blaster_selection.random {
                                    blaster_selection.random = false;
                                    blaster_selection.index = 0;
                                } else if blaster_selection.index == BLASTER_DEFS.len() - 1 {
                                    blaster_selection.random = true;
                                } else {
                                    blaster_selection.index += 1;
                                }
                                blaster_selection.pending_change = true;
                            }
                        });

                        ui.add_space(20.0);

                        // Opponents row
                        ui.label(egui::RichText::new("OPPONENTS").size(11.0).color(egui::Color32::from_rgba_unmultiplied(130, 130, 130, 180)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space((panel_w - btn_size * 2.0 - 100.0) / 2.0);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new("<").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if ai_enemy_count.random {
                                    ai_enemy_count.random = false;
                                    ai_enemy_count.count = 10;
                                } else if ai_enemy_count.count == 0 {
                                    ai_enemy_count.random = true;
                                } else {
                                    ai_enemy_count.count -= 1;
                                }
                            }
                            let opp_label = if ai_enemy_count.random { "RANDOM".to_string() } else { format!("{}", ai_enemy_count.count) };
                            name_box(ui, &opp_label);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new(">").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if ai_enemy_count.random {
                                    ai_enemy_count.random = false;
                                    ai_enemy_count.count = 0;
                                } else if ai_enemy_count.count == 10 {
                                    ai_enemy_count.random = true;
                                } else {
                                    ai_enemy_count.count += 1;
                                }
                            }
                        });

                        ui.add_space(20.0);

                        // Health Points row
                        ui.label(egui::RichText::new("HEALTH POINTS").size(11.0).color(egui::Color32::from_rgba_unmultiplied(130, 130, 130, 180)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space((panel_w - btn_size * 2.0 - 100.0) / 2.0);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new("<").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if max_hp.random {
                                    max_hp.random = false;
                                    max_hp.hp = 10;
                                } else if max_hp.hp == 2 {
                                    max_hp.random = true;
                                } else {
                                    max_hp.hp -= 1;
                                }
                            }
                            let hp_label = if max_hp.random { "RANDOM".to_string() } else { format!("{}", max_hp.hp) };
                            name_box(ui, &hp_label);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new(">").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if max_hp.random {
                                    max_hp.random = false;
                                    max_hp.hp = 2;
                                } else if max_hp.hp == 10 {
                                    max_hp.random = true;
                                } else {
                                    max_hp.hp += 1;
                                }
                            }
                        });

                        ui.add_space(20.0);

                        // Rubber Bullets row
                        ui.label(egui::RichText::new("RUBBER BULLETS").size(11.0).color(egui::Color32::from_rgba_unmultiplied(130, 130, 130, 180)));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space((panel_w - btn_size * 2.0 - 100.0) / 2.0);
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new("<").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if rubber_bullets.random {
                                    rubber_bullets.random = false;
                                    rubber_bullets.enabled = true;
                                } else if rubber_bullets.enabled {
                                    rubber_bullets.enabled = false;
                                } else {
                                    rubber_bullets.random = true;
                                }
                            }
                            name_box(ui, if rubber_bullets.random { "RANDOM" } else if rubber_bullets.enabled { "ON" } else { "OFF" });
                            if ui.add_sized([btn_size, btn_size], egui::Button::new(
                                egui::RichText::new(">").size(16.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                            ).fill(egui::Color32::from_rgba_unmultiplied(50, 50, 50, 180))).clicked() {
                                if rubber_bullets.random {
                                    rubber_bullets.random = false;
                                    rubber_bullets.enabled = false;
                                } else if rubber_bullets.enabled {
                                    rubber_bullets.random = true;
                                } else {
                                    rubber_bullets.enabled = true;
                                }
                            }
                        });
                    },
                );

                ui.add_space(36.0);

                // Start button
                let start_resp = ui.add_sized(
                    [panel_w, 42.0],
                    egui::Button::new(
                        egui::RichText::new("START").size(18.0).strong().color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                    ).fill(egui::Color32::from_rgba_unmultiplied(80, 80, 80, 180)),
                );
                if start_resp.clicked() {
                    pending.0 = Some(GameState::Playing);
                }
            });
        });
}

fn death_ui(
    ctx: &egui::Context,
    outcome: &GameOutcome,
    pending: &mut PendingState,
) {
    let (title, subtitle) = if outcome.0 {
        ("VICTORY", "All enemies eliminated")
    } else {
        ("TERMINATED", "Your car was destroyed")
    };
    let text_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200);

    egui::CentralPanel::default()
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_unmultiplied(20, 20, 20, 255),
            ..default()
        })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(220.0);
                ui.label(egui::RichText::new(title).size(36.0).color(text_color).weak());
                ui.add_space(28.0);
                ui.label(egui::RichText::new(subtitle).size(14.0).color(text_color).strong());
                ui.add_space(48.0);

                let btn = ui.add_sized(
                    [160.0, 36.0],
                    egui::Button::new(
                        egui::RichText::new("RESTART").size(14.0).color(text_color),
                    ).fill(egui::Color32::from_rgba_unmultiplied(60, 60, 60, 160)),
                );
                if btn.clicked() {
                    pending.0 = Some(GameState::PreGame);
                }
            });
        });
}
fn playing_ui(
    ctx: &egui::Context,
    telemetry: &Telemetry,
    keys: &ButtonInput<KeyCode>,
) {
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

    egui::Area::new("bottom_right_speed".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(60, 60, 60, 160))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(120, 120, 120, 120)))
                .corner_radius(2.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}", speed_ms as i32))
                                .size(14.0)
                                .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
                        );
                        ui.label(
                            egui::RichText::new("m/s")
                                .size(14.0)
                                .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
                        );
                    });
                });
        });
}

fn draw_key(ui: &mut egui::Ui, label: &str, pressed: bool, width: f32, height: f32) {
    let bg = if pressed {
        egui::Color32::from_rgba_unmultiplied(100, 200, 255, 160)
    } else {
        egui::Color32::from_rgba_unmultiplied(60, 60, 60, 160)
    };
    let fg = if pressed {
        egui::Color32::BLACK
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)
    };
    let radius = 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, radius, bg);
    painter.rect_stroke(rect, radius, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(100, 100, 100, 120)), egui::StrokeKind::Outside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), fg);
}
