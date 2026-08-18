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
        "wood" => BlockId::WOOD,
        "leaves" => BlockId::LEAVES,
        "snow" => BlockId::SNOW,
        "red_sand" => BlockId::RED_SAND,
        "clay" => BlockId::CLAY,
        "gravel" => BlockId::GRAVEL,
        "cobblestone" => BlockId::COBBLESTONE,
        "obsidian" => BlockId::OBSIDIAN,
        "brick" => BlockId::BRICK,
        "moss" => BlockId::MOSS,
        "limestone" => BlockId::LIMESTONE,
        "lamp" => BlockId::LAMP,
        "glass" => BlockId::GLASS,
        "water" => BlockId::WATER,
        "metal" => BlockId::METAL,
        _ => return None,
    })
}

/// The name written to a file for a block id (inverse of `block_id_from_name`).
///
/// Delegates to `BlockId::name()` rather than keeping a third copy of the table.
/// Commit 0b02fe9 already caught the editor calling `LAMP` "air" because
/// `main.rs` carried its own stale match; this module carried a second full copy
/// that happened to agree, which is the same bug waiting for the next block.
/// Saving a map and naming it in the HUD now cannot disagree by construction.
pub fn block_name(b: BlockId) -> &'static str {
    b.name()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Save → load has to be lossless for every block a player can place. The
    /// parser is still a hand-written match (it has to be: it maps *file text*,
    /// including names we may retire, onto ids), so this is what keeps it from
    /// drifting away from `BlockId::name()` the way `main.rs` did in 0b02fe9.
    /// A missing arm here does not crash: `main.rs::apply_map` drops those
    /// voxels into its `skipped` counter, which lumps them together with
    /// out-of-range and out-of-chunk blocks and never names the block — so the
    /// village quietly loses a material and the log says only a number.
    #[test]
    fn every_placeable_block_round_trips_through_its_name() {
        for &id in BlockId::ALL_PLACEABLE {
            let name = block_name(id);
            assert_ne!(name, "unknown", "block {} has no name to save", id.0);
            assert_eq!(
                block_id_from_name(name),
                Some(id),
                "{name} saves but does not load back"
            );
        }
        assert_eq!(block_id_from_name("air"), Some(BlockId::AIR));
    }

    /// Hand-written maps are written by people and agents, not by the editor.
    #[test]
    fn names_are_read_case_and_space_insensitively() {
        assert_eq!(
            block_id_from_name("  CobbleStone \n"),
            Some(BlockId::COBBLESTONE)
        );
        assert_eq!(block_id_from_name("Lamp"), Some(BlockId::LAMP));
        assert_eq!(
            block_id_from_name("glass"),
            None,
            "unknown names must not guess"
        );
    }

    /// The map the game actually ships (`maps/edhari.json`) must parse with
    /// every one of its block names known — an unknown name is a hole in the
    /// village, and the loader reports it rather than filling it in.
    #[test]
    fn the_shipped_village_map_uses_only_known_blocks() {
        let text = include_str!("../../maps/edhari.json");
        let map = parse(text).expect("edhari.json must parse");
        assert_eq!(map.version, MAP_VERSION);
        for b in &map.blocks {
            assert!(
                block_id_from_name(&b.block).is_some(),
                "edhari.json places unknown block {:?} at ({}, {}, {})",
                b.block,
                b.x,
                b.y,
                b.z
            );
        }
    }
}
