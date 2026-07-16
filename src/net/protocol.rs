use serde::{Deserialize, Serialize};

pub const GAME_PORT: u16 = 42070;
pub const DISCOVERY_PORT: u16 = 42069;
pub const MAX_PLAYERS: usize = 8;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {
    Hello { username: String, car_index: usize, blaster_index: usize },
    Input {
        sequence: u64,
        throttle: f32,
        steer: f32,
        braking: bool,
        boosting: bool,
        shoot: bool,
    },
    Ready,
    Respawn,
    TeamChange { team: u8 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    Accept { client_id: u64, settings: GameSettings },
    LobbyUpdate { players: Vec<PlayerInfo>, settings: GameSettings },
    GameStarting { tick: u64 },
    Snapshot { tick: u64, cars: Vec<CarSnapshot>, bullets: Vec<BulletSnapshot> },
    GameOver { winner_team: Option<u8> },
    PlayerJoined { info: PlayerInfo },
    PlayerLeft { client_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerInfo {
    pub client_id: u64,
    pub username: String,
    pub car_index: usize,
    pub blaster_index: usize,
    pub team: u8,
    pub health: u8,
    pub alive: bool,
    pub ready: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CarSnapshot {
    pub client_id: u64,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub velocity: [f32; 3],
    pub health: u8,
    pub car_index: usize,
    pub blaster_index: usize,
    pub team: u8,
    pub username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BulletSnapshot {
    pub id: u64,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSettings {
    pub max_hp: u8,
    pub ai_count: usize,
    pub respawn_enabled: bool,
    pub teams_enabled: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { max_hp: 3, ai_count: 3, respawn_enabled: false, teams_enabled: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerAdvertisement {
    pub name: String,
    pub port: u16,
    pub player_count: usize,
    pub max_players: usize,
    pub settings: GameSettings,
}

#[derive(Clone, Debug)]
pub struct DiscoveredServer {
    pub adv: ServerAdvertisement,
    pub addr: std::net::SocketAddr,
}
