pub mod protocol;
pub mod socket;
pub mod server;
pub mod client;

use bevy::prelude::*;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<client::DiscoveredServers>()
            .init_resource::<client::ReceivedSnapshot>()
            .init_resource::<client::LobbyData>()
            .add_systems(Update, (
                server::server_broadcast_system,
                client::discovery_listen_system,
                client::client_receive_system,
            ));
    }
}
