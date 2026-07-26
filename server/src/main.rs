//! Voxelforge authoritative server.
//!
//! Standalone Rust process — no Bevy, no rendering. This binary owns the
//! canonical world state, validates every voxel change, and streams chunk data
//! to clients. Keeping it separate from Bevy's render ECS prevents coupling the
//! simulation tick to the GPU frame, which is the root cause of most voxel MMO
//! netcode debt.
//!
//! Architecture at scaffold stage:
//!   world  — in-memory chunk store (HashMap → RocksDB later)
//!   net    — one tokio task per client, text-command protocol (→ QUIC later)
//!
//! Usage:
//!   cargo run -p voxelforge-server
//!   echo "chunk 0 0 0" | nc 127.0.0.1 7700

mod net;
mod world;

use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;

const BIND_ADDR: &str = "127.0.0.1:7700";
const WARMUP_RADIUS: i32 = 4; // pre-generate 9×9 = 81 chunks around origin

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "voxelforge_server=info".into()),
        )
        .init();

    let state = Arc::new(world::WorldState::new());
    state.warmup(WARMUP_RADIUS);

    let listener = TcpListener::bind(BIND_ADDR).await?;
    info!("Voxelforge server listening on {BIND_ADDR}");
    info!("Protocol: newline-delimited text (scaffold) — upgrade to QUIC when ready");

    loop {
        let (socket, addr) = listener.accept().await?;
        info!(%addr, "client connected");
        let state = Arc::clone(&state);
        tokio::spawn(net::handle_connection(socket, state));
    }
}
