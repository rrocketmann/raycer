use std::collections::HashMap;
use std::net::SocketAddr;
use bevy::prelude::*;
use rand::Rng;
use avian3d::prelude::*;
use crate::car::{Health, DamageTracker, CarVisual, PlayerCar, AiCar, CarInput, CAR_DEFS, mount_y, spawn_health_indicators};
use crate::blaster::{Bullet, BLASTER_DEFS};
use crate::net::protocol::*;
use crate::net::socket::NetworkThread;
use crate::{OwnerClient, CarModelIndex, BlasterModelIndex, Team};

#[derive(Resource)]
pub struct GameServer {
    pub net: NetworkThread,
    pub clients: HashMap<u64, ClientConnection>,
    pub next_client_id: u64,
    pub settings: GameSettings,
    pub tick: u64,
    pub game_started: bool,
    pub player_info: Vec<PlayerInfo>,
    pub server_name: String,
}

pub struct ClientConnection {
    pub addr: SocketAddr,
    pub info: PlayerInfo,
    pub input: ClientInputState,
    pub last_seen: std::time::Instant,
}

#[derive(Default, Clone)]
pub struct ClientInputState {
    pub throttle: f32,
    pub steer: f32,
    pub braking: bool,
    pub boosting: bool,
    pub sequence: u64,
}

impl GameServer {
    pub fn new(settings: GameSettings, name: String) -> Result<Self, String> {
        let net = NetworkThread::start_server(GAME_PORT)?;
        Ok(Self {
            net,
            clients: HashMap::new(),
            next_client_id: 1,
            settings,
            tick: 0,
            game_started: false,
            player_info: Vec::new(),
            server_name: name,
        })
    }

    pub fn handle_messages(&mut self) {
        while let Some(pkt) = self.net.try_recv() {
            if let Ok(msg) = bincode::deserialize::<ClientMessage>(&pkt.data) {
                let now = std::time::Instant::now();
                match msg {
                    ClientMessage::Hello { username, car_index, blaster_index } => {
                        if self.clients.len() >= MAX_PLAYERS { continue; }
                        let id = self.next_client_id;
                        self.next_client_id += 1;
                        let info = PlayerInfo {
                            client_id: id,
                            username,
                            car_index,
                            blaster_index,
                            team: 0,
                            health: self.settings.max_hp,
                            alive: true,
                            ready: false,
                        };
                        self.clients.insert(id, ClientConnection {
                            addr: pkt.addr,
                            info: info.clone(),
                            input: ClientInputState::default(),
                            last_seen: now,
                        });
                        self.player_info.push(info.clone());
                        let accept = ServerMessage::Accept { client_id: id, settings: self.settings.clone() };
                        self.send_to(id, &accept);
                        self.broadcast_lobby_update();
                    }
                    ClientMessage::Input { sequence, throttle, steer, braking, boosting, shoot: _ } => {
                        if let Some(client) = self.clients.values_mut().find(|c| c.addr == pkt.addr) {
                            if sequence > client.input.sequence {
                                client.input = ClientInputState { throttle, steer, braking, boosting, sequence };
                            }
                            client.last_seen = now;
                        }
                    }
                    ClientMessage::Ready => {
                        if let Some(client) = self.clients.values_mut().find(|c| c.addr == pkt.addr) {
                            client.info.ready = true;
                            if let Some(p) = self.player_info.iter_mut().find(|p| p.client_id == client.info.client_id) {
                                p.ready = true;
                            }
                            self.broadcast_lobby_update();
                        }
                    }
                    ClientMessage::Respawn => {}
                    ClientMessage::TeamChange { team } => {
                        if let Some(client) = self.clients.values_mut().find(|c| c.addr == pkt.addr) {
                            if self.settings.teams_enabled {
                                client.info.team = team.min(2);
                                if let Some(p) = self.player_info.iter_mut().find(|p| p.client_id == client.info.client_id) {
                                    p.team = team.min(2);
                                }
                                self.broadcast_lobby_update();
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn send_to(&self, client_id: u64, msg: &ServerMessage) {
        if let Ok(data) = bincode::serialize(msg) {
            if let Some(client) = self.clients.get(&client_id) {
                self.net.send(client.addr, data);
            }
        }
    }

    pub fn broadcast(&self, msg: &ServerMessage) {
        if let Ok(data) = bincode::serialize(msg) {
            for client in self.clients.values() {
                self.net.send(client.addr, data.clone());
            }
        }
    }

    pub fn broadcast_lobby_update(&self) {
        self.broadcast(&ServerMessage::LobbyUpdate {
            players: self.player_info.clone(),
            settings: self.settings.clone(),
        });
    }

    pub fn send_snapshot(&self, cars: Vec<CarSnapshot>, bullets: Vec<BulletSnapshot>) {
        if self.clients.is_empty() { return; }
        self.broadcast(&ServerMessage::Snapshot { tick: self.tick, cars, bullets });
    }
}

// ── Broadcast LAN advertisement ──
pub fn server_broadcast_system(mut server: Option<ResMut<GameServer>>) {
    let Some(ref mut server) = server else { return };
    let mut buf = Vec::new();
    bincode::serialize_into(&mut buf, &ServerAdvertisement {
        name: server.server_name.clone(),
        port: GAME_PORT,
        player_count: server.clients.len() + 1,
        max_players: MAX_PLAYERS,
        settings: server.settings.clone(),
    }).ok();
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        sock.set_broadcast(true).ok();
        sock.send_to(&buf, format!("255.255.255.255:{}", DISCOVERY_PORT)).ok();
    }
}

// ── Snapshot generation ──
pub fn server_snapshot_system(
    server: Option<Res<GameServer>>,
    car_query: Query<(&Transform, &Health, Option<&OwnerClient>, Option<&CarModelIndex>, Option<&BlasterModelIndex>, Option<&Team>)>,
    bullet_query: Query<(&Transform, &Bullet, Option<&OwnerClient>)>,
) {
    let Some(server) = server else { return };
    if !server.game_started { return; }
    let umap: HashMap<u64, &str> = server.player_info.iter().map(|p| (p.client_id, p.username.as_str())).collect();
    let cars: Vec<CarSnapshot> = car_query.iter().map(|(tf, health, owner, ci, bi, team)| {
        let cid = owner.map(|o| o.0).unwrap_or(u64::MAX);
        CarSnapshot {
            client_id: cid,
            position: [tf.translation.x, tf.translation.y, tf.translation.z],
            rotation: [tf.rotation.x, tf.rotation.y, tf.rotation.z, tf.rotation.w],
            velocity: [0.0, 0.0, 0.0],
            health: health.0,
            car_index: ci.map(|c| c.0).unwrap_or(0),
            blaster_index: bi.map(|b| b.0).unwrap_or(0),
            team: team.map(|t| t.0).unwrap_or(0),
            username: umap.get(&cid).unwrap_or(&"").to_string(),
        }
    }).collect();
    let mut bullet_id = 0u64;
    let bullets: Vec<BulletSnapshot> = bullet_query.iter().map(|(tf, bullet, _owner)| {
        let id = bullet_id;
        bullet_id += 1;
        BulletSnapshot {
            id,
            position: [tf.translation.x, tf.translation.y, tf.translation.z],
            velocity: [bullet.velocity.x, bullet.velocity.y, bullet.velocity.z],
        }
    }).collect();
    server.send_snapshot(cars, bullets);
}

// ── Apply remote client inputs to their car components ──
pub fn apply_client_inputs(
    server: Option<Res<GameServer>>,
    mut query: Query<(&OwnerClient, &mut CarInput)>,
) {
    let Some(server) = server else { return };
    for (oc, mut input) in query.iter_mut() {
        if let Some(client) = server.clients.get(&oc.0) {
            let ci = &client.input;
            input.throttle = ci.throttle;
            input.steer = ci.steer;
            input.braking = ci.braking;
            input.boosting = ci.boosting;
        }
    }
}

// ── Simple physics for remote cars (not host, not AI) ──
pub fn drive_remote_cars(
    time: Res<Time>,
    mut query: Query<(&mut LinearVelocity, &mut AngularVelocity, &CarInput, &Rotation), (Without<PlayerCar>, Without<AiCar>, With<OwnerClient>)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut vel, mut ang, input, rot) in query.iter_mut() {
        let max_speed = 30.0;
        let accel = 25.0;
        let forward = rot * Vec3::Z;
        let right = rot * Vec3::X;

        let current_forward = vel.0.dot(forward);
        let target = input.throttle * max_speed;
        let diff = target - current_forward;
        vel.0 += forward * diff.clamp(-accel * dt, accel * dt);

        let lateral = vel.0.dot(right);
        vel.0 -= right * lateral * 0.15 * dt * 60.0;

        let speed_factor = (current_forward.abs() / max_speed).max(0.1);
        ang.0.y = input.steer * 3.0 * speed_factor;

        if input.braking {
            vel.0 *= 0.97;
        }

        let speed = vel.0.length();
        if speed > max_speed {
            vel.0 = vel.0 / speed * max_speed;
        }
    }
}

// ── Spawn a car entity for a remote client ──
fn spawn_remote_car(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    client_id: u64,
    car_index: usize,
    blaster_index: usize,
    team: u8,
    max_hp: u8,
    position: Vec3,
) {
    let def = &CAR_DEFS[car_index.min(CAR_DEFS.len() - 1)];
    let car_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.path));
    let blaster_def = &BLASTER_DEFS[blaster_index.min(BLASTER_DEFS.len() - 1)];
    let blaster_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(blaster_def.path));
    let half = def.collider.y * 0.5;
    let mount = Vec3::new(0.0, mount_y(def.collider.y), 0.0);

    let mut ent = commands.spawn((
        RigidBody::Dynamic,
        Position(position),
        Rotation::default(),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
        LinearDamping(0.5),
        AngularDamping(1.0),
        MaxLinearSpeed(80.0),
        MaxAngularSpeed(4.0),
        CenterOfMass(Vec3::ZERO),
        Friction::new(0.01),
    ));
    ent.insert(SweptCcd::NON_LINEAR);
    ent.insert(Mass(6.0));
    ent.insert(GravityScale(1.0));
    ent.insert(OwnerClient(client_id));
    ent.insert(Health(max_hp));
    ent.insert(CarModelIndex(car_index));
    ent.insert(BlasterModelIndex(blaster_index));
    ent.insert(Team(team));
    ent.insert(DamageTracker::default());
    ent.insert(CarInput::default());
    ent.insert(Visibility::Visible);
    let root = ent.id();
    let _ = commands.entity(root).with_children(|parent| {
        parent.spawn((
            Collider::cuboid(def.collider.x, def.collider.y, def.collider.z),
            Transform::from_translation(Vec3::new(0.0, half, 0.0)),
            CollisionLayers::new(LayerMask(0b010), LayerMask(0xFFFFFFFF)),
        ));
        parent.spawn((SceneRoot(car_scene), CarVisual));
        parent.spawn((
            SceneRoot(blaster_scene),
            Transform::from_translation(mount)
                .with_scale(Vec3::splat(blaster_def.scale))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ));
    }).id();
    spawn_health_indicators(root, commands, meshes, materials, def.collider.y, max_hp);
}

// ── Spawn cars for all connected clients when game starts ──
pub fn spawn_client_cars_on_start(
    server: Option<Res<GameServer>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(server) = server else { return };
    let max_hp = server.settings.max_hp;
    for (i, info) in server.player_info.iter().enumerate() {
        if info.client_id == 0 { continue; } // skip host
        let angle = i as f32 * std::f32::consts::TAU / server.player_info.len().max(1) as f32;
        let pos = Vec3::new(angle.cos() * 35.0, 3.0, angle.sin() * 35.0);
        spawn_remote_car(&mut commands, &asset_server, &mut meshes, &mut materials,
            info.client_id, info.car_index, info.blaster_index, info.team, max_hp, pos);
    }
}

// ── Spawn car when a client connects mid-game ──
pub fn spawn_midgame_client_cars(
    server: Option<Res<GameServer>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<&OwnerClient>,
) {
    let Some(server) = server else { return };
    if !server.game_started { return; }
    for info in &server.player_info {
        if info.client_id == 0 { continue; }
        let already_spawned = existing.iter().any(|oc| oc.0 == info.client_id);
        if !already_spawned {
            let mut rng = rand::rng();
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let pos = Vec3::new(angle.cos() * 35.0, 3.0, angle.sin() * 35.0);
            spawn_remote_car(&mut commands, &asset_server, &mut meshes, &mut materials,
                info.client_id, info.car_index, info.blaster_index, info.team, server.settings.max_hp, pos);
        }
    }
}

// ── Remove stale/disconnected clients ──
pub fn remove_stale_clients(
    mut server: Option<ResMut<GameServer>>,
    mut commands: Commands,
    car_query: Query<(Entity, &OwnerClient)>,
) {
    let Some(ref mut server) = server else { return };
    let now = std::time::Instant::now();
    let stale: Vec<u64> = server.clients.iter()
        .filter(|(_, c)| now.duration_since(c.last_seen).as_secs() > 15)
        .map(|(id, _)| *id)
        .collect();
    for id in stale {
        server.player_info.retain(|p| p.client_id != id);
        server.clients.remove(&id);
        for (e, oc) in car_query.iter() {
            if oc.0 == id { commands.entity(e).despawn(); }
        }
        server.broadcast_lobby_update();
    }
}

// ── Respawn system ──
#[derive(Component)]
pub struct RespawnTimer(pub Timer);

pub fn respawn_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Health, &mut RespawnTimer)>,
) {
    for (entity, mut tf, mut health, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            health.0 = 10;
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            tf.translation = Vec3::new(angle.cos() * 30.0, 3.0, angle.sin() * 30.0);
            commands.entity(entity).remove::<RespawnTimer>();
        }
    }
}

// ── Connection message handler ──
pub fn handle_server_connections(mut server: Option<ResMut<GameServer>>) {
    let Some(ref mut server) = server else { return };
    server.handle_messages();
}
