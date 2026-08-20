//! Client-side voxel helpers: greedy mesh builder, procedural textures, and the
//! per-block surface table.
//!
//! All block-type and chunk-data definitions live in `voxelforge_sim`; this
//! module only contains the Bevy-coupled surface: Mesh construction, the atlas
//! Image and the StandardMaterial each block wears. Keeping the mesher here (not
//! in sim) prevents a Bevy dependency from leaking into the shared crate.
//!
//! ## Two meshing paths
//!
//! [`greedy_mesh_chunk`] is the original: one mesh, one atlas material, one draw
//! call per chunk. Only the far LOD (`streaming::build_lod_children`) still uses
//! it. Its limitation is structural — a merged quad spanning N blocks still gets
//! ONE atlas tile stretched across it, so the texture cannot repeat per block.
//!
//! [`greedy_mesh_chunk_split`] is the fix, and what the near/editable world is
//! meshed with (`main::remesh_chunk_entity`): one mesh **per block type**, each with
//! its own single-tile texture sampled with `ImageAddressMode::Repeat` and UVs
//! measured in blocks. A 12×3 merged quad then samples the tile 12×3 times — real
//! per-block texel density, and no atlas neighbourhood to bleed from at all. It
//! also unlocks a real [`block_material`] per type (roughness/metallic/emissive),
//! which a single shared material can never express.
//!
//! Both paths carry vertex ambient occlusion.
//!
//! ## Material response
//!
//! The split path additionally wears a per-type **normal map** and
//! **metallic-roughness map** ([`build_face_normal_map`],
//! [`build_face_metallic_roughness`]), both derived from the same
//! [`tile_shade`] pattern that paints the albedo, so colour, relief and finish
//! cannot drift out of register. The atlas/LOD path deliberately gets neither:
//! it stretches one tile across a whole merged quad, so per-texel relief there
//! would smear.
//!
//! ## Per-instance variation
//!
//! Even with per-type material maps, every block of the same type shares the
//! same tile — a long cobblestone wall samples the same roughness and colour
//! every block, reading as one uniform surface. The split path adds a
//! deterministic world-space micro-variation to each quad's vertex colours
//! (subtle per-channel colour noise + micro-AO), so every face gets a
//! slightly different colour cast and the wall reads as individual blocks
//! instead of one flat plane. The atlas/LOD path deliberately gets none:
//! it is a single merged draw far from the camera and has no per-block
//! identity to vary.
//!
//! Block edges are carried by vertex AO alone — the AO curve was deepened so
//! inside corners read without depending on post-FX SSAO.
//!
//! ## Runtime levers (env vars)
//!
//! `VOXELFORGE_FLAT_MATERIAL=1` — base-colour-only (the pre-material look).
//! `VOXELFORGE_FLAT_INSTANCE=1`  — drop per-instance colour noise.
//! `VOXELFORGE_MAT_RELIEF` / `VOXELFORGE_MAT_ROUGH_VAR` — scale amplitudes.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, CHUNK_SIZE};

use crate::block_atlas::{self, Face, TileSet};

/// A client-side decorative block: the wall lantern.
///
/// Defined here first (before `sim/src/block.rs` had id 16); now promoted to a
/// real [`BlockId::LAMP`] const in sim (commit a3bb308), so the local is
/// redundant.  Kept as an alias so existing match arms (`LAMP => …`) don't need a
/// rename — the value is the same constant either way.
///
/// `BlockId(16).name()` returns `"lamp"` (not `"unknown"`) since the promotion,
/// so the lamp CAN round-trip through a JSON map file.
pub const LAMP: BlockId = BlockId::LAMP;

/// Number of block types the texture tables cover — the 16 sim blocks, [`LAMP`],
/// [`BlockId::GLASS`], [`BlockId::WATER`], [`BlockId::METAL`] and the five
/// shaped blocks (ids 20-24, see `client/src/block_shapes.rs`).
///
/// A shaped block needs its whole material row exactly like a cube does: it is
/// meshed separately but textured through the SAME `build_block_materials`
/// table, so leaving this at 20 would index a stair's material out of bounds and
/// fall back to stone.
const N_TILES: usize = 25;

/// Edge length of a **procedurally baked** tile, and nothing else.
///
/// This used to be *the* tile size, which quietly made it two constants wearing
/// one name: the size `tile_shade` paints at, and the size the engine was
/// willing to accept from an art manifest. The second reading pinned the file
/// set to 16 px and threw away anything else (see [`tile_set`]), so a 64 px art
/// drop could not reach a frame at all — while `atlas.json` promised in writing
/// that `tile_px` may change with no Rust change.
///
/// The two are now separate. This one is the fallback bake size and stays 16:
/// every `tile_shade` pattern's period (plank courses of 4, brick heads every 8,
/// the lantern's radial falloff) is written against it. The *source* size is
/// [`source_tile_px`], read from the manifest at runtime.
const PROC_TILE_PX: usize = 16;

/// Faces a block distinguishes: top, side, bottom (see [`Face`]).
const N_FACES: usize = 3;

/// Columns in the far-LOD atlas — one per (block, face) pair.
const N_COLS: usize = N_TILES * N_FACES;

/// Widest far-LOD atlas this will build, in texels.
///
/// wgpu's `downlevel_defaults` guarantee `max_texture_dimension_2d` of 8192, and
/// the LOD atlas is the one image here whose width scales with the manifest:
/// [`N_COLS`] = 54 columns, so a 151 px manifest is the largest that still fits.
/// Past that the tiles are packed at a smaller cell and [`atlas_tile_px`] says
/// so out loud — a texture that silently fails to allocate is a black world.
const MAX_ATLAS_WIDTH: usize = 8192;

// ---------------------------------------------------------------------------
// the file-backed art set
// ---------------------------------------------------------------------------

/// The manifest tile set, decoded once, or `None` when there is no usable one.
///
/// `OnceLock` for the same reason `main::atlas_mesh_forced` uses one: this is
/// read from the material builder *and* from the far-LOD atlas builder, the
/// answer cannot change mid-run, and re-decoding a folder of PNGs per caller is
/// a silly way to spend a startup.
///
/// Every failure path is loud and lands on the procedural tiles rather than on a
/// magenta world: a missing folder is a normal state for a source checkout that
/// has not pulled the art yet.
fn tile_set() -> Option<&'static TileSet> {
    static SET: std::sync::OnceLock<Option<TileSet>> = std::sync::OnceLock::new();
    SET.get_or_init(|| match block_atlas::load_tiles(None) {
        Ok(Some(set)) => {
            // Any even `tile_px` — `block_atlas::load_tiles` already rejects odd
            // and zero, and every consumer here now sizes itself off the set.
            //
            // There used to be a `== PROC_TILE_PX` gate on this arm that dropped
            // the whole file set when the manifest said anything but 16. It cost
            // Monanisa's entire 64 px drop: albedo, normals and roughness all
            // fell on the floor together, silently as far as a frame was
            // concerned, and `atlas.json`'s own header promised the opposite
            // ("`tile_px` may change too, as long as it is even. No Rust change,
            // no rebuild"). The doc was right and the code was wrong.
            println!(
                "BLOCK_ART file-backed dir={} tiles={} kinds={} tile_px={}",
                set.dir.display(),
                set.tiles.len(),
                set.kinds.len(),
                set.tile_px
            );
            Some(set)
        }
        Ok(None) => None, // mode=off; block_atlas already said so
        Err(e) => {
            println!("BLOCK_ART unavailable ({e}) — procedural tiles kept");
            None
        }
    })
    .as_ref()
}

/// The manifest's cross-quad vegetation kinds, `(kind, albedo file)` — the
/// foliage scatter's source of truth for which sprites to place and where their
/// art lives (relative to `assets/textures/blocks`). Empty when no atlas loaded
/// or the manifest predates the `"mode": "cross"` render mode.
pub fn cross_vegetation() -> Vec<(String, String)> {
    tile_set()
        .map(|s| s.cross_kinds().map(|(k, f)| (k.to_string(), f.to_string())).collect())
        .unwrap_or_default()
}

/// The render mode this block's `kinds` entry declares, or `None` for a cube.
///
/// The manifest half of the shaped palette: `atlas.json` says a `stair_wood` is
/// `"mode": "stair"` and `block_shapes` turns that word into geometry, so a new
/// shaped material is an art edit plus a `BlockId` rather than a mesher change.
///
/// `None` also covers "no art set loaded at all" (`VOXELFORGE_ATLAS_MODE=off`);
/// `block_shapes::shape_of` falls back to the sim's own `is_shaped` there, so
/// the procedural path still builds stairs rather than boxes.
pub fn block_shape_mode(id: BlockId) -> Option<&'static str> {
    tile_set()?.shape_mode(atlas_kind(id))
}

/// The manifest `kinds` entry a block type wears — its own sim name.
///
/// Deliberately not a translation table. `atlas.json` maps kind → tiles, so a
/// block that wants file art gets a kind named after it *in the manifest*, and
/// the mapping stays where an artist can edit it. A Rust-side alias list would
/// be a second contract that has to agree with the first one.
#[inline]
fn atlas_kind(id: BlockId) -> &'static str {
    id.name()
}

/// The raw manifest tile for one face of one block, if the art set has it.
///
/// Raw, not luminance-normalised: [`block_material`] hands `base_color` white to
/// a textured block, so the tile *is* the albedo here. (The packed-atlas path in
/// `block_atlas::load` normalises instead, because there the tile multiplies a
/// hero material's existing graded colour — same tiles, two jobs.)
fn face_tile(id: BlockId, face: Face) -> Option<&'static [u8]> {
    tile_set()?.face_tile(atlas_kind(id), face)
}

/// Edge length of the ACTIVE art source, in texels: the manifest's `tile_px`
/// when a file set loaded, [`PROC_TILE_PX`] when nothing did.
///
/// The other half of the constant that used to be [`PROC_TILE_PX`]. Everything
/// that bakes a texture *around* the art — the standalone face texture, the
/// three derived maps, the LOD atlas cell, the atlas UV inset — measures itself
/// here, so a manifest at 16, 64 or 128 px needs no rebuild, which is what
/// `atlas.json` has claimed all along.
fn source_tile_px() -> usize {
    tile_set().map_or(PROC_TILE_PX, |s| s.tile_px as usize)
}

/// Edge length of the tile [`face_texels`] returns for ONE face.
///
/// Per face, not per set: a manifest that names no kind for cobblestone leaves
/// that block on its 16 px procedural tile while the blocks beside it wear the
/// artist's 64 px art, and the split path is happy with that — each face bakes
/// its own standalone texture, so sizes never have to agree. Only [`build_atlas`],
/// which packs them all into one image, has to reconcile them.
fn face_px(id: BlockId, face: Face) -> usize {
    match tile_set() {
        Some(set) if set.face_tile(atlas_kind(id), face).is_some() => set.tile_px as usize,
        _ => PROC_TILE_PX,
    }
}

/// The cell size the far-LOD atlas packs its [`N_COLS`] columns at.
///
/// The source size, clamped so the packed image cannot exceed
/// [`MAX_ATLAS_WIDTH`]. Tiles that are not this size — the procedural 16 px ones
/// standing next to a 64 px art set, or every tile at all if the clamp bit — are
/// resampled into it by [`scale_tile_nearest`].
///
/// Printed once, because "how wide is the atlas this run" is exactly the sort of
/// number that is obvious in a debugger and invisible in a bug report.
fn atlas_tile_px() -> usize {
    static PX: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *PX.get_or_init(|| {
        let want = source_tile_px();
        let cap = MAX_ATLAS_WIDTH / N_COLS;
        let px = want.min(cap).max(1);
        if px < want {
            println!(
                "BLOCK_ART LOD atlas capped: {N_COLS} cols × {want}px = {}px exceeds \
                 max_texture_dimension_2d {MAX_ATLAS_WIDTH}; packing at {px}px",
                N_COLS * want
            );
        }
        println!(
            "BLOCK_ART LOD atlas {}x{} ({} KiB) cell={px}px",
            N_COLS * px,
            px,
            N_COLS * px * px * 4 / 1024
        );
        px
    })
}

/// Nearest-neighbour resample of a square RGBA tile, `src_px` → `dst_px`.
///
/// Nearest and not a filter, for the same reason [`voxel_sampler`] mags with
/// `Nearest`: this is pixel art, and the only case this runs in anger — a 16 px
/// procedural tile taking its column in a 64 px atlas — is an exact 4× where
/// nearest is not an approximation but the answer. It is written for any ratio
/// so that the [`MAX_ATLAS_WIDTH`] clamp has something to fall on.
fn scale_tile_nearest(src: &[u8], src_px: usize, dst_px: usize) -> Vec<u8> {
    if src_px == dst_px {
        return src.to_vec();
    }
    let mut out = vec![0u8; dst_px * dst_px * 4];
    for y in 0..dst_px {
        let sy = y * src_px / dst_px;
        for x in 0..dst_px {
            let sx = x * src_px / dst_px;
            let s = (sy * src_px + sx) * 4;
            let d = (y * dst_px + x) * 4;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    out
}

/// The face slot a block ACTUALLY needs a separate mesh and material for.
///
/// Splitting the split path per face costs draw calls: a chunk of six block
/// types went from six children to as many as eighteen. Almost none of that is
/// earned — a stone block's top is its side, and so is every procedural block's,
/// so those three buckets would hold three copies of one material.
///
/// So a block only splits when its art says it should. [`has_per_face_art`] asks
/// the manifest whether the three faces resolve to different *tiles*; if they do
/// not, every face folds onto `Side` and the chunk emits exactly the children it
/// emitted before this pass. Grass and logs pay for their tops. Nothing else
/// pays anything.
#[inline]
fn face_slot(id: BlockId, face: Face) -> Face {
    if has_per_face_art(id) {
        face
    } else {
        Face::Side
    }
}

/// Does this block's art actually differ between faces?
///
/// Cached per block id: `sweep` asks this once per emitted quad, and the answer
/// is a property of a manifest that is read once at startup.
fn has_per_face_art(id: BlockId) -> bool {
    static PER_FACE: std::sync::OnceLock<[bool; 256]> = std::sync::OnceLock::new();
    PER_FACE.get_or_init(|| {
        let mut out = [false; 256];
        if let Some(set) = tile_set() {
            for (i, slot) in out.iter_mut().enumerate() {
                if let Some(f) = set.kinds.get(atlas_kind(BlockId(i as u8))) {
                    *slot = f.top != f.side || f.bottom != f.side;
                }
            }
        }
        out
    })[id.0 as usize]
}

/// Vertex-AO shade per occlusion level (3 = fully open, 0 = fully boxed in).
///
/// This is the single knob that makes a voxel room read as a room instead of as
/// a flat-lit box: the reference kitchen's depth is almost entirely the soft
/// darkening where two surfaces meet. Deepened from the original gentle curve
/// so block edges read without depending on post-FX SSAO — the per-instance
/// colour noise (§instance_variation) breaks up what would otherwise be a
/// uniform gradient across a long merged wall.
const AO_SHADE: [f32; 4] = [0.28, 0.52, 0.76, 1.0];

/// How far up a face looks for something hanging over it, in blocks.
///
/// The corner term above only ever sees the ONE ring of cells touching the face,
/// so a floor under an eave three blocks up is lit exactly like open ground —
/// which is the flat-paper read in the outdoor plates. This is the second term:
/// straight up from the air in front of the face, nearest hit wins.
const CONTACT_RANGE: i32 = 4;

/// Darkening at a hard contact (a block sitting directly over the face), fading
/// linearly to nothing at [`CONTACT_RANGE`].
///
/// Deliberately smaller than one AO step: it stacks ON TOP of the corner term,
/// and an eave that also forms a corner must not slam the vertex to black.
const CONTACT_MAX: f32 = 0.34;

/// Quantisation of the contact term, in steps, before it enters the merge key.
///
/// The value is a per-corner average of four columns, so it is naturally
/// continuous — and a continuous merge key would shatter every merged quad.
/// Twelve steps keeps the gradient smooth to the eye while still letting a run
/// of equally-shaded cells merge.
const CONTACT_STEPS: u8 = 12;

// ---------------------------------------------------------------------------
// per-block surface response
// ---------------------------------------------------------------------------

/// How one block type responds to light. Consumed by [`block_material`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockSurface {
    pub perceptual_roughness: f32,
    pub metallic: f32,
    pub reflectance: f32,
    /// Linear emissive radiance. Values above 1.0 are what a bloom pass picks up.
    pub emissive: LinearRgba,
    /// Base-colour alpha. `< 1.0` pairs with [`BlockSurface::alpha_blend`].
    pub alpha: f32,
    /// Whether this block draws in the transparent pass.
    pub alpha_blend: bool,
    /// Cut the texture's alpha out instead of blending it — the plant answer.
    ///
    /// A cross-quad leaf sprite is mostly empty texels, and blending them costs
    /// a sorted transparent pass AND makes two plants overlap wrong. A masked
    /// cutout is opaque-pass, sorts for free, and casts a leaf-shaped shadow
    /// instead of a square one. Wins over [`Self::alpha_blend`] when both are set.
    pub alpha_mask: bool,
    /// Draw the back of the mesh too.
    ///
    /// Chunk meshes are closed shells, so this is `false` for every cube. A
    /// crossed plant quad is the exception it exists for: a single-sided leaf is
    /// invisible from one half of the compass.
    pub double_sided: bool,
}

impl Default for BlockSurface {
    fn default() -> Self {
        // Plain matte mineral: the sane fallback for any id without an entry.
        Self {
            perceptual_roughness: 0.95,
            metallic: 0.0,
            reflectance: 0.08,
            emissive: LinearRgba::BLACK,
            alpha: 1.0,
            alpha_blend: false,
            alpha_mask: false,
            double_sided: false,
        }
    }
}

/// The surface response for one block type.
///
/// The numbers are grouped by material family rather than tuned one block at a
/// time, so a new block lands in a family instead of inventing a fourth kind of
/// stone. Families:
///
/// * **satin wood** — rough enough to have no mirror, smooth enough to catch a
///   long soft highlight down a plank. This is the single biggest difference
///   between the reference art and a flat voxel render.
/// * **matte mineral** — stone, cobble, brick, clay, gravel: no specular story.
/// * **powder** — sand and snow: matte but bright, slightly higher reflectance
///   so a low sun still skims them.
/// * **foliage** — grass, leaves, moss: matte and slightly translucent-looking
///   via a lifted reflectance floor; no metal.
/// * **glass** — obsidian, the darkest and smoothest block in the palette, is
///   what the reflective-pane surface is tuned on (the sim has no glass id).
/// * **emitter** — [`LAMP`].
pub fn block_surface(id: BlockId) -> BlockSurface {
    match id {
        BlockId::WOOD => BlockSurface {
            perceptual_roughness: 0.58,
            reflectance: 0.34,
            ..default()
        },
        BlockId::SAND | BlockId::RED_SAND => BlockSurface {
            perceptual_roughness: 0.88,
            reflectance: 0.14,
            ..default()
        },
        BlockId::SNOW => BlockSurface {
            perceptual_roughness: 0.72,
            reflectance: 0.26,
            ..default()
        },
        BlockId::GRASS | BlockId::LEAVES | BlockId::MOSS => BlockSurface {
            perceptual_roughness: 0.90,
            reflectance: 0.12,
            ..default()
        },
        BlockId::OBSIDIAN => BlockSurface {
            perceptual_roughness: 0.06,
            metallic: 0.0,
            reflectance: 0.92,
            alpha: 0.66,
            alpha_blend: true,
            ..default()
        },
        BlockId::LIMESTONE => BlockSurface {
            perceptual_roughness: 0.82,
            reflectance: 0.16,
            ..default()
        },
        // A real pane. `alpha` stays 1.0 and `alpha_blend` is on: the see-through
        // is the TEXTURE's alpha channel, not a material-wide fade. That
        // distinction is the whole feature — a uniform 0.4 material dims the
        // leading, the mullions and the highlight along with the pane, and the
        // result reads as a ghost block rather than as glass in a frame.
        //
        // `glass_opaque()` is the A/B lever that puts it back the way it was.
        BlockId::GLASS => BlockSurface {
            perceptual_roughness: 0.08,
            metallic: 0.0,
            reflectance: 0.50,
            alpha: 1.0,
            alpha_blend: !glass_opaque(),
            ..default()
        },
        // The river. The OPPOSITE transparency contract from the pane: the
        // artist's `water.png` is an opaque tile, so the see-through comes from
        // a material-wide alpha here — which is correct for water, because
        // water has no leading or mullions to preserve; what must survive the
        // fade is the SPECULAR, and a PBR specular lives above the diffuse term
        // and does not fade with `base_color.a`. Near-mirror roughness so the
        // low sun lays a glitter path down the channel, reflectance high for
        // the same reason. The bed under it renders because water is not
        // opaque (see `hides`), and the surface sits 2/16 below the cell top
        // (see [`WATER_TOP_OFFSET`]).
        BlockId::WATER => BlockSurface {
            perceptual_roughness: 0.06,
            metallic: 0.0,
            reflectance: 0.55,
            alpha: 0.62,
            alpha_blend: true,
            ..default()
        },
        // The one block in the palette allowed to answer "metal" — the whole
        // reason [`MrSource`] distinguishes spellings is so this can exist
        // without every grey `*_r.png` in the folder turning to chrome with it.
        // See `authored_map`: the roughness spelling's blue channel is authored
        // FROM this table, never trusted from the file.
        BlockId::METAL => BlockSurface {
            perceptual_roughness: 0.34,
            metallic: 1.0,
            reflectance: 0.60,
            ..default()
        },
        LAMP => BlockSurface {
            perceptual_roughness: 0.45,
            reflectance: 0.30,
            // Warm lantern, pushed past 1.0 so Flamingo's bloom actually catches
            // it instead of merely tinting the texel.
            emissive: LinearRgba::rgb(9.0, 4.6, 1.5),
            ..default()
        },
        // STONE, DIRT, COBBLESTONE, GRAVEL, BRICK, CLAY — matte mineral.
        _ => BlockSurface::default(),
    }
}

/// How much per-texel *shape* one block type has, and how far its finish varies
/// across a face. Consumed by [`build_face_normal_map`] and
/// [`build_face_metallic_roughness`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockRelief {
    /// Normal-map amplitude, `0.0` = deliberately flat (no map is built at all).
    pub relief: f32,
    /// How far roughness swings either side of the table value across a tile.
    pub roughness_spread: f32,
}

/// The relief grade for one block type.
///
/// This is a decision per material, not a global filter — a pile of loose
/// cobbles and a plaster wall do not have the same amount of shape, and running
/// one amplitude over both is what makes a procedural bump pass read as noise.
pub fn block_relief(id: BlockId) -> BlockRelief {
    let g = |relief, roughness_spread| BlockRelief {
        relief,
        roughness_spread,
    };
    match id {
        // A pile of separate loose objects — the strongest relief in the palette.
        BlockId::COBBLESTONE | BlockId::GRAVEL => g(1.00, 0.12),
        // The dark gaps in the tile are real holes between blades.
        BlockId::GRASS | BlockId::LEAVES | BlockId::MOSS => g(0.85, 0.08),
        // Recessed mortar joints, flat-ish faces.
        BlockId::BRICK => g(0.80, 0.12),
        // Deepest grooves *and* the smoothest face between them — that spread is
        // the read; a plank with a uniform finish is a plank-coloured board.
        BlockId::WOOD => g(0.75, 0.16),
        BlockId::DIRT => g(0.70, 0.08),
        // Plaster: broad soft undulation, not detail.
        BlockId::LIMESTONE => g(0.45, 0.10),
        // Grain finer than a texel — sparkle, not shape.
        BlockId::SAND | BlockId::RED_SAND => g(0.35, 0.06),
        // Almost no shape, wide finish spread: that is why snow glitters.
        BlockId::SNOW => g(0.25, 0.18),
        // A glowing surface has no shading to modulate.
        LAMP => g(0.15, 0.00),
        // Deliberately flat. The diagonal sheen is a reflection, not a ridge —
        // bumping a pane only frosts it and kills the mirror, so it gets no
        // normal map at all.
        BlockId::OBSIDIAN => g(0.00, 0.03),
        // Same reasoning as obsidian, and one more: a normal map derived from a
        // tile whose interest is in the ALPHA channel would read the transparent
        // pane as a hole and emboss its own frame. Flat, no map.
        BlockId::GLASS => g(0.00, 0.00),
        // The river's shape is the ripples in the artist's `water_n.png`, which
        // `face_maps` binds as an AUTHORED map independent of this table — a
        // derived relief derived from the albedo's luminance would fight the
        // drawn ripples instead of following them. Derived: flat, like a pane.
        BlockId::WATER => g(0.00, 0.04),
        // Sheet metal: drawn curvature at a modest amplitude, and a finish that
        // swings between brushed and polished — the widest tell of the family.
        BlockId::METAL => g(0.30, 0.10),
        // STONE, CLAY — matte mineral.
        _ => g(0.55, 0.10),
    }
}

/// `VOXELFORGE_FLAT_MATERIAL=1` — build the palette base-colour-only: no normal
/// map, no roughness map, exactly what shipped before this pass.
///
/// It exists for the same reason `main::atlas_mesh_forced` does: the fix gets
/// photographed **against itself out of one binary** — same exe, same map, same
/// seed, same camera, one variable.
fn flat_material() -> bool {
    matches!(std::env::var("VOXELFORGE_FLAT_MATERIAL"), Ok(v) if !v.is_empty() && v != "0")
}

/// `VOXELFORGE_FLAT_INSTANCE=1` — disable per-instance colour variation across
/// quads, matching the flat-uniform look that shipped before this pass.
///
/// Paired with [`flat_material`] for the full A/B lever: set both to `1` to
/// render indistinguishable from the pre-material pass; set only
/// `FLAT_INSTANCE` to keep normal/roughness maps but drop per-face colour
/// noise.
fn flat_instance() -> bool {
    matches!(std::env::var("VOXELFORGE_FLAT_INSTANCE"), Ok(v) if !v.is_empty() && v != "0")
}

/// `VOXELFORGE_AO` — the A/B lever over every mesher-side occlusion term.
///
/// * unset ⇒ `1.0`, the shipped look.
/// * `off` / `0` ⇒ `0.0`: every vertex fully lit, which is exactly the frame
///   this pass is measured against — flat-lit blocks, no corner, no eave.
/// * any other non-negative number scales the darkening, so a diagnostic plate
///   can be shot at `2` out of the SAME binary when the question is "does the
///   attribute reach the pixels at all" rather than "is it strong enough".
///
/// One `OnceLock`, because this is read once per emitted vertex.
fn ao_strength() -> f32 {
    static AO: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *AO.get_or_init(|| {
        let raw = std::env::var("VOXELFORGE_AO").unwrap_or_default();
        let t = raw.trim();
        let v = if t.is_empty() {
            1.0
        } else if t.eq_ignore_ascii_case("off") || t.eq_ignore_ascii_case("false") {
            0.0
        } else {
            // A typo must not silently ship an unlit world, so anything
            // unparseable or negative falls back to the shipped strength.
            match t.parse::<f32>() {
                Ok(v) if v.is_finite() && v >= 0.0 => v.min(4.0),
                _ => 1.0,
            }
        };
        println!("AO strength {v} (VOXELFORGE_AO={t:?}) — corner + contact, per vertex");
        v
    })
}

/// `VOXELFORGE_AO_STATS=1` — print what the mesher actually emitted.
///
/// The whole question this pass opened with was un-answerable from a frame:
/// AO could be missing because the mesher never darkens anything, or because
/// the attribute never reaches the shader. One line per batch of chunks
/// separates those two worlds without a second build.
fn ao_stats_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(
        || matches!(std::env::var("VOXELFORGE_AO_STATS"), Ok(v) if !v.is_empty() && v != "0"),
    )
}

/// `VOXELFORGE_GLASS_OPAQUE=1` — render [`BlockId::GLASS`] the way it rendered
/// before it had an alpha channel: solid, sight-blocking, no transparent pass.
///
/// The A/B lever for the pane, and it has to reach further than the other two:
/// transparency is not only a material setting, it changes which faces the
/// MESHER emits (an opaque neighbour culls the face behind it, a pane does not).
/// So this is read here *and* in [`sweep`], and both read the same `OnceLock` —
/// a mesher and a material that disagree about whether glass is opaque produce
/// a chunk with holes in it, which is a much worse bug than either setting.
fn glass_opaque() -> bool {
    static OPAQUE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OPAQUE.get_or_init(|| {
        let on = matches!(std::env::var("VOXELFORGE_GLASS_OPAQUE"), Ok(v) if !v.is_empty() && v != "0");
        if on {
            println!("GLASS opaque (VOXELFORGE_GLASS_OPAQUE) — pre-fix reference render");
        }
        on
    })
}

/// Does this block hide the face behind it, as the *renderer* sees it?
///
/// [`BlockId::is_opaque`] with [`glass_opaque`] folded in, so the A/B lever
/// reaches the mesher. Every visibility and AO test in [`sweep`] goes through
/// here; nothing else should.
#[inline]
fn hides(b: BlockId) -> bool {
    b.is_opaque() || (glass_opaque() && b.is_solid())
}

/// A non-negative amplitude scale read from the environment, `1.0` by default.
///
/// Relief amplitude is the one number here that can only be judged from a
/// rendered frame, and this binary costs a fat-LTO link per rebuild; a runtime
/// scale turns a sweep from hours into minutes. A negative or non-numeric value
/// falls back to `1.0` rather than inverting every surface in the game on a typo.
fn env_scale(key: &str) -> f32 {
    match std::env::var(key).ok().and_then(|v| v.trim().parse::<f32>().ok()) {
        Some(v) if v.is_finite() && v >= 0.0 => v,
        _ => 1.0,
    }
}

/// The `perceptual_roughness` factor to hand a material that carries a roughness
/// map: the table value plus its whole spread.
///
/// Bevy *multiplies* `perceptual_roughness` by the map's green channel. Hand the
/// same factor to the mapped and unmapped cases and every mapped surface is
/// quietly smoother than the table says, with nothing anywhere reporting it. So
/// the factor is raised to the ceiling and the map dips down from there — the
/// effective roughness straddles the table's value instead of only cutting below
/// it.
pub fn roughness_ceiling(id: BlockId) -> f32 {
    let spread = block_relief(id).roughness_spread * env_scale("VOXELFORGE_MAT_ROUGH_VAR");
    (block_surface(id).perceptual_roughness + spread).min(1.0)
}

// ---------------------------------------------------------------------------
// authored PBR maps
// ---------------------------------------------------------------------------

/// Where the metallic-roughness map a material is wearing came from, because the
/// two provenances need OPPOSITE factors and getting that backwards is silent.
///
/// Bevy MULTIPLIES `perceptual_roughness` / `metallic` into the map's green /
/// blue channels. The procedural map ([`build_face_metallic_roughness`]) is
/// authored as a *ratio* against [`roughness_ceiling`] precisely so that
/// multiplication lands on the table's value — that contract is documented on
/// `roughness_ceiling` and must not change. An artist's file is the opposite: its
/// green channel IS the roughness they want to see, so the factor has to be
/// identity or every authored surface renders quietly smoother than the PNG says,
/// with nothing anywhere reporting it — the exact failure `roughness_ceiling` was
/// written to prevent, arriving from the other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MrSource {
    /// No map at all — the table's scalars stand alone.
    ///
    /// Spelled `Absent` and not `None`: a variant named `None` in a module that
    /// also matches on `Option` is a pattern-resolution trap waiting for the next
    /// reader, and it would cost a silent wrong branch, not a compile error.
    #[default]
    Absent,
    /// [`build_face_metallic_roughness`]: green is a ratio against the ceiling.
    Procedural,
    /// A `*_r.png`. Green is absolute roughness; BLUE IS NOT METALNESS.
    ///
    /// A roughness map is very often shipped as a grey PNG (R=G=B=roughness), and
    /// Bevy reads blue as metalness unconditionally. Take blue at face value and
    /// every rough surface in the game turns to metal. So this variant keeps the
    /// table's `metallic` factor (0.0 for everything in this palette), which
    /// multiplies whatever is in blue back down to nothing.
    AuthoredRoughness,
    /// A `*_mr.png`: green roughness, blue metalness, both absolute.
    AuthoredMr,
    /// A `*_orm.png`: the glTF packing — RED occlusion, green roughness, blue
    /// metalness. Its red channel is a real AO map, so the same texture feeds
    /// `occlusion_texture` as well and costs one upload, not two.
    AuthoredOrm,
}

impl MrSource {
    /// The `perceptual_roughness` factor to pair with this provenance.
    fn roughness_factor(self, id: BlockId) -> f32 {
        match self {
            MrSource::Absent => block_surface(id).perceptual_roughness,
            MrSource::Procedural => roughness_ceiling(id),
            // Identity: the file is the answer.
            _ => 1.0,
        }
    }

    /// The `metallic` factor to pair with it. See [`MrSource::AuthoredRoughness`]
    /// for why only the two *packed* formats are allowed to drive metalness.
    fn metallic_factor(self, id: BlockId) -> f32 {
        match self {
            MrSource::AuthoredMr | MrSource::AuthoredOrm => 1.0,
            _ => block_surface(id).metallic,
        }
    }
}

/// The texture set one face wears, already resolved between authored and derived.
#[derive(Default)]
pub struct BlockMaps {
    pub normal: Option<Handle<Image>>,
    pub metallic_roughness: Option<Handle<Image>>,
    pub occlusion: Option<Handle<Image>>,
    pub mr_source: MrSource,
}

/// File-name suffixes searched for an authored map, in preference order.
///
/// `_n` / `_r` are the two Monanisa's art drop is named for; the longer spellings
/// are here so a set exported straight out of Substance/Blender ("…_normal.png",
/// "…_orm.png") lands without a rename step. First hit wins, so adding a spelling
/// can never change what an existing folder resolves to.
const NORMAL_SUFFIXES: &[&str] = &["_n", "_normal", "_nrm"];
/// Roughness-only spellings — blue is NOT trusted, see [`MrSource::AuthoredRoughness`].
const ROUGHNESS_SUFFIXES: &[&str] = &["_r", "_rough", "_roughness"];
/// Packed metallic-roughness (green/blue) spellings.
const MR_SUFFIXES: &[&str] = &["_mr", "_metallic_roughness"];
/// Packed occlusion-roughness-metallic (red/green/blue) spellings.
const ORM_SUFFIXES: &[&str] = &["_orm", "_arm"];
/// Standalone ambient-occlusion spellings — red channel is read, nothing else.
const AO_SUFFIXES: &[&str] = &["_ao", "_occlusion"];

/// `VOXELFORGE_MAT_MAPS=off` — ignore authored PNGs and use the derived maps.
///
/// The A/B lever for the art drop itself: it answers "is this frame different
/// because of Monanisa's maps, or because of everything else that moved?" from
/// ONE binary, which is the only form of that answer worth quoting.
fn authored_maps_enabled() -> bool {
    !matches!(std::env::var("VOXELFORGE_MAT_MAPS"), Ok(v) if v.trim().eq_ignore_ascii_case("off"))
}

/// Load `<tile stem><suffix>.png` from the art folder for one face, first hit wins.
///
/// `Rgba8Unorm`, never sRGB: every map this finds is DATA (a direction, a
/// roughness, an occlusion), and an sRGB view would bend all three through a
/// transfer curve meant for colour. Any resolution is accepted — chunk UVs are
/// measured in blocks and the sampler repeats, so a 256² normal map over a 16²
/// albedo is a legal (and welcome) upgrade, not a mismatch.
///
/// Every miss is silent (a folder without the art is the normal state of a fresh
/// checkout) but every *failure* is loud: a PNG that exists and will not decode
/// is a broken delivery, and falling back without saying so is how it ships.
fn authored_map(id: BlockId, face: Face, suffixes: &[&str]) -> Option<Image> {
    if !authored_maps_enabled() {
        return None;
    }
    let set = tile_set()?;
    let file = set.face_file(atlas_kind(id), face)?;
    let stem = std::path::Path::new(file).file_stem()?.to_str()?;
    for suffix in suffixes {
        let path = set.dir.join(format!("{stem}{suffix}.png"));
        if !path.is_file() {
            continue;
        }
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                if w == 0 || h == 0 {
                    println!("BLOCK_PBR {} is empty — ignored", path.display());
                    continue;
                }
                println!("BLOCK_PBR authored {} {w}x{h}", path.display());
                let mut raw = rgba.into_raw();
                // The roughness-only spelling authors ONE channel. Bevy reads
                // blue as metalness unconditionally, and a grey `*_r.png`
                // (R=G=B) would silently make [`BlockId::METAL`]'s metalness
                // track its own roughness — polished patches turning to chrome
                // exactly where they are smoothest. So blue is authored HERE
                // from the surface table, never trusted from the file: for
                // every non-metal this writes 0 (byte-identical to what a
                // careful artist ships anyway), and for METAL it writes the
                // one 1.0 the table grants.
                if suffixes == ROUGHNESS_SUFFIXES {
                    let b = (block_surface(id).metallic.clamp(0.0, 1.0) * 255.0).round() as u8;
                    for p in raw.chunks_exact_mut(4) {
                        p[2] = b;
                    }
                }
                return Some(image_from_rgba(
                    w as usize,
                    h as usize,
                    raw,
                    TextureFormat::Rgba8Unorm,
                    voxel_sampler(true),
                ));
            }
            Err(e) => println!("BLOCK_PBR {} failed to decode ({e}) — derived map kept", path.display()),
        }
    }
    None
}

/// One face's four maps, authored where a file exists and derived where it does not.
///
/// This is the fallback contract in one place: the loader never *requires* the art
/// drop, so a checkout with an empty `assets/textures/blocks/` renders exactly what
/// it rendered before the loader existed, and a folder with only `brick_n.png` in
/// it gets an authored normal on brick and derived everything else. Per FACE, not
/// per block — grass ships a top and a side that have no business sharing a relief.
pub fn face_maps(id: BlockId, face: Face) -> FaceMaps {
    // `VOXELFORGE_FLAT_MATERIAL` is the kill switch for the whole surface rig and
    // has to outrank the art too, or the "before" half of every material A/B
    // quietly keeps whatever files happen to be on disk.
    if flat_material() {
        return FaceMaps::default();
    }

    let normal = authored_map(id, face, NORMAL_SUFFIXES).or_else(|| build_face_normal_map(id, face));

    // Ordered most-informative first: an ORM carries everything a `_r` does plus
    // AO, so finding one means there is nothing left for the thinner spellings to
    // add. Only the fall-through reaches the derived map.
    let (mr, mr_source) = if let Some(i) = authored_map(id, face, ORM_SUFFIXES) {
        (Some(i), MrSource::AuthoredOrm)
    } else if let Some(i) = authored_map(id, face, MR_SUFFIXES) {
        (Some(i), MrSource::AuthoredMr)
    } else if let Some(i) = authored_map(id, face, ROUGHNESS_SUFFIXES) {
        (Some(i), MrSource::AuthoredRoughness)
    } else {
        match build_face_metallic_roughness(id, face) {
            Some(i) => (Some(i), MrSource::Procedural),
            None => (None, MrSource::Absent),
        }
    };

    // An ORM's red channel IS the occlusion map, so that case is left empty here
    // and `build_block_materials` binds the SAME handle twice rather than decoding
    // the file again. A dedicated `_ao.png` still wins — an artist who shipped
    // both meant the standalone one.
    let occlusion = match authored_map(id, face, AO_SUFFIXES) {
        Some(i) => Some(i),
        None if mr_source == MrSource::AuthoredOrm => None,
        None => build_face_occlusion(id, face),
    };

    FaceMaps {
        normal,
        mr,
        occlusion,
        mr_source,
    }
}

/// [`face_maps`]'s result, before the images are handed to `Assets<Image>`.
#[derive(Default)]
pub struct FaceMaps {
    pub normal: Option<Image>,
    pub mr: Option<Image>,
    pub occlusion: Option<Image>,
    pub mr_source: MrSource,
}

/// The StandardMaterial for one block type, given its (repeating) tile texture
/// and — where the material has any — its normal / roughness / occlusion maps.
///
/// This is the per-type material the single shared atlas material can't be. Pair
/// it with [`greedy_mesh_chunk_split`], which produces one mesh per type *and*
/// the tangents a normal map is silently dropped without.
pub fn block_material(id: BlockId, texture: Handle<Image>, maps: BlockMaps) -> StandardMaterial {
    let s = block_surface(id);
    StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, s.alpha),
        base_color_texture: Some(texture),
        normal_map_texture: maps.normal,
        metallic_roughness_texture: maps.metallic_roughness,
        // Reads the RED channel only, and darkens the DIFFUSE INDIRECT term —
        // the same term SSAO bites, which is why the two compose instead of
        // double-darkening a sunlit face: under a 20 000-lux key a lit face is
        // almost all direct light and barely moves, while the inside of a mortar
        // joint is nearly all ambient and takes the full bite. That asymmetry is
        // the whole point — it is what stops a wall reading as a photograph of a
        // wall pasted onto a flat plane.
        occlusion_texture: maps.occlusion,
        perceptual_roughness: maps.mr_source.roughness_factor(id),
        metallic: maps.mr_source.metallic_factor(id),
        reflectance: s.reflectance,
        emissive: s.emissive,
        alpha_mode: if s.alpha_mask {
            // 0.5 rather than a hair above zero: the vegetation sprites are
            // authored with soft edges, and cutting at the midpoint keeps a leaf
            // silhouette crisp instead of fringed with half-transparent dust.
            AlphaMode::Mask(0.5)
        } else if s.alpha_blend {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        // Chunk meshes are closed shells; drawing the inside of them is wasted
        // fill and, for the blended panes, a double-blend. A crossed plant quad
        // is the one surface in the world that is genuinely two-sided.
        double_sided: s.double_sided,
        // A double-sided mesh whose back faces are still culled is a mesh that
        // is double-sided in name only — the pipeline reads `cull_mode`, and
        // Bevy does NOT derive one from the flag above.
        cull_mode: if s.double_sided {
            None
        } else {
            Some(bevy::render::render_resource::Face::Back)
        },
        ..default()
    }
}

/// Does this block belong in the shadow map?
///
/// A shadow map is binary: a mesh either occludes the sun completely or not at
/// all. A pane whose texture is 46/255 opaque across most of its area occludes
/// almost nothing, so leaving it in gives a clear window a solid black shadow —
/// the single most obvious way to render glass wrong, and Bevy does keep blended
/// meshes in the shadow pass (`bevy_pbr::render::light`, `MAY_DISCARD`).
///
/// Narrow on purpose: it names glass rather than testing `alpha_blend`, because
/// obsidian is also blended and at 0.66 alpha it *does* occlude most of the
/// light — its shadow is part of a signed-off frame and is not this pass's to
/// remove. Under [`glass_opaque`] the pane is opaque again and casts again, so
/// the A/B lever stays honest here too.
pub fn casts_shadow(id: BlockId) -> bool {
    // Water, like the pane, occludes almost nothing a shadow map can express —
    // a river's shadow is a black stripe down a valley, and the sunset-on-water
    // read depends on the sun reaching it. Unlike the pane there is no A/B
    // lever: an opaque river was never a shipped look to preserve.
    if id == BlockId::WATER {
        return false;
    }
    id != BlockId::GLASS || glass_opaque()
}

/// The single shared material for the atlas path ([`greedy_mesh_chunk`]).
///
/// One material can only carry ONE surface response, so this is the average of
/// the table above rather than any one block's answer — a touch smoother and more
/// reflective than the old hard-coded `0.95 / 0.1`, which made every surface in
/// the game read as chalk. Callers on the atlas path should prefer this over
/// hand-rolling the material so the two paths stay comparable.
pub fn atlas_material(atlas: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(atlas),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        reflectance: 0.18,
        ..default()
    }
}

/// Which (block, face) a split-path mesh draws — the split path's bucket key.
///
/// The split used to be per block alone, because a block wore one tile on all
/// six sides. Per-face art (grass with a top, a side and a bottom) makes that
/// impossible: one mesh carries one material carries one texture, so a block
/// with three faces is three meshes. The greedy mesher was already emitting a
/// quad per (block, face) — a mask cell holds a signed block id and the sweep
/// axis is fixed per pass, so a merged quad can NEVER straddle two faces or two
/// block types. Splitting the bucket that way costs no merge quality at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceKey {
    pub block: BlockId,
    pub face: Face,
}

impl FaceKey {
    /// Its slot in the [`build_block_materials`] table.
    #[inline]
    pub fn material_index(self) -> usize {
        material_index(self.block, self.face)
    }
}

/// Slot of one (block, face) in the material table. Dense, so the table is a
/// `Vec` indexed directly rather than a hash lookup inside a re-mesh.
#[inline]
pub fn material_index(block: BlockId, face: Face) -> usize {
    block.0 as usize * N_FACES + face.index()
}

/// Every (block, face)'s material, indexed by [`material_index`] — the split
/// path's table.
///
/// Built once at startup rather than on demand, because the sites that re-mesh a
/// chunk (an editor click, a map load, the streaming tick) hold `&mut World` and
/// `Assets<Mesh>` but have no business also borrowing `Assets<Image>` and
/// `Assets<StandardMaterial>`. Fifty-four 16×16 tiles is a few tens of KB of
/// texture; the alternative is threading two more asset borrows through every
/// edit path.
///
/// Block 0 is AIR — never meshed, so its three entries are only there to keep the
/// table indexable by raw block id.
pub fn build_block_materials(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    let ao_on = occlusion_maps_enabled();
    let mut out = Vec::with_capacity(N_COLS);
    for id in 0..N_TILES as u8 {
        let id = BlockId(id);
        for face in Face::ALL {
            let tile = images.add(build_face_texture(id, face));
            let m = face_maps(id, face);
            let metallic_roughness = m.mr.map(|i| images.add(i));
            let occlusion = match (ao_on, m.occlusion, m.mr_source) {
                (false, _, _) => None,
                (true, Some(i), _) => Some(images.add(i)),
                // An ORM already sits in `Assets<Image>`, and its red channel is
                // the occlusion map. Bind the handle a second time: one upload,
                // two slots.
                (true, None, MrSource::AuthoredOrm) => metallic_roughness.clone(),
                (true, None, _) => None,
            };
            out.push(materials.add(block_material(
                id,
                tile,
                BlockMaps {
                    normal: m.normal.map(|i| images.add(i)),
                    metallic_roughness,
                    occlusion,
                    mr_source: m.mr_source,
                },
            )));
        }
    }
    debug_assert_eq!(out.len(), N_COLS);
    out
}

/// Is the occlusion map bound at all?
///
/// The one-binary A/B for the third map, and it reads `VOXELFORGE_LOOK_GEN` — the
/// SAME variable `look.rs` forks its generations on, deliberately, so that one
/// value picks the whole before/after and there is no way to shoot a pair that is
/// half of one generation and half of the other. It is read here rather than
/// imported from `look.rs` because the shot binaries compose these modules in
/// different subsets, and a material that will not link without the look module
/// is a worse coupling than four lines of duplicated string matching.
///
/// `v1`/`v2`/`v3` ⇒ no occlusion texture (the surface rig as it shipped
/// 2026-08-17). Unset or `v4` ⇒ bound. `VOXELFORGE_MAT_AO=0` also removes it, by
/// driving [`build_face_occlusion`]'s strength to zero.
fn occlusion_maps_enabled() -> bool {
    !matches!(
        std::env::var("VOXELFORGE_LOOK_GEN")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "v1" | "1" | "legacy" | "v2" | "2" | "before" | "v3" | "3"
    )
}

// ---------------------------------------------------------------------------
// meshing
// ---------------------------------------------------------------------------

/// How far the surface of a water column sits below the top of its cell, in
/// 1/16ths of a block (2 ⇒ the meniscus is at 14/16 = 0.875).
///
/// The classic voxel-water read: a full cube of water next to a full cube of
/// bank is a hard step, and the eye refuses it. Two sixteenths is deep enough
/// to read as a surface from any angle and shallow enough that a submerged
/// column (water above) still joins its neighbour without a seam — a cell
/// with water above it carries offset 0, so the shaved surface quad and the
/// full-height column below it tile exactly.
const WATER_TOP_OFFSET: u8 = 2;

/// One exposed face cell in a slice mask.
///
/// `id` is 0 for "no face", `+block` for a front (+q) face and `-block` for a
/// back (-q) face. `ao` is the four corner occlusion levels.
///
/// AO is part of the value, not metadata, and that is the whole trick: two faces
/// may only merge when their occlusion matches as well as their block type.
/// Without it a wall merges into one quad, the corner darkening gets averaged
/// across the whole wall, and the AO silently disappears exactly where it was
/// supposed to appear.
///
/// `top` is [`WATER_TOP_OFFSET`] for the face of a surface water cell and 0 for
/// everything else — water only, never glass, because a pane keeps its whole
/// cell while a liquid has a surface to drop. It rides the SAME equality trick
/// as `ao`: a surface cell and a submerged cell of one column must never merge
/// (their side quads end at different heights), and comparing the offset here
/// is what keeps them apart.
///
/// `contact` is the second occlusion term, in [`CONTACT_STEPS`] steps per
/// corner: how much of the sky straight above this corner is roofed over within
/// [`CONTACT_RANGE`] blocks. It rides the same equality trick as `ao` for the
/// same reason — merged away, an eave's shadow would spread evenly over the
/// whole floor and stop being an eave's shadow.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct MaskFace {
    id: i32,
    ao: [u8; 4],
    contact: [u8; 4],
    top: u8,
}

/// Occlusion level 0..=3 for one face corner, from its two edge neighbours and
/// the diagonal one. Two edge neighbours present ⇒ fully occluded regardless of
/// the diagonal (the classic special case: the corner is sealed).
#[inline]
fn ao_corner(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (side1 as u8 + side2 as u8 + corner as u8)
    }
}

/// How roofed-over one column of air is: 1.0 for a block resting directly on it,
/// falling linearly to 0.0 at [`CONTACT_RANGE`], and 0.0 for open sky.
///
/// Nearest hit wins, so a low eave is not diluted by the empty sky above it.
#[inline]
fn contact_column(chunk: &ChunkData, x: i32, y: i32, z: i32) -> f32 {
    for k in 1..=CONTACT_RANGE {
        if hides(chunk.get(x, y + k, z)) {
            return (CONTACT_RANGE + 1 - k) as f32 / CONTACT_RANGE as f32;
        }
    }
    0.0
}

/// Vertex buffers for one draw call.
#[derive(Default)]
struct Buffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    /// Only the split path fills this — see [`Buffers::into_mesh`].
    tangents: Vec<[f32; 4]>,
    indices: Vec<u32>,
    quads: usize,
}

impl Buffers {
    fn into_mesh(self) -> Mesh {
        let tangents = self.tangents;
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        // Bevy's PBR pipeline switches on `VERTEX_COLORS` purely from the mesh
        // layout and multiplies it into base colour — which is how the AO term
        // reaches the shipping StandardMaterial without a custom shader.
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        // Bevy's PBR shader only applies a normal map under `VERTEX_TANGENTS`. A
        // mesh with no tangent attribute drops the whole map on the floor — no
        // warning, no error, and the frame renders identical to the flat
        // version. The atlas path leaves this empty on purpose (it wears no
        // normal map), so the attribute is only inserted when it was filled.
        if !tangents.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
        }
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Greedy-mesh a chunk into a single Bevy Mesh (atlas UVs, one draw call).
///
/// Classic Lysenko sweep: for each of the 3 axes, build a per-slice mask of
/// exposed faces, then merge co-planar same-block **same-AO** faces into the
/// largest possible quads. Returns the mesh plus the emitted quad count.
pub fn greedy_mesh_chunk(chunk: &ChunkData) -> (Mesh, usize) {
    let mut parts = sweep(chunk, false);
    // `sweep(_, false)` always yields exactly one bucket, even for an all-air
    // chunk — callers rely on getting a (possibly empty) mesh back.
    let (_, buf) = parts.pop().unwrap_or((0, Buffers::default()));
    let quads = buf.quads;
    (buf.into_mesh(), quads)
}

/// Greedy-mesh a chunk into one mesh **per block type**, with UVs measured in
/// blocks so each type's tile texture repeats once per block.
///
/// Returns `(block, mesh, quads)` per type present, ordered by block id. Pair
/// each mesh with [`block_material`] over [`build_face_texture`].
pub fn greedy_mesh_chunk_split(chunk: &ChunkData) -> Vec<(FaceKey, Mesh, usize)> {
    sweep(chunk, true)
        .into_iter()
        .map(|(key, buf)| {
            let quads = buf.quads;
            let block = BlockId((key / N_FACES) as u8);
            let face = Face::ALL[key % N_FACES];
            (FaceKey { block, face }, buf.into_mesh(), quads)
        })
        .collect()
}

/// The shared sweep behind both public meshers.
///
/// `split == false` funnels every face into one bucket with atlas UVs;
/// `split == true` gives each (block, face) its own bucket with per-block
/// repeating UVs. One implementation on purpose — two copies of a greedy mesher
/// drift, and the AO bookkeeping is the part you cannot afford to have two
/// versions of.
///
/// The returned key is a [`material_index`] on the split path and always `0` on
/// the atlas path.
fn sweep(chunk: &ChunkData, split: bool) -> Vec<(usize, Buffers)> {
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    /// Cumulative mesher-side occlusion tally behind [`ao_stats_enabled`]. The
    /// point is to be able to tell "the mesher emitted no dark vertices" from
    /// "the dark vertices never reached the shader" out of one binary — the two
    /// look identical in a frame and want opposite fixes.
    static AO_VERTS: AtomicU64 = AtomicU64::new(0);
    static AO_DARK: AtomicU64 = AtomicU64::new(0);
    static AO_MIN: AtomicU32 = AtomicU32::new(u32::MAX);
    static AO_CHUNKS: AtomicU64 = AtomicU64::new(0);

    let ao_scale = ao_strength();
    let stats = ao_stats_enabled();
    let dims = [CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE];
    // Indexed by material slot so the bucket lookup is O(1) inside the hot loop.
    let mut buckets: Vec<Option<Buffers>> = (0..256 * N_FACES).map(|_| None).collect();
    if !split {
        buckets[0] = Some(Buffers::default());
    }

    // Per-instance colour variation: every quad gets a deterministic world-space
    // noise that subtly shifts its albedo and micro-AO, so a long wall of the
    // same block type is not one uniform colour. Gated behind the A/B lever and
    // only applied to the split path — the atlas/LOD path has no per-block
    // identity to vary.
    let instance_noise = split && !flat_instance();
    let (wox, woy, woz) = chunk.pos.world_origin();

    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;
        let mut x = [0i32; 3];
        let mut q = [0i32; 3];
        q[d] = 1;

        let mut mask = vec![MaskFace::default(); (dims[u] * dims[v]) as usize];

        x[d] = -1;
        while x[d] < dims[d] {
            // ---- build mask for the plane between slice x[d] and x[d]+1 ----
            let mut n = 0usize;
            for j in 0..dims[v] {
                for i in 0..dims[u] {
                    x[u] = i;
                    x[v] = j;
                    let a = chunk.get(x[0], x[1], x[2]);
                    let b = chunk.get(x[0] + q[0], x[1] + q[1], x[2] + q[2]);
                    // A face exists where a block meets something that does not
                    // hide it. `a != b` is what stops a pane-to-pane join from
                    // drawing two internal surfaces inside a run of glass — and,
                    // now, inside a run of water: river-to-river faces are the
                    // same no-draw.
                    //
                    // At most one side can win when the tiebreak has an opinion:
                    // if both are solid, the loser is the one facing an opaque
                    // neighbour. The one tie left is two DIFFERENT see-through
                    // blocks meeting — a pane against a river — where the mask
                    // genuinely holds one face and `a`'s wins. The visible cost
                    // is a missing pane face at a glass/water contact, a rare
                    // adjacency no shipped map contains; growing a second mask
                    // layer for it is not worth the merge-key complexity today.
                    let show_a = a.is_solid() && !hides(b) && a != b;
                    let show_b = b.is_solid() && !hides(a) && a != b;
                    if show_a && show_b {
                        // The one tie the mask cannot hold both of, and it is
                        // now a real state rather than an impossible one: two
                        // DIFFERENT see-through blocks meeting — a pane against
                        // a river. `a`'s face wins and the pane's is dropped;
                        // the visible cost is one missing face at a
                        // glass/water contact, an adjacency no shipped map
                        // contains. Anything else reaching here is a bug in
                        // this predicate, not a supported tie.
                        debug_assert!(
                            !hides(a) && !hides(b),
                            "unexpected double-show at {x:?} — {a:?} vs {b:?}"
                        );
                    }

                    mask[n] = if !show_a && !show_b {
                        MaskFace::default()
                    } else {
                        // Which side draws decides both the winding and which
                        // layer the occluders are sampled from: AO is cast by
                        // the blocks sitting in front of the face, never behind.
                        let (id, air_d, cell) = if show_a {
                            (a.0 as i32, x[d] + 1, [x[0], x[1], x[2]])
                        } else {
                            (
                                -(b.0 as i32),
                                x[d],
                                [x[0] + q[0], x[1] + q[1], x[2] + q[2]],
                            )
                        };
                        let solid = |du: i32, dv: i32| -> bool {
                            let mut p = [0i32; 3];
                            p[d] = air_d;
                            p[u] = i + du;
                            p[v] = j + dv;
                            hides(chunk.get(p[0], p[1], p[2]))
                        };
                        // Corner order matches the emitted vertex order below:
                        // (0,0) (1,0) (1,1) (0,1) in (u,v).
                        let ao = [
                            ao_corner(solid(-1, 0), solid(0, -1), solid(-1, -1)),
                            ao_corner(solid(1, 0), solid(0, -1), solid(1, -1)),
                            ao_corner(solid(1, 0), solid(0, 1), solid(1, 1)),
                            ao_corner(solid(-1, 0), solid(0, 1), solid(-1, 1)),
                        ];
                        // The eave term. Sampled from the SAME air layer the
                        // corner term reads, one column of sky per cell, and
                        // averaged per corner over the four columns touching it
                        // — that averaging is what makes it a soft gradient
                        // across a floor instead of a block-shaped stamp.
                        //
                        // A down-facing face is skipped: the column straight up
                        // from the air beneath it starts with the face's OWN
                        // block, so every underside in the world would come back
                        // fully roofed and the term would carry no information.
                        // Undersides already read dark from the corner term and
                        // from having no sun on them.
                        // Computed even when the lever is OFF, and that is
                        // deliberate: `contact` is part of the merge key, so
                        // skipping it under `AO=off` would hand the "before"
                        // plate a DIFFERENT set of quads and a faster mesher,
                        // and a capture that fires 3.2 s after launch can
                        // photograph a different set of loaded chunks. Paying
                        // the cost on both sides makes the pair differ in
                        // vertex colour and nothing else.
                        let contact = if !(d == 1 && !show_a) {
                            let col = |du: i32, dv: i32| -> f32 {
                                let mut p = [0i32; 3];
                                p[d] = air_d;
                                p[u] = i + du;
                                p[v] = j + dv;
                                contact_column(chunk, p[0], p[1], p[2])
                            };
                            let (nn, zn, pn) = (col(-1, -1), col(0, -1), col(1, -1));
                            let (nz, zz, pz) = (col(-1, 0), col(0, 0), col(1, 0));
                            let (np, zp, pp) = (col(-1, 1), col(0, 1), col(1, 1));
                            let q = |a: f32, b: f32, c: f32, e: f32| -> u8 {
                                let m = (a + b + c + e) * 0.25 * CONTACT_STEPS as f32;
                                m.round().clamp(0.0, CONTACT_STEPS as f32) as u8
                            };
                            // Same corner order as `ao` above: (0,0) (1,0) (1,1) (0,1).
                            [
                                q(nn, zn, nz, zz),
                                q(zn, pn, zz, pz),
                                q(zz, pz, zp, pp),
                                q(nz, zz, np, zp),
                            ]
                        } else {
                            [0; 4]
                        };
                        // A liquid's face knows whether it belongs to a surface
                        // cell: submerged cells (water directly above) carry 0
                        // so their sides run full height and join the column
                        // above, surface cells carry [`WATER_TOP_OFFSET`] so the
                        // quad's top edge comes down to the meniscus. Out-of-
                        // chunk reads are AIR by `ChunkData::get`, which is the
                        // right answer at a chunk-top boundary too.
                        let top = if id.unsigned_abs() as u8 == BlockId::WATER.0 {
                            match chunk.get(cell[0], cell[1] + 1, cell[2]) {
                                BlockId::WATER => 0,
                                _ => WATER_TOP_OFFSET,
                            }
                        } else {
                            0
                        };
                        MaskFace {
                            id,
                            ao,
                            contact,
                            top,
                        }
                    };
                    n += 1;
                }
            }

            x[d] += 1;

            // ---- greedily merge the mask into quads ----
            let mut n = 0usize;
            for j in 0..dims[v] {
                let mut i = 0i32;
                while i < dims[u] {
                    let c = mask[n];
                    if c.id != 0 {
                        // width run along u
                        let mut w = 1i32;
                        while i + w < dims[u] && mask[n + w as usize] == c {
                            w += 1;
                        }
                        // height run along v
                        let mut h = 1i32;
                        'grow: while j + h < dims[v] {
                            for k in 0..w {
                                if mask[n + k as usize + (h * dims[u]) as usize] != c {
                                    break 'grow;
                                }
                            }
                            h += 1;
                        }

                        // emit the quad
                        x[u] = i;
                        x[v] = j;
                        let mut du = [0f32; 3];
                        du[u] = w as f32;
                        let mut dv = [0f32; 3];
                        dv[v] = h as f32;
                        let mut p = [x[0] as f32, x[1] as f32, x[2] as f32];

                        let front = c.id > 0;
                        let block = c.id.unsigned_abs() as u8;

                        // A water surface quad sits below the cell top. The top
                        // face's plane drops by the offset; a side face's upper
                        // edge comes down with it, so the shoreline shows the
                        // bank's side wall standing above the waterline instead
                        // of water meeting the world at a hard glass edge.
                        //
                        // `du`/`dv` each carry exactly one nonzero component,
                        // and for the two side sweeps that component is the
                        // vertical extent (d == 0 ⇒ u is Y, d == 2 ⇒ v is Y) —
                        // shaving it lowers only the quad's top edge, leaving
                        // the bottom edge on the bed where the column below
                        // continues. Safe against merging because the mask
                        // compares `top` (see [`MaskFace`]), and a merged run
                        // can never straddle surface and submerged cells: two
                        // vertically adjacent surface cells cannot exist, since
                        // the cell above a surface cell is air by definition.
                        let woff = if block == BlockId::WATER.0 {
                            c.top as f32 / 16.0
                        } else {
                            0.0
                        };
                        if woff > 0.0 {
                            if d == 1 && front {
                                p[1] -= woff;
                            } else if d == 0 {
                                du[1] -= woff;
                            } else if d == 2 {
                                dv[1] -= woff;
                            }
                        }
                        let mut nrm = [0f32; 3];
                        nrm[d] = if front { 1.0 } else { -1.0 };

                        // Which of the three faces this quad shows. Constant
                        // across the merge: `d` is fixed for the whole sweep
                        // pass and `front` is baked into the mask cell's sign,
                        // which the merge compares for equality. `face_slot`
                        // folds it back onto `Side` for a block whose faces are
                        // all the same tile, so those chunks keep their old
                        // draw-call count.
                        let face = face_slot(BlockId(block), Face::from_quad(d, front));

                        let key = if split {
                            material_index(BlockId(block), face)
                        } else {
                            0
                        };
                        let buf = buckets[key].get_or_insert_with(Buffers::default);

                        let base = buf.positions.len() as u32;
                        let v0 = p;
                        let v1 = [p[0] + du[0], p[1] + du[1], p[2] + du[2]];
                        let v2 = [
                            p[0] + du[0] + dv[0],
                            p[1] + du[1] + dv[1],
                            p[2] + du[2] + dv[2],
                        ];
                        let v3 = [p[0] + dv[0], p[1] + dv[1], p[2] + dv[2]];
                        buf.positions.extend_from_slice(&[v0, v1, v2, v3]);
                        buf.normals.extend_from_slice(&[nrm; 4]);

                        if split {
                            // The quads are axis-aligned and their UVs run
                            // straight down (du, dv), so the tangent frame is
                            // exact: no `generate_tangents()` pass, and no seam
                            // where a generated frame would flip.
                            let mut t = [0f32; 3];
                            t[u] = 1.0;
                            let mut b = [0f32; 3];
                            b[v] = 1.0;
                            // The shader rebuilds the bitangent as
                            // cross(N, T) * w, so w is whichever sign puts it
                            // back on the +v axis — back faces flip with their
                            // normal instead of being hard-coded per axis.
                            let sign = Vec3::from(nrm)
                                .cross(Vec3::from(t))
                                .dot(Vec3::from(b))
                                .signum();
                            buf.tangents
                                .extend_from_slice(&[[t[0], t[1], t[2], sign]; 4]);

                            // UVs in BLOCKS. With the tile texture sampled in
                            // `Repeat` (see `build_face_texture`) a 12×3 merged
                            // quad samples the tile 12×3 times — per-block texel
                            // density, and no atlas neighbour to bleed in.
                            let (fw, fh) = (w as f32, h as f32);
                            buf.uvs
                                .extend_from_slice(&[[0.0, 0.0], [fw, 0.0], [fw, fh], [0.0, fh]]);
                        } else {
                            // Atlas UVs: one tile stretched across the merged
                            // quad — the structural limitation of a shared-atlas
                            // greedy mesh, and the reason the split path exists.
                            //
                            // The column is per (block, face), matching
                            // `build_atlas`: the far LOD wears the same per-face
                            // art the near chunks do, so a grass block does not
                            // grow a dirt top at the LOD line.
                            //
                            // The half-texel inset is what keeps a merged quad
                            // from ever sampling the NEIGHBOURING tile at its
                            // edge; it is now applied on BOTH axes (v used to run
                            // a raw 0.0..1.0 and could pick up the wrap row).
                            let col = material_index(BlockId(block), face).min(N_COLS - 1);
                            let t = col as f32;
                            // Half a texel of the atlas AS PACKED — `atlas_tile_px`,
                            // not the procedural bake size. Insetting by 0.5/16
                            // into a 64 px atlas would inset by two texels and
                            // shave a visible sliver off every LOD block face.
                            let atlas_px = atlas_tile_px();
                            let inset_u = 0.5 / (N_COLS * atlas_px) as f32;
                            let inset_v = 0.5 / atlas_px as f32;
                            let u0 = t / N_COLS as f32 + inset_u;
                            let u1 = (t + 1.0) / N_COLS as f32 - inset_u;
                            let (v0t, v1t) = (inset_v, 1.0 - inset_v);
                            buf.uvs.extend_from_slice(&[
                                [u0, v0t],
                                [u1, v0t],
                                [u1, v1t],
                                [u0, v1t],
                            ]);
                        }

                        // Per-instance micro-variation: deterministic world-space
                        // noise shifts the colour cast and micro-AO of each
                        // quad independently, so a long wall of the same block
                        // type is not one uniform colour. Only the split path
                        // carries this — the atlas/LOD path is a single merged
                        // draw and has no per-block identity to vary.
                        let (nr, ng, nb, mao) = if instance_noise {
                            let wx = wox as f32 + p[0];
                            let wy = woy as f32 + p[1];
                            let wz = woz as f32 + p[2];
                            (
                                hash_quad(wx, wy, wz, 211) * 0.03 - 0.015,
                                hash_quad(wx, wy, wz, 223) * 0.03 - 0.015,
                                hash_quad(wx, wy, wz, 227) * 0.03 - 0.015,
                                hash_quad(wx, wy, wz, 201) * 0.04 - 0.02,
                            )
                        } else {
                            (0.0, 0.0, 0.0, 0.0)
                        };
                        for (k, a) in c.ao.into_iter().enumerate() {
                            // Corner term and eave term compose as one darkening
                            // amount, and the lever scales that amount — so
                            // `VOXELFORGE_AO=off` is a genuinely flat-lit frame
                            // (every vertex 1.0) rather than a weaker version of
                            // this one, and `=2` is the same frame with the
                            // darkening doubled. One binary, three plates.
                            let eave =
                                c.contact[k] as f32 / CONTACT_STEPS as f32 * CONTACT_MAX;
                            let dark = ((1.0 - AO_SHADE[a as usize]) + eave) * ao_scale;
                            let ao = (1.0 - dark).clamp(0.0, 1.0);
                            let bright = (ao + mao).clamp(0.0, 1.0);
                            if stats {
                                AO_VERTS.fetch_add(1, Ordering::Relaxed);
                                if ao < 0.995 {
                                    AO_DARK.fetch_add(1, Ordering::Relaxed);
                                }
                                AO_MIN.fetch_min((ao * 1000.0) as u32, Ordering::Relaxed);
                            }
                            buf.colors.push([
                                (bright + nr).clamp(0.0, 1.0),
                                (bright + ng).clamp(0.0, 1.0),
                                (bright + nb).clamp(0.0, 1.0),
                                1.0,
                            ]);
                        }

                        // Pick the diagonal that does NOT run between the two
                        // corners with the widest occlusion gap. Splitting the
                        // other way makes a shaded corner leak a hard triangular
                        // seam across the face — the classic voxel-AO artefact.
                        //
                        // Read off the COMBINED shade, not the corner level
                        // alone: a quad whose asymmetry comes from the eave term
                        // wants the same treatment, and with `AO_SHADE` evenly
                        // spaced this is bit-identical to the old comparison
                        // wherever the eave term is zero.
                        let lum = |k: usize| -> f32 {
                            AO_SHADE[c.ao[k] as usize]
                                - c.contact[k] as f32 / CONTACT_STEPS as f32 * CONTACT_MAX
                        };
                        let flip = lum(0) + lum(2) > lum(1) + lum(3);
                        let tris: [u32; 6] = match (front, flip) {
                            (true, false) => [0, 1, 2, 0, 2, 3],
                            (true, true) => [0, 1, 3, 1, 2, 3],
                            (false, false) => [0, 2, 1, 0, 3, 2],
                            (false, true) => [0, 3, 1, 1, 3, 2],
                        };
                        buf.indices.extend(tris.iter().map(|t| base + t));
                        buf.quads += 1;

                        // zero the consumed mask cells
                        for l in 0..h {
                            for k in 0..w {
                                mask[n + k as usize + (l * dims[u]) as usize] = MaskFace::default();
                            }
                        }
                        i += w;
                        n += w as usize;
                    } else {
                        i += 1;
                        n += 1;
                    }
                }
            }
        }
    }

    if stats {
        let n = AO_CHUNKS.fetch_add(1, Ordering::Relaxed) + 1;
        // Every 32 chunks, not every chunk: enough to read the trend in a
        // 4-second capture run without the log becoming the bottleneck.
        if n % 32 == 0 {
            let verts = AO_VERTS.load(Ordering::Relaxed).max(1);
            let dark = AO_DARK.load(Ordering::Relaxed);
            let min = AO_MIN.load(Ordering::Relaxed);
            println!(
                "AO_STATS chunks={n} verts={verts} dark={:.1}% min_shade={:.3} scale={ao_scale}",
                100.0 * dark as f64 / verts as f64,
                min as f32 / 1000.0,
            );
        }
    }

    buckets
        .into_iter()
        .enumerate()
        .filter_map(|(key, b)| b.map(|b| (key, b)))
        .collect()
}

// ---------------------------------------------------------------------------
// textures
// ---------------------------------------------------------------------------

/// Cheap deterministic 2-D hash — the source of every dither and blotch below.
#[inline]
fn hash2(x: u32, y: u32, salt: u32) -> u32 {
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        ^ salt.wrapping_mul(2654435761);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

/// Signed noise in `-amp..=amp`.
#[inline]
fn dither(x: u32, y: u32, salt: u32, amp: i32) -> i32 {
    (hash2(x, y, salt) >> 8) as i32 % (2 * amp + 1) - amp
}

/// Deterministic per-quad float in `0..1`, seeded from world-space position so
/// the same face gets the same variation after every re-mesh — the noise is
/// spatial, not temporal.
#[inline]
fn hash_quad(wx: f32, wy: f32, wz: f32, salt: u32) -> f32 {
    let h = hash2(wx as u32, wy as u32, salt);
    let h = h.wrapping_add(wz as u32 * 127u32);
    // 100-step normalisation enough to avoid visible banding, coarse enough
    // that a single-block offset reads as a distinct instance.
    (h >> 8) as f32 % 100.0 / 100.0
}

/// Per-texel shade for one block's 16×16 tile, at tile-local `(lx, ly)`.
///
/// Every pattern here is **seamless across the tile edge** — that is the whole
/// point of the rewrite. The old tile painted a 1-texel `-40` frame around all
/// four sides, which is where the thick black outline came from: greedy meshing
/// stretches ONE tile across a merged quad, so a 1-texel frame around a 16×16
/// tile becomes a fat dark band around a 16-block wall. The block-to-block read
/// now comes from vertex AO (which is where it comes from in the reference art
/// too), not from a painted border.
fn tile_shade(id: BlockId, lx: usize, ly: usize) -> i32 {
    let (x, y) = (lx as u32, ly as u32);
    let px = PROC_TILE_PX as u32;
    match id {
        // Plank courses, 4 texels tall: a soft groove at the course line and a
        // long grain streak along it. Height 4 divides 16, so it tiles.
        BlockId::WOOD => {
            let course = y / 4;
            let groove = if y % 4 == 0 { -16 } else { 0 };
            let grain = dither(x / 2, course, 11, 9);
            let fleck = dither(x, y, 12, 4);
            groove + grain + fleck
        }
        // Brick courses 4 tall with the head joint offset every other course —
        // period 8 in x, so it tiles too.
        BlockId::BRICK => {
            let course = y / 4;
            let offset = if course % 2 == 0 { 0 } else { 4 };
            let head = (x + offset) % 8 == 0;
            let bed = y % 4 == 0;
            if head || bed {
                -22
            } else {
                dither(x, y, 13, 7)
            }
        }
        // Chunky blotches: sample the hash at half resolution so the lumps read
        // as stones rather than as static.
        BlockId::COBBLESTONE | BlockId::GRAVEL => {
            dither(x / 2, y / 2, 21, 22) + dither(x, y, 22, 5)
        }
        BlockId::STONE | BlockId::CLAY | BlockId::LIMESTONE => {
            dither(x / 4, y / 4, 31, 9) + dither(x, y, 32, 5)
        }
        // Foliage: dense mottle with occasional dark gaps, so a leaf block does
        // not read as a flat green cube.
        BlockId::LEAVES | BlockId::MOSS | BlockId::GRASS => {
            let gap = hash2(x, y, 41) % 7 == 0;
            dither(x, y, 42, 16) + if gap { -26 } else { 0 }
        }
        BlockId::SNOW => dither(x, y, 51, 6),
        BlockId::SAND | BlockId::RED_SAND | BlockId::DIRT => dither(x, y, 61, 12),
        // Near-flat with a faint diagonal sheen — it is a polished pane, and its
        // interest is supposed to come from the specular, not from the albedo.
        BlockId::OBSIDIAN => {
            let sheen = if (x + y) % px == 0 { 18 } else { 0 };
            sheen + dither(x, y, 71, 4)
        }
        // Lantern: bright core falling off to a warm frame, so even without the
        // emissive term the tile reads as a light.
        LAMP => {
            let cx = x as i32 * 2 - (px as i32 - 1);
            let cy = y as i32 * 2 - (px as i32 - 1);
            let r = (cx * cx + cy * cy) / 40;
            (34 - r).clamp(-30, 34)
        }
        _ => dither(x, y, 91, 13),
    }
}

/// Peak absolute amplitude any [`tile_shade`] pattern reaches, used to normalise
/// shade into a height field. The lantern's bright core (`+34`) is the widest
/// swing in the palette.
const SHADE_SPAN: f32 = 34.0;

/// How far a normal tilts per unit of height slope: a quarter of the full swing
/// across two texels lands on roughly 45° at `relief == 1.0`.
const NORMAL_STRENGTH: f32 = 4.0;

/// The tile's height field at tile-local `(lx, ly)`, in `0.0..=1.0`, sampled
/// **wrapped**.
///
/// Derived from the same [`tile_shade`] pattern that paints the albedo rather
/// than from a second hand-authored pattern — that is the load-bearing decision
/// here: albedo, relief and finish cannot drift out of register, because there
/// is only one pattern. A normal map that disagrees with its own albedo is
/// exactly what reads as plastic.
///
/// Wrapped, not clamped, on purpose. The tiles are sampled in `Repeat` and every
/// `tile_shade` pattern is seamless, so a clamped edge would bake a one-texel
/// ridge into every block boundary — the same class of bug as the old painted
/// border, just in the normal instead of the colour.
fn tile_height(id: BlockId, lx: i32, ly: i32) -> f32 {
    let px = PROC_TILE_PX as i32;
    let x = lx.rem_euclid(px) as usize;
    let y = ly.rem_euclid(px) as usize;
    (0.5 + tile_shade(id, x, y) as f32 / (2.0 * SHADE_SPAN)).clamp(0.0, 1.0)
}

/// The height field for one FACE, in `0.0..=1.0`, sampled wrapped.
///
/// The same "one pattern, three maps" rule as [`tile_height`], extended to the
/// file-backed set: when a face wears an artist's tile, its relief is read off
/// **that tile's own luminance**, not off the procedural pattern the block used
/// to wear. Deriving the normal map from a pattern the eye can no longer see is
/// how a textured wall ends up lit like a different wall — grooves catching the
/// sun where the art has none.
///
/// Luminance in linear light for the same reason `block_atlas` averages there:
/// sRGB bytes are not proportional to light, and a height field built from them
/// crushes its own midtones.
///
/// The wrap modulus is the TILE'S OWN size, resolved in the same match that
/// finds it. Reading an artist's 64² tile with a 16 stride is not a rounding
/// error — it walks the first four rows of the image and calls them the whole
/// surface, so a normal map built from a 64 px art set would be four rows of it
/// tiled sixteen times over.
fn face_height(id: BlockId, face: Face, lx: i32, ly: i32) -> f32 {
    match tile_set().and_then(|s| s.face_tile(atlas_kind(id), face).map(|t| (s.tile_px as usize, t)))
    {
        Some((px, t)) => {
            let x = lx.rem_euclid(px as i32) as usize;
            let y = ly.rem_euclid(px as i32) as usize;
            let i = (y * px + x) * 4;
            (0.2126 * srgb_to_linear(t[i])
                + 0.7152 * srgb_to_linear(t[i + 1])
                + 0.0722 * srgb_to_linear(t[i + 2]))
            .clamp(0.0, 1.0)
        }
        None => tile_height(id, lx, ly),
    }
}

/// sRGB byte → linear float. A local copy of `block_atlas`'s, because that one
/// is a private detail of its packing and this one is a private detail of the
/// height field; making either public would tie two unrelated modules together
/// over four lines of arithmetic.
#[inline]
fn srgb_to_linear(b: u8) -> f32 {
    let s = b as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// `-1.0..=1.0` → `0..=255`: the tangent-space normal encoding.
#[inline]
fn encode_unorm(v: f32) -> u8 {
    ((v * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The per-texel shading normal for one block type, or `None` for a material
/// that is deliberately flat.
///
/// This is the half of the fix that albedo cannot do. Before it, every texel of
/// a cobble wall handed the sun the same normal, so the painted stones in the
/// tile stayed a *picture* of stones: move the sun and the entire wall brightens
/// together. With it, a plank groove, a mortar joint and the gap between two
/// cobbles catch and lose the light on their own.
///
/// `Rgba8Unorm` — a normal map is data, not colour, and an sRGB view of it would
/// bend every normal toward the viewer.
pub fn build_face_normal_map(id: BlockId, face: Face) -> Option<Image> {
    if flat_material() {
        return None;
    }
    let relief = block_relief(id).relief * env_scale("VOXELFORGE_MAT_RELIEF");
    if relief <= 0.0 {
        return None;
    }

    // The face's OWN size: a derived map has to be per-texel with the albedo it
    // was derived from, or the relief lands on a grid the colour does not have.
    let tpx = face_px(id, face);
    let mut data = vec![0u8; tpx * tpx * 4];
    for ly in 0..tpx as i32 {
        for lx in 0..tpx as i32 {
            let h = |dx: i32, dy: i32| face_height(id, face, lx + dx, ly + dy);
            // Central differences: symmetric, so a groove tilts both of its
            // walls by the same amount instead of leaning the whole tile one way.
            let dhdu = (h(1, 0) - h(-1, 0)) * 0.5;
            let dhdv = (h(0, 1) - h(0, -1)) * 0.5;
            let n = Vec3::new(
                -dhdu * relief * NORMAL_STRENGTH,
                -dhdv * relief * NORMAL_STRENGTH,
                1.0,
            )
            .normalize();
            let px = (ly as usize * tpx + lx as usize) * 4;
            data[px] = encode_unorm(n.x);
            data[px + 1] = encode_unorm(n.y);
            data[px + 2] = encode_unorm(n.z);
            data[px + 3] = 255;
        }
    }
    Some(image_from_rgba(
        tpx,
        tpx,
        data,
        TextureFormat::Rgba8Unorm,
        voxel_sampler(true),
    ))
}

/// The per-texel finish for one block type, or `None` for a material with no
/// finish variation to express.
///
/// Bevy's `StandardMaterial` reads **green = roughness, blue = metallic** and
/// multiplies both by their factors, so the green channel here is the ratio
/// against [`roughness_ceiling`], never an absolute. Blue stays 0: nothing in
/// this palette is a metal.
pub fn build_face_metallic_roughness(id: BlockId, face: Face) -> Option<Image> {
    if flat_material() {
        return None;
    }
    let spread = block_relief(id).roughness_spread * env_scale("VOXELFORGE_MAT_ROUGH_VAR");
    if spread <= 0.0 {
        return None;
    }

    let base = block_surface(id).perceptual_roughness;
    let ceiling = roughness_ceiling(id);
    let tpx = face_px(id, face);
    let mut data = vec![0u8; tpx * tpx * 4];
    for ly in 0..tpx {
        for lx in 0..tpx {
            let h = face_height(id, face, lx as i32, ly as i32);
            // Hollows dusty, high points polished — dust settles where the
            // surface is worn away, and what stands proud is what gets rubbed.
            let worn = spread * (1.0 - 2.0 * h);
            // Plus coarse patchiness — FOUR patches across the tile whatever the
            // tile's resolution, so a 64 px art set gets the same broad "this
            // stretch of wall is duller than that one" read and not sixteen
            // times finer speckle. (`lx * 4 / tpx` is `lx / 4` exactly at 16 px,
            // so the procedural tiles come out byte-identical.)
            let patch = spread
                * 0.35
                * dither((lx * 4 / tpx) as u32, (ly * 4 / tpx) as u32, 81, 100) as f32
                / 100.0;
            let rough = (base + worn + patch).clamp(0.0, ceiling);
            let px = (ly * tpx + lx) * 4;
            data[px] = 255; // unused by StandardMaterial (occlusion slot)
            data[px + 1] = (rough / ceiling * 255.0).round() as u8;
            data[px + 2] = 0;
            data[px + 3] = 255;
        }
    }
    Some(image_from_rgba(
        tpx,
        tpx,
        data,
        TextureFormat::Rgba8Unorm,
        voxel_sampler(true),
    ))
}

/// How dark a fully-occluded texel gets, as a fraction removed from the ambient
/// term. Scaled per block by [`BlockRelief::relief`], so a material declared flat
/// gets no AO for the same reason it gets no normal map.
///
/// 0.85 and not 1.0: an occlusion map multiplies the *whole* indirect term, and a
/// texel that reaches 0 has no sky, no bounce and no IBL at all — a mortar joint
/// that renders as a hole punched through the wall. The floor below is the second
/// half of the same guard.
pub const AO_MAP_STRENGTH: f32 = 0.85;

/// The darkest an AO texel is allowed to get. See [`AO_MAP_STRENGTH`].
pub const AO_MAP_FLOOR: f32 = 0.32;

/// How hard a height *difference* is turned into occlusion. Heights are 0..1
/// luminance, and the gap between a brick face and its mortar joint is ~0.15–0.3
/// of that range, so a gain of 3 puts a deep joint at full occlusion and leaves
/// ordinary tile noise nearly untouched.
const AO_CAVITY_GAIN: f32 = 3.0;

/// Radius of the neighbourhood a texel is compared against, in texels, **at
/// [`PROC_TILE_PX`]**.
///
/// AO is "how much of the sky can this point see", and at tile scale the honest
/// cheap answer is "how far below its surroundings does it sit". Radius 2 (a 5×5
/// box) is wide enough to see across a mortar joint at 16 px/tile and narrow
/// enough that a plank edge still reads as an edge instead of a gradient.
///
/// Scaled per tile by [`ao_radius`]: the sentence above only holds "at 16
/// px/tile", and a joint drawn on a 64 px tile is four times as many texels
/// wide, so a fixed radius would look across a quarter of it and grade the
/// inside of the joint as open sky.
const AO_RADIUS: i32 = 2;

/// [`AO_RADIUS`] in the texels of a tile that is `tpx` across — the same
/// fraction of a tile at any resolution, never below 1.
#[inline]
fn ao_radius(tpx: usize) -> i32 {
    ((AO_RADIUS as usize * tpx / PROC_TILE_PX) as i32).max(1)
}

/// One face's height field, materialised once at its own resolution.
///
/// [`face_height`] resolves the manifest per call (a map lookup keyed by a
/// block's name), which is free at 16 px and is not at 64: the occlusion pass
/// alone takes `(2r+1)²` samples per texel, and with the radius scaling too that
/// is 289 × 4096 × 54 lookups for one art set. Sampling the field once and
/// indexing it wrapped is the same arithmetic with the lookup hoisted out.
fn face_height_field(id: BlockId, face: Face, tpx: usize) -> Vec<f32> {
    let mut h = vec![0.0f32; tpx * tpx];
    for ly in 0..tpx {
        for lx in 0..tpx {
            h[ly * tpx + lx] = face_height(id, face, lx as i32, ly as i32);
        }
    }
    h
}

/// The per-texel ambient occlusion for one face, or `None` where the material has
/// no relief to occlude.
///
/// THE THIRD MAP, AND THE ONE THE OTHER TWO CANNOT FAKE. A normal map changes
/// which way a texel faces, so it only pays out where there is a *direct* light
/// to catch — move into open shade and a normal-mapped wall is exactly as flat as
/// an unmapped one, because ambient arrives from everywhere and does not care
/// which way anything points. That is the flatness left in the frame after the
/// normal/roughness pass: every surface not in the sun is lit by a term with no
/// spatial structure whatsoever. Occlusion is the map that gives that term
/// structure — the joint stays dark when the sun leaves.
///
/// Derived from the same height field as the other two ([`face_height`], so it
/// reads the artist's tile where there is one) for the reason the module header
/// gives: three maps disagreeing about where the grooves are is worse than none.
pub fn build_face_occlusion(id: BlockId, face: Face) -> Option<Image> {
    if flat_material() {
        return None;
    }
    let strength =
        block_relief(id).relief * AO_MAP_STRENGTH * env_scale("VOXELFORGE_MAT_AO");
    if strength <= 0.0 {
        return None;
    }

    let tpx = face_px(id, face);
    let px = tpx as i32;
    let radius = ao_radius(tpx);
    let taps = ((2 * radius + 1) * (2 * radius + 1)) as f32;
    let field = face_height_field(id, face, tpx);
    let at = |x: i32, y: i32| field[y.rem_euclid(px) as usize * tpx + x.rem_euclid(px) as usize];
    let mut data = vec![0u8; tpx * tpx * 4];
    for ly in 0..px {
        for lx in 0..px {
            let h = at(lx, ly);
            // Wrapped, like every other sample here: the tile repeats across a
            // merged quad, so a neighbourhood that stopped at the edge would draw
            // a dark seam down every 16th column of a long wall.
            let mut local = 0.0;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    local += at(lx + dx, ly + dy);
                }
            }
            local /= taps;
            // Only *below* its surroundings occludes. A texel standing proud is
            // not "negatively occluded" — clamping at zero keeps this a
            // darkening-only term, so it can never brighten past the lighting
            // rig's own answer.
            let cavity = ((local - h) * AO_CAVITY_GAIN).clamp(0.0, 1.0);
            let ao = (1.0 - strength * cavity).max(AO_MAP_FLOOR);
            let v = (ao * 255.0).round() as u8;
            let i = (ly as usize * tpx + lx as usize) * 4;
            // Bevy reads RED. Green and blue mirror it so that a dumped PNG is a
            // legible greyscale AO map instead of a red-tinted puzzle, and so
            // that binding this by mistake as an `_orm` still means the same
            // thing in the one channel that matters.
            data[i] = v;
            data[i + 1] = v;
            data[i + 2] = v;
            data[i + 3] = 255;
        }
    }
    Some(image_from_rgba(
        tpx,
        tpx,
        data,
        TextureFormat::Rgba8Unorm,
        voxel_sampler(true),
    ))
}

/// Base colour for a tile — one lookup, straight off the sim palette.
///
/// The lamp used to be special-cased here with its own literal, from back when
/// `BlockId::LAMP` did not exist in sim. It does now, and carrying a second copy
/// of one block's colour is how a palette drifts: `import.rs::closest_block`
/// reads `base_color()` while the renderer read this, so the same lamp was two
/// different oranges depending on who asked. The literal moved into
/// `block.rs::base_color()` byte-for-byte, so this unification repaints nothing.
fn tile_base(id: BlockId) -> [u8; 3] {
    id.base_color()
}

/// The nearest-neighbour sampler every voxel texture uses.
///
/// Point filtering is the entire "crisp pixels, not mush" requirement: with
/// `Linear` the 16×16 tiles blur into smears the moment a quad is bigger than
/// the texture, which is always. `mipmap_filter` is Nearest as well — these
/// images ship without mip chains, and a Linear mip filter on a chain that isn't
/// there is a silent trap for whoever adds one later.
fn voxel_sampler(repeat: bool) -> ImageSampler {
    let mode = if repeat {
        ImageAddressMode::Repeat
    } else {
        ImageAddressMode::ClampToEdge
    };
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: Some("voxel".into()),
        address_mode_u: mode,
        address_mode_v: mode,
        address_mode_w: mode,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    })
}

/// `format` is the caller's call and matters: albedo is `Rgba8UnormSrgb`, but a
/// normal or roughness map is data — viewing one through sRGB silently bends
/// every value it carries.
fn image_from_rgba(
    w: usize,
    h: usize,
    data: Vec<u8>,
    format: TextureFormat,
    sampler: ImageSampler,
) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = sampler;
    image
}

/// One block face's RGBA texels, `face_px(id, face)` square: the artist's tile
/// when the manifest has one, otherwise the procedural pattern at
/// [`PROC_TILE_PX`].
///
/// The single place the two art sources meet. Every consumer — the near split
/// path's per-face texture, the far LOD atlas, the height field behind the
/// normal and roughness maps — reads through here, so a block cannot be painted
/// from the file set and lit from the procedural one.
///
/// The two sources are allowed to disagree about size. Ask [`face_px`] what came
/// back; do not assume.
fn face_texels(id: BlockId, face: Face) -> Vec<u8> {
    if let Some(t) = face_tile(id, face) {
        return t.to_vec();
    }
    let mut data = vec![0u8; PROC_TILE_PX * PROC_TILE_PX * 4];
    let base = tile_base(id);
    for ly in 0..PROC_TILE_PX {
        for lx in 0..PROC_TILE_PX {
            let shade = tile_shade(id, lx, ly);
            let px = (ly * PROC_TILE_PX + lx) * 4;
            for c in 0..3 {
                data[px + c] = (base[c] as i32 + shade).clamp(0, 255) as u8;
            }
            data[px + 3] = 255;
        }
    }
    data
}

/// Build the far-LOD tile atlas: one column per (block, face), laid out by
/// [`material_index`].
///
/// Used by the single-material path ([`greedy_mesh_chunk`]), which is the far
/// LOD ring. Clamped to the edge, because that path's UVs address one tile
/// inside a shared image and a `Repeat` mode there would wrap into the
/// neighbouring column's tile.
///
/// It reads the same [`face_texels`] the near chunks do. That is not tidiness:
/// the LOD ring sits directly behind the near ring in every frame, and two art
/// sets meeting at that boundary is a visible line across the world.
///
/// This is the ONE consumer that cannot let the two sources keep their own
/// sizes: every column has to be the same width or the layout stops being
/// `col * cell`. So each tile is resampled into [`atlas_tile_px`] — an exact 4×
/// point upscale for a 16 px procedural tile standing beside a 64 px art set,
/// which is what "nearest" means for pixel art, not an approximation of it.
pub fn build_atlas() -> Image {
    let cell = atlas_tile_px();
    let w = N_COLS * cell;
    let h = cell;
    let mut data = vec![0u8; w * h * 4];

    for col in 0..N_COLS {
        let id = BlockId((col / N_FACES) as u8);
        let face = Face::ALL[col % N_FACES];
        let tile = scale_tile_nearest(&face_texels(id, face), face_px(id, face), cell);
        for ty in 0..h {
            for lx in 0..cell {
                let src = (ty * cell + lx) * 4;
                let dst = (ty * w + col * cell + lx) * 4;
                data[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
            }
        }
    }

    image_from_rgba(
        w,
        h,
        data,
        TextureFormat::Rgba8UnormSrgb,
        voxel_sampler(false),
    )
}

/// Build the standalone tile for one block face, at that face's own resolution,
/// sampled with `Repeat`.
///
/// Used by the split path, where UVs are measured in blocks — so this texture
/// tiles once per block no matter how many blocks a merged quad covers. That is
/// the reason the near path cannot simply address a window inside the packed
/// `block_atlas` image: no address mode repeats a *sub-rectangle*.
///
/// Standalone is also why the near path needs no resampling at all: each face
/// gets its own image, so a 64 px artist tile and a 16 px procedural one live
/// side by side at full resolution and neither is stretched to meet the other.
pub fn build_face_texture(id: BlockId, face: Face) -> Image {
    let tpx = face_px(id, face);
    image_from_rgba(
        tpx,
        tpx,
        face_texels(id, face),
        TextureFormat::Rgba8UnormSrgb,
        voxel_sampler(true),
    )
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;
    use voxelforge_sim::chunk::ChunkPos;

    fn colors(mesh: &Mesh) -> Vec<[f32; 4]> {
        match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(VertexAttributeValues::Float32x4(v)) => v.clone(),
            _ => panic!("mesh carries no Float32x4 COLOR attribute"),
        }
    }

    fn normals(mesh: &Mesh) -> Vec<[f32; 3]> {
        match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
            _ => panic!("mesh has no normals"),
        }
    }

    fn uvs(mesh: &Mesh) -> Vec<[f32; 2]> {
        match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(VertexAttributeValues::Float32x2(v)) => v.clone(),
            _ => panic!("mesh carries no Float32x2 UV_0 attribute"),
        }
    }

    /// A 4×1×4 stone pad on the chunk floor, plus an optional wall, so AO has
    /// something to be cast by.
    fn pad(wall: bool) -> ChunkData {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        for z in 4..8 {
            for x in 4..8 {
                c.set(x, 4, z, BlockId::STONE);
            }
        }
        if wall {
            for y in 5..8 {
                for z in 4..8 {
                    c.set(4, y, z, BlockId::STONE);
                }
            }
        }
        c
    }

    // ---- item 4: the black outline ----

    /// The atlas must not paint a dark frame around its tiles. That frame WAS
    /// the thick black outline: greedy meshing stretches one tile across a
    /// merged quad, so a 1-texel border becomes a band metres wide.
    #[test]
    fn atlas_tiles_have_no_painted_border() {
        let img = build_atlas();
        let cell = atlas_tile_px();
        let w = N_COLS * cell;
        let data = img.data.as_ref().expect("atlas has pixel data");
        let lum = |tx: usize, ty: usize| -> i32 {
            let p = (ty * w + tx) * 4;
            data[p] as i32 + data[p + 1] as i32 + data[p + 2] as i32
        };
        // Dirt: a flat mineral whose pattern is pure low-amplitude dither, so
        // any edge-vs-centre gap would have to be a painted frame. An artist's
        // tile is allowed a dark edge (mortar, leading, a plank groove) without
        // that being the old bug, and the test binary's working directory is the
        // crate root, where `assets/textures/blocks` does not resolve — so this
        // reads the procedural generator, which is what it is about.
        let t = material_index(BlockId::DIRT, Face::Side) * cell;
        let centre = lum(t + cell / 2, cell / 2);
        for k in 0..cell {
            for (tx, ty) in [(t + k, 0), (t + k, cell - 1), (t, k), (t + cell - 1, k)] {
                let edge = lum(tx, ty);
                assert!(
                    (edge - centre).abs() < 60,
                    "edge texel ({tx},{ty}) lum {edge} vs centre {centre}: looks like a painted border"
                );
            }
        }
    }

    /// The old bug once more, from the other side: the four corner texels of a
    /// tile used to be the darkest thing in it by a mile.
    #[test]
    fn atlas_tile_corners_are_not_the_darkest_texels() {
        let img = build_atlas();
        let cell = atlas_tile_px();
        let w = N_COLS * cell;
        let data = img.data.as_ref().unwrap();
        let t = material_index(BlockId::DIRT, Face::Side) * cell;
        let lum = |tx: usize, ty: usize| -> i32 {
            let p = (ty * w + tx) * 4;
            data[p] as i32 + data[p + 1] as i32 + data[p + 2] as i32
        };
        let corner = lum(t, 0);
        let darkest_interior = (1..cell - 1)
            .flat_map(|y| (1..cell - 1).map(move |x| (x, y)))
            .map(|(x, y)| lum(t + x, y))
            .min()
            .unwrap();
        assert!(
            corner >= darkest_interior,
            "corner {corner} darker than the darkest interior texel {darkest_interior}"
        );
    }

    // ---- item 1: sampler ----

    #[test]
    fn atlas_and_tile_samplers_are_point_filtered() {
        for img in [build_atlas(), build_face_texture(BlockId::WOOD, Face::Side)] {
            let ImageSampler::Descriptor(d) = &img.sampler else {
                panic!("voxel textures must pin an explicit sampler descriptor");
            };
            assert_eq!(d.mag_filter, ImageFilterMode::Nearest);
            assert_eq!(d.min_filter, ImageFilterMode::Nearest);
            assert_eq!(d.mipmap_filter, ImageFilterMode::Nearest);
        }
    }

    /// The per-block tile must REPEAT (the split path's UVs run 0..w blocks) and
    /// the atlas must NOT (its UVs address one tile inside a shared image).
    #[test]
    fn tile_repeats_and_atlas_clamps() {
        let ImageSampler::Descriptor(tile) = &build_face_texture(BlockId::STONE, Face::Side).sampler
        else {
            panic!()
        };
        assert_eq!(tile.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(tile.address_mode_v, ImageAddressMode::Repeat);

        let ImageSampler::Descriptor(atlas) = &build_atlas().sampler else {
            panic!()
        };
        assert_eq!(atlas.address_mode_u, ImageAddressMode::ClampToEdge);
        assert_eq!(atlas.address_mode_v, ImageAddressMode::ClampToEdge);
    }

    /// Atlas UVs must stay strictly inside their own tile on BOTH axes — the v
    /// axis used to run a raw 0.0..1.0 and could sample the wrap row.
    #[test]
    fn atlas_uvs_stay_inside_their_tile() {
        let (mesh, _) = greedy_mesh_chunk(&pad(false));
        let tile_w = 1.0 / N_COLS as f32;
        for [u, v] in uvs(&mesh) {
            // The column a UV lands in must be one of stone's three, and the UV
            // must sit strictly inside it — a half-texel short of both borders.
            let col = (u / tile_w).floor() as usize;
            assert_eq!(
                col / N_FACES,
                BlockId::STONE.0 as usize,
                "u {u} landed in column {col}, which is not stone's"
            );
            let (lo, hi) = (col as f32 * tile_w, (col + 1) as f32 * tile_w);
            assert!(u > lo && u < hi, "u {u} touches the edge of column {col}");
            assert!(v > 0.0 && v < 1.0, "v {v} touches the tile edge");
        }
    }

    // ---- item 2: vertex AO ----

    #[test]
    fn open_faces_are_fully_lit() {
        let (mesh, _) = greedy_mesh_chunk(&pad(false));
        // A lone pad has no neighbour able to occlude anything: every corner is
        // level 3.
        for c in colors(&mesh) {
            assert_eq!(c, [1.0, 1.0, 1.0, 1.0], "unoccluded face should be unshaded");
        }
    }

    #[test]
    fn a_wall_darkens_the_floor_beside_it() {
        let (mesh, _) = greedy_mesh_chunk(&pad(true));
        let shades: Vec<f32> = colors(&mesh).iter().map(|c| c[0]).collect();
        let min = shades.iter().cloned().fold(f32::INFINITY, f32::min);
        assert!(
            min < 1.0,
            "a wall standing on the pad must occlude something; min shade was {min}"
        );
        assert!(
            shades.iter().any(|s| (*s - 1.0).abs() < 1e-6),
            "faces facing away from the wall must stay fully lit"
        );
        // Every emitted shade has to come off the lattice, not out of an
        // average: one of the four corner levels, minus a whole number of eave
        // steps. Built from the live constants rather than pinned literals, so
        // a retune of either table moves the test with it.
        let lattice: Vec<f32> = AO_SHADE
            .iter()
            .flat_map(|a| {
                (0..=CONTACT_STEPS)
                    .map(move |s| a - s as f32 / CONTACT_STEPS as f32 * CONTACT_MAX)
            })
            .collect();
        for s in shades {
            assert!(
                lattice.iter().any(|a| (a - s).abs() < 1e-6),
                "shade {s} is neither an AO level nor a level minus whole eave steps"
            );
        }
    }

    /// The eave. A roof three blocks over the pad touches nothing the corner
    /// term can see — every cell of that floor has open air on all four sides —
    /// so before the contact term this floor rendered exactly as bright as open
    /// ground, which is the flat-paper read the outdoor plates were shot on.
    #[test]
    fn a_roof_overhead_darkens_the_floor_under_it() {
        let mut c = pad(false);
        for z in 4..8 {
            for x in 4..8 {
                c.set(x, 7, z, BlockId::STONE);
            }
        }
        let roofed = colors(&greedy_mesh_chunk(&c).0)
            .iter()
            .map(|c| c[0])
            .fold(f32::INFINITY, f32::min);
        let open = colors(&greedy_mesh_chunk(&pad(false)).0)
            .iter()
            .map(|c| c[0])
            .fold(f32::INFINITY, f32::min);
        assert_eq!(open, 1.0, "the bare pad is the fully-lit control");
        assert!(
            roofed < 0.95,
            "a roof {} blocks up must darken the floor under it; min shade was {roofed}",
            3
        );
    }

    /// The eave term has to FADE, or it is a stamp rather than a shadow: the
    /// same roof further up darkens less, and past [`CONTACT_RANGE`] not at all.
    #[test]
    fn the_eave_fades_with_height_and_stops_at_the_range() {
        let floor_min = |gap: i32| -> f32 {
            let mut c = pad(false);
            for z in 4..8 {
                for x in 4..8 {
                    c.set(x, 4 + gap, z, BlockId::STONE);
                }
            }
            // A plain min over the mesh is safe even though the roof's own
            // quads are in it: the roof's underside is a down-facing face and
            // the contact term skips those by construction, and nothing here
            // touches anything, so the corner term is flat 3 throughout. The
            // only thing that can move this number is the eave.
            colors(&greedy_mesh_chunk(&c).0)
                .iter()
                .map(|c| c[0])
                .fold(f32::INFINITY, f32::min)
        };
        let near = floor_min(2);
        let far = floor_min(4);
        let beyond = floor_min(CONTACT_RANGE + 2);
        assert!(near < far, "a lower roof must bite harder ({near} vs {far})");
        assert!(
            far < 1.0,
            "a roof inside the range must still bite ({far})"
        );
        assert_eq!(
            beyond, 1.0,
            "a roof past CONTACT_RANGE must leave the floor fully lit"
        );
    }

    /// The A/B lever. `off` has to produce the genuinely flat frame — every
    /// vertex 1.0, both terms gone — or the "before" plate is not a before.
    #[test]
    fn the_ao_lever_switches_the_whole_darkening_off() {
        // `ao_strength` is a process-wide OnceLock, so this asserts the arithmetic
        // the lever drives rather than re-reading the environment mid-process.
        for level in 0..4u8 {
            for step in 0..=CONTACT_STEPS {
                let eave = step as f32 / CONTACT_STEPS as f32 * CONTACT_MAX;
                let shade = |scale: f32| {
                    (1.0 - ((1.0 - AO_SHADE[level as usize]) + eave) * scale).clamp(0.0, 1.0)
                };
                assert_eq!(shade(0.0), 1.0, "AO=off must leave every vertex fully lit");
                assert!(
                    shade(2.0) <= shade(1.0),
                    "a bigger scale must never brighten a corner"
                );
            }
        }
        assert_eq!(
            AO_SHADE[3], 1.0,
            "level 3 is the open-sky corner and must cost nothing"
        );
    }

    /// AO belongs in the merge key. Without it the wall's occlusion would be
    /// averaged over one big quad and vanish; with it the run breaks where the
    /// occlusion changes, so the same pad emits MORE quads once a wall is added.
    #[test]
    fn ao_breaks_greedy_runs_instead_of_being_averaged_away() {
        let (_, open) = greedy_mesh_chunk(&pad(false));
        let (_, walled) = greedy_mesh_chunk(&pad(true));
        assert!(
            walled > open,
            "occlusion variation must split merged quads ({walled} vs {open})"
        );
    }

    /// A flat unoccluded slab must still merge all the way — AO in the merge key
    /// is not allowed to cost us greedy meshing on open ground.
    #[test]
    fn a_flat_slab_still_merges_into_one_quad_per_side() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                c.set(x, 0, z, BlockId::STONE);
            }
        }
        // A full-width 1-block slab: top, bottom and the four sides = 6 quads.
        let (_, quads) = greedy_mesh_chunk(&c);
        assert_eq!(quads, 6, "flat slab should stay fully merged");
    }

    // ---- item 3 / split path ----

    #[test]
    fn split_emits_one_mesh_per_block_type_and_loses_no_quads() {
        let mut c = pad(true);
        for z in 4..8 {
            c.set(6, 8, z, BlockId::WOOD);
        }
        let (_, total) = greedy_mesh_chunk(&c);
        let parts = greedy_mesh_chunk_split(&c);

        // Which blocks got buckets, and in material-table order. Deliberately
        // NOT an assertion on the exact face list: whether a block splits into
        // one bucket or three is a property of the ART SET on disk (see
        // `face_slot`), and a test that pinned it would pass or fail depending
        // on whether the checkout has pulled `assets/`.
        let blocks: Vec<u8> = {
            let mut v: Vec<u8> = parts.iter().map(|(k, _, _)| k.block.0).collect();
            v.dedup();
            v
        };
        assert_eq!(blocks, vec![BlockId::STONE.0, BlockId::WOOD.0]);
        assert!(
            parts.windows(2).all(|w| w[0].0.material_index() < w[1].0.material_index()),
            "buckets must come back in material-table order"
        );

        // What IS invariant: every quad in a bucket really shows that bucket's
        // face. This is the assertion the per-face UVs actually depend on — a
        // quad in the `Top` bucket wearing a side normal would wear the wrong
        // tile, and no amount of manifest data can excuse it.
        for (key, mesh, _) in &parts {
            for n in normals(mesh) {
                let shown = Face::from_quad(if n[1].abs() > 0.5 { 1 } else { 0 }, n[1] > 0.5);
                assert_eq!(
                    face_slot(key.block, shown),
                    key.face,
                    "a quad with normal {n:?} landed in the {:?} bucket",
                    key.face
                );
            }
        }
        assert_eq!(
            parts.iter().map(|(_, _, q)| q).sum::<usize>(),
            total,
            "the split path must cover exactly the same faces"
        );
    }

    /// The split path's UVs are measured in BLOCKS, so a merged quad's UV extent
    /// equals its size — that is what makes the tile repeat per block instead of
    /// stretching, and it is why the split path cannot bleed across an atlas.
    #[test]
    fn split_uvs_repeat_once_per_block() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                c.set(x, 0, z, BlockId::STONE);
            }
        }
        let parts = greedy_mesh_chunk_split(&c);
        let (_, mesh, _) = parts.first().expect("stone mesh");
        let max_u = uvs(mesh).iter().map(|p| p[0]).fold(0.0f32, f32::max);
        assert_eq!(
            max_u, CHUNK_SIZE as f32,
            "a {CHUNK_SIZE}-block-wide quad must span {CHUNK_SIZE} tile repeats"
        );
    }

    #[test]
    fn surfaces_separate_wood_stone_glass_and_lamp() {
        let wood = block_surface(BlockId::WOOD);
        let stone = block_surface(BlockId::STONE);
        let glass = block_surface(BlockId::OBSIDIAN);
        let lamp = block_surface(LAMP);

        assert!(
            wood.perceptual_roughness < stone.perceptual_roughness,
            "wood must be satin against matte stone"
        );
        assert!(wood.reflectance > stone.reflectance);
        assert!(
            glass.perceptual_roughness < wood.perceptual_roughness && glass.reflectance > 0.5,
            "the pane must be the smoothest, most reflective surface"
        );
        assert!(glass.alpha_blend && glass.alpha < 1.0);
        assert_eq!(stone.emissive, LinearRgba::BLACK);
        assert!(
            lamp.emissive.red > 1.0,
            "a lantern that never exceeds 1.0 is invisible to bloom"
        );
        // Exactly one block in the palette is a metal, on purpose.
        assert_eq!(
            block_surface(BlockId::METAL).metallic,
            1.0,
            "metal is the point of the block"
        );
        for id in BlockId::ALL_PLACEABLE
            .iter()
            .copied()
            .chain([LAMP])
            .filter(|id| *id != BlockId::METAL)
        {
            assert_eq!(block_surface(id).metallic, 0.0, "{} is not metal", id.name());
        }
    }

    // ---- water: a river has a surface, and you can see its bed ----

    /// Positions and normals of a split mesh, as parallel quad groups.
    fn quad_geometry(mesh: &Mesh) -> Vec<([f32; 3], [[f32; 3]; 4])> {
        let pos = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
            _ => panic!("no positions"),
        };
        let nrm = normals(mesh);
        pos.chunks(4)
            .zip(nrm.chunks(4))
            .map(|(p, n)| (n[0], [p[0], p[1], p[2], p[3]]))
            .collect()
    }

    /// The load-bearing geometry of [`WATER_TOP_OFFSET`]: a lone surface cell's
    /// mesh must stop 2/16 short of the cell top — top face AND side walls, or
    /// the river reads as a hard glass step against its bank.
    #[test]
    fn a_surface_water_cell_stops_below_the_cell_top() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        c.set(8, 8, 8, BlockId::WATER);
        let parts = greedy_mesh_chunk_split(&c);
        let (_, mesh, quads) = parts
            .iter()
            .find(|(k, _, _)| k.block == BlockId::WATER)
            .expect("water mesh");
        assert_eq!(*quads, 6, "a lone cell draws six faces");
        let want = 8.0 + 1.0 - WATER_TOP_OFFSET as f32 / 16.0;
        let max_y = quad_geometry(mesh)
            .iter()
            .flat_map(|(_, p)| p.iter())
            .map(|v| v[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            (max_y - want).abs() < 1e-4,
            "water's highest vertex is {max_y}, the surface should sit at {want}"
        );
        // And the bottom cap stays on the bed — the shave is top-only.
        let min_y = quad_geometry(mesh)
            .iter()
            .flat_map(|(_, p)| p.iter())
            .map(|v| v[1])
            .fold(f32::INFINITY, f32::min);
        assert!((min_y - 8.0).abs() < 1e-4, "the bed edge moved: {min_y}");
    }

    /// A column keeps one surface: the submerged cells run full height and the
    /// surface cell's shaved sides land exactly on top of them, with no
    /// internal water-to-water face in between.
    #[test]
    fn a_water_column_has_one_surface_and_no_internal_faces() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        for y in 7..9 {
            c.set(8, y, 8, BlockId::WATER);
        }
        let parts = greedy_mesh_chunk_split(&c);
        let (key, mesh, quads) = parts
            .iter()
            .find(|(k, _, _)| k.block == BlockId::WATER)
            .expect("water mesh");
        assert_eq!(key.block, BlockId::WATER);
        // 1×1×2 bar: 2 caps + 4 sides = 6 quads, exactly like stone or glass.
        assert_eq!(*quads, 6, "internal water faces were emitted");
        // The sides are two quads (full-height cell below, shaved cell above);
        // the shaved one must END at 14/16 above the bed cell, not leave a gap.
        let top = 9.0 - WATER_TOP_OFFSET as f32 / 16.0;
        let seam = 8.0; // boundary between the submerged and surface cells
        let mut saw_full = false;
        let mut saw_shaved = false;
        for (n, p) in quad_geometry(mesh) {
            if n[1].abs() > 0.5 {
                continue; // caps
            }
            let ys: Vec<f32> = p.iter().map(|v| v[1]).collect();
            if ys.iter().all(|y| (*y - seam).abs() < 1e-4 || (*y - (seam - 1.0)).abs() < 1e-4) {
                saw_full = true; // spans [7, 8]
            } else if ys.contains(&top) && ys.contains(&seam) {
                saw_shaved = true; // spans [8, 8.875]
            }
        }
        assert!(saw_full, "the submerged cell's side must run full height");
        assert!(saw_shaved, "the surface cell's side must land on the meniscus");
    }

    /// The see-through claim, the reason `is_opaque` grew a second exception:
    /// the sand under a river keeps its top face, and the river above it draws
    /// no face against the sand (nothing to see there) — you look THROUGH the
    /// water at the bed, not at a hole.
    #[test]
    fn water_does_not_hide_the_bed_below_it() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        c.set(8, 8, 8, BlockId::SAND);
        c.set(8, 9, 8, BlockId::WATER);
        let parts = greedy_mesh_chunk_split(&c);
        let quads = |b: BlockId| -> usize {
            parts
                .iter()
                .filter(|(k, _, _)| k.block == b)
                .map(|(_, _, q)| q)
                .sum()
        };
        // The sand keeps all six faces — a naive `is_solid` neighbour test
        // would cull its top and leave a hole under the river.
        assert_eq!(quads(BlockId::SAND), 6, "the water culled the bed below it");
        // The water loses only its bottom face (against the sand); its top is
        // the lowered surface and its four sides stand against air.
        assert_eq!(quads(BlockId::WATER), 5);
        // And the material actually blends, or the geometry is a lie.
        let s = block_surface(BlockId::WATER);
        assert!(s.alpha_blend && s.alpha < 1.0);
        assert!(!casts_shadow(BlockId::WATER), "a river must not shade its valley");
        assert!(casts_shadow(BlockId::METAL), "a metal block is an occluder");
    }

    #[test]
    fn block_material_carries_the_surface_and_the_texture() {
        let m = block_material(LAMP, Handle::default(), BlockMaps::default());
        assert_eq!(m.emissive, block_surface(LAMP).emissive);
        assert!(m.base_color_texture.is_some());
        let pane = block_material(BlockId::OBSIDIAN, Handle::default(), BlockMaps::default());
        assert!(matches!(pane.alpha_mode, AlphaMode::Blend));
        assert!(pane.base_color.alpha() < 1.0);
    }

    // ---- the authored-PBR contract ----

    /// The trap this whole `MrSource` enum exists for.
    ///
    /// A roughness map is very commonly a GREY png (R=G=B=roughness), and Bevy
    /// reads blue as metalness unconditionally and multiplies the `metallic`
    /// factor into it. Let that factor go to 1.0 for an authored map and every
    /// rough surface in the game turns to chrome — a bug that would arrive with
    /// the art drop, not with the code, and so would be blamed on the art.
    #[test]
    fn an_authored_roughness_map_can_never_make_metal() {
        for id in BlockId::ALL_PLACEABLE.iter().copied().chain([LAMP]) {
            let m = block_material(
                id,
                Handle::default(),
                BlockMaps {
                    metallic_roughness: Some(Handle::default()),
                    mr_source: MrSource::AuthoredRoughness,
                    ..default()
                },
            );
            assert_eq!(
                m.metallic,
                block_surface(id).metallic,
                "{}: a *_r.png must not drive metalness",
                id.name()
            );
            // …while its green channel IS the answer, so the factor is identity.
            assert_eq!(
                m.perceptual_roughness,
                1.0,
                "{}: an authored roughness map must not be re-scaled by the ceiling",
                id.name()
            );
        }
    }

    /// The other half: a PACKED map (`_mr` / `_orm`) is authored with metalness
    /// in blue on purpose, so there the factor must be identity too.
    #[test]
    fn a_packed_authored_map_drives_both_channels() {
        for src in [MrSource::AuthoredMr, MrSource::AuthoredOrm] {
            let m = block_material(
                BlockId::STONE,
                Handle::default(),
                BlockMaps {
                    metallic_roughness: Some(Handle::default()),
                    mr_source: src,
                    ..default()
                },
            );
            assert_eq!(m.metallic, 1.0, "{src:?}: blue is metalness");
            assert_eq!(m.perceptual_roughness, 1.0, "{src:?}: green is roughness");
        }
    }

    /// The procedural map's contract is the OPPOSITE one and must not have moved:
    /// its green is a ratio against [`roughness_ceiling`].
    #[test]
    fn the_procedural_map_still_carries_the_ceiling() {
        for id in [BlockId::WOOD, BlockId::COBBLESTONE, BlockId::SNOW] {
            let m = block_material(
                id,
                Handle::default(),
                BlockMaps {
                    metallic_roughness: Some(Handle::default()),
                    mr_source: MrSource::Procedural,
                    ..default()
                },
            );
            assert_eq!(m.perceptual_roughness, roughness_ceiling(id), "{}", id.name());
            assert_eq!(m.metallic, block_surface(id).metallic, "{}", id.name());
        }
    }

    /// The occlusion map has to reach `occlusion_texture` — the slot is new, and a
    /// map built but never bound is the single most likely way this pass ships as
    /// a no-op.
    #[test]
    fn the_occlusion_map_reaches_its_slot() {
        let m = block_material(
            BlockId::BRICK,
            Handle::default(),
            BlockMaps {
                occlusion: Some(Handle::default()),
                ..default()
            },
        );
        assert!(m.occlusion_texture.is_some());
        assert!(block_material(BlockId::BRICK, Handle::default(), BlockMaps::default())
            .occlusion_texture
            .is_none());
    }

    /// AO is a DARKENING-ONLY term with a floor: it must dip below white
    /// somewhere on a relief tile, and must never reach black anywhere, or a
    /// mortar joint renders as a hole punched through the wall.
    #[test]
    fn occlusion_darkens_cavities_without_reaching_black() {
        let img = build_face_occlusion(BlockId::BRICK, Face::Side).expect("brick has relief");
        let red: Vec<u8> = img
            .data
            .as_ref()
            .expect("occlusion map has pixel data")
            .chunks_exact(4)
            .map(|p| p[0])
            .collect();
        let (min, max) = (*red.iter().min().unwrap(), *red.iter().max().unwrap());
        // Darkening-only: nothing may sit above the unoccluded 1.0.
        assert_eq!(max, 255, "AO must leave open surfaces untouched");
        assert!(min < max, "an AO map with no variation is not an AO map");
        assert!(
            min as f32 / 255.0 >= AO_MAP_FLOOR - 0.01,
            "AO floor breached: {min}/255 is below {AO_MAP_FLOOR}"
        );
    }

    /// A material declared flat gets no AO, for the same reason it gets no normal
    /// map — three maps that disagree about whether a surface has relief is worse
    /// than none of them.
    #[test]
    fn a_flat_material_gets_no_occlusion() {
        for id in [BlockId::GLASS, BlockId::OBSIDIAN] {
            assert!(
                build_face_normal_map(id, Face::Side).is_none(),
                "{}: precondition — this block is declared flat",
                id.name()
            );
            assert!(
                build_face_occlusion(id, Face::Side).is_none(),
                "{}: a flat material must not carry an occlusion map",
                id.name()
            );
        }
    }

    /// Every relief block gets all THREE maps, not two. The pass is only as good
    /// as its least-covered surface.
    #[test]
    fn every_relief_block_gets_all_three_maps() {
        for id in BlockId::ALL_PLACEABLE.iter().copied().chain([LAMP]) {
            if build_face_normal_map(id, Face::Side).is_none() {
                continue; // declared flat — covered by the test above
            }
            assert!(
                build_face_occlusion(id, Face::Side).is_some(),
                "{}: has relief but no occlusion map",
                id.name()
            );
        }
    }

    // ---- material response: relief + finish ----

    fn tangents(mesh: &Mesh) -> Option<Vec<[f32; 4]>> {
        match mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
            Some(VertexAttributeValues::Float32x4(v)) => Some(v.clone()),
            _ => None,
        }
    }

    /// Bevy MULTIPLIES `perceptual_roughness` by the map's green channel. Hand
    /// the mapped case the same factor as the unmapped one and every mapped
    /// surface is quietly smoother than the table says, with nothing anywhere
    /// reporting it. The factor has to rise to the ceiling so the map can dip
    /// down from there and straddle the table value.
    #[test]
    fn a_roughness_map_raises_the_factor_to_the_ceiling() {
        for id in [BlockId::WOOD, BlockId::COBBLESTONE, BlockId::SNOW] {
            let base = block_surface(id).perceptual_roughness;
            let ceiling = roughness_ceiling(id);
            assert!(
                ceiling > base,
                "{}: ceiling {ceiling} must sit above the table value {base}",
                id.name()
            );

            let mapped = block_material(
                id,
                Handle::default(),
                BlockMaps {
                    metallic_roughness: Some(Handle::default()),
                    mr_source: MrSource::Procedural,
                    ..default()
                },
            );
            assert_eq!(
                mapped.perceptual_roughness,
                ceiling,
                "{}: a mapped material must carry the ceiling as its factor",
                id.name()
            );
            let plain = block_material(id, Handle::default(), BlockMaps::default());
            assert_eq!(
                plain.perceptual_roughness,
                base,
                "{}: an unmapped material must carry the table value",
                id.name()
            );
        }
    }

    /// The green channel is a RATIO against the ceiling, so the effective
    /// roughness (`ceiling * g`) has to land either side of the table value —
    /// not only below it.
    #[test]
    fn the_roughness_map_straddles_the_table_value() {
        let id = BlockId::WOOD;
        let base = block_surface(id).perceptual_roughness;
        let ceiling = roughness_ceiling(id);
        let img =
            build_face_metallic_roughness(id, Face::Side).expect("wood has a finish spread");
        let data = img.data.as_ref().unwrap();

        let n = face_px(id, Face::Side).pow(2);
        let eff: Vec<f32> = (0..n)
            .map(|i| data[i * 4 + 1] as f32 / 255.0 * ceiling)
            .collect();
        let lo = eff.iter().cloned().fold(f32::INFINITY, f32::min);
        let hi = eff.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            lo < base && hi > base,
            "effective roughness {lo}..{hi} must straddle the table value {base}"
        );
        // Blue is metallic. Nothing in this palette is a metal.
        assert!((0..n).all(|i| data[i * 4 + 2] == 0));
    }

    /// Wrapped, not clamped. Every `tile_shade` pattern is seamless and the
    /// tiles are sampled in `Repeat`, so a clamped edge would bake a one-texel
    /// ridge into every block boundary — the painted-border bug again, moved
    /// from the colour into the normal.
    #[test]
    fn tile_height_wraps_instead_of_clamping() {
        for id in [BlockId::WOOD, BlockId::BRICK, BlockId::COBBLESTONE] {
            let last = PROC_TILE_PX as i32 - 1;
            for k in 0..PROC_TILE_PX as i32 {
                assert_eq!(
                    tile_height(id, -1, k),
                    tile_height(id, last, k),
                    "{}: u wrapped sample must equal the far edge",
                    id.name()
                );
                assert_eq!(
                    tile_height(id, k, -1),
                    tile_height(id, k, last),
                    "{}: v wrapped sample must equal the far edge",
                    id.name()
                );
            }
        }
    }

    /// A normal map must decode to unit-ish normals pointing OUT of the surface.
    /// An all-flat map (every texel `z == 1`) would mean the pattern never
    /// reached the map at all.
    #[test]
    fn the_normal_map_encodes_outward_normals_and_is_not_flat() {
        let img = build_face_normal_map(BlockId::BRICK, Face::Side).expect("brick has relief");
        assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        let data = img.data.as_ref().unwrap();
        let tpx = face_px(BlockId::BRICK, Face::Side);
        let mut tilted = 0;
        for i in 0..tpx * tpx {
            let d = |c: usize| data[i * 4 + c] as f32 / 255.0 * 2.0 - 1.0;
            let (x, y, z) = (d(0), d(1), d(2));
            assert!(z > 0.0, "texel {i} points into the surface");
            let len = (x * x + y * y + z * z).sqrt();
            assert!((len - 1.0).abs() < 0.02, "texel {i} normal length {len}");
            if x.abs() > 0.05 || y.abs() > 0.05 {
                tilted += 1;
            }
        }
        assert!(
            tilted > tpx,
            "only {tilted} texels carry any tilt — the pattern never reached the map"
        );
    }

    /// The grade in the material table is a decision per material, and some of
    /// its entries are load-bearing zeroes: a bumped pane frosts over and loses
    /// its mirror, a glowing surface has no shading to modulate, and water's
    /// shape belongs to its authored ripples, not a derived bump. Rather than
    /// pin the flat set by hand (it drifted stale the moment glass landed), the
    /// derived maps are checked AGAINST the tables that drive them.
    #[test]
    fn derived_maps_follow_the_relief_tables() {
        assert!(
            build_face_normal_map(BlockId::OBSIDIAN, Face::Side).is_none(),
            "bumping the pane only frosts it and kills the reflection"
        );
        assert!(
            build_face_metallic_roughness(BlockId::OBSIDIAN, Face::Side).is_some(),
            "the pane still varies its finish, just not its shape"
        );
        assert!(
            build_face_metallic_roughness(LAMP, Face::Side).is_none(),
            "a glowing surface has no shading to modulate"
        );
        for id in BlockId::ALL_PLACEABLE.iter().copied().chain([LAMP]) {
            let r = block_relief(id);
            assert_eq!(
                build_face_normal_map(id, Face::Side).is_some(),
                r.relief > 0.0,
                "{}: normal map disagrees with its relief grade",
                id.name()
            );
            assert_eq!(
                build_face_metallic_roughness(id, Face::Side).is_some(),
                r.roughness_spread > 0.0,
                "{}: roughness map disagrees with its finish spread",
                id.name()
            );
            if r.relief > 0.0 {
                assert!(
                    build_face_occlusion(id, Face::Side).is_some(),
                    "{}: has relief but no occlusion map",
                    id.name()
                );
            }
        }
    }

    /// Without `ATTRIBUTE_TANGENT` Bevy drops the normal map with no warning and
    /// no error, and the frame renders identical to the flat version — so the
    /// attribute's presence is the only thing standing between this pass and it
    /// silently doing nothing. The atlas path must NOT get them: it wears no
    /// normal map, and one tile stretched over a merged quad would smear.
    #[test]
    fn only_the_split_path_carries_tangents() {
        let parts = greedy_mesh_chunk_split(&pad(true));
        let (_, mesh, _) = parts.first().expect("stone mesh");
        let t = tangents(mesh).expect("the split path must emit tangents");
        assert_eq!(t.len(), mesh.count_vertices());
        for [x, y, z, w] in t {
            let len = (x * x + y * y + z * z).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "tangent {x},{y},{z} is not unit");
            assert!(w == 1.0 || w == -1.0, "handedness {w} must be ±1");
        }

        let (atlas, _) = greedy_mesh_chunk(&pad(true));
        assert!(
            tangents(&atlas).is_none(),
            "the atlas/LOD path must stay tangent-free"
        );
    }

    /// The tangent frame has to agree with the UVs it is a frame FOR: T points
    /// along +u, and cross(N, T) * w has to land back on +v. Get the handedness
    /// wrong and every back face lights as though the sun moved.
    #[test]
    fn the_tangent_frame_matches_the_uv_axes() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        c.set(8, 8, 8, BlockId::STONE);
        let parts = greedy_mesh_chunk_split(&c);
        let (_, mesh, _) = parts.first().unwrap();
        let pos = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
            _ => panic!("no positions"),
        };
        let nrm = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
            _ => panic!("no normals"),
        };
        let uv = uvs(mesh);
        let tan = tangents(mesh).unwrap();

        // A lone block emits 6 single-block quads, 4 vertices each.
        for q in 0..pos.len() / 4 {
            let (v0, v1, v3) = (pos[q * 4], pos[q * 4 + 1], pos[q * 4 + 3]);
            let (uv0, uv1, uv3) = (uv[q * 4], uv[q * 4 + 1], uv[q * 4 + 3]);
            // v0→v1 is the +u edge, v0→v3 the +v edge (see the emit order).
            assert_eq!([uv1[0] - uv0[0], uv1[1] - uv0[1]], [1.0, 0.0]);
            assert_eq!([uv3[0] - uv0[0], uv3[1] - uv0[1]], [0.0, 1.0]);

            let du = Vec3::from(v1) - Vec3::from(v0);
            let dv = Vec3::from(v3) - Vec3::from(v0);
            let t = Vec3::new(tan[q * 4][0], tan[q * 4][1], tan[q * 4][2]);
            let w = tan[q * 4][3];
            assert!((t - du.normalize()).length() < 1e-5, "T must run along +u");
            let bitangent = Vec3::from(nrm[q * 4]).cross(t) * w;
            assert!(
                (bitangent - dv.normalize()).length() < 1e-5,
                "cross(N, T) * {w} must land back on +v"
            );
        }
    }

    // ---- glass: solid to a player, invisible to the mesher ----

    /// The load-bearing claim. A pane must NOT cull the face behind it, or the
    /// see-through is a see-through onto a hole in the world.
    #[test]
    fn a_pane_does_not_cull_the_block_behind_it() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        c.set(8, 8, 8, BlockId::STONE);
        c.set(8, 8, 9, BlockId::GLASS);
        let parts = greedy_mesh_chunk_split(&c);

        // Summed over faces, because how many buckets a block splits into
        // depends on the art set (`face_slot`) and this claim does not.
        let quads = |b: BlockId| -> usize {
            parts
                .iter()
                .filter(|(k, _, _)| k.block == b)
                .map(|(_, _, q)| q)
                .sum()
        };
        // Both blocks keep all six of their faces: the stone's +z face is behind
        // glass and still drawn, and the glass draws its own -z face over it.
        // Culling either one is what a naive `is_opaque` neighbour test does.
        assert_eq!(quads(BlockId::STONE), 6, "the pane culled the stone behind it");
        assert_eq!(quads(BlockId::GLASS), 6, "the stone culled the pane in front of it");
    }

    /// …and the other half: a RUN of glass must not draw the surfaces inside
    /// itself, or a thick window is a stack of half-lit panes.
    #[test]
    fn a_run_of_glass_draws_no_internal_faces() {
        let mut c = ChunkData::empty(ChunkPos::new(0, 0, 0));
        for z in 8..12 {
            c.set(8, 8, z, BlockId::GLASS);
        }
        let total: usize = greedy_mesh_chunk_split(&c)
            .iter()
            .map(|(_, _, q)| q)
            .sum();
        // A 1×1×4 bar: 2 end caps + 4 long sides = 6 quads, exactly as for stone.
        assert_eq!(total, 6, "internal pane-to-pane faces were emitted");
    }

    /// Glass occupies its cell. The mesher's opinion must not leak into
    /// collision, map saving or spawn clearance — all of which ask `is_solid`.
    #[test]
    fn glass_is_solid_even_though_it_is_not_opaque() {
        assert!(BlockId::GLASS.is_solid(), "you must not walk through a pane");
        assert!(!BlockId::GLASS.is_opaque(), "a pane must not hide what is behind it");
        // And nothing else in the palette changed meaning.
        for &id in BlockId::ALL_PLACEABLE {
            if id != BlockId::GLASS {
                assert_eq!(id.is_solid(), id.is_opaque(), "{} changed meaning", id.name());
            }
        }
    }

    /// The pane draws in the transparent pass with `base_color` alpha left at
    /// 1.0 — the see-through has to come from the TEXTURE, so the leading and
    /// the glint stay solid while the pane between them does not.
    #[test]
    fn the_pane_blends_from_its_texture_not_from_a_material_fade() {
        let s = block_surface(BlockId::GLASS);
        assert!(s.alpha_blend, "glass must draw in the transparent pass");
        assert_eq!(s.alpha, 1.0, "a material-wide fade would dim the leading too");
        let m = block_material(BlockId::GLASS, Handle::default(), BlockMaps::default());
        assert!(matches!(m.alpha_mode, AlphaMode::Blend));
        assert!(!m.double_sided, "a two-sided pane double-blends with itself");
        // No normal map: a map derived from a tile whose interest is in the
        // alpha channel would emboss the frame it is supposed to see past.
        assert!(build_face_normal_map(BlockId::GLASS, Face::Side).is_none());
    }

    /// The shipped `glass.png` must actually carry an alpha channel. Without it
    /// `AlphaMode::Blend` is an opaque block that pays for sorting — which is
    /// exactly the state this pass started from.
    #[test]
    fn the_shipped_glass_tile_has_real_alpha() {
        let path = std::path::Path::new("../assets/textures/blocks/glass.png");
        if !path.exists() {
            eprintln!("skip: {} not present in this checkout", path.display());
            return;
        }
        let img = image::open(path).expect("glass.png decodes").to_rgba8();
        let alphas: Vec<u8> = img.pixels().map(|p| p.0[3]).collect();
        let clear = alphas.iter().filter(|&&a| a < 128).count();
        assert!(
            alphas.iter().any(|&a| a > 200),
            "nothing in the tile is opaque — the leading and the glint are gone"
        );
        assert!(
            clear * 2 > alphas.len(),
            "only {clear}/{} texels are see-through; this is still a solid tile",
            alphas.len()
        );
    }

    // ---- the file-backed art set ----

    /// The wiring the whole feature hangs off: `voxel.rs` looks a block up in
    /// the manifest **by its own sim name** ([`atlas_kind`]). Rename a kind in
    /// `atlas.json` and that block drops silently back to its procedural tile —
    /// no error, no missing texture, just the art quietly not landing. So the
    /// names that are supposed to be wired are pinned here.
    #[test]
    fn the_shipped_manifest_names_kinds_after_sim_blocks() {
        let path = std::path::Path::new("../assets/textures/blocks/atlas.json");
        if !path.exists() {
            eprintln!("skip: {} not present in this checkout", path.display());
            return;
        }
        let raw = std::fs::read_to_string(path).expect("manifest reads");
        let doc: serde_json::Value = serde_json::from_str(&raw).expect("manifest parses");
        let kinds = doc["kinds"].as_object().expect("manifest has kinds");
        for id in [
            BlockId::GRASS,
            BlockId::SAND,
            BlockId::STONE,
            BlockId::WOOD,
            BlockId::LEAVES,
            BlockId::LIMESTONE,
            BlockId::GLASS,
            BlockId::WATER,
            BlockId::METAL,
        ] {
            assert!(
                kinds.contains_key(atlas_kind(id)),
                "atlas.json has no kind {:?} — {} silently falls back to its procedural tile",
                atlas_kind(id),
                id.name()
            );
        }
    }

    /// Every palette entry — including the client-side lamp — must have a tile
    /// in the atlas, or the shared-material path indexes past its own image.
    #[test]
    fn every_palette_block_has_an_atlas_tile() {
        assert!((LAMP.0 as usize) < N_TILES);
        let img = build_atlas();
        assert_eq!(img.width() as usize, N_COLS * atlas_tile_px());
        assert!(
            img.width() as usize <= MAX_ATLAS_WIDTH,
            "the LOD atlas is {} texels wide — past the guaranteed max_texture_dimension_2d",
            img.width()
        );
        for id in BlockId::ALL_PLACEABLE.iter().copied().chain([LAMP]) {
            assert_ne!(
                tile_base(id),
                [255, 0, 255],
                "{} falls through to error magenta",
                id.name()
            );
        }
    }

    // ---- manifest-driven tile size ----

    /// A 4× point upscale must duplicate texels, not invent them: this is the
    /// one operation standing between a 16 px procedural tile and its column in
    /// a 64 px atlas, and anything that blends there would smear a block's edge
    /// into its neighbour at the LOD ring.
    #[test]
    fn nearest_upscale_duplicates_texels_and_round_trips() {
        // A 2×2 tile with four distinguishable texels.
        let src: Vec<u8> = vec![
            10, 11, 12, 255, // (0,0)
            20, 21, 22, 255, // (1,0)
            30, 31, 32, 255, // (0,1)
            40, 41, 42, 255, // (1,1)
        ];
        let up = scale_tile_nearest(&src, 2, 8);
        assert_eq!(up.len(), 8 * 8 * 4);
        for y in 0..8usize {
            for x in 0..8usize {
                let want = ((y / 4) * 2 + (x / 4)) * 4;
                let got = (y * 8 + x) * 4;
                assert_eq!(
                    &up[got..got + 4],
                    &src[want..want + 4],
                    "texel ({x},{y}) is not its source texel"
                );
            }
        }
        // Same size in, same bytes out — the identity path the near split path
        // relies on to never touch an artist's pixels.
        assert_eq!(scale_tile_nearest(&src, 2, 2), src);
    }

    /// Every consumer must agree with [`face_texels`] about how big a tile is.
    /// The bug this replaces was exactly this disagreement: `face_height` read a
    /// 64² tile with a 16 stride, so a manifest at any other size had to be
    /// thrown away wholesale to keep the renderer honest.
    #[test]
    fn every_face_texel_buffer_matches_its_declared_size() {
        for id in 0..N_TILES as u8 {
            let id = BlockId(id);
            for face in Face::ALL {
                let px = face_px(id, face);
                assert_eq!(
                    face_texels(id, face).len(),
                    px * px * 4,
                    "{} {face:?}: texel buffer disagrees with face_px",
                    id.name()
                );
                let img = build_face_texture(id, face);
                assert_eq!(img.width() as usize, px);
                assert_eq!(img.height() as usize, px);
                for map in [
                    build_face_normal_map(id, face),
                    build_face_metallic_roughness(id, face),
                    build_face_occlusion(id, face),
                ]
                .into_iter()
                .flatten()
                {
                    assert_eq!(
                        (map.width() as usize, map.height() as usize),
                        (px, px),
                        "{} {face:?}: a derived map is not per-texel with its albedo",
                        id.name()
                    );
                }
            }
        }
    }
}
