//! Chunk position and dense block-storage types shared by client and server.
//!
//! No Bevy, no rendering — only pure data. The client builds a Bevy Mesh from
//! a `ChunkData`; the server owns `ChunkData` directly in its world state.

use crate::block::BlockId;
use crate::worldgen::terrain_height;

/// Chunk edge length in voxels (cube: CHUNK_SIZE^3 blocks per chunk).
pub const CHUNK_SIZE: i32 = 32;

const VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

/// 3-D chunk address in chunk-space (not voxel-space).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// World-space voxel origin (0,0,0-corner) of this chunk.
    #[inline]
    pub fn world_origin(self) -> (i32, i32, i32) {
        (
            self.x * CHUNK_SIZE,
            self.y * CHUNK_SIZE,
            self.z * CHUNK_SIZE,
        )
    }
}

/// Dense 32^3 voxel storage for one chunk.
///
/// Layout: `blocks[x + CHUNK_SIZE * (z + CHUNK_SIZE * y)]` — Y-major so that
/// vertical traversal is cache-friendly for the height-fill loop.
pub struct ChunkData {
    pub pos: ChunkPos,
    blocks: Vec<BlockId>,
}

impl ChunkData {
    /// Generate terrain for this chunk position.
    ///
    /// Currently only chunk y=0 receives terrain; y<0 is all stone,
    /// y>0 is all air. This matches the Phase-0 client behaviour exactly.
    pub fn generate(pos: ChunkPos) -> Self {
        let mut blocks = vec![BlockId::AIR; VOLUME];

        let (ox, oy, oz) = pos.world_origin();

        if pos.y < 0 {
            // Below ground: fully packed stone.
            blocks.fill(BlockId::STONE);
        } else if pos.y == 0 {
            // Ground-level slab: terrain fill up to height.
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let wx = (ox + x) as f32;
                    let wz = (oz + z) as f32;
                    let h = terrain_height(wx, wz);
                    for y in 0..=h.min(CHUNK_SIZE - 1) {
                        let world_y = oy + y;
                        let b = if y == h {
                            if world_y < 11 { BlockId::SAND } else { BlockId::GRASS }
                        } else if y > h - 4 {
                            BlockId::DIRT
                        } else {
                            BlockId::STONE
                        };
                        blocks[Self::idx(x, y, z)] = b;
                    }
                }
            }
        }
        // y > 0: leave as AIR (default).

        Self { pos, blocks }
    }

    /// An all-air chunk — the blank canvas a loaded map file fills in. Used by the
    /// map editor: a saved map fully describes its own blocks, so it starts from
    /// empty space (no procedural terrain) and sets exactly what the file lists.
    pub fn empty(pos: ChunkPos) -> Self {
        Self {
            pos,
            blocks: vec![BlockId::AIR; VOLUME],
        }
    }

    /// Block at local chunk coordinates; returns AIR for out-of-bounds.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        if x < 0 || y < 0 || z < 0 || x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            BlockId::AIR
        } else {
            self.blocks[Self::idx(x, y, z)]
        }
    }

    /// Set the block at local chunk coordinates; out-of-bounds is a no-op.
    ///
    /// This is the write path behind player edits (place/break). Callers are
    /// responsible for re-meshing the chunk afterwards.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if x < 0 || y < 0 || z < 0 || x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return;
        }
        self.blocks[Self::idx(x, y, z)] = block;
    }

    /// Direct indexed access for bulk operations (serialisation, compression).
    #[inline]
    pub fn get_raw(&self, i: usize) -> BlockId {
        self.blocks[i]
    }

    /// Number of solid (non-air) blocks — useful as a quick sanity check.
    pub fn solid_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.is_opaque()).count()
    }

    #[inline]
    fn idx(x: i32, y: i32, z: i32) -> usize {
        (x + CHUNK_SIZE * (z + CHUNK_SIZE * y)) as usize
    }
}
