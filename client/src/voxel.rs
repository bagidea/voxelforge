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

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, CHUNK_SIZE};

/// A client-side decorative block: the wall lantern.
///
/// The sim palette (`sim/src/block.rs`) stops at id 15 and is kevin's lane, but
/// its own doc says the palette is open-ended and "the client adds its colour to
/// the atlas". Nothing in the reference art reads without a light source that is
/// actually emitting — a lantern painted bright but not *emissive* is invisible
/// to bloom — so the id is defined here rather than not at all.
///
/// KNOWN LIMIT while it lives here: `BlockId(16).name()` is `"unknown"`, so a
/// lamp cannot round-trip through a JSON map file. Promoting this to a real
/// `BlockId::LAMP` const in sim is a one-line change and is in the report.
pub const LAMP: BlockId = BlockId(16);

/// Number of tiles in the (horizontal) atlas — the 16 sim blocks plus [`LAMP`].
const N_TILES: usize = 17;
const TILE_PX: usize = 16;

/// Vertex-AO shade per occlusion level (3 = fully open, 0 = fully boxed in).
///
/// This is the single knob that makes a voxel room read as a room instead of as
/// a flat-lit box: the reference kitchen's depth is almost entirely the soft
/// darkening where two surfaces meet. Kept gentle at the top end on purpose —
/// `look.rs` also runs SSAO on the playable game, and two occlusion terms
/// stacking at full strength turns every inside corner into a black smear.
const AO_SHADE: [f32; 4] = [0.45, 0.66, 0.84, 1.0];

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

/// The StandardMaterial for one block type, given its (repeating) tile texture.
///
/// This is the per-type material the single shared atlas material can't be. Pair
/// it with [`greedy_mesh_chunk_split`], which produces one mesh per type.
pub fn block_material(id: BlockId, texture: Handle<Image>) -> StandardMaterial {
    let s = block_surface(id);
    StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, s.alpha),
        base_color_texture: Some(texture),
        perceptual_roughness: s.perceptual_roughness,
        metallic: s.metallic,
        reflectance: s.reflectance,
        emissive: s.emissive,
        alpha_mode: if s.alpha_blend {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        // Chunk meshes are closed shells; drawing the inside of them is wasted
        // fill and, for the blended panes, a double-blend.
        double_sided: false,
        ..default()
    }
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

/// Every block type's material, indexed by `BlockId.0` — the split path's table.
///
/// Built once at startup rather than on demand, because the sites that re-mesh a
/// chunk (an editor click, a map load, the streaming tick) hold `&mut World` and
/// `Assets<Mesh>` but have no business also borrowing `Assets<Image>` and
/// `Assets<StandardMaterial>`. Seventeen 16×16 tiles is a few KB of texture; the
/// alternative is threading two more asset borrows through every edit path.
///
/// Index 0 is AIR — never meshed, so its entry is only there to keep the table
/// indexable by raw block id.
pub fn build_block_materials(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    (0..N_TILES as u8)
        .map(|id| {
            let id = BlockId(id);
            let tile = images.add(build_block_texture(id));
            materials.add(block_material(id, tile))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// meshing
// ---------------------------------------------------------------------------

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
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Face {
    id: i32,
    ao: [u8; 4],
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

/// Vertex buffers for one draw call.
#[derive(Default)]
struct Buffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    quads: usize,
}

impl Buffers {
    fn into_mesh(self) -> Mesh {
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
/// each mesh with [`block_material`] over [`build_block_texture`].
pub fn greedy_mesh_chunk_split(chunk: &ChunkData) -> Vec<(BlockId, Mesh, usize)> {
    sweep(chunk, true)
        .into_iter()
        .map(|(id, buf)| {
            let quads = buf.quads;
            (BlockId(id), buf.into_mesh(), quads)
        })
        .collect()
}

/// The shared sweep behind both public meshers.
///
/// `split == false` funnels every face into one bucket with atlas UVs;
/// `split == true` gives each block type its own bucket with per-block repeating
/// UVs. One implementation on purpose — two copies of a greedy mesher drift, and
/// the AO bookkeeping is the part you cannot afford to have two versions of.
fn sweep(chunk: &ChunkData, split: bool) -> Vec<(u8, Buffers)> {
    let dims = [CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE];
    // Indexed by block id so the bucket lookup is O(1) inside the hot loop.
    let mut buckets: Vec<Option<Buffers>> = (0..256).map(|_| None).collect();
    if !split {
        buckets[0] = Some(Buffers::default());
    }

    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;
        let mut x = [0i32; 3];
        let mut q = [0i32; 3];
        q[d] = 1;

        let mut mask = vec![Face::default(); (dims[u] * dims[v]) as usize];

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
                    let sa = a.is_opaque();
                    let sb = b.is_opaque();

                    mask[n] = if sa == sb {
                        Face::default()
                    } else {
                        // Which side is air decides both the winding and which
                        // layer the occluders are sampled from: AO is cast by
                        // the blocks sitting in front of the face, never behind.
                        let (id, air_d) = if sa {
                            (a.0 as i32, x[d] + 1)
                        } else {
                            (-(b.0 as i32), x[d])
                        };
                        let solid = |du: i32, dv: i32| -> bool {
                            let mut p = [0i32; 3];
                            p[d] = air_d;
                            p[u] = i + du;
                            p[v] = j + dv;
                            chunk.get(p[0], p[1], p[2]).is_opaque()
                        };
                        // Corner order matches the emitted vertex order below:
                        // (0,0) (1,0) (1,1) (0,1) in (u,v).
                        let ao = [
                            ao_corner(solid(-1, 0), solid(0, -1), solid(-1, -1)),
                            ao_corner(solid(1, 0), solid(0, -1), solid(1, -1)),
                            ao_corner(solid(1, 0), solid(0, 1), solid(1, 1)),
                            ao_corner(solid(-1, 0), solid(0, 1), solid(-1, 1)),
                        ];
                        Face { id, ao }
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
                        let p = [x[0] as f32, x[1] as f32, x[2] as f32];

                        let front = c.id > 0;
                        let block = c.id.unsigned_abs() as u8;
                        let mut nrm = [0f32; 3];
                        nrm[d] = if front { 1.0 } else { -1.0 };

                        let key = if split { block } else { 0 };
                        let buf = buckets[key as usize].get_or_insert_with(Buffers::default);

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
                            // UVs in BLOCKS. With the tile texture sampled in
                            // `Repeat` (see `build_block_texture`) a 12×3 merged
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
                            // The half-texel inset is what keeps a merged quad
                            // from ever sampling the NEIGHBOURING tile at its
                            // edge; it is now applied on BOTH axes (v used to run
                            // a raw 0.0..1.0 and could pick up the wrap row).
                            let t = (block as usize).min(N_TILES - 1) as f32;
                            let inset_u = 0.5 / (N_TILES * TILE_PX) as f32;
                            let inset_v = 0.5 / TILE_PX as f32;
                            let u0 = t / N_TILES as f32 + inset_u;
                            let u1 = (t + 1.0) / N_TILES as f32 - inset_u;
                            let (v0t, v1t) = (inset_v, 1.0 - inset_v);
                            buf.uvs.extend_from_slice(&[
                                [u0, v0t],
                                [u1, v0t],
                                [u1, v1t],
                                [u0, v1t],
                            ]);
                        }

                        for a in c.ao {
                            let s = AO_SHADE[a as usize];
                            buf.colors.push([s, s, s, 1.0]);
                        }

                        // Pick the diagonal that does NOT run between the two
                        // corners with the widest occlusion gap. Splitting the
                        // other way makes a shaded corner leak a hard triangular
                        // seam across the face — the classic voxel-AO artefact.
                        let flip = c.ao[0] as u16 + c.ao[2] as u16 > c.ao[1] as u16 + c.ao[3] as u16;
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
                                mask[n + k as usize + (l * dims[u]) as usize] = Face::default();
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

    buckets
        .into_iter()
        .enumerate()
        .filter_map(|(id, b)| b.map(|b| (id as u8, b)))
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
    let px = TILE_PX as u32;
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

/// Base colour for a tile, including [`LAMP`] which the sim palette has no entry
/// for.
fn tile_base(id: BlockId) -> [u8; 3] {
    if id == LAMP {
        [255, 196, 118]
    } else {
        id.base_color()
    }
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

fn image_from_rgba(w: usize, h: usize, data: Vec<u8>, sampler: ImageSampler) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = sampler;
    image
}

/// Build the tile atlas procedurally (no shipped PNG => identical on every
/// machine, no asset-path headaches). Tile index == block ID.
///
/// Used by the single-material path ([`greedy_mesh_chunk`]). Clamped to the
/// edge, because that path's UVs address one tile inside a shared image and a
/// `Repeat` mode there would wrap into the neighbouring block's tile.
pub fn build_atlas() -> Image {
    let w = N_TILES * TILE_PX;
    let h = TILE_PX;
    let mut data = vec![0u8; w * h * 4];

    for ty in 0..h {
        for tx in 0..w {
            let id = BlockId((tx / TILE_PX) as u8);
            let lx = tx % TILE_PX;
            let base = tile_base(id);
            let shade = tile_shade(id, lx, ty);
            let px = (ty * w + tx) * 4;
            for c in 0..3 {
                data[px + c] = (base[c] as i32 + shade).clamp(0, 255) as u8;
            }
            data[px + 3] = 255;
        }
    }

    image_from_rgba(w, h, data, voxel_sampler(false))
}

/// Build the standalone 16×16 tile for one block type, sampled with `Repeat`.
///
/// Used by the split path, where UVs are measured in blocks — so this texture
/// tiles once per block no matter how many blocks a merged quad covers.
pub fn build_block_texture(id: BlockId) -> Image {
    let mut data = vec![0u8; TILE_PX * TILE_PX * 4];
    let base = tile_base(id);
    for ly in 0..TILE_PX {
        for lx in 0..TILE_PX {
            let shade = tile_shade(id, lx, ly);
            let px = (ly * TILE_PX + lx) * 4;
            for c in 0..3 {
                data[px + c] = (base[c] as i32 + shade).clamp(0, 255) as u8;
            }
            data[px + 3] = 255;
        }
    }
    image_from_rgba(TILE_PX, TILE_PX, data, voxel_sampler(true))
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
        let w = N_TILES * TILE_PX;
        let data = img.data.as_ref().expect("atlas has pixel data");
        let lum = |tx: usize, ty: usize| -> i32 {
            let p = (ty * w + tx) * 4;
            data[p] as i32 + data[p + 1] as i32 + data[p + 2] as i32
        };
        // Stone: a flat mineral whose pattern is pure low-amplitude dither, so
        // any edge-vs-centre gap would have to be a painted frame.
        let t = BlockId::STONE.0 as usize * TILE_PX;
        let centre = lum(t + 8, 8);
        for k in 0..TILE_PX {
            for (tx, ty) in [
                (t + k, 0),
                (t + k, TILE_PX - 1),
                (t, k),
                (t + TILE_PX - 1, k),
            ] {
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
        let w = N_TILES * TILE_PX;
        let data = img.data.as_ref().unwrap();
        let t = BlockId::LIMESTONE.0 as usize * TILE_PX;
        let lum = |tx: usize, ty: usize| -> i32 {
            let p = (ty * w + tx) * 4;
            data[p] as i32 + data[p + 1] as i32 + data[p + 2] as i32
        };
        let corner = lum(t, 0);
        let darkest_interior = (1..TILE_PX - 1)
            .flat_map(|y| (1..TILE_PX - 1).map(move |x| (x, y)))
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
        for img in [build_atlas(), build_block_texture(BlockId::WOOD)] {
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
        let ImageSampler::Descriptor(tile) = &build_block_texture(BlockId::STONE).sampler else {
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
        let tile_w = 1.0 / N_TILES as f32;
        let lo = BlockId::STONE.0 as f32 * tile_w;
        let hi = lo + tile_w;
        for [u, v] in uvs(&mesh) {
            assert!(u > lo && u < hi, "u {u} outside tile ({lo}..{hi})");
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
        // Every emitted shade has to come from the table, not from an average.
        for s in shades {
            assert!(
                AO_SHADE.iter().any(|a| (a - s).abs() < 1e-6),
                "shade {s} is not one of the four AO levels"
            );
        }
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
        let ids: Vec<u8> = parts.iter().map(|(b, _, _)| b.0).collect();
        assert_eq!(ids, vec![BlockId::STONE.0, BlockId::WOOD.0]);
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
        // Nothing in this palette is a metal.
        for id in BlockId::ALL_PLACEABLE.iter().copied().chain([LAMP]) {
            assert_eq!(block_surface(id).metallic, 0.0, "{} is not metal", id.name());
        }
    }

    #[test]
    fn block_material_carries_the_surface_and_the_texture() {
        let m = block_material(LAMP, Handle::default());
        assert_eq!(m.emissive, block_surface(LAMP).emissive);
        assert!(m.base_color_texture.is_some());
        let pane = block_material(BlockId::OBSIDIAN, Handle::default());
        assert!(matches!(pane.alpha_mode, AlphaMode::Blend));
        assert!(pane.base_color.alpha() < 1.0);
    }

    /// Every palette entry — including the client-side lamp — must have a tile
    /// in the atlas, or the shared-material path indexes past its own image.
    #[test]
    fn every_palette_block_has_an_atlas_tile() {
        assert!((LAMP.0 as usize) < N_TILES);
        let img = build_atlas();
        assert_eq!(img.width() as usize, N_TILES * TILE_PX);
        for id in BlockId::ALL_PLACEABLE.iter().copied().chain([LAMP]) {
            assert_ne!(
                tile_base(id),
                [255, 0, 255],
                "{} falls through to error magenta",
                id.name()
            );
        }
    }
}
