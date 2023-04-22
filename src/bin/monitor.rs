use anyhow::Result;
use clap::Parser;
use grobot::{NetworkUpdate, PORT};
use serde_json::from_str;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;
use tracing::{info, Level};

const BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = Level::INFO)]
    /// Logging level
    log_level: Level,
    #[clap(short, long, default_value_t = PORT)]
    // Port
    port: u16,
    #[clap(short = 'L', long, default_value_t = BIND_ADDR)]
    // Listen address
    listen_addr: Ipv4Addr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let bind_addr = SocketAddrV4::new(args.listen_addr, args.port);
    let sock = UdpSocket::bind(bind_addr).await?;

    loop {
        // Receive on the socket
        let mut buf = vec![0u8; 1024];
        let (len, addr) = sock.recv_from(&mut buf).await?;
        info!("Received {} bytes from {}", len, addr);
        let update: NetworkUpdate = from_str(&String::from_utf8_lossy(&buf[..len]))?;
        info!("Received update {:?}", update);
    }
}
