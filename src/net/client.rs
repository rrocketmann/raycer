use std::net::SocketAddr;
use bevy::prelude::*;
use crate::net::protocol::*;
use crate::net::socket::{NetworkThread, BroadcastReceiver};

#[derive(Resource)]
pub struct GameClient {
    pub net: NetworkThread,
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub settings: GameSettings,
    pub connected: bool,
    pub sequence: u64,
}

#[derive(Resource, Default)]
pub struct DiscoveredServers(pub Vec<DiscoveredServer>);

#[derive(Resource)]
pub struct ClientBroadcastReceiver(pub BroadcastReceiver);

#[derive(Resource, Default)]
pub struct ReceivedSnapshot {
    pub tick: u64,
    pub cars: Vec<CarSnapshot>,
    pub bullets: Vec<BulletSnapshot>,
}

#[derive(Resource, Default)]
pub struct LobbyData {
    pub players: Vec<PlayerInfo>,
    pub settings: GameSettings,
}

impl GameClient {
    pub fn connect(server: SocketAddr) -> Result<Self, String> {
        let net = NetworkThread::start_client(server)?;
        Ok(Self {
            net,
            client_id: 0,
            server_addr: server,
            settings: GameSettings::default(),
            connected: false,
            sequence: 0,
        })
    }

    pub fn send(&self, msg: &ClientMessage) {
        if let Ok(data) = bincode::serialize(msg) {
            let addr = if self.server_addr.is_ipv4() {
                let ip = self.server_addr.ip();
                SocketAddr::new(ip, GAME_PORT)
            } else {
                self.server_addr
            };
            self.net.send(addr, data);
        }
    }

    pub fn send_hello(&self, username: &str, car_index: usize, blaster_index: usize) {
        self.send(&ClientMessage::Hello {
            username: username.to_string(),
            car_index,
            blaster_index,
        });
    }

    pub fn send_input(&mut self, throttle: f32, steer: f32, braking: bool, boosting: bool, shoot: bool) {
        self.sequence += 1;
        let seq = self.sequence;
        self.send(&ClientMessage::Input {
            sequence: seq,
            throttle,
            steer,
            braking,
            boosting,
            shoot,
        });
    }

    pub fn send_ready(&self) {
        self.send(&ClientMessage::Ready);
    }

    pub fn poll(&self) -> Option<ServerMessage> {
        while let Some(pkt) = self.net.try_recv() {
            if let Ok(msg) = bincode::deserialize::<ServerMessage>(&pkt.data) {
                return Some(msg);
            }
        }
        None
    }
}

pub fn discovery_listen_system(
    receiver: Res<ClientBroadcastReceiver>,
    mut discovered: ResMut<DiscoveredServers>,
) {
    while let Some((data, addr)) = receiver.0.poll() {
        if let Ok(adv) = bincode::deserialize::<ServerAdvertisement>(&data) {
            if !discovered.0.iter().any(|s| s.adv.port == adv.port) {
                discovered.0.push(DiscoveredServer { adv, addr });
            }
        }
    }
}

pub fn client_receive_system(
    mut client: Option<ResMut<GameClient>>,
    mut lobby: ResMut<LobbyData>,
    mut snapshot: ResMut<ReceivedSnapshot>,
    mut next_state: ResMut<NextState<crate::GameState>>,
) {
    let Some(ref mut client) = client else { return };
    while let Some(msg) = client.poll() {
        match msg {
            ServerMessage::Accept { client_id, settings } => {
                client.client_id = client_id;
                client.settings = settings;
                client.connected = true;
            }
            ServerMessage::LobbyUpdate { players, settings } => {
                lobby.players = players;
                lobby.settings = settings;
            }
            ServerMessage::Snapshot { tick, cars, bullets } => {
                snapshot.tick = tick;
                snapshot.cars = cars;
                snapshot.bullets = bullets;
            }
            ServerMessage::GameStarting { .. } => {
                next_state.set(crate::GameState::Playing);
            }
            ServerMessage::GameOver { .. } => {}
            ServerMessage::PlayerJoined { .. } => {}
            ServerMessage::PlayerLeft { .. } => {}
        }
    }
}
