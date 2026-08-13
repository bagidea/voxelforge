//! Import pipeline: MagicaVoxel .vox → world blocks, glTF .glb/.gltf → entity
//! spawn, `ModelCatalog` asset browser, and `SpawnModel` message for the editor.
//!
//! The whole asset pipeline goes through Bevy's `AssetServer` reading from the
//! filesystem — the client is native-only (Steam).
//!
//! ## Wiring (add to main.rs)
//!
//! ```ignore
//! mod import;                           // near the other `mod` declarations
//! ```
//!
//! In `app` builder (after `add_plugins(DefaultPlugins ...)`):
//!
//! ```ignore
//! .add_plugins(import::ImportPlugin)
//! ```
//!
//! ## New dependencies (add to client/Cargo.toml)
//!
//! ```toml
//! dot_vox = "5"
//! ```

use std::path::Path;

use bevy::asset::io::Reader;
use bevy::asset::AssetLoader;
use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;
use voxelforge_sim::block::BlockId;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ImportPlugin;

impl Plugin for ImportPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<VoxAsset>()
            .register_asset_loader(VoxAssetLoader)
            .add_message::<SpawnModel>()
            .init_resource::<ModelCatalog>()
            .add_systems(Startup, scan_catalog)
            .add_systems(Update, (apply_spawn_model, realize_vox_models));
    }
}

// ---------------------------------------------------------------------------
// VoxAsset — raw .vox bytes loaded through the Bevy asset pipeline
// ---------------------------------------------------------------------------

/// Raw bytes from a `.vox` file, loaded through Bevy's `AssetServer`.
///
/// The server reads from disk; callers just get `&[u8]` to feed into
/// [`parse_vox_bytes`].
#[derive(Asset, TypePath, Clone)]
pub(crate) struct VoxAsset(Vec<u8>);

/// Loads `.vox` files as raw bytes — no parsing, just the file contents.
///
/// Parsing is deferred to [`parse_vox_bytes`] so the same logic is testable
/// without any I/O.
#[derive(TypePath)]
struct VoxAssetLoader;

impl AssetLoader for VoxAssetLoader {
    type Asset = VoxAsset;
    type Settings = ();
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(VoxAsset(bytes))
    }

    fn extensions(&self) -> &[&str] {
        &["vox"]
    }
}

// ---------------------------------------------------------------------------
// SpawnModel — fired by the editor / scene loader to place a model
// ---------------------------------------------------------------------------

/// Place a model in the world. The editor fires this; the import plugin
/// handles the actual asset loading and entity spawn.
#[derive(Clone, Debug)]
pub struct SpawnModel {
    /// Path relative to `assets/`, e.g. `"models/cottage.vox"` or
    /// `"models/hero.glb"`.
    pub path: String,
    /// World-space transform stamped on the model's root entity.
    pub transform: Transform,
}

impl Message for SpawnModel {}

// ---------------------------------------------------------------------------
// ModelCatalog — built at startup by scanning `assets/models/`
// ---------------------------------------------------------------------------

/// Shared asset-browser state. Built once at startup; Yamamoto's UI reads it
/// to populate the model palette.
#[derive(Resource, Clone, Debug, Default)]
pub struct ModelCatalog {
    /// All importable models, sorted by name for stable UI ordering.
    pub entries: Vec<ModelEntry>,
}

/// One entry in the model catalog — enough for a thumbnail + tooltip row.
#[derive(Clone, Debug)]
pub struct ModelEntry {
    /// Display name (file stem, no extension).
    pub name: String,
    /// Path relative to `assets/` (e.g. `"models/cottage.vox"`).
    pub path: String,
    /// What pipeline handles this file.
    pub kind: ModelKind,
}

/// Which import pipeline a model file routes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    /// MagicaVoxel `.vox` — parsed into world blocks.
    Voxel,
    /// glTF 2.0 `.glb` or `.gltf` — spawned as a Bevy scene.
    Gltf,
}

// ---------------------------------------------------------------------------
// Scan `assets/models/` → ModelCatalog (Startup)
// ---------------------------------------------------------------------------

/// Populates [`ModelCatalog`] by scanning `assets/models/` on the filesystem.
fn scan_catalog(mut catalog: ResMut<ModelCatalog>) {
    let dir = Path::new("assets/models");
    if !dir.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let kind = match ext.to_ascii_lowercase().as_str() {
            "vox" => ModelKind::Voxel,
            "glb" | "gltf" => ModelKind::Gltf,
            _ => continue,
        };
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();
        let rel = path
            .strip_prefix("assets/")
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        catalog.entries.push(ModelEntry { name, path: rel, kind });
    }
    catalog.entries.sort_by(|a, b| a.name.cmp(&b.name));
}

// ---------------------------------------------------------------------------
// SpawnModel dispatcher (Update) — reads messages, spawns placeholder entities
// ---------------------------------------------------------------------------

/// Marker component — the import pipeline hasn't built this model's mesh yet.
/// Holds the `Handle<VoxAsset>` so [`realize_vox_models`] can check whether the
/// asset has finished loading.
#[derive(Component)]
struct PendingVoxModel {
    handle: Handle<VoxAsset>,
    path: String,
}

fn apply_spawn_model(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut messages: MessageReader<SpawnModel>,
) {
    for ev in messages.read() {
        let ext = extension(&ev.path);
        match ext {
            "vox" => {
                // Start loading through AssetServer (filesystem).
                let handle = asset_server.load::<VoxAsset>(&ev.path);
                commands.spawn((
                    ev.transform,
                    Visibility::default(),
                    PendingVoxModel {
                        handle,
                        path: ev.path.clone(),
                    },
                    Name::new(format!("vox:{}", ev.path)),
                ));
                info!("IMPORT vox queued path={}", ev.path);
            }
            "glb" | "gltf" => {
                let gltf_path = if ev.path.starts_with("models/") {
                    ev.path.clone()
                } else {
                    format!("models/{}", ev.path)
                };
                // Bevy 0.19 glTF: load via AssetServer; scene auto-spawns.
                let _ = asset_server.load::<bevy::gltf::Gltf>(&gltf_path);
                info!("IMPORT gltf load started path={}", gltf_path);
            }
            _ => {
                error!("IMPORT unknown model kind path={}", ev.path);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Vox model realisation (Update) — runs every frame, picks up loaded assets
// ---------------------------------------------------------------------------

fn realize_vox_models(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vox_assets: Res<Assets<VoxAsset>>,
    pending: Query<(Entity, &PendingVoxModel, &Transform)>,
) {
    for (entity, pending, _transform) in &pending {
        // Check whether the AssetServer has finished loading this .vox.
        let Some(vox_data) = vox_assets.get(&pending.handle) else {
            continue; // still loading — try again next frame
        };

        let voxels = match parse_vox_bytes(&vox_data.0) {
            Ok(v) => v,
            Err(e) => {
                error!("IMPORT vox failed path={}: {e}", pending.path);
                commands.entity(entity).remove::<PendingVoxModel>();
                continue;
            }
        };

        info!(
            "IMPORT vox path={} voxel_count={}",
            pending.path,
            voxels.len()
        );

        // Spawn each voxel as a 1×1×1 cube child with a solid-colour material.
        // Large models should use greedy meshing; for editor preview this is
        // correct and simple.
        spawn_voxel_cubes(
            &mut commands,
            entity,
            &mut meshes,
            &mut materials,
            &voxels,
        );

        // Consume the marker so it only fires once.
        commands.entity(entity).remove::<PendingVoxModel>();
    }
}

/// Spawn one child cube per voxel, coloured by its mapped BlockId.
fn spawn_voxel_cubes(
    commands: &mut Commands,
    root: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    voxels: &[(i32, i32, i32, BlockId)],
) {
    // Material cache — reuse the same handle for every voxel of the same block type.
    let mut mat_cache: std::collections::HashMap<u8, Handle<StandardMaterial>> =
        std::collections::HashMap::new();

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    for (lx, ly, lz, block) in voxels.iter().take(4096) {
        // Hard cap at 4096 voxels per model to prevent accidental freezes.
        let color = block.base_color();
        let mat = mat_cache.entry(block.0).or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(
                    color[0] as f32 / 255.0,
                    color[1] as f32 / 255.0,
                    color[2] as f32 / 255.0,
                ),
                perceptual_roughness: 0.9,
                metallic: 0.0,
                ..default()
            })
        });

        let mut child = commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(*lx as f32, *ly as f32, *lz as f32),
        ));
        child.set_parent_in_place(root);
    }

    if voxels.len() > 4096 {
        warn!(
            "IMPORT vox model clipped to 4096/{} voxels (limit)",
            voxels.len()
        );
    }
}

// ---------------------------------------------------------------------------
// .vox → BlockId mapping
// ---------------------------------------------------------------------------

#[inline]
fn extension(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
}

/// Parse raw `.vox` bytes → list of `(x, y, z, BlockId)`. Uses `dot_vox`.
///
/// Pure computation — no I/O, works on both native and WASM. Bytes come from
/// [`VoxAsset`] (loaded via AssetServer) or from test fixtures.
fn parse_vox_bytes(data: &[u8]) -> Result<Vec<(i32, i32, i32, BlockId)>, String> {
    let dot = dot_vox::load_bytes(data).map_err(|e| format!("dot_vox: {e}"))?;
    let mut voxels = Vec::new();
    for model in &dot.models {
        for v in &model.voxels {
            // dot_vox 5.x stores Voxel.i as 0-based (already adjusted from the
            // on-disk 1-based index), so use it directly to index the palette.
            let color = dot
                .palette
                .get(v.i as usize)
                .copied()
                .unwrap_or(dot_vox::Color { r: 128, g: 128, b: 128, a: 255 });
            let block = closest_block(color.r, color.g, color.b);
            voxels.push((v.x as i32, v.y as i32, v.z as i32, block));
        }
    }
    Ok(voxels)
}

/// Map an sRGB colour to the closest `BlockId` by Euclidean distance over the
/// `base_color` palette.  Single source of truth — no hardcoded palette copy.
pub fn closest_block(r: u8, g: u8, b: u8) -> BlockId {
    let target = [r as i32, g as i32, b as i32];
    let mut best = BlockId::STONE;
    let mut best_d = i32::MAX;
    for &b in BlockId::ALL_PLACEABLE {
        let c = b.base_color();
        let dr = c[0] as i32 - target[0];
        let dg = c[1] as i32 - target[1];
        let db = c[2] as i32 - target[2];
        let d = dr * dr + dg * dg + db * db;
        if d < best_d {
            best_d = d;
            best = b;
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid .vox file (version 150, 4×4×4, 2 voxels, no RGBA).
    fn minimal_vox_bytes() -> Vec<u8> {
        let mut b = Vec::with_capacity(68);

        // VOX header
        b.extend_from_slice(b"VOX ");
        b.extend_from_slice(&150u32.to_le_bytes());

        // MAIN chunk: content=0, children=48 (SIZE 24 + XYZI 24)
        b.extend_from_slice(b"MAIN");
        b.extend_from_slice(&0u32.to_le_bytes()); // content
        b.extend_from_slice(&48u32.to_le_bytes()); // children

        // SIZE chunk: 4×4×4
        b.extend_from_slice(b"SIZE");
        b.extend_from_slice(&12u32.to_le_bytes()); // content = 3×i32
        b.extend_from_slice(&0u32.to_le_bytes()); // children
        b.extend_from_slice(&4u32.to_le_bytes()); // x
        b.extend_from_slice(&4u32.to_le_bytes()); // y
        b.extend_from_slice(&4u32.to_le_bytes()); // z

        // XYZI chunk: 2 voxels
        let n: u32 = 2;
        let content_size = 4 + n * 4; // count(u32) + n×(x,y,z,i)
        b.extend_from_slice(b"XYZI");
        b.extend_from_slice(&content_size.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // children
        b.extend_from_slice(&n.to_le_bytes()); // count
        // Voxel 1 @ (1,1,1) colour idx 1
        b.extend_from_slice(&[1, 1, 1, 1]);
        // Voxel 2 @ (2,1,1) colour idx 2
        b.extend_from_slice(&[2, 1, 1, 2]);

        assert_eq!(b.len(), 68); // sanity
        b
    }

    #[test]
    fn parse_minimal_vox_count() {
        let data = minimal_vox_bytes();
        let result = parse_vox_bytes(&data);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let voxels = result.unwrap();
        assert_eq!(voxels.len(), 2, "expected 2 voxels, got {}", voxels.len());
        // Positions
        assert_eq!((voxels[0].0, voxels[0].1, voxels[0].2), (1, 1, 1));
        assert_eq!((voxels[1].0, voxels[1].1, voxels[1].2), (2, 1, 1));
        // Both must map to opaque blocks
        assert!(voxels[0].3.is_opaque(), "voxel 0 should be opaque");
        assert!(voxels[1].3.is_opaque(), "voxel 1 should be opaque");
    }

    /// Feeding a palette colour straight back in must return the block it came
    /// from — over the whole table, read off `base_color()` rather than spelled
    /// out as hex. The version this replaced hand-wrote six colours, and when
    /// Monanisa's §6.1 palette landed, five of the six stopped naming the block
    /// they were paired with: four (grass, dirt, snow, obsidian) kept "passing"
    /// because the old value was still the nearest one, and the fifth had been
    /// quietly red ever since — grey `(128,128,138)` resolves to `cobblestone`,
    /// not `stone`. A literal cannot notice the table moved underneath it.
    ///
    /// WHAT THIS TEST ACTUALLY CATCHES — narrower than it looks, and the note
    /// that stood here claimed otherwise ("a tight test by construction … 4.7
    /// apart … nothing weaker tells them apart"). It doesn't. A block always
    /// scores `d == 0` against its own colour, and `closest_block` keeps the
    /// incumbent on ties (`d < best_d`, strict), so the only way an entry can
    /// fail to round-trip is if an EARLIER entry holds the exact same colour.
    /// It is a duplicate-colour check, and it stays green for any palette whose
    /// entries are distinct — move `stone` to within 1.0 of `cobblestone` and
    /// this still passes. Tightness is pinned by
    /// [`closest_block_palette_pairs_stay_far_enough_apart`] instead.
    ///
    /// It is still worth its line: it is the ONLY duplicate check that covers
    /// `lamp`. `block.rs`'s `designer_colours_are_distinct` walks `DESIGNER_HEX`,
    /// which deliberately omits lamp (no designer hex yet), so a lamp albedo
    /// that collided with another block would slip past it and land here.
    #[test]
    fn closest_block_round_trips_every_palette_colour() {
        for &id in BlockId::ALL_PLACEABLE {
            let [r, g, b] = id.base_color();
            assert_eq!(
                closest_block(r, g, b),
                id,
                "{} does not round-trip its own base_color {:?}",
                id.name(),
                [r, g, b]
            );
        }
    }

    /// The palette's closest pair, pinned as a FLOOR. This is the guard the
    /// round-trip test above was mistakenly credited with: it is the one that
    /// goes red if someone moves two blocks nearer each other, which is what
    /// actually degrades the `.vox` importer — the tighter the closest pair, the
    /// less colour noise it takes to import a wall of `stone` as `cobblestone`.
    ///
    /// Squared distance, so the arithmetic stays in integers exactly as
    /// `closest_block` does it. `22` is `stone` #8f8776 vs `cobblestone` #8c8a78
    /// — Δ(3, −3, −2), i.e. 4.69 apart in sRGB — measured over Monanisa's §6.1
    /// palette as shipped.
    ///
    /// A floor, not a fixture: widening the palette is a improvement and should
    /// not fail. If this ever goes red the question is not "which number do I
    /// change" but "did the designer mean to put those two materials that close
    /// together" — the palette is hers (`docs/block-palette.md` §6.1), and the
    /// fix for a genuinely-too-close pair lives in the importer (match by block
    /// name), never in the hex.
    #[test]
    fn closest_block_palette_pairs_stay_far_enough_apart() {
        const MIN_D2: i32 = 22;

        let mut worst = i32::MAX;
        let mut pair = (BlockId::AIR, BlockId::AIR);
        for (i, &a) in BlockId::ALL_PLACEABLE.iter().enumerate() {
            for &b in &BlockId::ALL_PLACEABLE[i + 1..] {
                let (ca, cb) = (a.base_color(), b.base_color());
                let d2: i32 = (0..3)
                    .map(|c| {
                        let d = ca[c] as i32 - cb[c] as i32;
                        d * d
                    })
                    .sum();
                if d2 < worst {
                    worst = d2;
                    pair = (a, b);
                }
            }
        }
        assert!(
            worst >= MIN_D2,
            "{} and {} are only d²={} apart (floor {}): the palette got tighter \
             than the importer was pinned against",
            pair.0.name(),
            pair.1.name(),
            worst,
            MIN_D2
        );
    }

    /// A colour equal to no palette entry still lands on its nearest one —
    /// proves the search really is nearest-neighbour, which a plain exact-match
    /// lookup would also satisfy above.
    ///
    /// Nudged off `stone`, not `obsidian`. Obsidian #1a1620 is the most isolated
    /// entry in the palette — its nearest neighbour is thousands of d² away — so
    /// a nudge there is the easiest case there is and passes under almost any
    /// mistake. `stone` is half of the closest pair (see
    /// [`closest_block_palette_pairs_stay_far_enough_apart`]), so this watches
    /// the boundary that is actually contested: +2 on every channel is d²=12
    /// from `stone` and d²=26 from `cobblestone`, and the assert is that it
    /// still comes back `stone`.
    #[test]
    fn closest_block_snaps_a_near_colour_to_its_neighbour() {
        let exact = BlockId::STONE.base_color();
        let near = exact.map(|c| c.saturating_add(2));
        assert_ne!(near, exact, "the nudge degenerated into an exact match");
        assert_eq!(closest_block(near[0], near[1], near[2]), BlockId::STONE);
    }

    #[test]
    fn closest_block_produces_only_opaque() {
        // Feed a variety of colours — every result should be opaque.
        for (r, g, b) in [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (128, 128, 128),
            (255, 255, 255),
            (0, 0, 0),
            (200, 130, 70),
            (110, 100, 94),
        ] {
            let block = closest_block(r, g, b);
            assert!(block.is_opaque(), "closest_block({r},{g},{b}) → {block:?} should be opaque");
            assert!(block.0 > 0, "closest_block({r},{g},{b}) → id={} should not be AIR", block.0);
        }
    }

    #[test]
    fn minimal_vox_bytes_are_valid_magic() {
        let data = minimal_vox_bytes();
        assert_eq!(&data[0..4], b"VOX ");
        assert_eq!(u32::from_le_bytes([data[4], data[5], data[6], data[7]]), 150);
    }
}
