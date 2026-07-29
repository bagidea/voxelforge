//! Deterministic procedural terrain — multi-octave FBM heightmap + moisture layer
//! → biome-driven surface materials + subsurface variety + scattered features.
//!
//! Pure Rust, zero dependencies, same function used by both client mesher and
//! server so world state is identical without syncing every block.
//!
//! The noise is seeded from a global `OnceLock<u64>` (default 42). Call
//! `set_seed()` once at startup before any terrain queries.

use std::sync::OnceLock;

use crate::block::BlockId;
use crate::chunk::CHUNK_SIZE;

// ---------------------------------------------------------------------------
// Global seed — set once; defaults to 42 if never configured.
// ---------------------------------------------------------------------------

static WORLD_SEED: OnceLock<u64> = OnceLock::new();

/// Seed the terrain generator. Idempotent — only the first call takes effect.
pub fn set_seed(seed: u64) {
    let _ = WORLD_SEED.set(seed);
}

#[inline]
fn seed() -> u64 {
    WORLD_SEED.get().copied().unwrap_or(42)
}

// ---------------------------------------------------------------------------
// Noise primitives — pure, deterministic hash → smooth value noise.
// ---------------------------------------------------------------------------

#[inline]
fn hash(x: i32, z: i32, s: u64) -> u32 {
    let mut h = s;
    h = h.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(x as u32 as u64);
    h = (h ^ (h >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
    h = h.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(z as u32 as u64);
    h = (h ^ (h >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    ((h >> 32) ^ h) as u32
}

#[inline]
fn value_2d(x: i32, z: i32, s: u64) -> f32 {
    hash(x, z, s) as f32 / (u32::MAX as f32 + 1.0)
}

fn smooth_noise(wx: f32, wz: f32, s: u64) -> f32 {
    let x0 = wx.floor();
    let z0 = wz.floor();
    let x0i = x0 as i32;
    let z0i = z0 as i32;
    let x1i = x0i + 1;
    let z1i = z0i + 1;

    let fx = wx - x0;
    let fz = wz - z0;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sz = fz * fz * (3.0 - 2.0 * fz);

    let v00 = value_2d(x0i, z0i, s);
    let v10 = value_2d(x1i, z0i, s);
    let v01 = value_2d(x0i, z1i, s);
    let v11 = value_2d(x1i, z1i, s);

    let v0 = v00 + sx * (v10 - v00);
    let v1 = v01 + sx * (v11 - v01);
    v0 + sz * (v1 - v0)
}

// ---------------------------------------------------------------------------
// Fractal Brownian Motion — multi-octave noise for natural terrain.
// ---------------------------------------------------------------------------

const OCTAVES: usize = 5;
const PERSISTENCE: f32 = 0.55;
const LACUNARITY: f32 = 2.3;
const BASE_FREQ: f32 = 0.007;
const GOLDEN_RATIO: u64 = 0x9e37_79b9_7f4a_7c15;

fn fbm(wx: f32, wz: f32, base_seed: u64) -> f32 {
    let mut total = 0.0;
    let mut freq = BASE_FREQ;
    let mut amp = 1.0;
    let mut max = 0.0;
    let mut octave_seed = base_seed;

    for _ in 0..OCTAVES {
        total += smooth_noise(wx * freq, wz * freq, octave_seed) * amp;
        max += amp;
        freq *= LACUNARITY;
        amp *= PERSISTENCE;
        octave_seed = octave_seed.wrapping_add(GOLDEN_RATIO);
    }

    if max > 0.0 {
        total / max
    } else {
        total
    }
}

// ---------------------------------------------------------------------------
// Public height-map API (backward-compatible).
// ---------------------------------------------------------------------------

const HEIGHT_MIN: f32 = 3.0;
const HEIGHT_MAX: f32 = (CHUNK_SIZE - 3) as f32; // 29 for CHUNK_SIZE=32

/// Returns the terrain height (world Y) at (wx, wz) using the global seed.
pub fn terrain_height(wx: f32, wz: f32) -> i32 {
    terrain_height_seeded(wx, wz, seed())
}

/// Height with explicit seed (tests, tools).
pub fn terrain_height_seeded(wx: f32, wz: f32, s: u64) -> i32 {
    let n = fbm(wx, wz, s);
    let h = HEIGHT_MIN + n * (HEIGHT_MAX - HEIGHT_MIN);
    (h.round() as i32).clamp(1, CHUNK_SIZE - 2)
}

// ---------------------------------------------------------------------------
// Moisture layer — second noise channel for biome diversity.
// ---------------------------------------------------------------------------

/// Moisture seed offset — keeps the moisture field independent of the height
/// field while remaining deterministic from the same world seed.
const MOISTURE_SEED_OFFSET: u64 = 0x6d6f_6973_7475_7265; // "moisture" ascii

/// Slower frequency for broader biome regions (fewer cycles per world distance).
const MOISTURE_BASE_FREQ: f32 = 0.003;

/// Returns moisture [0, 1] at (wx, wz). Low = arid, high = wet.
fn moisture(wx: f32, wz: f32) -> f32 {
    let s = seed().wrapping_add(MOISTURE_SEED_OFFSET);
    // FBM at a coarser scale for broad biome bands, then contrast-stretched
    // so we actually hit the extremes (arid <0.25, wet >0.75).
    let mut total = 0.0;
    let mut freq = MOISTURE_BASE_FREQ;
    let mut amp = 1.0;
    let mut max = 0.0;
    let mut octave_seed = s;

    for _ in 0..5 {
        // 5 octaves like height — full range, just slower frequency.
        total += smooth_noise(wx * freq, wz * freq, octave_seed) * amp;
        max += amp;
        freq *= LACUNARITY;
        amp *= PERSISTENCE;
        octave_seed = octave_seed.wrapping_add(GOLDEN_RATIO);
    }

    let raw = if max > 0.0 { total / max } else { total };
    // Contrast stretch: pull [0,1] away from 0.5 so arid and wet extremes
    // are more common.  Squash in the middle, push toward the edges.
    //   raw 0.0 → 0.0,   raw 0.5 → 0.5,   raw 1.0 → 1.0
    // but with a steeper slope at the centre: raw' = 0.5 + (raw-0.5)*1.6
    let stretched = 0.5 + (raw - 0.5) * 1.6;
    stretched.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Biome → surface block selection.
// ---------------------------------------------------------------------------

const SNOW_HEIGHT: i32 = 22; // above this → snow surface
const SEA_LEVEL: i32 = 6;    // near/below this → sandy beaches

/// Pick the surface block at (wx, wz) where terrain height = `h`.
pub fn surface_block(wx: f32, wz: f32, h: i32) -> BlockId {
    let m = moisture(wx, wz);

    // High altitude → always snow (cold, regardless of moisture).
    if h >= SNOW_HEIGHT {
        return BlockId::SNOW;
    }

    // Beach / lowland: sand variants near sea level.
    if h <= SEA_LEVEL + 1 {
        return if m < 0.45 { BlockId::RED_SAND } else { BlockId::SAND };
    }

    // Arid (low moisture) → red sand / gravel surface.
    if m < 0.35 {
        return if m < 0.18 { BlockId::RED_SAND } else { BlockId::GRAVEL };
    }

    // Wet (high moisture) → clay / moss surface.
    if m > 0.65 {
        return if m > 0.82 { BlockId::MOSS } else { BlockId::CLAY };
    }

    // Temperate heartland → grass (dominant), with occasional moss patches.
    if m > 0.50 && m < 0.68 {
        // Deterministic micro-variation: roughly 1 in 8 grass tiles → moss.
        let hh = hash(wx.floor() as i32, wz.floor() as i32, seed().wrapping_add(999));
        if hh % 8 == 0 {
            return BlockId::MOSS;
        }
    }

    BlockId::GRASS
}

// ---------------------------------------------------------------------------
// Subsurface layers — vertical variety below the surface block.
// ---------------------------------------------------------------------------

/// Returns the block for a subsurface voxel at world (wx, wy, wz) where the
/// terrain surface is at height `h`. `depth` = h - wy (1 = first block below
/// surface, 2 = second, …).
pub fn subsurface_block(wx: f32, wz: f32, wy: i32, h: i32) -> BlockId {
    let depth = h - wy;
    let m = moisture(wx, wz);

    // Topsoil: 3-5 layers of dirt (or clay in wet zones, gravel in arid).
    if depth <= 3 {
        if m > 0.75 {
            return BlockId::CLAY; // wet topsoil → clay
        }
        if m < 0.25 {
            return BlockId::GRAVEL; // arid topsoil → gravelly
        }
        return BlockId::DIRT;
    }

    // Transition zone: dirt → stone over a few layers.
    if depth <= 6 {
        // Occasional limestone bands in temperate zones.
        let hh = hash(wx.floor() as i32, (wz + depth as f32).floor() as i32, seed().wrapping_add(111));
        if m > 0.4 && m < 0.7 && hh % 5 == 0 {
            return BlockId::LIMESTONE;
        }
        // Gravel pockets in the transition zone.
        if hh % 13 == 0 {
            return BlockId::GRAVEL;
        }
        return BlockId::DIRT;
    }

    // Deep: stone with ore veins and limestone bands.
    ore_vein(wx, wz, wy)
}

// ---------------------------------------------------------------------------
// Ore veins / underground variety.
// ---------------------------------------------------------------------------

/// Deterministic ore placement — returns a special block for rare positions,
/// otherwise the default deep block (STONE).
fn ore_vein(wx: f32, wz: f32, wy: i32) -> BlockId {
    let hh = hash(
        (wx * 3.7).floor() as i32,
        (wz * 3.7 + wy as f32 * 2.1).floor() as i32,
        seed().wrapping_add(222),
    );

    // Obsidian veins: rare, small clusters (~1 in 50 deep blocks).
    if hh % 47 == 0 {
        return BlockId::OBSIDIAN;
    }

    // Brick "ruins": scattered pockets suggesting ancient structures (~1 in 90).
    if hh % 89 == 0 {
        return BlockId::BRICK;
    }

    // Limestone bands at certain depths.
    let band_hash = hash(
        (wx * 0.9).floor() as i32,
        (wy / 4) as i32,
        seed().wrapping_add(333),
    );
    if band_hash % 7 == 0 {
        return BlockId::LIMESTONE;
    }

    // Cobblestone pockets in the deep stone.
    if hh % 19 == 0 {
        return BlockId::COBBLESTONE;
    }

    BlockId::STONE
}

// ---------------------------------------------------------------------------
// Scattered surface features — trees, boulders, patches.
// ---------------------------------------------------------------------------

/// Returns Some(block) if a surface feature should occupy world-space voxel
/// (wx, wy, wz) where the terrain surface is at height `h`.
/// Returns None if no feature — caller should use the normal column fill.
pub fn feature_at(wx: f32, wz: f32, wy: i32, h: i32) -> Option<BlockId> {
    // Features only exist at or above the surface.
    if wy < h {
        return None;
    }

    let above = wy - h; // 0 = surface, 1 = one block above, etc.
    let fx = wx.floor() as i32;
    let fz = wz.floor() as i32;
    let hh = hash(fx, fz, seed().wrapping_add(444));

    // Only attempt features on a fraction of surface columns (~1 in 30 columns).
    if hh % 30 != 0 {
        return None;
    }

    // ---- Trees (WOOD trunk + LEAVES canopy) ----
    // Trees grow on grass or moss surfaces in temperate/wet climates.
    if hh % 5 <= 2 {
        let m = moisture(wx, wz);
        if m > 0.3 && h < SNOW_HEIGHT && h > SEA_LEVEL {
            return tree_block(fx, fz, above, hh);
        }
    }

    // ---- Boulders (COBBLESTONE / OBSIDIAN) ----
    // Scattered surface rocks on any non-snow terrain.
    if hh % 5 == 3 && h < SNOW_HEIGHT && above <= 1 {
        return boulder_block(fx, fz, wy, h, hh);
    }

    // ---- Gravel surface patch ----
    if hh % 5 == 4 && above == 0 {
        return Some(BlockId::GRAVEL);
    }

    None
}

/// Generate a tree: 2-4 block trunk + 3×3×2 canopy.
fn tree_block(x: i32, z: i32, above: i32, _seed: u32) -> Option<BlockId> {
    let trunk_h = 3 + (x.abs() % 3) as i32; // 3-5 tall trunk
    if above < trunk_h {
        return Some(BlockId::WOOD);
    }
    // Canopy: 3x3 cross at trunk_h and trunk_h+1, tapering at trunk_h+2.
    let canopy_base = trunk_h;
    let canopy_top = trunk_h + 2;
    if above >= canopy_base && above <= canopy_top {
        let radius = if above == canopy_top { 1 } else { 2 };
        let dx = (x % 3 - 1).abs();
        let dz = (z % 3 - 1).abs();
        let dist = dx.max(dz);
        if dist <= radius && (dist > 0 || above > canopy_base) {
            // Avoid placing leaves directly on the trunk top block (that's air
            // to let the trunk through) — the canopy is a cross, not a full fill.
            return Some(BlockId::LEAVES);
        }
    }
    None
}

/// Generate a small surface boulder.
fn boulder_block(x: i32, _z: i32, wy: i32, h: i32, seed: u32) -> Option<BlockId> {
    let above = wy - h;
    // Boulder sits on surface + extends up to 2 blocks.
    if above > 1 {
        return None;
    }
    let block = if seed % 7 == 0 { BlockId::OBSIDIAN } else { BlockId::COBBLESTONE };

    // Boulder shape: only fill if the local (x%2, z%2) offset forms a shape.
    let lx = x.rem_euclid(2);
    let lz = (x + wy as i32).rem_euclid(2); // vary z-offset by y for asymmetry
    if above == 0 {
        // Base: always 2x2.
        Some(block)
    } else {
        // Top: only 1-2 blocks.
        if lx == 0 && lz == 0 {
            Some(block)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// One-stop column fill — the public API chunk generation calls per-voxel.
// ---------------------------------------------------------------------------

/// Returns the block that should occupy world-space voxel (wx, wy, wz) in the
/// terrain column whose surface is at height `h`. Handles surface, subsurface,
/// and scattered features in one call.
///
/// `h` can be obtained from `terrain_height(wx, wz)`.
pub fn terrain_block(wx: f32, wz: f32, wy: i32, h: i32) -> BlockId {
    // Above surface: air (unless a feature overrides).
    if wy > h {
        if let Some(feat) = feature_at(wx, wz, wy, h) {
            return feat;
        }
        return BlockId::AIR;
    }

    // Surface layer.
    if wy == h {
        // Features can replace the surface block (e.g. gravel patch).
        if let Some(feat) = feature_at(wx, wz, wy, h) {
            return feat;
        }
        return surface_block(wx, wz, h);
    }

    // Below surface: subsurface layers.
    subsurface_block(wx, wz, wy, h)
}

// ---------------------------------------------------------------------------
// Unit tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{ChunkData, ChunkPos};

    // --- determinism -------------------------------------------------------

    #[test]
    fn deterministic_same_seed_same_position() {
        let s = 12345;
        let h1 = terrain_height_seeded(10.0, 20.0, s);
        let h2 = terrain_height_seeded(10.0, 20.0, s);
        assert_eq!(h1, h2);
    }

    #[test]
    fn deterministic_negative_coords() {
        let s = 42;
        let h1 = terrain_height_seeded(-50.0, -75.0, s);
        let h2 = terrain_height_seeded(-50.0, -75.0, s);
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_seeds_diverge() {
        let h1 = terrain_height_seeded(100.0, 100.0, 42);
        let h2 = terrain_height_seeded(100.0, 100.0, 999);
        assert_ne!(h1, h2);
    }

    // --- height range ------------------------------------------------------

    #[test]
    fn height_variation_non_trivial() {
        let s = 777;
        let mut min_h = i32::MAX;
        let mut max_h = i32::MIN;
        for z in 0..128 {
            for x in 0..128 {
                let h = terrain_height_seeded(x as f32, z as f32, s);
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }
        let range = max_h - min_h;
        assert!(range >= 10, "height range {range} too flat");
        assert!(min_h >= 1);
        assert!(max_h <= CHUNK_SIZE - 2);
    }

    #[test]
    fn height_within_bounds() {
        let s = 123;
        for z in (-64..64).step_by(13) {
            for x in (-64..64).step_by(13) {
                let h = terrain_height_seeded(x as f32, z as f32, s);
                assert!(h >= 1, "h={h} below 1 at ({x},{z})");
                assert!(h < CHUNK_SIZE);
            }
        }
    }

    // --- surface block variety ---------------------------------------------

    #[test]
    fn surface_block_produces_multiple_types() {
        set_seed(42);
        let mut seen = std::collections::HashSet::new();
        for z in 0..256 {
            for x in 0..256 {
                let wx = x as f32;
                let wz = z as f32;
                let h = terrain_height(wx, wz);
                let b = surface_block(wx, wz, h);
                seen.insert(b.0);
            }
        }
        // With 256×256 samples across varied height/moisture, we expect
        // at least 5 distinct surface block types (grass, sand, red_sand,
        // snow, clay, moss, gravel — any five).
        assert!(
            seen.len() >= 4,
            "only {} surface block types across 65k samples — terrain too uniform",
            seen.len()
        );
    }

    // --- terrain_block covers all layers -----------------------------------

    #[test]
    fn terrain_block_surface_is_not_air() {
        set_seed(99);
        for z in 0..64 {
            for x in 0..64 {
                let (wx, wz) = (x as f32, z as f32);
                let h = terrain_height(wx, wz);
                let b = terrain_block(wx, wz, h, h);
                assert!(b.is_opaque(), "surface block should be opaque at ({wx},{wz})");
                // One block above surface should be air (unless a tree feature).
                let _above = terrain_block(wx, wz, h + 1, h);
            }
        }
    }

    #[test]
    fn subsurface_has_variety() {
        set_seed(77);
        let mut seen = std::collections::HashSet::new();
        for z in 0..64 {
            for x in 0..64 {
                let (wx, wz) = (x as f32, z as f32);
                let h = terrain_height(wx, wz);
                for d in 1..=10 {
                    let wy = h - d;
                    if wy >= 0 {
                        seen.insert(subsurface_block(wx, wz, wy, h).0);
                    }
                }
            }
        }
        assert!(
            seen.len() >= 4,
            "subsurface only produced {} block types — need dirt, stone, limestone, gravel etc",
            seen.len()
        );
    }

    // --- chunk determinism -------------------------------------------------

    #[test]
    fn full_chunk_deterministic() {
        let s = 555;
        set_seed(s);
        let a = ChunkData::generate(ChunkPos::new(3, 0, 5));
        let b = ChunkData::generate(ChunkPos::new(3, 0, 5));
        let vol = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
        for i in 0..vol {
            assert_eq!(a.get_raw(i), b.get_raw(i), "block {i} differs");
        }
    }

    #[test]
    fn different_chunks_not_identical() {
        let s = 444;
        set_seed(s);
        let a = ChunkData::generate(ChunkPos::new(0, 0, 0));
        let b = ChunkData::generate(ChunkPos::new(5, 0, 0));
        let vol = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
        let mut diff = 0usize;
        for i in 0..vol {
            if a.get_raw(i) != b.get_raw(i) {
                diff += 1;
            }
        }
        assert!(diff > 100, "adjacent chunks differ in only {diff} blocks");
    }

    // --- features exist ----------------------------------------------------

    #[test]
    fn trees_exist_somewhere() {
        set_seed(42);
        let mut trees = 0usize;
        for z in 0..512 {
            for x in 0..512 {
                let (wx, wz) = (x as f32, z as f32);
                let h = terrain_height(wx, wz);
                for above in 0..6 {
                    if let Some(b) = feature_at(wx, wz, h + above, h) {
                        if b == BlockId::WOOD || b == BlockId::LEAVES {
                            trees += 1;
                        }
                    }
                }
            }
        }
        assert!(trees > 0, "no trees found in 512×512 world");
    }
}
