use std::collections::HashMap;
use std::net::SocketAddr;
use bevy::prelude::*;
use crate::car::{PlayerCar, AiCar, Health, CAR_DEFS};
use crate::net::protocol::*;
use crate::net::socket::NetworkThread;
use crate::{OwnerClient, CarModelIndex, BlasterModelIndex};

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
}

#[derive(Default, Clone)]
pub struct ClientInputState {
    pub throttle: f32,
    pub steer: f32,
    pub braking: bool,
    pub boosting: bool,
    pub shoot: bool,
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
                        });
                        self.player_info.push(info.clone());

                        let accept = ServerMessage::Accept { client_id: id, settings: self.settings.clone() };
                        self.send_to(id, &accept);
                        self.broadcast_lobby_update();
                    }
                    ClientMessage::Input { sequence, throttle, steer, braking, boosting, shoot } => {
                        if let Some(client) = self.clients.values_mut()
                            .find(|c| c.addr == pkt.addr) {
                            if sequence > client.input.sequence {
                                client.input = ClientInputState {
                                    throttle, steer, braking, boosting, shoot, sequence,
                                };
                            }
                        }
                    }
                    ClientMessage::Ready => {
                        if let Some(client) = self.clients.values_mut()
                            .find(|c| c.addr == pkt.addr) {
                            client.info.ready = true;
                            if let Some(p) = self.player_info.iter_mut().find(|p| p.client_id == client.info.client_id) {
                                p.ready = true;
                            }
                            self.broadcast_lobby_update();
                        }
                    }
                    ClientMessage::Respawn => {}
                    ClientMessage::TeamChange { team } => {
                        if let Some(client) = self.clients.values_mut()
                            .find(|c| c.addr == pkt.addr) {
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
        let msg = ServerMessage::LobbyUpdate {
            players: self.player_info.clone(),
            settings: self.settings.clone(),
        };
        self.broadcast(&msg);
    }

    pub fn send_snapshot(&self, cars: Vec<CarSnapshot>, bullets: Vec<BulletSnapshot>) {
        if self.clients.is_empty() { return; }
        let msg = ServerMessage::Snapshot {
            tick: self.tick,
            cars,
            bullets,
        };
        self.broadcast(&msg);
    }
}

pub fn server_broadcast_system(mut server: Option<ResMut<GameServer>>) {
    let Some(ref mut server) = server else { return };
    if !server.game_started { return; }
    let mut buf = Vec::new();
    bincode::serialize_into(&mut buf, &ServerAdvertisement {
        name: server.server_name.clone(),
        port: GAME_PORT,
        player_count: server.clients.len() + 1,
        max_players: MAX_PLAYERS,
        settings: server.settings.clone(),
    }).ok();
    let sock = match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => { s.set_broadcast(true).ok(); s }
        Err(_) => return,
    };
    sock.send_to(&buf, format!("255.255.255.255:{}", DISCOVERY_PORT)).ok();
}

pub fn all_players_ready(server: &GameServer) -> bool {
    server.clients.values().all(|c| c.info.ready)
}

pub fn server_snapshot_system(
    server: Option<Res<GameServer>>,
    car_query: Query<(&Transform, &Health, Option<&OwnerClient>, Option<&CarModelIndex>, Option<&BlasterModelIndex>)>,
) {
    let Some(server) = server else { return };
    if !server.game_started { return; }
    let username_map: HashMap<u64, &str> = server.player_info.iter().map(|p| (p.client_id, p.username.as_str())).collect();
    let cars: Vec<CarSnapshot> = car_query.iter().filter_map(|(tf, health, owner, car_idx, blaster_idx)| {
        let client_id = owner.map(|o| o.0).unwrap_or(u64::MAX);
        let ci = car_idx.map(|c| c.0).unwrap_or(0);
        let bi = blaster_idx.map(|b| b.0).unwrap_or(0);
        let pos = tf.translation;
        let rot = tf.rotation;
        Some(CarSnapshot {
            client_id,
            position: [pos.x, pos.y, pos.z],
            rotation: [rot.x, rot.y, rot.z, rot.w],
            velocity: [0.0, 0.0, 0.0],
            health: health.0,
            car_index: ci,
            blaster_index: bi,
            team: 0,
            username: username_map.get(&client_id).unwrap_or(&"").to_string(),
        })
    }).collect();
    server.send_snapshot(cars, Vec::new());
}

pub fn spawn_client_cars(
    server: Res<GameServer>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let _ = server;
    let _ = asset_server;
}

pub fn apply_client_inputs(
    server: Res<GameServer>,
    mut query: Query<(&OwnerClient, &mut crate::car::CarInput)>,
) {
    for (owner, mut input) in query.iter_mut() {
        if let Some(client) = server.clients.get(&owner.0) {
            let ci = &client.input;
            input.throttle = ci.throttle;
            input.steer = ci.steer;
            input.braking = ci.braking;
            input.boosting = ci.boosting;
        }
    }
}

// We'll also need to handle client car spawn when they connect while game is running.
// For now, they join at lobby only.
pub fn handle_server_connections(
    mut server: Option<ResMut<GameServer>>,
    mut _commands: Commands,
) {
    let Some(ref mut server) = server else { return };
    server.handle_messages();
}
