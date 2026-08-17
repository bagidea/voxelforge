//! Chunk streaming plugin — async loading/unloading around the player,
//! two LOD levels, and frustum culling so the world feels large and runs fast.
//!
//! ## Design
//!
//! * **Async terrain gen** — `ChunkData::generate` runs on background threads
//!   via `std::thread::spawn` + `mpsc`; the main thread only pays for meshing.
//! * **LOD 0** (near) — `greedy_mesh_chunk_split` at full 32³ resolution: one
//!   child mesh per block type, each with its own repeating tile material (see
//!   [`build_lod_children`]).
//! * **LOD 1** (far) — downsample 2×2×2 → 1 block, re-greedy-mesh at 16³,
//!   then scale vertices ×2.  ~1/8 the quads, visually coarser.
//! * **Frustum culling** — toggle `Visibility` per chunk each frame against
//!   the camera's `Frustum` component (Bevy core, zero extra cost).
//! * **Churn guard** — each frame queues at most N async tasks and spawns at
//!   most M mesh entities; a full 25×25 ring fills gradually without a hitch.
//!
//! ## Integration
//!
//! Add one line in `main()`:  `.add_plugins(streaming::StreamingPlugin)`
//! The plugin reads `crate::World`, `crate::FlyCam`, and spawns one parent
//! entity per chunk (`Transform` + `Visibility` + `StreamedChunk`) whose
//! children carry the `Mesh3d` + `MeshMaterial3d` — exactly the shape
//! `crate::spawn_chunk` builds.

use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Mutex};

use bevy::math::{Affine3A, Vec3A};
use bevy::prelude::*;
use bevy::camera::primitives::{Aabb, Frustum};

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, ChunkPos, CHUNK_SIZE};

use crate::voxel::greedy_mesh_chunk;

// ---------------------------------------------------------------------------
// plugin
// ---------------------------------------------------------------------------

pub struct StreamingPlugin;

impl Plugin for StreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreamingConfig>()
            .init_resource::<StreamState>()
            .add_systems(
                Update,
                (streaming_tick, frustum_cull).chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

/// Tunables for the streaming system — all env-driven so the perf cliff can
/// be isolated without a recompile.
#[derive(Resource)]
pub struct StreamingConfig {
    /// Half-side of the square loaded around the player (in chunks).
    /// Default 12 → 25×25 = 625 chunks max.
    pub view_distance: i32,
    /// Chunks beyond this distance from the player switch to LOD 1.
    pub lod_transition: i32,
    /// Max new async terrain-gen tasks spawned per frame.
    pub max_spawn_per_frame: usize,
    /// Max completed tasks whose mesh entity is spawned per frame.
    pub max_mesh_per_frame: usize,
    /// Extra ring of chunks kept loaded beyond `view_distance` before
    /// unloading, so moving back and forth across a boundary doesn't
    /// thrash unload/reload.
    pub unload_margin: i32,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            view_distance: 12,
            lod_transition: 6,
            max_spawn_per_frame: 4,
            max_mesh_per_frame: 4,
            unload_margin: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// async plumbing
// ---------------------------------------------------------------------------

/// Payload sent from a background thread back to the main thread.
struct ChunkReady {
    key: (i32, i32),
    data: ChunkData,
}

/// Marker component on every chunk entity the streaming plugin spawns.
#[derive(Component)]
pub struct StreamedChunk {
    #[allow(dead_code)]
    pub key: (i32, i32),
    /// 0 = full, 1 = downsampled.
    #[allow(dead_code)]
    pub lod: u8,
}

// ---------------------------------------------------------------------------
// state
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct StreamState {
    /// Chunks already spawned as entities (loaded set).
    loaded: HashSet<(i32, i32)>,
    /// Chunks whose terrain gen has been dispatched but hasn't landed yet.
    pending: HashSet<(i32, i32)>,
    /// Sender kept so background threads can push results.
    tx: mpsc::Sender<ChunkReady>,
    /// Receiver guarded by a mutex so the whole struct is `Sync`.
    rx: Mutex<mpsc::Receiver<ChunkReady>>,
    /// Player's chunk coordinate last frame — streaming only responds to moves.
    last_player_chunk: (i32, i32),
    /// Entities keyed by chunk coord (for unload / LOD switch).
    chunk_entities: HashMap<(i32, i32), (Entity, u8)>,
}

impl Default for StreamState {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            loaded: HashSet::new(),
            pending: HashSet::new(),
            tx,
            rx: Mutex::new(rx),
            last_player_chunk: (i32::MAX, i32::MAX),
            chunk_entities: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// LOD meshing
// ---------------------------------------------------------------------------

/// LOD 1: collapse each 2×2×2 region into its dominant block type, produce a
/// 16³ `ChunkData`, greedy-mesh it, then scale vertices ×2 so the mesh covers
/// the same world-space footprint as a full-res chunk.
fn lod1_mesh(chunk: &ChunkData) -> (Mesh, usize) {
    let factor: i32 = 2;
    let fs = CHUNK_SIZE / factor; // 16

    let mut lod_data = ChunkData::empty(ChunkPos::new(chunk.pos.x, chunk.pos.y, chunk.pos.z));

    for y in 0..fs {
        for z in 0..fs {
            for x in 0..fs {
                // One slot per block id, not per "the sixteen ids that existed
                // when this was written". The old `[0u8; 16]` + `idx < 16` gate
                // silently deleted every block from id 16 up out of the far
                // ring: the lantern first, and glass the moment it arrived. A
                // block that vanishes at the LOD line is a hole in the world
                // that only appears when you walk away from it.
                let mut counts = [0u8; 256];
                for dy in 0..factor {
                    for dz in 0..factor {
                        for dx in 0..factor {
                            let b = chunk.get(
                                x * factor + dx,
                                y * factor + dy,
                                z * factor + dz,
                            );
                            // is_solid: the LOD cares which block fills the
                            // cell, not whether you can see through it.
                            if b.is_solid() {
                                counts[b.0 as usize] += 1;
                            }
                        }
                    }
                }
                // Pick the most common solid block; leave air if all empty.
                // Slot 0 is air and is never counted, so it can never win.
                if let Some((idx, _)) = counts
                    .iter()
                    .enumerate()
                    .filter(|(_, &c)| c > 0)
                    .max_by_key(|(_, &c)| c)
                {
                    lod_data.set(x, y, z, BlockId(idx as u8));
                }
            }
        }
    }

    let (mut mesh, quads) = greedy_mesh_chunk(&lod_data);

    // Scale vertex positions so the 16³ mesh fills the 32³ footprint.
    // The `attribute_mut` returns `Option<&mut VertexAttributeValues>`;
    // we match the float32x3 variant.
    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(ref mut positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in positions.iter_mut() {
            p[0] *= factor as f32;
            p[1] *= factor as f32;
            p[2] *= factor as f32;
        }
    }

    (mesh, quads)
}

/// Fill a chunk entity with the drawable children for one LOD level, replacing
/// whatever it had. Returns the chunk's quad count.
///
/// **LOD 0 uses the split mesher** — one child per block type, each with its own
/// tile sampled `Repeat` — so near chunks get per-block texel density instead of
/// one atlas tile stretched across a merged quad.
///
/// **LOD 1 deliberately stays on the single atlas mesh.** A far chunk has already
/// had each 2×2×2 region collapsed to its dominant block, so there is no
/// per-block detail left for a repeating tile to resolve; and multiplying a far
/// chunk's draw calls by the number of block types present is exactly the cost
/// the LOD exists to avoid. The tile stretching that is visible up close is not
/// visible at `lod_transition` chunks away.
fn build_lod_children(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &crate::World,
    entity: Entity,
    chunk: &ChunkData,
    lod: u8,
) -> usize {
    if lod == 0 {
        return crate::remesh_chunk_entity(commands, meshes, world, entity, chunk);
    }
    let (mesh, quads) = lod1_mesh(chunk);
    commands.entity(entity).despawn_children();
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(world.material.clone()),
        ChildOf(entity),
    ));
    quads
}

// ---------------------------------------------------------------------------
// system: streaming tick
// ---------------------------------------------------------------------------

/// Runs every frame. Decides which chunks to load, processes completed async
/// tasks, and evicts chunks the player has left behind.
#[allow(clippy::too_many_arguments)]
fn streaming_tick(
    player_q: Query<&Transform, With<crate::FlyCam>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world_res: ResMut<crate::World>,
    config: Res<StreamingConfig>,
    mut state: ResMut<StreamState>,
) {
    // ── 1. seed loaded set from existing World.chunks (first run / map load) ──
    for (&key, slot) in world_res.chunks.iter() {
        if state.loaded.insert(key) {
            state.chunk_entities.insert(key, (slot.entity, 0u8));
        }
    }

    // ── 2. determine player chunk ──
    let Ok(ptf) = player_q.single() else {
        return;
    };
    let pc = (
        ptf.translation.x.div_euclid(CHUNK_SIZE as f32) as i32,
        ptf.translation.z.div_euclid(CHUNK_SIZE as f32) as i32,
    );

    // Only recompute the desired set when the player enters a new chunk.
    if pc == state.last_player_chunk {
        // Fast path — process completed tasks every frame.
        drain_completed(&mut commands, &mut meshes, &mut world_res, &config, &mut state);
        return;
    }
    state.last_player_chunk = pc;

    let r = config.view_distance;
    let lod_r = config.lod_transition;

    // ── 3. build desired set (view_distance ring) ──
    let mut desired: Vec<((i32, i32), u8)> =
        Vec::with_capacity(((r * 2 + 1) * (r * 2 + 1)) as usize);
    for dz in -r..=r {
        for dx in -r..=r {
            let key = (pc.0 + dx, pc.1 + dz);
            let dist = dx.abs().max(dz.abs());
            let lod = if dist > lod_r { 1u8 } else { 0u8 };
            desired.push((key, lod));
        }
    }

    // Sort by distance — chunks closest to the player load first.
    desired.sort_by_key(|((x, z), _)| (x - pc.0).abs().max((z - pc.1).abs()));

    // ── 4. unload chunks outside desired + margin ──
    let margin_r = r + config.unload_margin;
    let keep: HashSet<(i32, i32)> = desired.iter().map(|&(k, _)| k).collect();
    let to_unload: Vec<(i32, i32)> = state
        .loaded
        .iter()
        .filter(|k| !keep.contains(k) && ((k.0 - pc.0).abs() > margin_r || (k.1 - pc.1).abs() > margin_r))
        .copied()
        .collect();

    for key in &to_unload {
        unload_chunk(key, &mut commands, &mut world_res, &mut state);
    }

    // ── 5. queue loads for desired chunks not yet loaded or pending ──
    let mut spawned = 0usize;
    for &(key, lod) in &desired {
        if state.pending.contains(&key) {
            continue;
        }
        if state.loaded.contains(&key) {
            // Check if LOD needs to change.
            let current_lod = state
                .chunk_entities
                .get(&key)
                .map(|&(_, l)| l)
                .unwrap_or(0);
            if current_lod != lod {
                switch_lod(
                    key,
                    lod,
                    &mut commands,
                    &mut meshes,
                    &mut world_res,
                    &mut state,
                );
            }
            continue;
        }
        if spawned >= config.max_spawn_per_frame {
            break;
        }

        // Dispatch terrain gen to a background thread.
        let tx = state.tx.clone();
        state.pending.insert(key);
        state.loaded.insert(key);
        spawned += 1;

        std::thread::spawn(move || {
            let data = ChunkData::generate(ChunkPos::new(key.0, 0, key.1));
            let _ = tx.send(ChunkReady { key, data });
        });
    }

    // ── 6. process completed tasks ──
    drain_completed(&mut commands, &mut meshes, &mut world_res, &config, &mut state);
}

// ---------------------------------------------------------------------------
// drain completed async tasks
// ---------------------------------------------------------------------------

fn drain_completed(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world_res: &mut crate::World,
    config: &StreamingConfig,
    state: &mut ResMut<StreamState>,
) {
    let mut meshed = 0usize;
    while meshed < config.max_mesh_per_frame {
        let ready = {
            let rx = state.rx.lock().expect("streaming rx lock poisoned");
            match rx.try_recv() {
                Ok(r) => r,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        };

        let key = ready.key;

        // Compute LOD based on distance to player's current chunk.
        let lod = {
            let pc = state.last_player_chunk;
            let dist = (key.0 - pc.0).abs().max((key.1 - pc.1).abs());
            if dist > config.lod_transition {
                1u8
            } else {
                0u8
            }
        };

        // The chunk entity is the transform/visibility parent only — the meshes
        // hang off it as children (one per block type at LOD 0), which is what
        // keeps `frustum_cull` and `unload_chunk` addressing one entity.
        let entity = commands
            .spawn((
                Transform::from_xyz(
                    (key.0 * CHUNK_SIZE) as f32,
                    0.0,
                    (key.1 * CHUNK_SIZE) as f32,
                ),
                Visibility::default(),
                StreamedChunk { key, lod },
            ))
            .id();
        let quads = build_lod_children(commands, meshes, world_res, entity, &ready.data, lod);

        // Add to World.chunks so the HUD and editor see it.
        world_res.chunks.entry(key).or_insert(crate::ChunkSlot {
            data: ready.data,
            entity,
            quads,
        });
        // Update quad total: we might be re-inserting, so just use the new count.
        world_res.total_quads = world_res
            .chunks
            .values()
            .map(|s| s.quads)
            .sum();

        state.chunk_entities.insert(key, (entity, lod));
        state.pending.remove(&key);
        meshed += 1;
    }
}

// ---------------------------------------------------------------------------
// unload a chunk
// ---------------------------------------------------------------------------

fn unload_chunk(
    key: &(i32, i32),
    commands: &mut Commands,
    world_res: &mut crate::World,
    state: &mut ResMut<StreamState>,
) {
    // Despawn the entity.
    if let Some(&(entity, _)) = state.chunk_entities.get(key) {
        commands.entity(entity).despawn();
    }

    // Remove from World.chunks.
    world_res.chunks.remove(key);

    // Recompute quad total.
    world_res.total_quads = world_res.chunks.values().map(|s| s.quads).sum();

    state.loaded.remove(key);
    state.chunk_entities.remove(key);
}

// ---------------------------------------------------------------------------
// LOD switch
// ---------------------------------------------------------------------------

fn switch_lod(
    key: (i32, i32),
    new_lod: u8,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world_res: &mut crate::World,
    state: &mut ResMut<StreamState>,
) {
    // Grab the chunk data from World.chunks to mesh it.
    let Some(slot) = world_res.chunks.get(&key) else {
        return;
    };
    let entity = slot.entity;

    // Swaps the entity's children in place — the entity, and therefore every
    // handle held on it (`chunk_entities`, `ChunkSlot::entity`), survives.
    let quads = build_lod_children(commands, meshes, world_res, entity, &slot.data, new_lod);

    // Update quad tracking in-place.
    if let Some(slot_mut) = world_res.chunks.get_mut(&key) {
        slot_mut.quads = quads;
    }
    world_res.total_quads = world_res.chunks.values().map(|s| s.quads).sum();

    // Update tracking.
    state.chunk_entities.insert(key, (entity, new_lod));
}

// ---------------------------------------------------------------------------
// system: frustum culling
// ---------------------------------------------------------------------------

/// Toggle chunk `Visibility` based on the camera frustum each frame.
/// Chunks whose world-space AABB does not intersect the frustum are hidden;
/// Bevy's renderer then skips them entirely.
fn frustum_cull(
    cam_q: Query<&Frustum, With<Camera3d>>,
    mut chunks_q: Query<(&StreamedChunk, &Transform, &mut Visibility)>,
) {
    let Ok(frustum) = cam_q.single() else {
        return;
    };

    // Each chunk is a CHUNK_SIZE³ cube positioned at its world origin.
    let half = (CHUNK_SIZE as f32) * 0.5;
    let chunk_aabb = Aabb {
        center: Vec3A::new(half, half, half),
        half_extents: Vec3A::new(half, half, half),
    };

    for (_marker, xf, mut vis) in &mut chunks_q {
        // Build the world-from-local affine for this chunk's AABB.
        let world_from_local = Affine3A::from_scale_rotation_translation(
            xf.scale,
            xf.rotation,
            xf.translation,
        );

        let visible = frustum.intersects_obb(&chunk_aabb, &world_from_local, true, true);
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
