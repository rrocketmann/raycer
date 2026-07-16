use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::car::{CarSelection, Telemetry, CAR_DEFS};
use crate::blaster::{BlasterSelection, WeaponCharge, BLASTER_DEFS};
use crate::{GameState, AiEnemyCount, MaxHealthPoints, GameOutcome, PendingState, NetMode, PlayerName, PendingConnect, PendingHost, RoundCountdown};
use crate::net::client::{DiscoveredServers, LobbyData, GameClient};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, (
            pregame_ui_system.run_if(in_state(GameState::PreGame)),
            lobby_ui_system.run_if(in_state(GameState::MultiplayerLobby)),
            playing_ui_system.run_if(in_state(GameState::Playing)),
            death_ui_system.run_if(in_state(GameState::Eliminated)),
        ));
    }
}

fn pregame_ui_system(
    mut contexts: EguiContexts,
    mut car_selection: ResMut<CarSelection>,
    mut blaster_selection: ResMut<BlasterSelection>,
    mut ai_enemy_count: ResMut<AiEnemyCount>,
    mut max_hp: ResMut<MaxHealthPoints>,
    mut pending: ResMut<PendingState>,
    mut pending_host: ResMut<PendingHost>,
    mut discovered: ResMut<DiscoveredServers>,
    mut pending_connect: ResMut<PendingConnect>,
    mut show_popup: Local<bool>,
    mut hosting: Local<bool>,
    player_name: Res<PlayerName>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };
    pre_game_ui(ctx, &mut car_selection, &mut blaster_selection, &mut ai_enemy_count, &mut max_hp, &mut *pending, &mut *pending_host, &mut show_popup, &mut hosting, &mut discovered, &mut *pending_connect, &player_name);
}

fn lobby_ui_system(
    mut contexts: EguiContexts,
    mode: Res<NetMode>,
    name: Res<PlayerName>,
    mut pending: ResMut<PendingState>,
    mut discovered: ResMut<DiscoveredServers>,
    lobby_data: Res<LobbyData>,
    client: Option<Res<GameClient>>,
    mut car_selection: ResMut<CarSelection>,
    mut blaster_selection: ResMut<BlasterSelection>,
    mut ai_enemy_count: ResMut<AiEnemyCount>,
    mut max_hp: ResMut<MaxHealthPoints>,
    mut pending_connect: ResMut<PendingConnect>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };
    multiplayer_lobby_ui(ctx, &mode, &name, &mut *pending, &mut discovered, &lobby_data, client.as_deref(), &mut car_selection, &mut blaster_selection, &mut ai_enemy_count, &mut max_hp, &mut *pending_connect);
}

fn playing_ui_system(
    mut contexts: EguiContexts,
    telemetry: Res<Telemetry>,
    keys: Res<ButtonInput<KeyCode>>,
    charge: Res<WeaponCharge>,
    blaster_selection: Res<BlasterSelection>,
    countdown: Option<Res<crate::RoundCountdown>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::car::CarCamera>>,
    car_query: Query<(&Transform, &crate::car::Health, Option<&crate::OwnerClient>), (With<crate::car::PlayerCar>, Without<crate::car::AiCar>)>,
    ai_query: Query<(&Transform, &crate::car::Health), With<crate::car::AiCar>>,
    remote_query: Query<(&Transform, &crate::OwnerClient), With<crate::RemotePlayer>>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };
    playing_ui(ctx, &telemetry, &keys, &charge, &blaster_selection, countdown.as_deref());

    let Ok((camera, cam_global)) = camera_query.single() else { return };

    egui::Area::new(egui::Id::new("nametags"))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            for (tf, health, _owner) in car_query.iter() {
                draw_nametag(ui, camera, cam_global, tf, health.0);
            }
            for (tf, health) in ai_query.iter() {
                draw_nametag(ui, camera, cam_global, tf, health.0);
            }
            for (tf, _owner) in remote_query.iter() {
                draw_nametag(ui, camera, cam_global, tf, 0);
            }
        });
}

fn draw_nametag(ui: &mut egui::Ui, camera: &Camera, cam_global: &GlobalTransform, tf: &Transform, health: u8) {
    let Ok(pos) = camera.world_to_viewport(cam_global, tf.translation + Vec3::new(0.0, 3.0, 0.0)) else { return };
    let screen_pos = egui::pos2(pos.x, pos.y - 22.0);
    let text = format!("{} HP", health);
    let painter = ui.painter();
    let bw = text.len() as f32 * 7.0 + 12.0;
    let bg_rect = egui::Rect::from_center_size(screen_pos, egui::vec2(bw, 18.0));
    painter.rect_filled(bg_rect, 2.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180));
    painter.rect_stroke(bg_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 200, 50, 200)), egui::StrokeKind::Outside);
    painter.text(screen_pos, egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(12.0), egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
}

fn death_ui_system(mut contexts: EguiContexts, outcome: Res<GameOutcome>, mut pending: ResMut<PendingState>) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };
    death_ui(ctx, &outcome, &mut *pending);
}

// ── high-contrast palette ──
fn bg0() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(0, 0, 0, 210) }
fn bg1() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(20, 20, 20, 220) }
fn bg2() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(50, 50, 50, 220) }
fn gold() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(255, 200, 50, 255) }
fn gold_dim() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(200, 160, 40, 200) }
fn white() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255) }
fn gray() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(190, 190, 190, 220) }
fn border() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(80, 80, 80, 220) }

fn name_box(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, bg2());
    p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border()), egui::StrokeKind::Outside);
    p.text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(14.0), white());
}

fn row_btn(ui: &mut egui::Ui, label: &str, w: f32, h: f32) -> egui::Response {
    ui.add_sized(
        [w, h],
        egui::Button::new(egui::RichText::new(label).size(14.0).color(white()))
            .fill(bg2()).stroke(egui::Stroke::new(1.0, border())).corner_radius(4),
    )
}

fn accent_btn(ui: &mut egui::Ui, label: &str, w: f32, h: f32) -> egui::Response {
    ui.add_sized(
        [w, h],
        egui::Button::new(egui::RichText::new(label).size(h * 0.4).color(egui::Color32::BLACK).strong())
            .fill(gold()).corner_radius(4),
    )
}

// ── PRE-GAME ──
fn pre_game_ui(
    ctx: &egui::Context,
    cs: &mut CarSelection,
    bs: &mut BlasterSelection,
    ai: &mut AiEnemyCount,
    hp: &mut MaxHealthPoints,
    pending: &mut PendingState,
    pending_host: &mut PendingHost,
    show_popup: &mut bool,
    hosting: &mut bool,
    discovered: &mut DiscoveredServers,
    _pending_connect: &mut PendingConnect,
    _player_name: &PlayerName,
) {
    let pw = 260.0;
    let bsz = 32.0;

    egui::CentralPanel::default()
        .frame(egui::Frame { fill: bg0(), ..default() })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(egui::RichText::new("R A Y C E R").size(52.0).color(gold()).strong());
                ui.add_space(2.0);
                ui.add_space(36.0);

                ui.allocate_ui_with_layout(egui::vec2(pw, 280.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                    let g = 6.0;
                    let rw = bsz + g + 100.0 + g + bsz;

                    macro_rules! selrow {
                        ($lbl:expr, $rnd:expr, $nam:expr, $prv:expr, $nxt:expr) => {{
                            ui.label(egui::RichText::new($lbl).size(10.0).color(gray()).strong());
                            ui.add_space(3.0);
                            let dsp = if $rnd { "RANDOM".into() } else { $nam.to_string() };
                            ui.allocate_ui_with_layout(egui::vec2(rw, bsz), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(g, 0.0);
                                if row_btn(ui, "<", bsz, bsz).clicked() { $prv(); }
                                name_box(ui, &dsp);
                                if row_btn(ui, ">", bsz, bsz).clicked() { $nxt(); }
                            });
                            ui.add_space(12.0);
                        }};
                    }

                    selrow!("CAR", cs.random, CAR_DEFS[cs.display_index()].name, {
                        if cs.random { cs.random = false; cs.index = CAR_DEFS.len() - 1; }
                        else if cs.index == 0 { cs.random = true; }
                        else { cs.index -= 1; }
                        cs.pending_change = true;
                    }, {
                        if cs.random { cs.random = false; cs.index = 0; }
                        else if cs.index == CAR_DEFS.len() - 1 { cs.random = true; }
                        else { cs.index += 1; }
                        cs.pending_change = true;
                    });

                    selrow!("BLASTER", bs.random, BLASTER_DEFS[bs.display_index()].name, {
                        if bs.random { bs.random = false; bs.index = BLASTER_DEFS.len() - 1; }
                        else if bs.index == 0 { bs.random = true; }
                        else { bs.index -= 1; }
                        bs.pending_change = true;
                    }, {
                        if bs.random { bs.random = false; bs.index = 0; }
                        else if bs.index == BLASTER_DEFS.len() - 1 { bs.random = true; }
                        else { bs.index += 1; }
                        bs.pending_change = true;
                    });

                    selrow!("OPPONENTS", ai.random, format!("{}", ai.count), {
                        if ai.random { ai.random = false; ai.count = 10; }
                        else if ai.count == 0 { ai.random = true; }
                        else { ai.count -= 1; }
                    }, {
                        if ai.random { ai.random = false; ai.count = 0; }
                        else if ai.count == 10 { ai.random = true; }
                        else { ai.count += 1; }
                    });

                    selrow!("HEALTH", hp.random, format!("{}", hp.hp), {
                        if hp.random { hp.random = false; hp.hp = 10; }
                        else if hp.hp == 2 { hp.random = true; }
                        else { hp.hp -= 1; }
                    }, {
                        if hp.random { hp.random = false; hp.hp = 2; }
                        else if hp.hp == 10 { hp.random = true; }
                        else { hp.hp += 1; }
                    });
                });

                ui.add_space(12.0);

                let g = 6.0;
                let bw = 158.0 + g + 72.0;
                ui.allocate_ui_with_layout(egui::vec2(bw, 42.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(g, 0.0);
                    if accent_btn(ui, "START", 158.0, 42.0).clicked() {
                        pending.0 = Some(GameState::Playing);
                    }
                    if row_btn(ui, "JOIN", 72.0, 42.0).clicked() {
                        *show_popup = true;
                    }
                });
            });
        });

    // ── POPUP ──
    if *show_popup {
        let pp = 340.0;
        egui::Window::new("join_server")
            .title_bar(false)
            .fixed_size([pp, 420.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(egui::Frame { fill: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 235), stroke: egui::Stroke::new(1.0, border()), corner_radius: 6.0.into(), ..default() })
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);

                    if *hosting {
                        ui.label(egui::RichText::new("HOSTING").size(20.0).color(gold()).strong());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("configure your match").size(11.0).color(gray()));
                        ui.add_space(24.0);

                        let hp_ai_w = 30.0 + 6.0 + 26.0 + 6.0 + 22.0 + 6.0 + 26.0; // label + btn + gap + value + gap + btn
                        let sep = 20.0;
                        let total = hp_ai_w + sep + hp_ai_w;
                        ui.allocate_ui_with_layout(egui::vec2(total, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                            ui.label(egui::RichText::new("HP").size(12.0).color(gray()).strong());
                            ui.add_space(6.0);
                            if row_btn(ui, "\u{2212}", 26.0, 26.0).clicked() { hp.hp = hp.hp.saturating_sub(1).max(1); }
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(format!("{:>2}", hp.hp)).size(16.0).color(white()));
                            ui.add_space(6.0);
                            if row_btn(ui, "+", 26.0, 26.0).clicked() { hp.hp = hp.hp.saturating_add(1).min(20); }
                            ui.add_space(sep);
                            ui.label(egui::RichText::new("AI").size(12.0).color(gray()).strong());
                            ui.add_space(6.0);
                            if row_btn(ui, "\u{2212}", 26.0, 26.0).clicked() { ai.count = ai.count.saturating_sub(1); }
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(format!("{:>2}", ai.count)).size(16.0).color(white()));
                            ui.add_space(6.0);
                            if row_btn(ui, "+", 26.0, 26.0).clicked() { ai.count = ai.count.saturating_add(1).min(20); }
                        });

                        ui.add_space(24.0);
                        if accent_btn(ui, "START GAME", pp - 60.0, 40.0).clicked() {
                            *show_popup = false;
                            *hosting = false;
                            pending.0 = Some(GameState::Playing);
                        }
                        ui.add_space(10.0);
                        if row_btn(ui, "STOP HOSTING", 140.0, 28.0).clicked() {
                            *hosting = false;
                        }
                    } else {
                        ui.label(egui::RichText::new("MULTIPLAYER").size(18.0).color(gold()).strong());
                        ui.add_space(20.0);
                        if accent_btn(ui, "HOST NEW GAME", pp - 60.0, 38.0).clicked() {
                            pending_host.0 = true;
                            *hosting = true;
                        }
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for srv in discovered.0.iter() {
                                let label = format!("{}  |  {}p", srv.adv.name, srv.adv.player_count);
                                if row_btn(ui, &label, pp - 60.0, 30.0).clicked() {
                                    _pending_connect.0 = Some(srv.addr);
                                    pending.0 = Some(GameState::MultiplayerLobby);
                                    *show_popup = false;
                                }
                            }
                        });
                        ui.add_space(16.0);
                        if row_btn(ui, "CLOSE", 100.0, 28.0).clicked() {
                            *show_popup = false;
                        }
                    }
                    ui.add_space(8.0);
                });
            });
    }
}

// ── LOBBY ──
fn multiplayer_lobby_ui(
    ctx: &egui::Context,
    mode: &NetMode,
    _name: &PlayerName,
    pending: &mut PendingState,
    _discovered: &mut DiscoveredServers,
    _lobby: &LobbyData,
    client: Option<&GameClient>,
    _car_selection: &mut CarSelection,
    _blaster_selection: &mut BlasterSelection,
    ai_enemy_count: &mut AiEnemyCount,
    max_hp: &mut MaxHealthPoints,
    _pending_connect: &mut PendingConnect,
) {
    let pw = 240.0;

    egui::CentralPanel::default()
        .frame(egui::Frame { fill: bg0(), ..default() })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(160.0);

                match mode {
                    NetMode::Host { .. } => {
                        ui.label(egui::RichText::new("HOSTING").size(22.0).color(gold()).strong());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("configure your match").size(11.0).color(gray()));
                        ui.add_space(20.0);
                        let hw = 28.0 + 22.0 + 6.0 + 22.0; // label + btn + value + btn = 78
                        let sep = 16.0;
                        let tot = hw + sep + hw;
                        ui.allocate_ui_with_layout(egui::vec2(tot, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                            ui.label(egui::RichText::new("HP").size(12.0).color(gray()).strong());
                            ui.add_space(6.0);
                            if row_btn(ui, "\u{2212}", 22.0, 22.0).clicked() { max_hp.hp = max_hp.hp.saturating_sub(1).max(1); }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("{:>2}", max_hp.hp)).size(15.0).color(white()));
                            ui.add_space(4.0);
                            if row_btn(ui, "+", 22.0, 22.0).clicked() { max_hp.hp = max_hp.hp.saturating_add(1).min(20); }
                            ui.add_space(sep);
                            ui.label(egui::RichText::new("AI").size(12.0).color(gray()).strong());
                            ui.add_space(6.0);
                            if row_btn(ui, "\u{2212}", 22.0, 22.0).clicked() { ai_enemy_count.count = ai_enemy_count.count.saturating_sub(1); }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("{:>2}", ai_enemy_count.count)).size(15.0).color(white()));
                            ui.add_space(4.0);
                            if row_btn(ui, "+", 22.0, 22.0).clicked() { ai_enemy_count.count = ai_enemy_count.count.saturating_add(1).min(20); }
                        });
                        ui.add_space(24.0);
                        if accent_btn(ui, "START GAME", pw, 38.0).clicked() { pending.0 = Some(GameState::Playing); }
                    }
                    NetMode::Client => {
                        if let Some(client) = client {
                            if client.connected {
                                ui.label(egui::RichText::new("CONNECTED").size(18.0).color(gold()));
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new("waiting for host...").size(13.0).color(gray()));
                            } else {
                                ui.label(egui::RichText::new("CONNECTING...").size(16.0).color(gold_dim()));
                            }
                        }
                    }
                    NetMode::None => {}
                }

                ui.add_space(24.0);
                if row_btn(ui, "BACK TO MENU", pw * 0.5, 28.0).clicked() { pending.0 = Some(GameState::PreGame); }
            });
        });
}

// ── DEATH ──
fn death_ui(ctx: &egui::Context, outcome: &GameOutcome, pending: &mut PendingState) {
    let (title, sub) = if outcome.0 { ("VICTORY", "all enemies eliminated") } else { ("TERMINATED", "your car was destroyed") };

    egui::CentralPanel::default()
        .frame(egui::Frame { fill: bg0(), ..default() })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(220.0);
                ui.label(egui::RichText::new(title).size(42.0).color(gold()).strong());
                ui.add_space(6.0);
                ui.label(egui::RichText::new(sub).size(13.0).color(gray()));
                ui.add_space(40.0);
                if accent_btn(ui, "RESTART", 160.0, 38.0).clicked() { pending.0 = Some(GameState::PreGame); }
            });
        });
}

// ── PLAYING HUD ──
fn playing_ui(ctx: &egui::Context, telemetry: &Telemetry, keys: &ButtonInput<KeyCode>, charge: &WeaponCharge, bs: &BlasterSelection, countdown: Option<&RoundCountdown>) {
    let w_ = |k: KeyCode| keys.pressed(k);
    let w = w_(KeyCode::KeyW) || w_(KeyCode::ArrowUp);
    let a = w_(KeyCode::KeyA) || w_(KeyCode::ArrowLeft);
    let s = w_(KeyCode::KeyS) || w_(KeyCode::ArrowDown);
    let d = w_(KeyCode::KeyD) || w_(KeyCode::ArrowRight);
    let sp = w_(KeyCode::Space);
    let shift = w_(KeyCode::ShiftLeft) || w_(KeyCode::ShiftRight);

    if let Some(cd) = countdown {
        if cd.0.remaining_secs() > 0.0 {
            let secs = cd.0.remaining_secs().ceil() as u32;
            let label = if secs == 0 { "GO!".to_string() } else { secs.to_string() };
            let alpha = if secs <= 1 { ((cd.0.remaining_secs() * 2.0).sin().abs() * 0.5 + 0.5) as u8 } else { 255 };
            egui::Area::new("countdown".into()).anchor(egui::Align2::CENTER_CENTER, [0.0, -40.0]).show(ctx, |ui| {
                ui.label(egui::RichText::new(&label).size(72.0).color(egui::Color32::from_rgba_unmultiplied(255, 200, 50, alpha)).strong());
            });
        }
    }

    let def = &BLASTER_DEFS[bs.display_index()];
    let cr = (charge.0 / def.capacity).min(1.0);
    let speed = telemetry.speed_history.last().copied().unwrap_or(0.0) as i32;

    let kw = 32.0;
    let kh = 32.0;
    let gg = 4.0;

    egui::Area::new("keys".into()).anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -16.0)).show(ctx, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(gg, gg);
        ui.horizontal(|ui| { k(ui, "Q", w_(KeyCode::KeyQ), kw, kh); k(ui, "W", w, kw, kh); k(ui, "E", w_(KeyCode::KeyE), kw, kh); });
        ui.horizontal(|ui| { k(ui, "A", a, kw, kh); k(ui, "S", s, kw, kh); k(ui, "D", d, kw, kh); });
        ui.horizontal(|ui| { k(ui, "Shift", shift, kw * 2.0, kh); k(ui, "Space", sp, kw * 2.0, kh); });
    });

    egui::Area::new("speed".into()).anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0)).show(ctx, |ui| {
        egui::Frame { fill: bg1(), stroke: egui::Stroke::new(1.0, border()), corner_radius: 4.0.into(), inner_margin: egui::Margin::symmetric(12, 8), ..default() }
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{}", speed)).size(18.0).color(gold()).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("m/s").size(12.0).color(gray()));
                });
            });
    });

    egui::Area::new("charge".into()).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -24.0)).show(ctx, |ui| {
        let bw = 200.0;
        let bh = 8.0;
        let (r, _) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 2.0, bg2());
        p.rect_stroke(r, 2.0, egui::Stroke::new(1.0, border()), egui::StrokeKind::Outside);
        if cr > 0.0 { p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(bw * cr, bh)), 2.0, gold()); }
    });
}

fn k(ui: &mut egui::Ui, label: &str, pressed: bool, w: f32, h: f32) {
    let bg = if pressed { gold() } else { bg2() };
    let fg = if pressed { egui::Color32::BLACK } else { white() };
    let (r, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 3.0, bg);
    p.rect_stroke(r, 3.0, egui::Stroke::new(1.0, border()), egui::StrokeKind::Outside);
    p.text(r.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), fg);
}
