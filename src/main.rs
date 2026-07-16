use bevy::prelude::*;
use avian3d::dynamics::solver::SolverConfig;
use avian3d::prelude::*;
use rand::Rng;

use crate::car::{PlayerCar, Health, AiCar, ExplosionTimer, CarCamera, CarVisual, CarSelection, CAR_DEFS, mount_y};
use crate::blaster::{BlasterSelection, BLASTER_DEFS};
use crate::net::protocol::*;

mod ai;
mod blaster;
mod car;
mod net;
mod track;
mod ui;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    PreGame,
    MultiplayerLobby,
    Playing,
    Eliminated,
}

#[derive(Resource, Default)]
pub enum NetMode {
    #[default]
    None,
    Host {
        server_name: String,
    },
    Client,
}

#[derive(Resource)]
pub struct MaxHealthPoints {
    pub hp: u8,
    pub random: bool,
}

impl Default for MaxHealthPoints {
    fn default() -> Self { Self { hp: 3, random: false } }
}

#[derive(Resource, Default)]
pub struct GameOutcome(pub bool);

#[derive(Resource, Default)]
pub struct PendingState(Option<GameState>);

#[derive(Resource)]
struct LoadingAssets {
    handles: Vec<Handle<Scene>>,
}

#[derive(Resource)]
pub struct PlayerName(pub String);

impl Default for PlayerName {
    fn default() -> Self { Self("Player".into()) }
}

#[derive(Component)]
pub struct OwnerClient(pub u64);

#[derive(Component)]
pub struct RemotePlayer;

#[derive(Component)]
pub struct RemoteBullet;

#[derive(Component)]
pub struct CarModelIndex(pub usize);

#[derive(Component)]
pub struct BlasterModelIndex(pub usize);

#[derive(Component, Clone, Copy)]
pub struct Team(pub u8);

#[derive(Component, Clone)]
pub struct BulletOwner {
    pub client_id: u64,
    pub team: u8,
}

fn enter_pregame(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
    loading: Option<Res<LoadingAssets>>,
) {
    let Some(loading) = loading else {
        let paths = [
            car::CAR_DEFS[0].path,
            blaster::BLASTER_DEFS[0].path,
            "Map.glb",
        ];
        let handles: Vec<Handle<Scene>> = paths
            .iter()
            .map(|p| asset_server.load(GltfAssetLabel::Scene(0).from_asset(*p)))
            .collect();
        commands.insert_resource(LoadingAssets { handles });
        return;
    };

    if loading.handles.iter().all(|h| asset_server.is_loaded_with_dependencies(h)) {
        next_state.set(GameState::PreGame);
        commands.remove_resource::<LoadingAssets>();
    }
}

#[derive(Resource)]
pub struct AiEnemyCount {
    pub count: usize,
    pub random: bool,
}

impl Default for AiEnemyCount {
    fn default() -> Self { Self { count: 3, random: false } }
}

fn apply_pending_state(
    mut pending: ResMut<PendingState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(state) = pending.0.take() {
        next_state.set(state);
    }
}

fn check_game_state(
    mut commands: Commands,
    mut pending: ResMut<PendingState>,
    mut outcome: ResMut<GameOutcome>,
    player_query: Query<(Entity, &Health, &Position), With<PlayerCar>>,
    ai_query: Query<(), With<AiCar>>,
    exploding_query: Query<&ExplosionTimer>,
    enemy_count: Res<AiEnemyCount>,
) {
    for (entity, health, pos) in player_query.iter() {
        if pos.0.y < -20.0 {
            outcome.0 = false;
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.4, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            pending.0 = Some(GameState::Eliminated);
            return;
        }
        if health.0 == 0 && exploding_query.get(entity).is_err() {
            outcome.0 = false;
            commands.entity(entity).insert((
                ExplosionTimer(Timer::from_seconds(0.4, TimerMode::Once)),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
            pending.0 = Some(GameState::Eliminated);
            return;
        }
    }
    if enemy_count.count > 0 && ai_query.iter().count() == 0 {
        for (_, player_health, _) in player_query.iter() {
            if player_health.0 > 0 {
                outcome.0 = true;
                pending.0 = Some(GameState::Eliminated);
            }
        }
    }
}

fn resolve_random_options(
    mut enemy_count: ResMut<AiEnemyCount>,
    mut max_hp: ResMut<MaxHealthPoints>,
) {
    if enemy_count.random {
        enemy_count.count = rand::rng().random_range(0..=10);
        enemy_count.random = false;
    }
    if max_hp.random {
        max_hp.hp = rand::rng().random_range(2..=10);
        max_hp.random = false;
    }
}

fn start_server_game(mut server: Option<ResMut<net::server::GameServer>>) {
    if let Some(ref mut server) = server {
        server.game_started = true;
        server.broadcast(&net::protocol::ServerMessage::GameStarting { tick: 0 });
    }
}

fn stop_server_game(server: Option<ResMut<net::server::GameServer>>) {
    if let Some(mut s) = server {
        s.game_started = false;
    }
}

fn cleanup_multiplayer(mut commands: Commands) {
    commands.remove_resource::<net::server::GameServer>();
    commands.remove_resource::<net::client::GameClient>();
}

#[derive(Resource, Default)]
pub struct PendingConnect(pub Option<std::net::SocketAddr>);

#[derive(Resource, Default)]
pub struct PendingHost(pub bool);

fn handle_pending_host(
    mut commands: Commands,
    mut ph: ResMut<PendingHost>,
    ai_count: Res<AiEnemyCount>,
    max_hp: Res<MaxHealthPoints>,
    name: Res<PlayerName>,
    car_selection: Res<CarSelection>,
    blaster_selection: Res<BlasterSelection>,
) {
    if !ph.0 { return; }
    ph.0 = false;
    commands.insert_resource(NetMode::Host { server_name: "Raycer Game".into() });
    let settings = GameSettings {
        max_hp: max_hp.hp,
        ai_count: ai_count.count,
        ..default()
    };
    match net::server::GameServer::new(settings, name.0.clone()) {
        Ok(mut server) => {
            server.player_info.push(PlayerInfo {
                client_id: 0,
                username: name.0.clone(),
                car_index: car_selection.display_index(),
                blaster_index: blaster_selection.display_index(),
                team: 0,
                health: max_hp.hp,
                alive: true,
                ready: true,
            });
            commands.insert_resource(server);
        }
        Err(e) => error!("Failed to start server: {}", e),
    }
}

fn handle_pending_connect(
    mut commands: Commands,
    mut pending: ResMut<PendingConnect>,
    name: Res<PlayerName>,
    car_selection: Res<CarSelection>,
    blaster_selection: Res<BlasterSelection>,
) {
    if let Some(addr) = pending.0.take() {
        match net::client::GameClient::connect(addr) {
            Ok(client) => {
                client.send_hello(&name.0, car_selection.display_index(), blaster_selection.display_index());
                commands.insert_resource(client);
            }
            Err(e) => error!("Failed to connect: {}", e),
        }
    }
}

fn start_broadcast_receiver(
    mut commands: Commands,
    mode: Res<NetMode>,
) {
    if let NetMode::Client = &*mode {
        match net::socket::BroadcastReceiver::start() {
            Ok(r) => { commands.insert_resource(net::client::ClientBroadcastReceiver(r)); }
            Err(e) => error!("broadcast receiver: {}", e),
        }
    }
}

fn apply_client_snapshot(
    mut commands: Commands,
    snapshot: Res<net::client::ReceivedSnapshot>,
    client: Option<Res<net::client::GameClient>>,
    asset_server: Res<AssetServer>,
    existing: Query<(Entity, &OwnerClient)>,
) {
    if snapshot.is_changed() && client.is_some() {
        let mut seen: Vec<u64> = Vec::new();
        for car in &snapshot.cars {
            seen.push(car.client_id);
            let exists = existing.iter().any(|(_, oc)| oc.0 == car.client_id);
            if !exists {
                let pos = Vec3::new(car.position[0], car.position[1], car.position[2]);
                let rot = Quat::from_array(car.rotation);
                let def = &CAR_DEFS[car.car_index.min(CAR_DEFS.len() - 1)];
                let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
                let blaster_def = &BLASTER_DEFS[car.blaster_index.min(BLASTER_DEFS.len() - 1)];
                let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
                let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);
                commands.spawn((
                    RemotePlayer,
                    OwnerClient(car.client_id),
                    Transform::from_translation(pos).with_rotation(rot),
                    Visibility::Visible,
                )).with_children(|parent| {
                    parent.spawn((SceneRoot(car_scene), CarVisual));
                    parent.spawn((
                        SceneRoot(blaster_scene),
                        Transform::from_translation(mount)
                            .with_scale(Vec3::splat(blaster_def.scale))
                            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    ));
                });
            } else {
                for (entity, oc) in existing.iter() {
                    if oc.0 == car.client_id {
                        commands.entity(entity).insert(
                            Transform::from_translation(
                                Vec3::new(car.position[0], car.position[1], car.position[2])
                            ).with_rotation(Quat::from_array(car.rotation))
                        );
                    }
                }
            }
        }
        let to_despawn: Vec<Entity> = existing.iter().filter(|(_, oc)| !seen.contains(&oc.0)).map(|(e, _)| e).collect();
        for e in to_despawn {
            commands.entity(e).despawn();
        }
    }
}

fn client_capture_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut client: Option<ResMut<net::client::GameClient>>,
) {
    let Some(ref mut client) = client else { return };
    if !client.connected { return; }
    let throttle = if keys.pressed(KeyCode::KeyW) { 1.0 } else if keys.pressed(KeyCode::KeyS) { -0.5 } else { 0.0 };
    let steer = match (keys.pressed(KeyCode::KeyA), keys.pressed(KeyCode::KeyD)) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    };
    let shoot = mouse_buttons.pressed(MouseButton::Left);
    client.send_input(throttle, steer, keys.pressed(KeyCode::Space), keys.pressed(KeyCode::ShiftLeft), shoot);
}

fn main() {
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "raycer".into(),
            canvas: Some("#raycer".into()),
            ..default()
        }),
        ..default()
    };

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity(Vec3::NEG_Y * 120.0))
        .insert_resource(SubstepCount(12))
        .insert_resource(SolverConfig {
            contact_damping_ratio: 15.0,
            max_overlap_solve_speed: 8.0,
            ..default()
        })
        .add_plugins(bevy_egui::EguiPlugin::default())
        .init_state::<GameState>()
        .init_resource::<AiEnemyCount>()
        .init_resource::<MaxHealthPoints>()
        .init_resource::<GameOutcome>()
        .init_resource::<PendingState>()
        .init_resource::<NetMode>()
        .init_resource::<PlayerName>()
        .init_resource::<PendingConnect>()
        .init_resource::<PendingHost>()
        .init_resource::<net::client::DiscoveredServers>()
        .init_resource::<net::client::ReceivedSnapshot>()
        .init_resource::<net::client::LobbyData>()
        .add_systems(OnEnter(GameState::Playing), (resolve_random_options, start_server_game))
        .add_systems(OnExit(GameState::Playing), stop_server_game)
        .add_systems(OnEnter(GameState::MultiplayerLobby), start_broadcast_receiver)
        .add_systems(OnExit(GameState::MultiplayerLobby), cleanup_multiplayer)
        .add_systems(OnExit(GameState::Playing), cleanup_multiplayer)
        .add_systems(Update, enter_pregame.run_if(in_state(GameState::Loading)))
        .add_systems(Update, check_game_state.run_if(in_state(GameState::Playing)))
        .add_systems(Update, apply_pending_state)
        .add_systems(Update, (handle_pending_host, handle_pending_connect))
        .add_systems(Update, (apply_client_snapshot, client_capture_input))
        .add_systems(Update, (
            net::server::server_broadcast_system,
            net::server::server_snapshot_system,
            net::server::respawn_system,
            net::server::apply_client_inputs,
            net::server::handle_server_connections,
            net::client::discovery_listen_system,
            net::client::client_receive_system,
        ))
        .add_plugins((ai::AiPlugin, blaster::BlasterPlugin, car::CarPlugin, track::TrackPlugin, ui::UiPlugin));

    #[cfg(feature = "dev")]
    app.add_plugins(avian3d::debug_render::PhysicsDebugPlugin);

    app.run();
}
