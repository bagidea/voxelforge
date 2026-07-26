//! Per-connection handler — one tokio task per client.
//!
//! Protocol at scaffold stage: newline-delimited text commands (easy to poke
//! with nc/telnet during early dev). Upgrade path: replace with a binary-framed
//! QUIC stream (quinn) once the command set stabilises.
//!
//! Interest management is intentionally absent here — that goes in a separate
//! module once we have multiple clients; for now the server responds to explicit
//! chunk requests rather than pushing based on position.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, warn};

use voxelforge_sim::chunk::ChunkPos;

use crate::world::WorldState;

/// Handle one client connection until it disconnects.
///
/// Supported commands (one per line):
///   `chunk <x> <y> <z>` — request chunk at chunk-space position
///   `ping`               — liveness check
pub async fn handle_connection(stream: TcpStream, state: Arc<WorldState>) {
    let peer = stream.peer_addr().ok();
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_owned();
        debug!(?peer, cmd = %line, "recv");

        if line == "ping" {
            if writer.write_all(b"pong\n").await.is_err() {
                break;
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("chunk ") {
            let coords: Vec<i32> = rest
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if coords.len() != 3 {
                warn!(?peer, "malformed chunk command: {line}");
                let _ = writer.write_all(b"err bad_args\n").await;
                continue;
            }

            let pos = ChunkPos::new(coords[0], coords[1], coords[2]);
            let chunk = state.get_or_generate(pos);

            // Scaffold response: solid block count proves the server did real work.
            // Production: replace with a compressed binary payload.
            let resp = format!(
                "chunk_data {},{},{} solid={}\n",
                pos.x,
                pos.y,
                pos.z,
                chunk.solid_count(),
            );
            if writer.write_all(resp.as_bytes()).await.is_err() {
                break;
            }
        } else {
            warn!(?peer, "unknown command: {line}");
            let _ = writer.write_all(b"err unknown_cmd\n").await;
        }
    }

    debug!(?peer, "disconnected");
}
