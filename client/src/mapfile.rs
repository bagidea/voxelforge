//! Voxelforge map files — a human- and agent-authorable JSON world description.
//!
//! A map is: the world size (in chunks) + an explicit list of solid blocks. Air is
//! implicit — any voxel not listed is empty. That keeps hand-written maps tiny and
//! makes the format trivial for a person or an AI agent to generate from scratch.
//! The full spec lives in `maps/FORMAT.md`.
//!
//! This module is pure data ↔ text: no Bevy, no World. The client (`main.rs`) turns
//! a live `World` into a `MapFile` and back, and does the disk I/O.

use serde::{Deserialize, Serialize};
use voxelforge_sim::block::BlockId;

/// Bumped when the on-disk schema changes in a breaking way. Loaders warn (but still
/// try) on a newer version so an old binary fails loud, not silent.
pub const MAP_VERSION: u32 = 1;

/// One saved map. Serialises to the JSON documented in `maps/FORMAT.md`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MapFile {
    /// Schema version — always `1` for now.
    pub version: u32,
    /// Free-text label shown in the HUD; safe to leave empty.
    #[serde(default)]
    pub name: String,
    /// World extent in chunks (each chunk is 32³ voxels).
    pub size: MapSize,
    /// Every solid voxel, in world-voxel coordinates. Order is irrelevant on load.
    pub blocks: Vec<MapBlock>,
}

/// World extent, measured in chunks along X and Z (Y is a single 32-tall layer).
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MapSize {
    pub chunks_x: i32,
    pub chunks_z: i32,
}

/// One solid block at a world-voxel coordinate. `block` is a name (see `block_name`).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MapBlock {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: String,
}

/// Map a block name (as written in a file) to its id. Unknown names → `None` so the
/// loader can skip and report them instead of guessing.
pub fn block_id_from_name(name: &str) -> Option<BlockId> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "air" => BlockId::AIR,
        "grass" => BlockId::GRASS,
        "dirt" => BlockId::DIRT,
        "stone" => BlockId::STONE,
        "sand" => BlockId::SAND,
        _ => return None,
    })
}

/// The name written to a file for a block id (inverse of `block_id_from_name`).
pub fn block_name(b: BlockId) -> &'static str {
    match b {
        BlockId::GRASS => "grass",
        BlockId::DIRT => "dirt",
        BlockId::STONE => "stone",
        BlockId::SAND => "sand",
        _ => "air",
    }
}

/// Parse a map from its JSON text. Errors carry the serde message so a malformed
/// hand-written file explains itself.
pub fn parse(text: &str) -> Result<MapFile, String> {
    serde_json::from_str::<MapFile>(text).map_err(|e| e.to_string())
}

/// Render a map to pretty JSON (stable key order via the struct field order).
pub fn to_text(map: &MapFile) -> Result<String, String> {
    serde_json::to_string_pretty(map).map_err(|e| e.to_string())
}
