//! File-backed block texture atlas: per-face UVs, mipmapped, bleed-free.
//!
//! ## Why this module exists
//!
//! `voxel.rs` already paints blocks, but it paints them **from code**: every
//! tile is a `tile_shade` match arm compiled into the binary. That is fine for
//! the gameplay chunks and useless to an artist — there is no file to replace.
//! The hero/beauty scene (`hero.rs`) does not even go through that path: it
//! spawns one flat-shaded `Cuboid` per cell wearing a solid `base_color`, which
//! is why the signed-off beauty frame reads as untextured colour blocks with a
//! block-grid checker (adjacent cells carry deliberately-paired tones).
//!
//! This module is the file half. It reads a manifest and a folder of 16×16 PNGs
//! at startup and hands back:
//!
//! * one atlas `Image` with a real mip chain, and
//! * per-**kind** UV windows split into `top` / `side` / `bottom`, and
//! * [`cube_mesh`], a unit cube whose six faces already address those windows.
//!
//! ## Swapping the art without touching Rust
//!
//! `assets/textures/blocks/atlas.json` is the whole contract:
//!
//! ```json
//! {
//!   "tile_px": 16,
//!   "tiles": [ { "name": "oak_planks", "file": "oak_planks.png" } ],
//!   "kinds": { "plank": { "all": "oak_planks" } }
//! }
//! ```
//!
//! A new art set drops its PNGs in the same folder and edits `tiles`/`kinds`.
//! Nothing here is compiled in: tile order, tile count, file names and the
//! kind→face mapping are all data. `VOXELFORGE_ATLAS_DIR` repoints the folder
//! for an A/B without even editing the manifest.
//!
//! ## No bleeding, with mipmaps
//!
//! Naively packing 16×16 tiles edge-to-edge and generating mipmaps is the
//! classic voxel-atlas bug: at mip 2 a texel is the average of a 4×4 block, and
//! near a tile border that block straddles the neighbouring tile, so grass
//! smears dirt-brown into stone at distance. Insetting UVs by half a texel does
//! not fix it — the mip texel is already wrong before sampling.
//!
//! The fix here is structural. Each tile is packed into a **cell twice its
//! size**, filled with the tile wrapped 2×2 and offset by half a tile, and the
//! UV window addresses only the centre `tile_px` square:
//!
//! ```text
//!  cell = 32×32                UV window = centre 16×16
//!  ┌────────┬────────┐         ┌────────┐
//!  │ tile   │ tile   │         │        │  the window's content is the tile,
//!  ├────────┼────────┤   →     │  tile  │  and everything a filter can reach
//!  │ tile   │ tile   │         │        │  outside it is the SAME tile wrapped
//!  └────────┴────────┘         └────────┘
//! ```
//!
//! Two properties follow, and both are asserted by the tests below:
//!
//! 1. Cell size stays even at every mip level used ([`MIP_LEVELS`] = 4 → 32, 16,
//!    8, 4), so a plain 2×2 box downsample never averages across a cell border.
//!    Each tile's mip chain is therefore *its own*, not its neighbour's.
//! 2. The gutter around the window is ≥ 1 texel at the coarsest level, so a
//!    linear/trilinear filter walking off the window edge lands on the wrapped
//!    copy of the same tile — exactly what `AddressMode::Repeat` would give a
//!    standalone texture — and never on a different block.
//!
//! Cost of the gutter is 4× atlas memory. At 12 tiles that is 384×32 RGBA
//! ≈ 49 KB before mips; irrelevant.
//!
//! ## Filtering
//!
//! `mag = Nearest` — this is pixel art and a block face one metre from the
//! camera must show hard texels, not a blur. `min`/`mipmap = Linear` — a 16 px
//! tile on a far block covers well under one screen pixel, and nearest
//! minification on that is a shimmer generator. Nearest up close, trilinear far
//! away is the standard pairing and neither half is negotiable.
//!
//! ## Modes (env `VOXELFORGE_ATLAS_MODE`)
//!
//! * `detail` (default) — each tile is normalised to **unit mean luminance**, so
//!   it multiplies a material's existing `base_color` without moving its average
//!   brightness. The signed-off beauty grade survives; the block gains grain.
//! * `albedo` — tiles are used raw and callers are expected to drop `base_color`
//!   to white, so the art set's own palette drives the frame.
//! * `off` — no atlas at all; the caller keeps its flat materials.
//!
//! The mode is read once and applies to the whole atlas, so a look A/B is an env
//! flip on one binary rather than a rebuild.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::MeshBuilder;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;

/// Where the manifest and its tiles live, relative to the working directory.
pub const DEFAULT_ATLAS_DIR: &str = "assets/textures/blocks";

/// Mip levels the packed atlas carries.
///
/// Four, not "as many as fit": the gutter halves with every level, and at level
/// 4 a 32 px cell is down to 2 px — a 1 px window with a 0.5 px gutter, which a
/// linear filter walks straight out of. Four levels take a 16 px tile to 2 px,
/// far past the point where a block is a pixel on screen.
pub const MIP_LEVELS: u32 = 4;

// ---------------------------------------------------------------------------
// manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TileEntry {
    name: String,
    file: String,
}

/// Which tile each face of a kind wears. `all` is shorthand for "the same tile
/// on every face" and is what most materials want.
#[derive(Debug, Deserialize)]
struct KindEntry {
    #[serde(default)]
    all: Option<String>,
    #[serde(default)]
    top: Option<String>,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    bottom: Option<String>,
    /// `"cross"` marks a cross-quad billboard kind (the seven vegetation
    /// sprites); absent (or anything else) means a cube kind. Read only by
    /// [`TileSet::cross`] / the foliage scatter.
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    tile_px: u32,
    tiles: Vec<TileEntry>,
    kinds: BTreeMap<String, KindEntry>,
}

/// How the atlas modulates a material — see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasMode {
    Off,
    Detail,
    Albedo,
}

impl AtlasMode {
    /// Read `VOXELFORGE_ATLAS_MODE`. Anything unrecognised falls back to
    /// `Detail` **loudly** — a typo silently disabling the texture pass is the
    /// kind of thing that gets shipped and then blamed on the art.
    pub fn from_env() -> Self {
        match std::env::var("VOXELFORGE_ATLAS_MODE")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "" | "detail" => AtlasMode::Detail,
            "albedo" | "raw" => AtlasMode::Albedo,
            "off" | "0" | "none" => AtlasMode::Off,
            other => {
                println!("ATLAS_MODE unknown value {other:?} — using detail");
                AtlasMode::Detail
            }
        }
    }
}

// ---------------------------------------------------------------------------
// built atlas
// ---------------------------------------------------------------------------

/// A UV window into the atlas: `[u0, v0, u1, v1]`, already inset to the cell
/// centre so the caller never has to know about the gutter.
pub type UvRect = [f32; 4];

/// Which tile index each face of a kind wears, into [`TileSet::tiles`].
#[derive(Clone, Copy, Debug)]
pub struct FaceTiles {
    pub top: usize,
    pub side: usize,
    pub bottom: usize,
}

/// The manifest decoded but **not yet packed**: raw tiles plus the kind→face
/// mapping.
///
/// [`load`] packs these into one atlas image, which is what the hero/beauty
/// scene wants (one material, one draw call, cube meshes addressing windows).
/// The gameplay chunk mesher wants the opposite — one *standalone repeating*
/// texture per (block, face), because its UVs are measured in blocks so a merged
/// 12×3 quad tiles the texture 12×3 times. A window inside a shared atlas cannot
/// do that at any address mode.
///
/// So the two consumers share the **art** (this struct: the same manifest, the
/// same PNGs, the same face mapping) and part ways only at packing. That is the
/// line the split is drawn on: one source of tiles, two ways to address them.
pub struct TileSet {
    pub tile_px: u32,
    /// Raw decoded RGBA, `tile_px²` each, in manifest order. Not normalised —
    /// [`load`] applies [`AtlasMode::Detail`] on its own copy, so a caller that
    /// wants the artist's albedo verbatim gets it.
    pub tiles: Vec<Vec<u8>>,
    /// Each tile's `file` from the manifest, same order and length as [`Self::tiles`].
    ///
    /// Kept because a decoded RGBA buffer cannot tell you what it was called, and
    /// the PBR pass in `voxel.rs` needs exactly that: an authored normal map is
    /// found by *name* (`oak_planks.png` → `oak_planks_n.png`), the one convention
    /// that lets an artist add maps by dropping files in the folder without also
    /// editing `atlas.json` and without this module growing a second manifest
    /// schema for them. Storing the name here rather than re-reading the manifest
    /// downstream keeps `atlas.json` parsed in exactly one place.
    pub files: Vec<String>,
    pub kinds: BTreeMap<String, FaceTiles>,
    /// Cross-quad billboard kinds, `kind` → albedo file (relative to the atlas
    /// dir), from manifest entries declaring `"mode": "cross"`. The seven
    /// vegetation sprites live here; the foliage scatter reads them so the art
    /// stays editable in `atlas.json` with no Rust change.
    pub cross: BTreeMap<String, String>,
    pub mode: AtlasMode,
    pub dir: PathBuf,
}

impl TileSet {
    /// The raw tile one face of `kind` wears, or `None` if the manifest has no
    /// such kind.
    pub fn face_tile(&self, kind: &str, face: Face) -> Option<&[u8]> {
        Some(&self.tiles[self.face_index(kind, face)?])
    }

    /// That same face's source file name, for finding its companion PBR maps.
    pub fn face_file(&self, kind: &str, face: Face) -> Option<&str> {
        Some(self.files[self.face_index(kind, face)?].as_str())
    }

    fn face_index(&self, kind: &str, face: Face) -> Option<usize> {
        let f = self.kinds.get(kind)?;
        Some(match face {
            Face::Top => f.top,
            Face::Side => f.side,
            Face::Bottom => f.bottom,
        })
    }

    /// The `(kind, albedo file)` pairs whose manifest entry declares
    /// `"mode": "cross"` — the cross-quad billboard vegetation. Empty when the
    /// manifest has none (or predates the render mode).
    pub fn cross_kinds(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.cross.iter().map(|(k, f)| (k.as_str(), f.as_str()))
    }
}

/// The three faces a block kind distinguishes.
///
/// Not six: a voxel block's four sides are the same tile in every art set worth
/// shipping, and a per-side split would triple the material count to express a
/// difference no one has asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Top,
    Side,
    Bottom,
}

impl Face {
    /// All three, in the order their material-table slots run.
    pub const ALL: [Face; 3] = [Face::Top, Face::Side, Face::Bottom];

    /// Dense 0..3 index — the offset inside a per-block material/mesh group.
    #[inline]
    pub fn index(self) -> usize {
        match self {
            Face::Top => 0,
            Face::Side => 1,
            Face::Bottom => 2,
        }
    }

    /// The face a quad shows, from its sweep axis and winding.
    ///
    /// `axis` is the sweep's `d` (0=x, 1=y, 2=z) and `front` is the +axis
    /// winding. Only the y axis distinguishes top from bottom; every other
    /// normal is a side.
    #[inline]
    pub fn from_quad(axis: usize, front: bool) -> Face {
        match (axis, front) {
            (1, true) => Face::Top,
            (1, false) => Face::Bottom,
            _ => Face::Side,
        }
    }
}

/// The three windows one block kind needs.
#[derive(Clone, Copy, Debug)]
pub struct FaceUv {
    pub top: UvRect,
    pub side: UvRect,
    pub bottom: UvRect,
}

/// Everything the renderer needs: the packed image and the per-kind windows.
pub struct BlockAtlas {
    pub image: Image,
    pub kinds: BTreeMap<String, FaceUv>,
    pub mode: AtlasMode,
    /// Source folder, printed so a runlog records which art set was baked in.
    pub dir: PathBuf,
}

impl BlockAtlas {
    pub fn face_uv(&self, kind: &str) -> Option<&FaceUv> {
        self.kinds.get(kind)
    }
}

/// Read the manifest and decode its tiles — everything [`load`] does except the
/// packing, and the entry point for a caller that packs differently.
///
/// Returns `Ok(None)` when the mode is `off`.
pub fn load_tiles(dir: Option<&Path>) -> Result<Option<TileSet>, String> {
    let mode = AtlasMode::from_env();
    if mode == AtlasMode::Off {
        println!("ATLAS mode=off — flat materials kept");
        return Ok(None);
    }
    let dir: PathBuf = dir.map(Path::to_path_buf).unwrap_or_else(|| {
        std::env::var("VOXELFORGE_ATLAS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_ATLAS_DIR))
    });

    let manifest_path = dir.join("atlas.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", manifest_path.display()))?;

    let tile_px = manifest.tile_px;
    if tile_px == 0 || tile_px % 2 != 0 {
        return Err(format!("tile_px must be even and non-zero, got {tile_px}"));
    }
    if manifest.tiles.is_empty() {
        return Err("manifest lists no tiles".into());
    }

    // ---- decode every tile ------------------------------------------------
    let mut tiles: Vec<Vec<u8>> = Vec::with_capacity(manifest.tiles.len());
    let mut files: Vec<String> = Vec::with_capacity(manifest.tiles.len());
    let mut index_of: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, entry) in manifest.tiles.iter().enumerate() {
        let path = dir.join(&entry.file);
        let img = image::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let rgba = img.to_rgba8();
        if rgba.width() != tile_px || rgba.height() != tile_px {
            return Err(format!(
                "{}: expected {tile_px}×{tile_px}, got {}×{}",
                path.display(),
                rgba.width(),
                rgba.height()
            ));
        }
        tiles.push(rgba.into_raw());
        files.push(entry.file.clone());
        index_of.insert(entry.name.as_str(), i);
    }

    // ---- resolve kind → face tile indices ---------------------------------
    let mut kinds = BTreeMap::new();
    for (kind, spec) in &manifest.kinds {
        let pick = |slot: &Option<String>, what: &str| -> Result<usize, String> {
            let name = slot
                .as_deref()
                .or(spec.all.as_deref())
                .ok_or_else(|| format!("kind {kind:?} has no tile for {what} and no `all`"))?;
            index_of
                .get(name)
                .copied()
                .ok_or_else(|| format!("kind {kind:?} names unknown tile {name:?}"))
        };
        kinds.insert(
            kind.clone(),
            FaceTiles {
                top: pick(&spec.top, "top")?,
                side: pick(&spec.side, "side")?,
                bottom: pick(&spec.bottom, "bottom")?,
            },
        );
    }

    // ---- resolve cross-quad billboard kinds ---------------------------------
    let mut cross = BTreeMap::new();
    for (kind, spec) in &manifest.kinds {
        if spec.mode.as_deref() != Some("cross") {
            continue;
        }
        let Some(&i) = spec.all.as_deref().and_then(|n| index_of.get(n)) else {
            return Err(format!(
                "kind {kind:?} declares \"mode\": \"cross\" but has no `all` tile"
            ));
        };
        cross.insert(kind.clone(), files[i].clone());
    }

    Ok(Some(TileSet {
        tile_px,
        tiles,
        files,
        kinds,
        cross,
        mode,
        dir,
    }))
}

/// Load + pack the atlas, or explain why not.
///
/// Returns `Ok(None)` when the mode is `off`, so a caller can treat "no atlas"
/// and "atlas disabled" the same way without matching on the mode itself.
pub fn load(dir: Option<&Path>) -> Result<Option<BlockAtlas>, String> {
    let Some(set) = load_tiles(dir)? else {
        return Ok(None);
    };
    let TileSet {
        tile_px,
        mut tiles,
        kinds: face_tiles,
        mode,
        dir,
        // The packed-atlas consumer addresses tiles by index; only the per-face
        // material path in `voxel.rs` cares what they were called.
        files: _,
        cross: _,
    } = set;

    // `Detail` normalises a *copy*: `load_tiles` hands back the artist's albedo
    // verbatim, and only the packed-atlas consumer wants it rescaled.
    if mode == AtlasMode::Detail {
        for t in &mut tiles {
            normalise_to_unit_luma(t);
        }
    }

    // ---- kind → face UV windows -------------------------------------------
    let cell_px = tile_px * 2;
    let atlas_w = cell_px * tiles.len() as u32;
    let atlas_h = cell_px;
    let mut kinds = BTreeMap::new();
    for (kind, f) in &face_tiles {
        kinds.insert(
            kind.clone(),
            FaceUv {
                top: window_uv(f.top, tile_px, atlas_w, atlas_h),
                side: window_uv(f.side, tile_px, atlas_w, atlas_h),
                bottom: window_uv(f.bottom, tile_px, atlas_w, atlas_h),
            },
        );
    }

    // ---- pack + mip -------------------------------------------------------
    let mip0 = pack_cells(&tiles, tile_px);
    let data = build_mip_chain(mip0, atlas_w, atlas_h, MIP_LEVELS);

    let mut image = Image::new_uninit(
        Extent3d {
            width: atlas_w,
            height: atlas_h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = MIP_LEVELS;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        // Nearest up close (pixel art), trilinear far away (no shimmer).
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        // Clamp, not Repeat: every UV addresses a window strictly inside the
        // atlas, so wrapping could only ever be a bug hiding itself.
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });

    println!(
        "ATLAS mode={mode:?} dir={} tiles={} kinds={} size={atlas_w}x{atlas_h} mips={MIP_LEVELS}",
        dir.display(),
        tiles.len(),
        kinds.len()
    );
    Ok(Some(BlockAtlas {
        image,
        kinds,
        mode,
        dir,
    }))
}

/// The UV window of cell `i`: the centre `tile_px` square of its `2·tile_px` cell.
fn window_uv(i: usize, tile_px: u32, atlas_w: u32, atlas_h: u32) -> UvRect {
    let cell = tile_px * 2;
    let x0 = i as u32 * cell + tile_px / 2;
    let y0 = tile_px / 2;
    [
        x0 as f32 / atlas_w as f32,
        y0 as f32 / atlas_h as f32,
        (x0 + tile_px) as f32 / atlas_w as f32,
        (y0 + tile_px) as f32 / atlas_h as f32,
    ]
}

/// Scale a tile so its mean luminance is 1.0 in linear light.
///
/// This is what lets a texture land on a signed-off frame without relighting it:
/// the tile then multiplies a material's `base_color` by an average of one, so
/// the block's mean albedo is exactly what it was and only its *variation* is
/// new. Done in linear space because sRGB bytes are not proportional to light —
/// averaging them and calling it brightness is the usual way a "neutral" texture
/// quietly darkens a scene.
fn normalise_to_unit_luma(px: &mut [u8]) {
    let mut sum = 0.0f64;
    let n = (px.len() / 4) as f64;
    for p in px.chunks_exact(4) {
        sum += (0.2126 * srgb_to_linear(p[0])
            + 0.7152 * srgb_to_linear(p[1])
            + 0.0722 * srgb_to_linear(p[2])) as f64;
    }
    let mean = (sum / n) as f32;
    if mean <= f32::EPSILON {
        return;
    }
    let gain = 1.0 / mean;
    for p in px.chunks_exact_mut(4) {
        for c in 0..3 {
            p[c] = linear_to_srgb(srgb_to_linear(p[c]) * gain);
        }
    }
}

#[inline]
fn srgb_to_linear(b: u8) -> f32 {
    let s = b as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn linear_to_srgb(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let s = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Lay every tile into its own `2·tile_px` cell, wrapped and half-tile offset.
///
/// The offset is what makes the *centre* window a faithful copy of the tile: a
/// plain 2×2 tiling would put the tile's own seam through the middle of the
/// window. With the offset, window texel `(0,0)` is tile texel `(0,0)` and the
/// gutter is the tile continuing past its own edge.
fn pack_cells(tiles: &[Vec<u8>], tile_px: u32) -> Vec<u8> {
    let cell = tile_px * 2;
    let w = cell * tiles.len() as u32;
    let mut out = vec![0u8; (w * cell * 4) as usize];
    let half = tile_px / 2;
    for (i, tile) in tiles.iter().enumerate() {
        let ox = i as u32 * cell;
        for cy in 0..cell {
            for cx in 0..cell {
                let tx = (cx + tile_px - half) % tile_px;
                let ty = (cy + tile_px - half) % tile_px;
                let src = ((ty * tile_px + tx) * 4) as usize;
                let dst = (((cy * w) + ox + cx) * 4) as usize;
                out[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
            }
        }
    }
    out
}

/// Append `levels - 1` box-downsampled mips after `mip0`.
///
/// The downsample is a plain global 2×2 average, which is only safe because
/// every cell is `2·tile_px` wide and stays even for all [`MIP_LEVELS`] levels —
/// so a 2×2 footprint can never straddle two cells. `debug_assert` holds the
/// invariant rather than a comment alone.
fn build_mip_chain(mip0: Vec<u8>, w: u32, h: u32, levels: u32) -> Vec<u8> {
    let mut out = mip0;
    let mut prev_start = 0usize;
    let (mut pw, mut ph) = (w, h);
    for _ in 1..levels {
        debug_assert!(pw % 2 == 0 && ph % 2 == 0, "mip {pw}×{ph} would straddle cells");
        let (nw, nh) = (pw / 2, ph / 2);
        let mut level = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    // Average in LINEAR light, not in sRGB bytes: a mip built by
                    // averaging sRGB is systematically too dark, which is how a
                    // textured wall gains a grey cast as the camera pulls back.
                    let s = |dx: u32, dy: u32| -> f32 {
                        let i = prev_start + ((((y * 2 + dy) * pw) + x * 2 + dx) * 4 + c) as usize;
                        if c == 3 {
                            out[i] as f32 / 255.0
                        } else {
                            srgb_to_linear(out[i])
                        }
                    };
                    let avg = (s(0, 0) + s(1, 0) + s(0, 1) + s(1, 1)) * 0.25;
                    let o = ((y * nw + x) * 4 + c) as usize;
                    level[o] = if c == 3 {
                        (avg * 255.0).round() as u8
                    } else {
                        linear_to_srgb(avg)
                    };
                }
            }
        }
        prev_start = out.len();
        out.extend_from_slice(&level);
        pw = nw;
        ph = nh;
    }
    out
}

// ---------------------------------------------------------------------------
// mesh
// ---------------------------------------------------------------------------

/// A unit cube whose six faces sample `uv`'s top / side / bottom windows.
///
/// Built by rewriting the UVs of Bevy's own `Cuboid` mesh rather than emitting
/// vertices by hand. That is deliberate: the cuboid's winding, index order and
/// per-face normals are already correct, and a hand-rolled cube that gets one
/// face's winding backwards fails as an invisible hole in a render — an
/// expensive way to find a typo when a build is twenty minutes.
///
/// Each vertex keeps its own 0..1 face UV; only the remap into the window is new,
/// so a face's texel orientation is whatever Bevy already chose.
pub fn cube_mesh(uv: &FaceUv) -> Mesh {
    let mut mesh = Cuboid::new(1.0, 1.0, 1.0).mesh().build();
    let normals: Vec<[f32; 3]> = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|a| a.as_float3())
        .expect("cuboid mesh has normals")
        .to_vec();
    let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(bevy::render::mesh::VertexAttributeValues::Float32x2(v)) => v.clone(),
        _ => panic!("cuboid mesh has no UV_0"),
    };
    let remapped: Vec<[f32; 2]> = normals
        .iter()
        .zip(uvs.iter())
        .map(|(n, t)| {
            let w = if n[1] > 0.5 {
                uv.top
            } else if n[1] < -0.5 {
                uv.bottom
            } else {
                uv.side
            };
            [
                w[0] + t[0] * (w[2] - w[0]),
                w[1] + t[1] * (w[3] - w[1]),
            ]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, remapped);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tile whose texels are all distinct so a bleed shows up as a value that
    /// could only have come from the neighbour.
    fn flat_tile(px: u32, value: u8) -> Vec<u8> {
        let mut v = vec![255u8; (px * px * 4) as usize];
        for p in v.chunks_exact_mut(4) {
            p[0] = value;
            p[1] = value;
            p[2] = value;
        }
        v
    }

    #[test]
    fn window_is_the_tile_and_the_gutter_is_its_wrap() {
        let px = 16u32;
        // A tile with a unique value per texel, so any mix-up is detectable.
        let mut tile = vec![255u8; (px * px * 4) as usize];
        for y in 0..px {
            for x in 0..px {
                let i = ((y * px + x) * 4) as usize;
                tile[i] = (y * px + x) as u8;
                tile[i + 1] = 0;
                tile[i + 2] = 0;
            }
        }
        let packed = pack_cells(&[tile.clone()], px);
        let cell = px * 2;
        let half = px / 2;
        for y in 0..px {
            for x in 0..px {
                let src = ((y * px + x) * 4) as usize;
                let dst = (((y + half) * cell + x + half) * 4) as usize;
                assert_eq!(
                    packed[dst], tile[src],
                    "window texel ({x},{y}) is not the tile's own"
                );
            }
        }
        // And a texel one step outside the window is the tile wrapped, i.e. the
        // last row — not black, and not another tile.
        let above = (((half - 1) * cell + half) * 4) as usize;
        assert_eq!(packed[above], tile[(((px - 1) * px) * 4) as usize]);
    }

    /// The load-bearing anti-bleed claim: mip a two-tile atlas of pure 0 and
    /// pure 255 and assert neither tile's window ever moves toward the other.
    #[test]
    fn mips_never_average_across_a_cell() {
        let px = 16u32;
        let tiles = vec![flat_tile(px, 0), flat_tile(px, 255)];
        let cell = px * 2;
        let (w, h) = (cell * 2, cell);
        let data = build_mip_chain(pack_cells(&tiles, px), w, h, MIP_LEVELS);

        let mut off = 0usize;
        let (mut lw, mut lh) = (w, h);
        for level in 0..MIP_LEVELS {
            let lcell = cell >> level;
            for (i, expect) in [0u8, 255u8].into_iter().enumerate() {
                for y in 0..lh {
                    for x in (i as u32 * lcell)..((i as u32 + 1) * lcell) {
                        let v = data[off + (((y * lw) + x) * 4) as usize];
                        assert_eq!(
                            v, expect,
                            "mip {level} cell {i} texel ({x},{y}) = {v}, bled from its neighbour"
                        );
                    }
                }
            }
            off += (lw * lh * 4) as usize;
            lw /= 2;
            lh /= 2;
        }
        assert_eq!(off, data.len(), "mip chain length does not match its levels");
    }

    /// Every gutter is at least one texel at the coarsest level — the condition
    /// that keeps a linear filter inside its own cell.
    #[test]
    fn coarsest_gutter_is_at_least_one_texel() {
        let tile_px = 16u32;
        let coarsest_cell = (tile_px * 2) >> (MIP_LEVELS - 1);
        let coarsest_window = tile_px >> (MIP_LEVELS - 1);
        assert!(coarsest_window >= 1, "window collapses at the coarsest mip");
        assert!(
            (coarsest_cell - coarsest_window) / 2 >= 1,
            "gutter is {} texel(s) at mip {}",
            (coarsest_cell - coarsest_window) / 2,
            MIP_LEVELS - 1
        );
    }

    /// Unit-mean normalisation must preserve mean luminance, not merely look
    /// like it does: a texture that quietly darkens is exactly the failure this
    /// mode exists to avoid.
    #[test]
    fn detail_mode_holds_mean_luminance_at_one() {
        let px = 16u32;
        let mut tile = vec![255u8; (px * px * 4) as usize];
        for (i, p) in tile.chunks_exact_mut(4).enumerate() {
            let v = (30 + (i % 90)) as u8;
            p[0] = v;
            p[1] = v;
            p[2] = v;
        }
        normalise_to_unit_luma(&mut tile);
        let mut sum = 0.0f64;
        for p in tile.chunks_exact(4) {
            sum += (0.2126 * srgb_to_linear(p[0])
                + 0.7152 * srgb_to_linear(p[1])
                + 0.0722 * srgb_to_linear(p[2])) as f64;
        }
        let mean = sum / (px * px) as f64;
        // 8-bit re-quantisation is the only error left; 2 % is generous for it.
        assert!(
            (mean - 1.0).abs() < 0.02,
            "mean luminance after normalise = {mean}, wanted 1.0"
        );
    }

    /// A UV window must sit strictly inside its own cell on both axes, or the
    /// gutter it was given is not actually protecting it.
    #[test]
    fn windows_stay_inside_their_cell() {
        let (tile_px, n) = (16u32, 5usize);
        let cell = tile_px * 2;
        let (w, h) = (cell * n as u32, cell);
        for i in 0..n {
            let [u0, v0, u1, v1] = window_uv(i, tile_px, w, h);
            let cell_u0 = (i as u32 * cell) as f32 / w as f32;
            let cell_u1 = ((i as u32 + 1) * cell) as f32 / w as f32;
            assert!(u0 > cell_u0 && u1 < cell_u1, "cell {i} window escapes in u");
            assert!(v0 > 0.0 && v1 < 1.0, "cell {i} window escapes in v");
        }
    }
}
