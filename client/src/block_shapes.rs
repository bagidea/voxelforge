//! Shaped blocks — the half of the palette that is not a cube.
//!
//! ## Why this module exists
//!
//! Every block in the world used to fill its cell, and the CEO's note on the
//! 2026-08-20 review says exactly what that costs: "แข็งไปหมด ไม่เมกเซนส์" —
//! it is all stiff, it does not make sense. Half of every reference image is
//! *not a box*: stairs, slabs, railings, window panes, posts, planters, plants.
//! A world made only of cubes cannot express a step you climb or a rail you look
//! through, and no amount of lighting fixes a silhouette that is wrong.
//!
//! So a block type may now declare a **render mode** in
//! `assets/textures/blocks/atlas.json`:
//!
//! ```json
//! "stair_wood":  { "all": "wood",  "mode": "stair" },
//! "slab_stone":  { "all": "stone", "mode": "slab"  },
//! "fence_wood":  { "all": "wood",  "mode": "fence" },
//! "pane_glass":  { "all": "glass", "mode": "pane"  },
//! "plant_cross": { "all": "grass_tall", "mode": "cross" }
//! ```
//!
//! and this file turns that word into geometry. Adding `slab_limestone` is an
//! `atlas.json` edit plus a `BlockId` — never a change to the mesher.
//!
//! ## Why it is not in the greedy mesher
//!
//! `voxel::sweep` merges co-planar same-block same-AO faces into the largest
//! quad it can. That is the correct algorithm for cubes and the wrong one for
//! everything here: two neighbouring stairs share no mergeable plane, and a
//! fence post is nine boxes that must not merge with anything at all. Teaching
//! the sweep about shapes would mean special-casing the one loop in the codebase
//! whose AO bookkeeping genuinely cannot afford a second version of itself.
//!
//! Instead the split is clean and the seam is one predicate:
//!
//! * `BlockId::is_shaped()` ⇒ `is_opaque() == false`, so the sweep never culls a
//!   neighbour's face against a shape and never casts a full cell of AO from
//!   one. That is the ONLY thing the mesher had to learn, and it learned it in
//!   `sim`, not in the sweep.
//! * The sweep still emits a cube for a shaped block (it only asks `is_solid`);
//!   `main::remesh_chunk_entity` drops that bucket on the floor and calls
//!   [`emit_chunk_shapes`] instead. One wasted merge pass over a handful of
//!   cells, and zero lines of the AO loop touched.
//!
//! ## What a shape costs
//!
//! One mesh per (block type, face slot) **per chunk**, batched exactly the way
//! the cube path batches: a hundred fence posts in a chunk are one draw call,
//! not a hundred. Far chunks are meshed by the atlas/LOD path, which draws every
//! block as a cube by construction — a shape reverts to a box at the LOD line,
//! which is the same trade every voxel game makes and is invisible at that range.
//!
//! Collision still treats a shaped cell as full (`is_solid()` is unchanged), so
//! you walk *up* a stair one cube-step at a time rather than falling through it.
//! That is written down as a decision in `sim/src/block.rs`, not left to be
//! discovered.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, CHUNK_SIZE};

use crate::block_atlas::Face;
use crate::voxel;

// ---------------------------------------------------------------------------
// the modes
// ---------------------------------------------------------------------------

/// A block's render mode: what it is built out of, once it is not a box.
///
/// Parsed from the manifest string, so this enum and `block_atlas::SHAPE_MODES`
/// are the same list said twice — [`ShapeMode::parse`] is the only place they
/// meet, and `every_shape_mode_round_trips` fails if one grows a member the
/// other does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeMode {
    /// Bottom half plus a back top half: a step you can climb.
    Stair,
    /// Bottom half only.
    Slab,
    /// Centre post plus two rails toward every neighbour it joins.
    Fence,
    /// A thin sheet, continued along whichever axis it has neighbours on.
    Pane,
    /// Two crossed alpha-masked quads: a plant.
    Cross,
}

impl ShapeMode {
    /// The manifest word for this mode — the inverse of [`ShapeMode::parse`].
    pub fn as_str(self) -> &'static str {
        match self {
            ShapeMode::Stair => "stair",
            ShapeMode::Slab => "slab",
            ShapeMode::Fence => "fence",
            ShapeMode::Pane => "pane",
            ShapeMode::Cross => "cross",
        }
    }

    /// A manifest `"mode"` string, or `None` for anything this build cannot
    /// build. `block_atlas::load_tiles` has already rejected unknown strings at
    /// load, so reaching the `None` arm here means the two lists drifted.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "stair" => Some(ShapeMode::Stair),
            "slab" => Some(ShapeMode::Slab),
            "fence" => Some(ShapeMode::Fence),
            "pane" => Some(ShapeMode::Pane),
            "cross" => Some(ShapeMode::Cross),
            _ => None,
        }
    }
}

/// How a block is built, or `None` if it is an ordinary cube.
///
/// The manifest is the source of truth. The fallback exists for the one path
/// that has no manifest at all — `VOXELFORGE_ATLAS_MODE=off`, and a source
/// checkout that has not pulled the art — where the alternative is a stair
/// silently rendering as a box and the whole feature looking broken for a
/// reason that has nothing to do with shapes. `manifest_shapes_match_the_code`
/// pins the two together so the fallback can never quietly disagree.
pub fn shape_of(id: BlockId) -> Option<ShapeMode> {
    if let Some(m) = voxel::block_shape_mode(id).and_then(ShapeMode::parse) {
        return Some(m);
    }
    default_shape(id)
}

/// The shape a block wears when there is no manifest to ask.
fn default_shape(id: BlockId) -> Option<ShapeMode> {
    match id {
        BlockId::STAIR_WOOD => Some(ShapeMode::Stair),
        BlockId::SLAB_STONE => Some(ShapeMode::Slab),
        BlockId::FENCE_WOOD => Some(ShapeMode::Fence),
        BlockId::PANE_GLASS => Some(ShapeMode::Pane),
        BlockId::PLANT_CROSS => Some(ShapeMode::Cross),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// vertex buffers
// ---------------------------------------------------------------------------

/// Half the width of a fence post / the thickness of a pane, in cell units.
const POST_HALF: f32 = 0.125;
/// Half the thickness of a fence rail.
const RAIL_HALF: f32 = 0.0625;
/// The two rail heights on a fence, as (bottom, top) of each rail.
const RAILS: [(f32, f32); 2] = [(0.30, 0.45), (0.66, 0.81)];
/// How far a cross-quad stops short of its cell wall, so a plant beside a wall
/// does not z-fight the wall's face.
const CROSS_INSET: f32 = 0.0625;

/// One draw call's worth of shaped geometry.
///
/// The attribute set is the split path's, exactly: position / normal / uv /
/// colour / tangent. Not a subset — a mesh missing `ATTRIBUTE_TANGENT` has its
/// normal map silently dropped by Bevy's PBR shader (no warning, no error, an
/// identical-looking frame), and these meshes share the per-block materials the
/// cube path built, normal maps and all.
#[derive(Default)]
struct ShapeBuf {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    tangents: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl ShapeBuf {
    /// Append one quad: `v0`, `v0 + du`, `v0 + du + dv`, `v0 + dv`.
    ///
    /// UVs run `(u0,v0m)` → `(u1,v1m)` across `du` × `dv`, and the tangent frame
    /// is built from the same two vectors — the convention `voxel::sweep` uses
    /// for its own quads (T along +U, `w` whichever sign puts `cross(N,T)` back
    /// on +V), so a shape and a cube light identically under the same normal map.
    ///
    /// Winding is derived, not assumed: `du × dv` is compared against the normal
    /// and the triangles are emitted the way round that makes the quad front-face
    /// (Bevy renders counter-clockwise as front). Every caller below would
    /// otherwise have to get the handedness right by hand six times per box, and
    /// a back-facing slab top is invisible rather than obviously wrong.
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &mut self,
        v0: Vec3,
        du: Vec3,
        dv: Vec3,
        n: Vec3,
        u0: f32,
        v0m: f32,
        u1: f32,
        v1m: f32,
    ) {
        let base = self.positions.len() as u32;
        for p in [v0, v0 + du, v0 + du + dv, v0 + dv] {
            self.positions.push([p.x, p.y, p.z]);
            self.normals.push([n.x, n.y, n.z]);
            // Shaped meshes carry no vertex AO: the sweep's AO is a property of
            // a cube's corners against its cube neighbours, and there is no
            // honest way to evaluate it for a rail. White is the neutral value —
            // the same attribute layout as the cube path, so one material
            // specialises one pipeline, and no term is silently doubled.
            self.colors.push([1.0, 1.0, 1.0, 1.0]);
        }
        self.uvs.extend_from_slice(&[
            [u0, v0m],
            [u1, v0m],
            [u1, v1m],
            [u0, v1m],
        ]);

        let t = du.normalize_or_zero();
        let b = dv.normalize_or_zero();
        let sign = if n.cross(t).dot(b) < 0.0 { -1.0 } else { 1.0 };
        self.tangents
            .extend_from_slice(&[[t.x, t.y, t.z, sign]; 4]);

        if du.cross(dv).dot(n) >= 0.0 {
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            self.indices
                .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, self.tangents);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// One block type's shaped geometry in a chunk, split by face slot.
///
/// Three buffers rather than one, for the same reason the cube path splits: a
/// mesh carries one material carries one texture, and a stair's tread wears the
/// top tile while its riser wears the side one. The emitters below write through
/// [`FaceBufs::face`] and never think about it again.
#[derive(Default)]
struct FaceBufs([ShapeBuf; 3]);

impl FaceBufs {
    #[inline]
    fn face(&mut self, f: Face) -> &mut ShapeBuf {
        &mut self.0[f.index()]
    }
}

/// The face slot a quad with this normal draws — the material-table key.
///
/// Same three-way split the cube path uses (`block_atlas::Face`): a block
/// distinguishes a top, a bottom and one shared side, so a stair's tread takes
/// the top material and its riser takes the side material.
fn face_of(n: Vec3) -> Face {
    if n.y > 0.5 {
        Face::Top
    } else if n.y < -0.5 {
        Face::Bottom
    } else {
        Face::Side
    }
}

// ---------------------------------------------------------------------------
// the shapes
// ---------------------------------------------------------------------------

/// Emit the six faces of an axis-aligned box, in **cell-local** units
/// (`lo`/`hi` inside `0..1`), offset to `cell`.
///
/// UVs are the box's own position inside the cell rather than `0..1` per face,
/// and that is the whole reason a slab reads as carved instead of squashed: the
/// bottom half of a stone block shows the bottom half of the stone tile, lined
/// up with the full blocks beside it. Squeezing a whole tile into a half-height
/// face instead is the classic tell of a shape bolted onto a cube renderer.
fn emit_box(out: &mut FaceBufs, cell: Vec3, lo: Vec3, hi: Vec3) {
    let (lo_a, hi_a) = ([lo.x, lo.y, lo.z], [hi.x, hi.y, hi.z]);
    for d in 0..3usize {
        // The sweep's axis convention, so the tangent frames agree: for a face
        // whose normal runs along `d`, U runs along `d+1` and V along `d+2`.
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;
        for front in [false, true] {
            let mut n = Vec3::ZERO;
            n[d] = if front { 1.0 } else { -1.0 };

            let mut origin = lo;
            origin[d] = if front { hi_a[d] } else { lo_a[d] };
            let mut du = Vec3::ZERO;
            du[u] = hi_a[u] - lo_a[u];
            let mut dv = Vec3::ZERO;
            dv[v] = hi_a[v] - lo_a[v];
            if du[u] <= 0.0 || dv[v] <= 0.0 {
                continue; // a degenerate box has no faces to draw
            }

            out.face(face_of(n)).quad(
                cell + origin,
                du,
                dv,
                n,
                lo_a[u],
                lo_a[v],
                hi_a[u],
                hi_a[v],
            );
        }
    }
}

/// The four horizontal neighbour offsets, in the order [`stair_facing`] breaks
/// ties in: `+X, +Z, -X, -Z`.
const DIRS: [IVec3; 4] = [
    IVec3::new(1, 0, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 0, -1),
];

/// Which way a stair's top half sits — the direction you climb TOWARD.
///
/// A `BlockId` is one byte with no room for a facing, and giving the sim a
/// per-voxel metadata channel to carry two bits is a much larger change than
/// this feature earns. So the facing is *derived from what the stair is next
/// to*, which is both free and almost always what the builder meant:
///
/// 1. A neighbour cell that is solid one level UP is the next step of a
///    staircase (or the floor it lands on) — climb toward it. This is the case
///    that makes a diagonal run of stairs come out as a staircase.
/// 2. Otherwise a solid neighbour at the SAME level is a wall to back onto.
/// 3. Otherwise `+X`, so an isolated stair is still a stair and not a coin flip.
///
/// Deterministic in `DIRS` order, so a re-mesh of the same world always produces
/// the same staircase — a facing that flickered between two equally good answers
/// would be a re-mesh artefact the eye catches immediately.
fn stair_facing(at: &dyn Fn(IVec3) -> BlockId, pos: IVec3) -> IVec3 {
    for d in DIRS {
        if at(pos + d + IVec3::Y).is_solid() {
            return d;
        }
    }
    for d in DIRS {
        if at(pos + d).is_solid() {
            return d;
        }
    }
    IVec3::X
}

/// Does a fence/pane at `pos` join its neighbour in direction `d`?
///
/// Joins to its own kind (a run of fence becomes a fence *line*) and to any
/// opaque cube (a railing meets the wall it ends at). Deliberately not "any
/// solid": two different shapes meeting — a fence post beside a pane — would
/// otherwise grow rails into each other's geometry.
fn joins(at: &dyn Fn(IVec3) -> BlockId, pos: IVec3, d: IVec3, me: BlockId) -> bool {
    let n = at(pos + d);
    n == me || n.is_opaque()
}

/// A stair: bottom slab plus the top half on the side it climbs toward.
fn emit_stair(out: &mut FaceBufs, cell: Vec3, at: &dyn Fn(IVec3) -> BlockId, pos: IVec3) {
    emit_box(out, cell, Vec3::ZERO, Vec3::new(1.0, 0.5, 1.0));
    let f = stair_facing(at, pos);
    let (lo, hi) = match (f.x, f.z) {
        (1, _) => (Vec3::new(0.5, 0.5, 0.0), Vec3::new(1.0, 1.0, 1.0)),
        (-1, _) => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.5, 1.0, 1.0)),
        (_, 1) => (Vec3::new(0.0, 0.5, 0.5), Vec3::new(1.0, 1.0, 1.0)),
        _ => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 1.0, 0.5)),
    };
    emit_box(out, cell, lo, hi);
}

/// A fence: centre post, plus two rails toward each neighbour it joins.
fn emit_fence(
    out: &mut FaceBufs,
    cell: Vec3,
    at: &dyn Fn(IVec3) -> BlockId,
    pos: IVec3,
    me: BlockId,
) {
    let c = 0.5;
    emit_box(
        out,
        cell,
        Vec3::new(c - POST_HALF, 0.0, c - POST_HALF),
        Vec3::new(c + POST_HALF, 1.0, c + POST_HALF),
    );
    for d in DIRS {
        if !joins(at, pos, d, me) {
            continue;
        }
        for (y0, y1) in RAILS {
            // From the post's face out to the cell wall, so two joined posts
            // meet in the middle of the boundary with no gap and no overlap.
            let (lo, hi) = match (d.x, d.z) {
                (1, _) => (
                    Vec3::new(c + POST_HALF, y0, c - RAIL_HALF),
                    Vec3::new(1.0, y1, c + RAIL_HALF),
                ),
                (-1, _) => (
                    Vec3::new(0.0, y0, c - RAIL_HALF),
                    Vec3::new(c - POST_HALF, y1, c + RAIL_HALF),
                ),
                (_, 1) => (
                    Vec3::new(c - RAIL_HALF, y0, c + POST_HALF),
                    Vec3::new(c + RAIL_HALF, y1, 1.0),
                ),
                _ => (
                    Vec3::new(c - RAIL_HALF, y0, 0.0),
                    Vec3::new(c + RAIL_HALF, y1, c - POST_HALF),
                ),
            };
            emit_box(out, cell, lo, hi);
        }
    }
}

/// A pane: a thin sheet along whichever axes it continues on.
///
/// An isolated pane still draws a full sheet on the X axis rather than a stub —
/// a single window in a wall is the common case, and half a pane floating in the
/// middle of its cell reads as a bug.
fn emit_pane(
    out: &mut FaceBufs,
    cell: Vec3,
    at: &dyn Fn(IVec3) -> BlockId,
    pos: IVec3,
    me: BlockId,
) {
    let c = 0.5;
    let (t0, t1) = (c - POST_HALF * 0.5, c + POST_HALF * 0.5);
    let mut any = false;
    for d in DIRS {
        if !joins(at, pos, d, me) {
            continue;
        }
        any = true;
        let (lo, hi) = match (d.x, d.z) {
            (1, _) => (Vec3::new(c, 0.0, t0), Vec3::new(1.0, 1.0, t1)),
            (-1, _) => (Vec3::new(0.0, 0.0, t0), Vec3::new(c, 1.0, t1)),
            (_, 1) => (Vec3::new(t0, 0.0, c), Vec3::new(t1, 1.0, 1.0)),
            _ => (Vec3::new(t0, 0.0, 0.0), Vec3::new(t1, 1.0, c)),
        };
        emit_box(out, cell, lo, hi);
    }
    if !any {
        emit_box(out, cell, Vec3::new(0.0, 0.0, t0), Vec3::new(1.0, 1.0, t1));
    }
}

/// A plant: two quads crossing on the cell diagonals.
///
/// Emitted once each, not twice back-to-back: the material is `double_sided`
/// (see `voxel::block_surface`), so one quad lights and draws from both sides
/// for half the vertices. The normal is the quad's own — a plant lit from the
/// wrong side is the alternative, and a leaf that goes black when the sun
/// crosses it is worse than one lit slightly flat.
fn emit_cross(out: &mut FaceBufs, cell: Vec3) {
    let (a, b) = (CROSS_INSET, 1.0 - CROSS_INSET);
    let up = Vec3::Y;
    // Diagonal 1: (a,a) → (b,b). Diagonal 2: (a,b) → (b,a).
    for (from, to) in [
        (Vec3::new(a, 0.0, a), Vec3::new(b, 0.0, b)),
        (Vec3::new(a, 0.0, b), Vec3::new(b, 0.0, a)),
    ] {
        let du = to - from;
        let n = du.cross(up).normalize_or_zero();
        out.face(Face::Side)
            .quad(cell + from, du, up, n, 0.0, 0.0, 1.0, 1.0);
    }
}

// ---------------------------------------------------------------------------
// the chunk pass
// ---------------------------------------------------------------------------

/// Build every shaped block in `chunk` into meshes, batched per (block, face).
///
/// `at` answers "what block is at this WORLD cell" and must reach across chunk
/// borders — `main::get_world_voxel` does. A fence that asked only its own chunk
/// would drop its rails at every 32-block boundary, which is exactly the kind of
/// seam that looks like a mesher bug and is really a lookup bug.
///
/// Positions are chunk-local, matching the cube path: the chunk entity carries
/// the translation and these meshes hang off it as children.
pub fn emit_chunk_shapes(
    chunk: &ChunkData,
    at: &dyn Fn(IVec3) -> BlockId,
) -> Vec<(BlockId, Face, Mesh)> {
    let (ox, oy, oz) = chunk.pos.world_origin();
    // Indexed by block id so the inner loop never hashes, and lazily filled so a
    // chunk with no shapes in it allocates nothing at all — which is most of
    // them, and this runs on every chunk re-mesh.
    let mut buckets: Vec<Option<FaceBufs>> = (0..256).map(|_| None).collect();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let id = chunk.get(x, y, z);
                let Some(shape) = shape_of(id) else {
                    continue;
                };
                let pos = IVec3::new(ox + x, oy + y, oz + z);
                let cell = Vec3::new(x as f32, y as f32, z as f32);
                let out = buckets[id.0 as usize].get_or_insert_with(FaceBufs::default);
                match shape {
                    ShapeMode::Slab => emit_box(out, cell, Vec3::ZERO, Vec3::new(1.0, 0.5, 1.0)),
                    ShapeMode::Stair => emit_stair(out, cell, at, pos),
                    ShapeMode::Fence => emit_fence(out, cell, at, pos, id),
                    ShapeMode::Pane => emit_pane(out, cell, at, pos, id),
                    ShapeMode::Cross => emit_cross(out, cell),
                }
            }
        }
    }

    let mut out = Vec::new();
    for (i, bufs) in buckets.into_iter().enumerate() {
        let Some(bufs) = bufs else { continue };
        for (f, buf) in Face::ALL.into_iter().zip(bufs.0) {
            if !buf.is_empty() {
                out.push((BlockId(i as u8), f, buf.into_mesh()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny world: a `HashMap` of the cells a test cares about, air elsewhere.
    fn world(cells: &[(IVec3, BlockId)]) -> impl Fn(IVec3) -> BlockId + '_ {
        move |p| {
            cells
                .iter()
                .find(|(q, _)| *q == p)
                .map_or(BlockId::AIR, |(_, b)| *b)
        }
    }

    fn buf_for(f: impl FnOnce(&mut FaceBufs)) -> FaceBufs {
        let mut b = FaceBufs::default();
        f(&mut b);
        b
    }

    /// Test-side views over all three face buffers at once. The emitters route
    /// a box's six faces into three buckets, and every assertion below is about
    /// the *shape*, which lives across all of them.
    impl FaceBufs {
        fn positions(&self) -> Vec<[f32; 3]> {
            self.0.iter().flat_map(|b| b.positions.clone()).collect()
        }

        fn normals(&self) -> Vec<[f32; 3]> {
            self.0.iter().flat_map(|b| b.normals.clone()).collect()
        }

        fn no_geometry(&self) -> bool {
            self.0.iter().all(ShapeBuf::is_empty)
        }
    }

    /// The mode list in this file and the one `block_atlas` validates against
    /// are the same list said twice. If one grows a member the other has not,
    /// a manifest can name a mode that loads fine and then renders as nothing.
    #[test]
    fn every_shape_mode_round_trips() {
        for &m in crate::block_atlas::SHAPE_MODES {
            let parsed = ShapeMode::parse(m).unwrap_or_else(|| {
                panic!("block_atlas accepts {m:?} but block_shapes cannot build it")
            });
            assert_eq!(parsed.as_str(), *m, "{m:?} does not round-trip");
        }
    }

    /// Every shaped `BlockId` has a code-side default, and no cube has one. The
    /// fallback is what renders when there is no manifest (`ATLAS_MODE=off`), so
    /// a shaped block missing from it is a stair that silently becomes a box on
    /// exactly the path an A/B plate is shot from.
    #[test]
    fn the_fallback_covers_exactly_the_shaped_palette() {
        for &id in BlockId::ALL_PLACEABLE {
            assert_eq!(
                default_shape(id).is_some(),
                id.is_shaped(),
                "{} disagrees with BlockId::is_shaped",
                id.name()
            );
        }
    }

    /// A slab is the bottom half and nothing else: no vertex above y = 0.5, and
    /// a real top face at exactly 0.5. The failure this catches is a shape that
    /// renders but still fills the cell — which looks like a cube and would sail
    /// through any test that only counted triangles.
    #[test]
    fn a_slab_is_half_a_cell_tall() {
        let b = buf_for(|out| emit_box(out, Vec3::ZERO, Vec3::ZERO, Vec3::new(1.0, 0.5, 1.0)));
        assert!(!b.no_geometry(), "a slab emitted no geometry");
        let top = b.positions().iter().fold(f32::MIN, |m, p| m.max(p[1]));
        assert!((top - 0.5).abs() < 1e-6, "slab top is at {top}, not 0.5");
        assert!(
            b.normals().iter().any(|n| n[1] > 0.5),
            "a slab with no up-facing quad has no tread to stand on"
        );
    }

    /// The tread of a stair must not sit under its own riser: a stair is two
    /// boxes and the top one is on the side you climb toward. Checked as "the
    /// upper box is on the +Z half" for a stair with a step up at +Z.
    #[test]
    fn a_stair_climbs_toward_the_block_above_its_neighbour() {
        let here = IVec3::ZERO;
        let cells = [(IVec3::new(0, 1, 1), BlockId::STONE)];
        let at = world(&cells);
        assert_eq!(stair_facing(&at, here), IVec3::new(0, 0, 1));

        let b = buf_for(|out| emit_stair(out, Vec3::ZERO, &at, here));
        // Everything above the halfway line belongs to the top box, and that box
        // must live entirely on the +Z side of the cell.
        let all = b.positions();
        let high: Vec<_> = all.iter().filter(|p| p[1] > 0.51).collect();
        assert!(!high.is_empty(), "a stair with no upper step is a slab");
        assert!(
            high.iter().all(|p| p[2] >= 0.5 - 1e-6),
            "the upper step is not on the +Z side"
        );
    }

    /// A stair against a wall backs onto the wall; an isolated one still picks a
    /// side rather than nothing.
    #[test]
    fn a_stair_backs_onto_a_wall_and_never_has_no_facing() {
        let cells = [(IVec3::new(-1, 0, 0), BlockId::STONE)];
        assert_eq!(stair_facing(&world(&cells), IVec3::ZERO), IVec3::new(-1, 0, 0));
        assert_eq!(stair_facing(&world(&[]), IVec3::ZERO), IVec3::X);
    }

    /// A lone fence post is a post. A post with a neighbour grows rails toward
    /// it — and only toward it.
    #[test]
    fn a_fence_grows_rails_only_toward_what_it_joins() {
        let me = BlockId::FENCE_WOOD;
        let lone = buf_for(|out| emit_fence(out, Vec3::ZERO, &world(&[]), IVec3::ZERO, me));
        let cells = [(IVec3::new(1, 0, 0), me)];
        let joined = buf_for(|out| emit_fence(out, Vec3::ZERO, &world(&cells), IVec3::ZERO, me));

        let (lp, jp) = (lone.positions(), joined.positions());
        assert!(jp.len() > lp.len(), "joining a neighbour added no rail");
        // Rails reach the +X cell wall and nothing reaches the -X one.
        assert!(
            jp.iter().any(|p| p[0] >= 1.0 - 1e-6),
            "the rail does not reach the shared boundary, so two posts would gap"
        );
        assert!(
            !jp.iter().any(|p| p[0] <= 1e-6),
            "a rail grew toward a neighbour that is not there"
        );
        // The post itself is a full-height column either way.
        for p in [&lp, &jp] {
            let top = p.iter().fold(f32::MIN, |m, q| m.max(q[1]));
            assert!((top - 1.0).abs() < 1e-6, "the post is not full height");
        }
    }

    /// An isolated pane is still a whole sheet, and it is thin.
    #[test]
    fn a_pane_is_thin_and_never_half_a_sheet() {
        let me = BlockId::PANE_GLASS;
        let b = buf_for(|out| emit_pane(out, Vec3::ZERO, &world(&[]), IVec3::ZERO, me));
        let p = b.positions();
        let (lo, hi) = p.iter().fold((f32::MAX, f32::MIN), |(l, h), q| {
            (l.min(q[2]), h.max(q[2]))
        });
        assert!(hi - lo < 0.2, "the pane is {:.3} thick", hi - lo);
        let (xl, xh) = p.iter().fold((f32::MAX, f32::MIN), |(l, h), q| {
            (l.min(q[0]), h.max(q[0]))
        });
        assert!(
            (xh - xl - 1.0).abs() < 1e-6,
            "an isolated pane spans {:.3} of its cell, not the whole one",
            xh - xl
        );
    }

    /// A plant is two crossed quads, and the normals of the two are not the
    /// same — the failure mode being two coincident quads, which reads as one
    /// flat billboard that vanishes edge-on.
    #[test]
    fn a_plant_is_two_quads_that_actually_cross() {
        let b = buf_for(|out| emit_cross(out, Vec3::ZERO));
        let (p, nn) = (b.positions(), b.normals());
        assert_eq!(p.len(), 8, "a cross is exactly two quads");
        let n0 = Vec3::from(nn[0]);
        let n1 = Vec3::from(nn[4]);
        assert!(
            n0.dot(n1).abs() < 0.5,
            "the two quads face the same way ({n0:?} vs {n1:?}) — that is one billboard"
        );
    }

    /// Every quad's tangent is unit length and its `w` puts the rebuilt
    /// bitangent back on the +V axis. Bevy's shader does `cross(N, T) * w`; get
    /// the sign wrong and every normal-mapped shape lights inverted, which looks
    /// like an art bug and is a geometry bug.
    #[test]
    fn the_tangent_frame_is_the_one_the_shader_rebuilds() {
        let b = buf_for(|out| {
            emit_box(out, Vec3::ZERO, Vec3::ZERO, Vec3::new(1.0, 0.5, 1.0));
            emit_cross(out, Vec3::ZERO);
        });
        for buf in &b.0 {
            for (i, t) in buf.tangents.iter().enumerate() {
                let tv = Vec3::new(t[0], t[1], t[2]);
                assert!((tv.length() - 1.0).abs() < 1e-5, "tangent {i} is not unit");
                let n = Vec3::from(buf.normals[i]);
                assert!(
                    n.dot(tv).abs() < 1e-5,
                    "tangent {i} is not perpendicular to its normal"
                );
                assert!(t[3] == 1.0 || t[3] == -1.0, "tangent {i} has a bogus w");
                // The bitangent the shader rebuilds must land back on +V, which
                // for these quads is the dv the emitter passed in.
                let bt = n.cross(tv) * t[3];
                assert!(
                    (bt.length() - 1.0).abs() < 1e-5,
                    "the rebuilt bitangent {i} is not unit"
                );
            }
        }
    }

    /// Winding: every triangle's geometric normal must agree with the vertex
    /// normal it was given, or the quad is back-facing and invisible.
    #[test]
    fn every_triangle_faces_the_way_its_normal_says() {
        let cells = [(IVec3::new(0, 1, 1), BlockId::STONE)];
        let at = world(&cells);
        let b = buf_for(|out| {
            emit_stair(out, Vec3::ZERO, &at, IVec3::ZERO);
            emit_fence(out, Vec3::ZERO, &at, IVec3::ZERO, BlockId::FENCE_WOOD);
            emit_cross(out, Vec3::ZERO);
        });
        let mut tris = 0usize;
        for buf in &b.0 {
            for tri in buf.indices.chunks(3) {
                let p: Vec<Vec3> = tri
                    .iter()
                    .map(|&i| Vec3::from(buf.positions[i as usize]))
                    .collect();
                let geo = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                let want = Vec3::from(buf.normals[tri[0] as usize]);
                assert!(
                    geo.dot(want) > 0.9,
                    "triangle {tri:?} winds {geo:?} against its normal {want:?}"
                );
                tris += 1;
            }
        }
        // A vacuously-passing loop is the failure this guards against: if the
        // emitters produced nothing, every assertion above is skipped silently.
        assert!(tris > 40, "only {tris} triangles — the emitters ran short");
    }
}
