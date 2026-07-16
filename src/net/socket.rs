use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct OutgoingPacket {
    pub addr: SocketAddr,
    pub data: Vec<u8>,
}

pub struct IncomingPacket {
    pub addr: SocketAddr,
    pub data: Vec<u8>,
}

pub struct NetworkThread {
    pub sender: mpsc::Sender<OutgoingPacket>,
    pub receiver: Mutex<mpsc::Receiver<IncomingPacket>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl NetworkThread {
    pub fn start_server(bind_port: u16) -> Result<Self, String> {
        let addr = format!("0.0.0.0:{}", bind_port);
        let socket = UdpSocket::bind(&addr).map_err(|e| format!("bind: {}", e))?;
        socket.set_nonblocking(true).map_err(|e| format!("nonblock: {}", e))?;
        socket.set_broadcast(true).ok();
        Self::start(socket)
    }

    pub fn start_client(server_addr: SocketAddr) -> Result<Self, String> {
        let local_addr = if server_addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let socket = UdpSocket::bind(local_addr).map_err(|e| format!("bind: {}", e))?;
        socket.set_nonblocking(true).map_err(|e| format!("nonblock: {}", e))?;
        socket.connect(server_addr).map_err(|e| format!("connect: {}", e))?;
        Self::start(socket)
    }

    fn start(socket: UdpSocket) -> Result<Self, String> {
        let (tx_out, rx_out) = mpsc::channel::<OutgoingPacket>();
        let (tx_in, rx_in) = mpsc::channel::<IncomingPacket>();
        let socket = Arc::new(socket);

        let sock = socket.clone();
        let _sender = tx_out.clone();
        let handle = thread::spawn(move || {
            let mut buf = vec![0u8; 4096];
            loop {
                if let Ok(len) = sock.recv_from(&mut buf) {
                    let data = buf[..len.0].to_vec();
                    if tx_in.send(IncomingPacket { addr: len.1, data }).is_err() {
                        break;
                    }
                }
                match rx_out.try_recv() {
                    Ok(pkt) => {
                        sock.send_to(&pkt.data, pkt.addr).ok();
                    }
                    Err(mpsc::TryRecvError::Disconnected) => break,
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        Ok(Self { sender: tx_out, receiver: Mutex::new(rx_in), handle: Some(handle) })
    }

    pub fn send(&self, addr: SocketAddr, data: Vec<u8>) {
        self.sender.send(OutgoingPacket { addr, data }).ok();
    }

    pub fn try_recv(&self) -> Option<IncomingPacket> {
        self.receiver.lock().unwrap().try_recv().ok()
    }
}

pub struct BroadcastReceiver {
    socket: UdpSocket,
}

impl BroadcastReceiver {
    pub fn start() -> Result<Self, String> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", crate::net::protocol::DISCOVERY_PORT))
            .map_err(|e| format!("broadcast bind: {}", e))?;
        socket.set_nonblocking(true).map_err(|e| format!("broadcast nonblock: {}", e))?;
        socket.set_read_timeout(Some(std::time::Duration::from_millis(100))).ok();
        Ok(Self { socket })
    }

    pub fn poll(&self) -> Option<(Vec<u8>, SocketAddr)> {
        let mut buf = [0u8; 2048];
        match self.socket.recv_from(&mut buf) {
            Ok((len, addr)) => Some((buf[..len].to_vec(), addr)),
            Err(_) => None,
        }
    }
}

pub struct Broadcaster {
    socket: UdpSocket,
}

impl Broadcaster {
    pub fn start() -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("broadcaster bind: {}", e))?;
        socket.set_broadcast(true).map_err(|e| format!("broadcast set: {}", e))?;
        Ok(Self { socket })
    }

    pub fn broadcast(&self, data: &[u8]) {
        self.socket.send_to(data, format!("255.255.255.255:{}", crate::net::protocol::DISCOVERY_PORT)).ok();
    }
}
