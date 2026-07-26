//! Server-side world state.
//!
//! Owns every loaded chunk behind an RwLock so async connection tasks can read
//! in parallel and write without contention. Chunk generation happens on the
//! write path; reads are lock-free once the chunk is cached.
//!
//! Upgrade path: replace the in-memory HashMap with a two-tier hot/cold store
//! (HashMap in front of RocksDB/Redb) once chunk count exceeds ~10k.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use voxelforge_sim::chunk::{ChunkData, ChunkPos};

pub struct WorldState {
    chunks: RwLock<HashMap<ChunkPos, Arc<ChunkData>>>,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            chunks: RwLock::new(HashMap::new()),
        }
    }

    /// Pre-generate a square region around the origin so the first client join
    /// doesn't block on chunk generation.
    pub fn warmup(&self, radius: i32) {
        let mut map = self.chunks.write().expect("world lock poisoned");
        for z in -radius..=radius {
            for x in -radius..=radius {
                let pos = ChunkPos::new(x, 0, z);
                map.entry(pos)
                    .or_insert_with(|| Arc::new(ChunkData::generate(pos)));
            }
        }
        tracing::info!(chunks = map.len(), "warmup complete");
    }

    /// Return a cached chunk or generate it on first access.
    pub fn get_or_generate(&self, pos: ChunkPos) -> Arc<ChunkData> {
        // Fast read path.
        {
            let map = self.chunks.read().expect("world lock poisoned");
            if let Some(c) = map.get(&pos) {
                return c.clone();
            }
        }
        // Slow write path — generate then insert.
        let chunk = Arc::new(ChunkData::generate(pos));
        self.chunks
            .write()
            .expect("world lock poisoned")
            .insert(pos, chunk.clone());
        chunk
    }
}
