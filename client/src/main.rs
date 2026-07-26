//! Voxelforge client — Phase 0 go/no-go spike (Rust + Bevy 0.19 + wgpu).
//!
//! Proves: chunk gen → greedy mesh → texture-atlas StandardMaterial → fly
//! camera, on both native (wgpu/DX12/Vulkan) and web (WebGPU). Ships a built-in
//! ramp benchmark that spawns more chunks every couple seconds and reports the
//! largest chunk count that still holds >= 60 FPS.
//!
//! Chunk data and world-gen live in `voxelforge_sim` (shared with the server).
//! The greedy mesher and atlas live in `voxel` (Bevy-coupled, client-only).

mod hero;
mod mapfile;
mod voxel;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::text::FontSize;
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow};
use std::collections::HashMap;

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, ChunkPos, CHUNK_SIZE as CHUNK};
use voxelforge_sim::worldgen::{self, terrain_height};
use voxel::{build_atlas, greedy_mesh_chunk};

// ---------------------------------------------------------------------------
// Config (env vars on native, ?query params on web)
// ---------------------------------------------------------------------------

#[derive(Resource, Clone)]
struct Cfg {
    bench: bool,
    grid: usize,
    shot: Option<String>,
    /// Phase 1 hero look-shot scene instead of terrain.
    hero: bool,
    /// Force a present mode (root-causing the FPS cliff): one of
    /// vsync|novsync|fifo|mailbox|immediate. None => auto (novsync in bench).
    present: Option<String>,
    /// Cold-boot the ramp at this grid side instead of 1 (cliff isolation).
    start_side: i32,
    /// Scripted place/break demo (headless proof of the edit→remesh loop).
    edit_demo: bool,
    /// Scripted walk-physics demo (headless proof of gravity + voxel collision).
    walk_demo: bool,
    /// Load a saved map file at startup (world = file contents, no procedural terrain).
    map_load: Option<String>,
    /// Author-a-tiny-map demo: start blank, build a scene, save it here, then shoot.
    map_save: Option<String>,
    // Hero-shot tunables (env-driven so the shot re-frames without a recompile).
    cam: Option<[f32; 7]>, // ex,ey,ez, tx,ty,tz, fov_deg
    sun: Option<[f32; 3]>, // elevation_deg, azimuth_deg, illuminance
    dof: Option<[f32; 2]>, // focal_distance, aperture_f_stops
    fog: Option<f32>,      // volumetric density_factor
    exposure: Option<f32>, // camera ev100
    grade: Option<[f32; 3]>, // post grade: temperature, post_saturation, contrast
    // Hero bounce/highlight knobs — env-driven so the 3 hardest gates
    // (G3 shadow floor, G5/G6 window roll-off) tune WITHOUT a recompile.
    ambient: Option<f32>,  // AmbientLight brightness (lux)
    emissive: Option<f32>, // scale on the window pane emissive
    dfog: Option<f32>,     // DistanceFog density
    soft: Option<f32>,     // PCSS soft_shadow_size (sun apparent size; wider = softer)
    seed: u64,            // world-gen seed (VOXELFORGE_SEED, default 42)
}

/// Parse "a,b,c" env into a fixed float array (all-or-nothing).
#[cfg(not(target_arch = "wasm32"))]
fn env_floats<const N: usize>(key: &str) -> Option<[f32; N]> {
    let raw = std::env::var(key).ok()?;
    let parts: Vec<f32> = raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    if parts.len() == N {
        let mut out = [0.0; N];
        out.copy_from_slice(&parts);
        Some(out)
    } else {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_cfg() -> Cfg {
    Cfg {
        bench: std::env::var("VOXELFORGE_BENCH").is_ok(),
        grid: std::env::var("VOXELFORGE_GRID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6),
        shot: std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty()),
        hero: std::env::var("VOXELFORGE_HERO").is_ok(),
        present: std::env::var("VOXELFORGE_PRESENT").ok().filter(|s| !s.is_empty()),
        start_side: std::env::var("VOXELFORGE_START_SIDE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1)
            .max(1),
        edit_demo: std::env::var("VOXELFORGE_EDIT_DEMO").is_ok(),
        walk_demo: std::env::var("VOXELFORGE_WALK_DEMO").is_ok(),
        map_load: std::env::var("VOXELFORGE_MAP_LOAD").ok().filter(|s| !s.is_empty()),
        map_save: std::env::var("VOXELFORGE_MAP_SAVE").ok().filter(|s| !s.is_empty()),
        cam: env_floats("VOXELFORGE_CAM"),
        sun: env_floats("VOXELFORGE_SUN"),
        dof: env_floats("VOXELFORGE_DOF"),
        fog: std::env::var("VOXELFORGE_FOG").ok().and_then(|v| v.parse().ok()),
        exposure: std::env::var("VOXELFORGE_EXPOSURE").ok().and_then(|v| v.parse().ok()),
        grade: env_floats("VOXELFORGE_GRADE"),
        ambient: std::env::var("VOXELFORGE_AMBIENT").ok().and_then(|v| v.parse().ok()),
        emissive: std::env::var("VOXELFORGE_EMISSIVE").ok().and_then(|v| v.parse().ok()),
        dfog: std::env::var("VOXELFORGE_DFOG").ok().and_then(|v| v.parse().ok()),
        soft: std::env::var("VOXELFORGE_SOFT").ok().and_then(|v| v.parse().ok()),
        seed: std::env::var("VOXELFORGE_SEED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(42),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_cfg() -> Cfg {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    let bench = search.contains("bench");
    let grid = search
        .split(['?', '&'])
        .find_map(|kv| kv.strip_prefix("grid=").and_then(|v| v.parse().ok()))
        .unwrap_or(6);
    Cfg {
        bench,
        grid,
        shot: None,
        hero: search.contains("hero"),
        present: None,
        start_side: 1,
        edit_demo: search.contains("editdemo"),
        walk_demo: search.contains("walkdemo"),
        map_load: None,
        map_save: None,
        cam: None,
        sun: None,
        dof: None,
        fog: None,
        exposure: None,
        grade: None,
        ambient: None,
        emissive: None,
        dfog: None,
        soft: None,
        seed: 42,
    }
}

#[cfg(target_arch = "wasm32")]
fn set_dom(id: &str, text: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
    {
        el.set_text_content(Some(text));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_dom(_id: &str, _text: &str) {}

// ---------------------------------------------------------------------------

/// One live chunk: its editable voxel data, the entity carrying its mesh, and
/// the quad count it currently contributes (kept in sync so the HUD total is
/// correct after edits re-mesh a chunk).
struct ChunkSlot {
    data: ChunkData,
    entity: Entity,
    quads: usize,
}

#[derive(Resource)]
struct World {
    material: Handle<StandardMaterial>,
    /// Keyed by (chunk_x, chunk_z) — only the y=0 layer is spawned in Phase 0.
    chunks: HashMap<(i32, i32), ChunkSlot>,
    total_quads: usize,
}

/// Two ways to be in the world: PLAY (walk/fly + single edits) and EDIT (free-fly
/// map building with the fill tool). Tab toggles between them; the HUD shows which.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorMode {
    Play,
    Edit,
}

/// Player build state: which block right-click places, how far edits reach, the
/// current mode, the pending fill corner, and where quick-save/load writes.
#[derive(Resource)]
struct Editor {
    selected: BlockId,
    reach: f32,
    mode: EditorMode,
    /// First corner of a box-fill (set by G in EDIT mode); the second G fills it.
    fill_anchor: Option<IVec3>,
    /// Path F5 saves to / F9 loads from.
    map_path: String,
    /// Transient HUD note (last save/load result), shown for a short while.
    status: String,
}

/// Fires the scripted "author a small map then save it" demo once (headless proof
/// that the editor writes a real, reloadable file).
#[derive(Resource)]
struct MapSaveDemo {
    done: bool,
}

/// Fires the scripted place/break demo exactly once (headless verify path).
#[derive(Resource)]
struct EditDemo {
    done: bool,
}

/// Fires the scripted walk-physics demo once (headless gravity/collision proof).
#[derive(Resource)]
struct WalkDemo {
    done: bool,
}

/// The player avatar (third-person). Its `Transform.translation` is the *eye*
/// position (feet + EYE_HEIGHT) — the same convention `move_body` has always used,
/// so all the walk physics is reused unchanged. The visible body mesh rides along
/// as a child, and the transform's Y-rotation is the avatar's facing (`face_yaw`).
#[derive(Component)]
struct FlyCam {
    /// The direction the avatar is currently turned to face (smoothed toward the
    /// movement direction each frame). Drives the body mesh's rotation.
    face_yaw: f32,
    /// Vertical (+residual) velocity used by walk mode's gravity/jump; unused in fly.
    vel: Vec3,
    /// true = grounded walking body (gravity + AABB voxel collision); false =
    /// free noclip fly (EDIT mode building). Toggled live with F.
    walking: bool,
    /// Set the frame the body rests on a solid voxel below — gates the jump.
    grounded: bool,
}

/// The orbit camera — rides a spring-arm/boom behind + above the avatar (Roblox
/// style). Mouse drives `yaw`/`pitch`; `dist` is the boom length, pulled in by
/// `camera_boom` when a wall would otherwise clip between camera and avatar.
#[derive(Component)]
struct OrbitCam {
    yaw: f32,
    pitch: f32,
    dist: f32,
}

#[derive(Component)]
struct HudText;

/// Drives the ramp benchmark: grows the visible grid every phase and records the
/// median FPS for each chunk count.
#[derive(Resource)]
struct Bench {
    active: bool,
    side: i32,
    phase_start: f32,
    samples: Vec<f32>,
    max_60: usize,
    finished: bool,
    took_shot: bool,
    shot: Option<String>,
    log: String,
}

const WARMUP: f32 = 1.2;
const PHASE: f32 = 2.0;
const MAX_SIDE: i32 = 32; // up to 1024 chunks (ramp stops early once FPS dips <55)

fn main() {
    let cfg = read_cfg();

    let present_mode = match cfg.present.as_deref() {
        Some("vsync") => PresentMode::AutoVsync,
        Some("novsync") => PresentMode::AutoNoVsync,
        Some("fifo") => PresentMode::Fifo,
        Some("fiforelaxed") => PresentMode::FifoRelaxed,
        Some("mailbox") => PresentMode::Mailbox,
        Some("immediate") => PresentMode::Immediate,
        _ if cfg.bench => PresentMode::AutoNoVsync,
        _ => PresentMode::AutoVsync,
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Voxelforge — Phase 0 spike".into(),
                    resolution: (1280u32, 720u32).into(),
                    canvas: Some("#bevy".into()),
                    fit_canvas_to_parent: true,
                    present_mode,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .insert_resource(ClearColor(Color::srgb(0.53, 0.72, 0.92)))
    .insert_resource(Bench {
        active: cfg.bench,
        side: cfg.start_side,
        phase_start: 0.0,
        samples: Vec::new(),
        max_60: 0,
        finished: false,
        took_shot: false,
        shot: cfg.shot.clone(),
        log: String::new(),
    });

    if cfg.hero {
        // A 4K directional shadow map gives the PCSS penumbra enough texels to
        // stay smooth instead of stair-stepping (default is 2048).
        app.insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
            .insert_resource(cfg)
            .add_systems(Startup, hero::setup_hero)
            .add_systems(Update, (fly_camera, screenshot_once));
    } else {
        app.insert_resource(cfg)
            .insert_resource(Editor {
                selected: BlockId::STONE,
                reach: 200.0,
                mode: EditorMode::Play,
                fill_anchor: None,
                map_path: "maps/quicksave.json".into(),
                status: String::new(),
            })
            .insert_resource(EditDemo { done: false })
            .insert_resource(WalkDemo { done: false })
            .insert_resource(MapSaveDemo { done: false })
            .add_systems(Startup, setup)
            // edit_voxels runs before fly_camera so the click that grabs the
            // cursor is not also read as a break; edits happen from click #2 on.
            .add_systems(
                Update,
                (
                    (edit_voxels, fly_camera).chain(),
                    editor_controls,
                    highlight_target,
                    edit_demo,
                    walk_demo,
                    map_save_demo,
                    hud,
                    bench_ramp,
                    screenshot_once,
                ),
            );
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<Cfg>,
    mut bench: ResMut<Bench>,
) {
    // Seed the terrain generator before any chunk queries happen.
    worldgen::set_seed(cfg.seed);

    let atlas = images.add(build_atlas());
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(atlas),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        reflectance: 0.1,
        ..default()
    });

    let mut world = World {
        material,
        chunks: HashMap::new(),
        total_quads: 0,
    };

    // Build the starting world: a loaded map file, a blank editor canvas, or the
    // procedural terrain (the default). A loaded map fully describes its own blocks,
    // so it starts from empty chunks and sets exactly what the file lists.
    let loaded_map = cfg.map_load.as_deref().and_then(|p| match load_map_file(p) {
        Ok(m) => {
            println!(
                "MAP_LOAD ok path={p} name=\"{}\" blocks={} size={}x{} chunks",
                m.name,
                m.blocks.len(),
                m.size.chunks_x,
                m.size.chunks_z
            );
            Some(m)
        }
        Err(e) => {
            eprintln!("MAP_LOAD FAIL path={p} err={e}");
            None
        }
    });

    let side;
    if let Some(map) = &loaded_map {
        let nx = map.size.chunks_x.max(1);
        let nz = map.size.chunks_z.max(1);
        for z in 0..nz {
            for x in 0..nx {
                spawn_empty_chunk(&mut commands, &mut meshes, &mut world, x, z);
            }
        }
        apply_map_blocks(&mut commands, &mut meshes, &mut world, map);
        side = nx.max(nz);
    } else if cfg.map_save.is_some() {
        // Author-from-scratch flow: one blank 32³ chunk the demo fills, then saves.
        spawn_empty_chunk(&mut commands, &mut meshes, &mut world, 0, 0);
        side = 1;
    } else {
        side = if bench.active { bench.side } else { cfg.grid as i32 };
        for z in 0..side {
            for x in 0..side {
                spawn_chunk(&mut commands, &mut meshes, &mut world, x, z);
            }
        }
    }

    // Sun.
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(60.0, 120.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Player avatar + orbit camera (third-person). The walk-demo drops a grounded
    // body high over the grid centre so gravity settles it onto the ground in the
    // screenshot; otherwise the avatar starts near the surface as a free-fly body
    // (EDIT building) you can drop into walk with F. The avatar's Transform is the
    // *eye* position — the same convention move_body uses — with a visible body mesh
    // riding along as a child, and a separate camera trailing on the boom.
    let c = side as f32 * CHUNK as f32 * 0.5;
    // On a loaded map the surface is whatever the file put down, so use a fixed safe
    // height instead of the procedural terrain height.
    let ground = if loaded_map.is_some() { 8 } else { terrain_height(c, c) };
    let (eye, face_yaw, walking) = if cfg.walk_demo {
        (
            Vec3::new(c, ground as f32 + EYE_HEIGHT + 12.0, c),
            -std::f32::consts::FRAC_PI_4,
            true,
        )
    } else {
        (Vec3::new(c, ground as f32 + EYE_HEIGHT + 1.0, c), 0.0, false)
    };
    let orbit_yaw = face_yaw;
    let orbit_pitch = -0.25;

    // The visible body: a capsule the size of the collision box, plus a small dark
    // "face" block on its front (local -Z) so its heading is legible when it turns.
    let body_mesh = meshes.add(Capsule3d::new(PLAYER_HALF_W, PLAYER_HEIGHT - 2.0 * PLAYER_HALF_W));
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.38, 0.16),
        perceptual_roughness: 0.7,
        ..default()
    });
    let face_mesh = meshes.add(Cuboid::new(0.34, 0.18, 0.12));
    let face_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.13),
        perceptual_roughness: 0.6,
        ..default()
    });
    // Capsule centre sits at the body's mid-height: feet + PLAYER_HEIGHT/2, i.e.
    // EYE_HEIGHT - PLAYER_HEIGHT/2 below the eye (the avatar's own origin).
    let body_dy = -(EYE_HEIGHT - PLAYER_HEIGHT * 0.5);
    commands
        .spawn((
            Transform::from_translation(eye)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, face_yaw)),
            Visibility::default(),
            FlyCam {
                face_yaw,
                vel: Vec3::ZERO,
                walking,
                grounded: false,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(body_mat),
                Transform::from_xyz(0.0, body_dy, 0.0),
            ));
            parent.spawn((
                Mesh3d(face_mesh),
                MeshMaterial3d(face_mat),
                Transform::from_xyz(0.0, body_dy + 0.55, -0.34),
            ));
        });

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(eye + Vec3::new(0.0, PIVOT_UP, BOOM_DIST)),
        OrbitCam {
            yaw: orbit_yaw,
            pitch: orbit_pitch,
            dist: BOOM_DIST,
        },
        AmbientLight {
            brightness: 380.0,
            ..default()
        },
    ));

    // HUD.
    commands.spawn((
        Text::new("booting…"),
        TextFont {
            font_size: FontSize::from(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));

    // Crosshair — the aim point the raycast edits fire from.
    commands.spawn((
        Text::new("+"),
        TextFont {
            font_size: FontSize::from(22.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            left: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-6.0),
                top: Val::Px(-13.0),
                ..default()
            },
            ..default()
        },
    ));

    commands.insert_resource(world);
    bench.phase_start = 0.0;
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    x: i32,
    z: i32,
) {
    if world.chunks.contains_key(&(x, z)) {
        return;
    }
    // Generate chunk data from the shared sim crate (same code path as the server).
    let chunk = ChunkData::generate(ChunkPos::new(x, 0, z));
    let (mesh, quads) = greedy_mesh_chunk(&chunk);
    world.total_quads += quads;
    let entity = commands
        .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(world.material.clone()),
            Transform::from_xyz((x * CHUNK) as f32, 0.0, (z * CHUNK) as f32),
        ))
        .id();
    world.chunks.insert(
        (x, z),
        ChunkSlot {
            data: chunk,
            entity,
            quads,
        },
    );
}

/// Like `spawn_chunk` but the chunk starts as pure air — the blank canvas a loaded
/// map (or the editor's "new map" flow) fills in block by block.
fn spawn_empty_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    x: i32,
    z: i32,
) {
    if world.chunks.contains_key(&(x, z)) {
        return;
    }
    let chunk = ChunkData::empty(ChunkPos::new(x, 0, z));
    let (mesh, quads) = greedy_mesh_chunk(&chunk);
    world.total_quads += quads;
    let entity = commands
        .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(world.material.clone()),
            Transform::from_xyz((x * CHUNK) as f32, 0.0, (z * CHUNK) as f32),
        ))
        .id();
    world.chunks.insert((x, z), ChunkSlot { data: chunk, entity, quads });
}

/// Re-mesh a single chunk in place and keep the HUD quad total in sync — the shared
/// tail of every bulk edit (map load, fill) that touches a chunk's data directly.
fn remesh_chunk(
    world: &mut World,
    key: (i32, i32),
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
) {
    let Some(slot) = world.chunks.get_mut(&key) else {
        return;
    };
    let (mesh, quads) = greedy_mesh_chunk(&slot.data);
    let delta = quads as isize - slot.quads as isize;
    slot.quads = quads;
    let entity = slot.entity;
    world.total_quads = (world.total_quads as isize + delta).max(0) as usize;
    commands.entity(entity).insert(Mesh3d(meshes.add(mesh)));
}

/// Stamp every block a map file lists into an already-spawned (empty) world, then
/// re-mesh only the chunks that changed. Skips out-of-range voxels and unknown block
/// names, reporting the count so a bad file is loud, not silently partial.
fn apply_map_blocks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    map: &mapfile::MapFile,
) {
    use std::collections::HashSet;
    let mut touched: HashSet<(i32, i32)> = HashSet::new();
    let mut set = 0usize;
    let mut skipped = 0usize;
    for mb in &map.blocks {
        let Some(id) = mapfile::block_id_from_name(&mb.block) else {
            skipped += 1;
            continue;
        };
        if mb.y < 0 || mb.y >= CHUNK {
            skipped += 1;
            continue;
        }
        let key = (mb.x.div_euclid(CHUNK), mb.z.div_euclid(CHUNK));
        let Some(slot) = world.chunks.get_mut(&key) else {
            skipped += 1;
            continue;
        };
        slot.data
            .set(mb.x.rem_euclid(CHUNK), mb.y, mb.z.rem_euclid(CHUNK), id);
        touched.insert(key);
        set += 1;
    }
    for key in touched {
        remesh_chunk(world, key, meshes, commands);
    }
    println!("MAP_APPLY set={set} skipped={skipped} total_quads={}", world.total_quads);
}

/// Snapshot the live world as a `MapFile`: its extent in chunks plus every solid
/// voxel in world coordinates (air is implicit, so it is never written).
fn world_to_map(world: &World, name: &str) -> mapfile::MapFile {
    let mut keys: Vec<(i32, i32)> = world.chunks.keys().copied().collect();
    keys.sort();
    let (mut max_cx, mut max_cz) = (0, 0);
    for &(cx, cz) in &keys {
        max_cx = max_cx.max(cx);
        max_cz = max_cz.max(cz);
    }
    let mut blocks = Vec::new();
    for (cx, cz) in keys {
        let slot = &world.chunks[&(cx, cz)];
        for y in 0..CHUNK {
            for z in 0..CHUNK {
                for x in 0..CHUNK {
                    let b = slot.data.get(x, y, z);
                    if b.is_opaque() {
                        blocks.push(mapfile::MapBlock {
                            x: cx * CHUNK + x,
                            y,
                            z: cz * CHUNK + z,
                            block: mapfile::block_name(b).into(),
                        });
                    }
                }
            }
        }
    }
    mapfile::MapFile {
        version: mapfile::MAP_VERSION,
        name: name.into(),
        size: mapfile::MapSize {
            chunks_x: max_cx + 1,
            chunks_z: max_cz + 1,
        },
        blocks,
    }
}

/// Write the world to disk as JSON, creating the `maps/` folder if needed. Returns
/// the number of solid blocks saved. Native-only (no filesystem on wasm).
#[cfg(not(target_arch = "wasm32"))]
fn save_world_to(world: &World, path: &str) -> Result<usize, String> {
    let name = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("map")
        .to_string();
    let map = world_to_map(world, &name);
    let text = mapfile::to_text(&map)?;
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(path, text).map_err(|e| e.to_string())?;
    Ok(map.blocks.len())
}

/// Read + parse a map file. Native-only (no filesystem on wasm).
#[cfg(not(target_arch = "wasm32"))]
fn load_map_file(path: &str) -> Result<mapfile::MapFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    mapfile::parse(&text)
}

#[cfg(target_arch = "wasm32")]
fn save_world_to(_world: &World, _path: &str) -> Result<usize, String> {
    Err("saving maps is not supported on web".into())
}

#[cfg(target_arch = "wasm32")]
fn load_map_file(_path: &str) -> Result<mapfile::MapFile, String> {
    Err("loading maps is not supported on web".into())
}

/// Fill an inclusive box of voxels with one block, re-meshing each touched chunk
/// once. The brush/fill tool and the scripted map-author demo both build through it.
/// Returns the number of voxels actually written.
fn box_fill(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    a: IVec3,
    b: IVec3,
    block: BlockId,
) -> usize {
    use std::collections::HashSet;
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(CHUNK - 1));
    let (z0, z1) = (a.z.min(b.z), a.z.max(b.z));
    let mut touched: HashSet<(i32, i32)> = HashSet::new();
    let mut n = 0usize;
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                let key = (x.div_euclid(CHUNK), z.div_euclid(CHUNK));
                let Some(slot) = world.chunks.get_mut(&key) else {
                    continue;
                };
                slot.data.set(x.rem_euclid(CHUNK), y, z.rem_euclid(CHUNK), block);
                touched.insert(key);
                n += 1;
            }
        }
    }
    for key in touched {
        remesh_chunk(world, key, meshes, commands);
    }
    n
}

/// Height of the topmost solid voxel in the column at world (wx, wz), or None if the
/// column is all air. Used to verify a loaded map is walkable at its own surface.
fn highest_solid(world: &World, wx: i32, wz: i32) -> Option<i32> {
    (0..CHUNK).rev().find(|&y| solid_at(world, wx, y, wz))
}

/// Rotate an angle toward a target by at most `max_step` radians, taking the
/// short way round the circle (used to swing the avatar's facing to its heading).
fn turn_toward(cur: f32, target: f32, max_step: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (target - cur).rem_euclid(TAU);
    if d > PI {
        d -= TAU;
    }
    cur + d.clamp(-max_step, max_step)
}

/// Third-person controller (Roblox style): the mouse orbits the camera on a boom
/// behind + above the avatar; WASD moves the avatar relative to where the camera
/// looks; the avatar turns to face its heading. Grounded walk reuses `move_body`
/// (gravity + AABB voxel collision + step-up) exactly; EDIT mode is a free noclip
/// fly. The camera follows every frame, pulling in against walls via `camera_boom`.
fn fly_camera(
    time: Res<Time>,
    cfg: Res<Cfg>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut player_q: Query<(&mut Transform, &mut FlyCam), Without<OrbitCam>>,
    mut cam_q: Query<(&mut Transform, &mut OrbitCam)>,
    world: Option<Res<World>>,
) {
    let Ok((mut ptf, mut fly)) = player_q.single_mut() else {
        return;
    };
    let Ok((mut ctf, mut orbit)) = cam_q.single_mut() else {
        return;
    };
    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };

    if mouse_btn.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    // F switches between grounded walk (gravity + collision) and free noclip fly.
    if keys.just_pressed(KeyCode::KeyF) {
        fly.walking = !fly.walking;
        fly.vel = Vec3::ZERO;
    }

    // Mouse orbits the camera (yaw around, pitch clamped so it never rolls over).
    if cursor.grab_mode == CursorGrabMode::Locked {
        let mut delta = Vec2::ZERO;
        for ev in motion.read() {
            delta += ev.delta;
        }
        orbit.yaw -= delta.x * 0.0025;
        orbit.pitch = (orbit.pitch - delta.y * 0.0025).clamp(PITCH_MIN, PITCH_MAX);
    } else {
        motion.clear();
    }

    let dt = time.delta_secs();

    // Camera orientation → the horizontal basis WASD moves along (camera-relative).
    let cam_rot =
        Quat::from_axis_angle(Vec3::Y, orbit.yaw) * Quat::from_axis_angle(Vec3::X, orbit.pitch);
    let fwd = cam_rot * Vec3::NEG_Z;
    let flat_fwd = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
    let flat_right = Vec3::new(-flat_fwd.z, 0.0, flat_fwd.x);

    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        wish += flat_fwd;
    }
    if keys.pressed(KeyCode::KeyS) {
        wish -= flat_fwd;
    }
    if keys.pressed(KeyCode::KeyD) {
        wish += flat_right;
    }
    if keys.pressed(KeyCode::KeyA) {
        wish -= flat_right;
    }
    // Headless proof: once grounded, auto-walk forward for a beat so the screenshot
    // catches the avatar mid-stride with the camera trailing behind it.
    if cfg.walk_demo && fly.walking && fly.grounded {
        let t = time.elapsed_secs();
        if (2.0..3.1).contains(&t) {
            wish += flat_fwd;
        }
    }
    let wish = wish.normalize_or_zero();

    if fly.walking {
        // ---- WALK: grounded body — reuse move_body's gravity/collision/step-up.
        if let Some(world) = world.as_deref() {
            let speed = if keys.pressed(KeyCode::ControlLeft) { 10.0 } else { 6.0 };
            let horiz = wish * speed * dt;
            fly.vel.y = (fly.vel.y - GRAVITY * dt).max(-TERMINAL);
            if fly.grounded && keys.just_pressed(KeyCode::Space) {
                fly.vel.y = JUMP_SPEED;
            }
            let delta = Vec3::new(horiz.x, fly.vel.y * dt, horiz.z);
            let can_step = fly.grounded && fly.vel.y <= 0.0;
            let (np, grounded) = move_body(world, ptf.translation, delta, can_step);
            ptf.translation = np;
            fly.grounded = grounded;
            if grounded && fly.vel.y < 0.0 {
                fly.vel.y = 0.0;
            }
        }
    } else {
        // ---- FLY: free noclip (EDIT building) — full 3D move, sprint on Ctrl.
        let mut dir = wish;
        if keys.pressed(KeyCode::Space) {
            dir += Vec3::Y;
        }
        if keys.pressed(KeyCode::ShiftLeft) {
            dir -= Vec3::Y;
        }
        let speed = if keys.pressed(KeyCode::ControlLeft) { 90.0 } else { 28.0 };
        if dir != Vec3::ZERO {
            ptf.translation += dir.normalize() * speed * dt;
        }
    }

    // Turn the avatar to face its heading (its body mesh's forward is local -Z).
    if wish != Vec3::ZERO {
        let target = (-wish.x).atan2(-wish.z);
        fly.face_yaw = turn_toward(fly.face_yaw, target, TURN_RATE * dt);
    }
    ptf.rotation = Quat::from_axis_angle(Vec3::Y, fly.face_yaw);

    // ---- Camera follow: ride the boom behind + above the avatar, pulled in when
    // a wall would come between the camera and the avatar (so it never clips).
    let pivot = ptf.translation + Vec3::Y * PIVOT_UP;
    let back = cam_rot * Vec3::Z; // pivot → camera (opposite the camera's forward)
    let dist = match world.as_deref() {
        Some(world) => camera_boom(world, pivot, back, BOOM_DIST),
        None => BOOM_DIST,
    };
    orbit.dist = dist;
    ctf.translation = pivot + back * dist;
    ctf.rotation = cam_rot;
}

// ---------------------------------------------------------------------------
// Voxel editing — raycast pick, then break (→AIR) or place (selected block).
// ---------------------------------------------------------------------------

/// Human label for a block id (HUD + demo log).
fn block_name(b: BlockId) -> &'static str {
    match b {
        BlockId::GRASS => "grass",
        BlockId::DIRT => "dirt",
        BlockId::STONE => "stone",
        BlockId::SAND => "sand",
        _ => "air",
    }
}

/// Is the world-space voxel (wx,wy,wz) solid? Only the y=0 chunk layer exists in
/// Phase 0, so anything outside 0..CHUNK vertically is empty air.
fn solid_at(world: &World, wx: i32, wy: i32, wz: i32) -> bool {
    if wy < 0 || wy >= CHUNK {
        return false;
    }
    let key = (wx.div_euclid(CHUNK), wz.div_euclid(CHUNK));
    let Some(slot) = world.chunks.get(&key) else {
        return false;
    };
    slot.data
        .get(wx.rem_euclid(CHUNK), wy, wz.rem_euclid(CHUNK))
        .is_opaque()
}

// ---------------------------------------------------------------------------
// Player body — AABB vs voxels (walk mode: gravity, jump, no clipping through).
// ---------------------------------------------------------------------------

/// The player capsule approximated as an axis-aligned box, in voxel units.
const PLAYER_HALF_W: f32 = 0.3; // half of the 0.6-wide footprint
const PLAYER_HEIGHT: f32 = 1.8; // feet → crown
const EYE_HEIGHT: f32 = 1.62; // feet → camera (0.18 head clearance)
const GRAVITY: f32 = 28.0; // voxel/s²
const JUMP_SPEED: f32 = 9.0; // ~1.4-block hop
const TERMINAL: f32 = 55.0; // fall-speed clamp
const STEP_HEIGHT: f32 = 1.0; // auto-climb a single-block ledge while walking
const STEP_CLEAR: f32 = 0.2; // extra head-room probed above the ledge before stepping

// ---- Third-person orbit camera (spring-arm / boom) ------------------------
const BOOM_DIST: f32 = 6.5; // how far the camera sits behind the avatar (max)
const BOOM_MARGIN: f32 = 0.35; // keep the camera this far off a wall it pulls up to
const PIVOT_UP: f32 = 0.35; // lift the look-pivot a touch above the eye for framing
const PITCH_MIN: f32 = -1.35; // clamp: don't roll under the avatar
const PITCH_MAX: f32 = 1.20; // clamp: don't roll over the top
const TURN_RATE: f32 = 12.0; // how fast the avatar turns to face its movement (rad/s)

/// Does the player body — camera (eye) at `eye` — overlap any solid voxel? A tiny
/// epsilon inset stops a body that merely *touches* a block face from sticking.
fn body_collides(world: &World, eye: Vec3) -> bool {
    const E: f32 = 1.0e-3;
    let min = Vec3::new(eye.x - PLAYER_HALF_W, eye.y - EYE_HEIGHT, eye.z - PLAYER_HALF_W);
    let max = Vec3::new(
        eye.x + PLAYER_HALF_W,
        eye.y - EYE_HEIGHT + PLAYER_HEIGHT,
        eye.z + PLAYER_HALF_W,
    );
    let (x0, x1) = ((min.x + E).floor() as i32, (max.x - E).floor() as i32);
    let (y0, y1) = ((min.y + E).floor() as i32, (max.y - E).floor() as i32);
    let (z0, z1) = ((min.z + E).floor() as i32, (max.z - E).floor() as i32);
    for vx in x0..=x1 {
        for vy in y0..=y1 {
            for vz in z0..=z1 {
                if solid_at(world, vx, vy, vz) {
                    return true;
                }
            }
        }
    }
    false
}

/// Lower the body until it just rests on the first solid voxel within `max`
/// below it, so a step-up lands flush on the ledge instead of hovering above it.
/// Returns the input unchanged if nothing solid is within reach (mid-air).
fn settle_down(world: &World, eye: Vec3, max: f32) -> Vec3 {
    const STEP: f32 = 0.05;
    let mut y = eye.y;
    let mut dropped = 0.0;
    while dropped < max {
        let below = Vec3::new(eye.x, y - STEP, eye.z);
        if body_collides(world, below) {
            break;
        }
        y -= STEP;
        dropped += STEP;
    }
    Vec3::new(eye.x, y, eye.z)
}

/// Try to slide the body along one horizontal axis. If the flat move is blocked
/// and `can_step` is set, attempt to auto-climb a single-block ledge: lift the
/// body by STEP_HEIGHT, move it forward, and settle it flush onto the step.
/// A wall taller than one block (or a low ceiling) leaves the body put.
fn step_axis(world: &World, p: Vec3, horiz: Vec3, can_step: bool) -> Vec3 {
    let flat = p + horiz;
    if !body_collides(world, flat) {
        return flat;
    }
    if !can_step {
        return p;
    }
    // Room to stand a block higher, both in place and after the forward move?
    let lift = STEP_HEIGHT + STEP_CLEAR;
    let up = Vec3::new(p.x, p.y + lift, p.z);
    let up_fwd = up + horiz;
    if body_collides(world, up) || body_collides(world, up_fwd) {
        return p; // ledge too tall or a ceiling in the way — stay blocked
    }
    // Only a ledge (solid within a step below) counts — never climb into open air.
    let landed = settle_down(world, up_fwd, lift);
    if landed.y >= up_fwd.y {
        return p; // nothing to rest on — that was a gap, not a step
    }
    landed
}

/// Advance the body one axis at a time, cancelling any move that would drive it
/// into a solid voxel (the classic per-axis voxel resolve — slides along walls).
/// With `can_step`, a blocked horizontal axis auto-climbs a single-block ledge so
/// walking up stairs/slopes doesn't need a jump on every step.
/// Returns the new eye position and whether it is resting on ground this step.
fn move_body(world: &World, eye: Vec3, delta: Vec3, can_step: bool) -> (Vec3, bool) {
    let mut p = eye;
    // Horizontal X then Z: a blocked axis is dropped so the other still slides.
    p = step_axis(world, p, Vec3::new(delta.x, 0.0, 0.0), can_step);
    p = step_axis(world, p, Vec3::new(0.0, 0.0, delta.z), can_step);
    // Vertical last: a blocked *downward* move means we landed (grounded).
    let ty = Vec3::new(p.x, p.y + delta.y, p.z);
    let mut grounded = false;
    if body_collides(world, ty) {
        if delta.y < 0.0 {
            grounded = true;
        }
    } else {
        p = ty;
    }
    // A step-up lands the body flush on the ledge, so report it grounded too —
    // otherwise the HUD flickers to WALK·air for a frame after every stair.
    if p.y > eye.y && body_collides(world, Vec3::new(p.x, p.y - 0.06, p.z)) {
        grounded = true;
    }
    (p, grounded)
}

/// How far back along `dir` (a unit boom vector pointing from the pivot toward the
/// camera) the camera can sit before a solid voxel would come between it and the
/// avatar. Marches out from the pivot with the same `solid_at` grid test the edit
/// raycast uses, stopping `BOOM_MARGIN` short of the first block it meets — so the
/// camera slides in against walls instead of clipping through them.
fn camera_boom(world: &World, pivot: Vec3, dir: Vec3, want: f32) -> f32 {
    const STEP: f32 = 0.1;
    let mut d = 0.0;
    while d < want {
        let p = pivot + dir * (d + BOOM_MARGIN);
        if solid_at(world, p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32) {
            return d;
        }
        d += STEP;
    }
    want
}

/// One raycast hit: the solid voxel struck and the empty cell just before it
/// (where a placed block lands).
struct RayHit {
    voxel: IVec3,
    prev: IVec3,
}

/// Amanatides & Woo voxel DDA: walk the grid from `origin` along `dir` until a
/// solid voxel is hit or `max_dist` is exceeded.
fn raycast_voxel(world: &World, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<RayHit> {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let mut v = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );
    let step = IVec3::new(
        dir.x.signum() as i32,
        dir.y.signum() as i32,
        dir.z.signum() as i32,
    );

    // Distance (t) to the first grid boundary on each axis, and the t-span of one
    // whole voxel per axis. INFINITY when the ray is flat on that axis.
    let next_boundary = |p: f32, s: i32| -> f32 {
        if s > 0 {
            p.floor() + 1.0
        } else {
            p.floor()
        }
    };
    let mut t_max = Vec3::new(
        if dir.x != 0.0 {
            (next_boundary(origin.x, step.x) - origin.x) / dir.x
        } else {
            f32::INFINITY
        },
        if dir.y != 0.0 {
            (next_boundary(origin.y, step.y) - origin.y) / dir.y
        } else {
            f32::INFINITY
        },
        if dir.z != 0.0 {
            (next_boundary(origin.z, step.z) - origin.z) / dir.z
        } else {
            f32::INFINITY
        },
    );
    let t_delta = Vec3::new(
        if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::INFINITY },
        if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::INFINITY },
        if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::INFINITY },
    );

    let mut prev = v;
    let mut t;
    // Bound the step count so a ray into open sky terminates.
    for _ in 0..(max_dist as i32 * 2 + 8) {
        if solid_at(world, v.x, v.y, v.z) {
            return Some(RayHit { voxel: v, prev });
        }
        prev = v;
        if t_max.x <= t_max.y && t_max.x <= t_max.z {
            v.x += step.x;
            t = t_max.x;
            t_max.x += t_delta.x;
        } else if t_max.y <= t_max.z {
            v.y += step.y;
            t = t_max.y;
            t_max.y += t_delta.y;
        } else {
            v.z += step.z;
            t = t_max.z;
            t_max.z += t_delta.z;
        }
        if t > max_dist {
            return None;
        }
    }
    None
}

/// Write a block at a world voxel and re-mesh only the chunk that owns it.
/// Returns true if a chunk was actually touched.
fn set_world_voxel(
    world: &mut World,
    voxel: IVec3,
    block: BlockId,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
) -> bool {
    if voxel.y < 0 || voxel.y >= CHUNK {
        return false;
    }
    let key = (voxel.x.div_euclid(CHUNK), voxel.z.div_euclid(CHUNK));
    let Some(slot) = world.chunks.get_mut(&key) else {
        return false;
    };
    slot.data.set(
        voxel.x.rem_euclid(CHUNK),
        voxel.y,
        voxel.z.rem_euclid(CHUNK),
        block,
    );
    let (mesh, quads) = greedy_mesh_chunk(&slot.data);
    let delta = quads as isize - slot.quads as isize;
    slot.quads = quads;
    let entity = slot.entity;
    world.total_quads = (world.total_quads as isize + delta).max(0) as usize;
    commands.entity(entity).insert(Mesh3d(meshes.add(mesh)));
    true
}

/// Draw a wireframe box around the voxel the camera is aimed at, so breaking and
/// placing have a clear target (the same raycast the edits fire from). Drawn every
/// frame with gizmos — no entity churn — and only when the ray actually hits.
fn highlight_target(
    cam: Query<&Transform, With<OrbitCam>>,
    world: Res<World>,
    editor: Res<Editor>,
    mut gizmos: Gizmos,
) {
    let Ok(tf) = cam.single() else {
        return;
    };
    let Some(hit) = raycast_voxel(&world, tf.translation, *tf.forward(), editor.reach) else {
        return;
    };
    // Voxel (v) spans [v, v+1]; centre it and inflate a hair so the outline sits
    // just outside the block faces (no z-fighting with the mesh).
    let centre = hit.voxel.as_vec3() + Vec3::splat(0.5);
    gizmos.cube(
        Transform::from_translation(centre).with_scale(Vec3::splat(1.006)),
        Color::srgb(0.02, 0.02, 0.02),
    );
}

/// Player edits: pick blocks with 1-4, break with left-click, place with right.
/// Only active while the cursor is captured (in "play" mode).
fn edit_voxels(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursors: Query<&CursorOptions, With<PrimaryWindow>>,
    cam: Query<&Transform, With<OrbitCam>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    mut editor: ResMut<Editor>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        editor.selected = BlockId::GRASS;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        editor.selected = BlockId::DIRT;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        editor.selected = BlockId::STONE;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        editor.selected = BlockId::SAND;
    }

    // Edits only fire once the cursor is captured, so the click that enters play
    // mode (and this system runs before fly_camera grabs it) is never an edit.
    let grabbed = cursors
        .single()
        .map(|c| c.grab_mode == CursorGrabMode::Locked)
        .unwrap_or(false);
    if !grabbed {
        return;
    }

    let break_it = mouse.just_pressed(MouseButton::Left);
    let place_it = mouse.just_pressed(MouseButton::Right);
    if !break_it && !place_it {
        return;
    }
    let Ok(tf) = cam.single() else {
        return;
    };
    let Some(hit) = raycast_voxel(&world, tf.translation, *tf.forward(), editor.reach) else {
        return;
    };
    if break_it {
        set_world_voxel(&mut world, hit.voxel, BlockId::AIR, &mut meshes, &mut commands);
    } else {
        let block = editor.selected;
        set_world_voxel(&mut world, hit.prev, block, &mut meshes, &mut commands);
    }
}

/// Editor shell: Tab flips PLAY↔EDIT, G runs the two-corner box-fill (EDIT mode),
/// F5 quick-saves the world to `maps/quicksave.json` and F9 loads it back. These sit
/// alongside the per-click break/place in `edit_voxels` — the fill just paints a
/// whole box in one shot instead of one voxel per click.
fn editor_controls(
    keys: Res<ButtonInput<KeyCode>>,
    cursors: Query<&CursorOptions, With<PrimaryWindow>>,
    cam: Query<&Transform, With<OrbitCam>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    mut editor: ResMut<Editor>,
    mut fly_q: Query<&mut FlyCam>,
) {
    // Tab: toggle mode. Entering EDIT forces free-fly so you move through the build.
    if keys.just_pressed(KeyCode::Tab) {
        editor.mode = match editor.mode {
            EditorMode::Play => EditorMode::Edit,
            EditorMode::Edit => EditorMode::Play,
        };
        editor.fill_anchor = None;
        if editor.mode == EditorMode::Edit {
            if let Ok(mut fly) = fly_q.single_mut() {
                fly.walking = false;
                fly.vel = Vec3::ZERO;
            }
        }
        editor.status = match editor.mode {
            EditorMode::Edit => "EDIT mode — G=fill, L/R=paint, F5=save F9=load".into(),
            EditorMode::Play => "PLAY mode".into(),
        };
    }

    let grabbed = cursors
        .single()
        .map(|c| c.grab_mode == CursorGrabMode::Locked)
        .unwrap_or(false);

    // G: box-fill (EDIT mode). First press marks a corner, second fills the box with
    // the selected block (AIR erases). Aim at a face; build lands in front of it.
    if editor.mode == EditorMode::Edit && grabbed && keys.just_pressed(KeyCode::KeyG) {
        if let Ok(tf) = cam.single() {
            if let Some(hit) = raycast_voxel(&world, tf.translation, *tf.forward(), editor.reach) {
                let target = if editor.selected == BlockId::AIR { hit.voxel } else { hit.prev };
                match editor.fill_anchor.take() {
                    None => {
                        editor.fill_anchor = Some(target);
                        editor.status =
                            format!("fill corner ({},{},{}) — G again", target.x, target.y, target.z);
                    }
                    Some(a) => {
                        let sel = editor.selected;
                        let n = box_fill(&mut commands, &mut meshes, &mut world, a, target, sel);
                        editor.status = format!("filled {n} × {}", block_name(sel));
                    }
                }
            }
        }
    }

    // F5: quick-save the live world to disk.
    if keys.just_pressed(KeyCode::F5) {
        let path = editor.map_path.clone();
        match save_world_to(&world, &path) {
            Ok(n) => {
                println!("MAP_SAVE ok blocks={n} path={path}");
                editor.status = format!("saved {n} blocks → {path}");
            }
            Err(e) => {
                eprintln!("MAP_SAVE FAIL {e}");
                editor.status = format!("save FAILED: {e}");
            }
        }
    }

    // F9: reload the quick-save slot — despawn the world and rebuild from the file.
    if keys.just_pressed(KeyCode::F9) {
        let path = editor.map_path.clone();
        match load_map_file(&path) {
            Ok(map) => {
                for slot in world.chunks.values() {
                    commands.entity(slot.entity).despawn();
                }
                world.chunks.clear();
                world.total_quads = 0;
                let nx = map.size.chunks_x.max(1);
                let nz = map.size.chunks_z.max(1);
                for z in 0..nz {
                    for x in 0..nx {
                        spawn_empty_chunk(&mut commands, &mut meshes, &mut world, x, z);
                    }
                }
                apply_map_blocks(&mut commands, &mut meshes, &mut world, &map);
                println!("MAP_LOAD(F9) ok blocks={} path={path}", map.blocks.len());
                editor.status = format!("loaded {} blocks ← {path}", map.blocks.len());
            }
            Err(e) => {
                eprintln!("MAP_LOAD FAIL {e}");
                editor.status = format!("load FAILED: {e}");
            }
        }
    }
}

/// Headless proof that the editor authors a real, reloadable map: build a small
/// recognisable scene into the blank world, then save it to `cfg.map_save`.
fn map_save_demo(
    time: Res<Time>,
    cfg: Res<Cfg>,
    mut demo: ResMut<MapSaveDemo>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
) {
    let Some(path) = cfg.map_save.clone() else {
        return;
    };
    if demo.done || time.elapsed_secs() < 1.0 {
        return;
    }
    demo.done = true;

    build_demo_scene(&mut commands, &mut meshes, &mut world);

    match save_world_to(&world, &path) {
        Ok(n) => println!(
            "MAP_SAVE ok blocks={n} path={path} total_quads={}",
            world.total_quads
        ),
        Err(e) => eprintln!("MAP_SAVE FAIL {e}"),
    }
}

/// A small, unmistakable build for the save demo (easy to eyeball in a screenshot):
/// a grass floor, a stone tower, a climbable dirt staircase, and a sand marker — so
/// all four block types land in the saved file.
fn build_demo_scene(commands: &mut Commands, meshes: &mut Assets<Mesh>, world: &mut World) {
    // Grass floor across the whole 32×32 chunk (single layer at y=0).
    box_fill(commands, meshes, world, IVec3::new(0, 0, 0), IVec3::new(31, 0, 31), BlockId::GRASS);
    // A 3×3 stone tower, 6 tall, off in one corner.
    box_fill(commands, meshes, world, IVec3::new(20, 1, 20), IVec3::new(22, 6, 22), BlockId::STONE);
    // A 5-step dirt staircase climbing in +X (each step one block taller) — proves
    // the saved map is walkable/steppable once reloaded.
    for i in 0..5 {
        let x = 6 + i;
        box_fill(commands, meshes, world, IVec3::new(x, 1, 14), IVec3::new(x, 1 + i, 18), BlockId::DIRT);
    }
    // A sand cube marker.
    box_fill(commands, meshes, world, IVec3::new(12, 1, 24), IVec3::new(13, 2, 25), BlockId::SAND);
}

/// Scripted proof of the edit loop for headless runs: one real forward raycast
/// (logged), then a broken-out crater and a placed stone tower — both routed
/// through the same set_world_voxel path a mouse click uses.
fn edit_demo(
    time: Res<Time>,
    cfg: Res<Cfg>,
    mut demo: ResMut<EditDemo>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    cam: Query<&Transform, With<OrbitCam>>,
) {
    if !cfg.edit_demo || demo.done || time.elapsed_secs() < 1.2 {
        return;
    }
    demo.done = true;

    // Prove the raycast finds terrain from the camera's real vantage.
    if let Ok(tf) = cam.single() {
        match raycast_voxel(&world, tf.translation, *tf.forward(), 400.0) {
            Some(hit) => println!(
                "RAYCAST hit voxel=({},{},{}) place_at=({},{},{})",
                hit.voxel.x, hit.voxel.y, hit.voxel.z, hit.prev.x, hit.prev.y, hit.prev.z
            ),
            None => println!("RAYCAST miss"),
        }
    }

    // Aim at the centre of the initial grid — where the camera is looking.
    let side = world.chunks.keys().map(|(x, _)| *x).max().unwrap_or(0) + 1;
    let cx = side * CHUNK / 2;
    let cz = side * CHUNK / 2;

    // BREAK: carve a bowl crater centred on the real surface height, digging a
    // hemisphere down into the terrain (only removing cells that are solid).
    let mut breaks = 0usize;
    let r = 9i32;
    for dz in -r..=r {
        for dx in -r..=r {
            let surf = terrain_height((cx + dx) as f32, (cz + dz) as f32);
            for dy in 0..=r {
                if (dx * dx + dz * dz + dy * dy) <= r * r {
                    let v = IVec3::new(cx + dx, surf - dy, cz + dz);
                    if solid_at(&world, v.x, v.y, v.z)
                        && set_world_voxel(&mut world, v, BlockId::AIR, &mut meshes, &mut commands)
                    {
                        breaks += 1;
                    }
                }
            }
        }
    }

    // PLACE: raise a 2×2 stone tower beside the crater, anchored to the surface
    // so the "build" is unmistakable against the sky.
    let mut places = 0usize;
    let (tx, tz) = (cx + r + 6, cz);
    let base = terrain_height(tx as f32, tz as f32);
    for dy in 0..16 {
        for dz in 0..2 {
            for dx in 0..2 {
                let v = IVec3::new(tx + dx, base + dy, tz + dz);
                if set_world_voxel(&mut world, v, BlockId::STONE, &mut meshes, &mut commands) {
                    places += 1;
                }
            }
        }
    }

    println!(
        "EDIT_DEMO breaks={breaks} places={places} total_quads={}",
        world.total_quads
    );
}

/// Headless proof of walk physics: the player is dropped from a height over the
/// grid centre; the first frame it rests on terrain, log the settle height vs the
/// real surface and exercise the collision predicate at known buried/sky points.
fn walk_demo(
    cfg: Res<Cfg>,
    mut demo: ResMut<WalkDemo>,
    world: Option<Res<World>>,
    cam: Query<(&Transform, &FlyCam)>,
) {
    if !cfg.walk_demo || demo.done {
        return;
    }
    let Some(world) = world.as_deref() else {
        return;
    };
    let Ok((tf, fly)) = cam.single() else {
        return;
    };
    if !fly.grounded {
        return;
    }
    demo.done = true;

    let eye = tf.translation;
    let feet_y = (eye.y - EYE_HEIGHT).round() as i32;
    // On a loaded map the surface is whatever the file put down (scan the column);
    // on procedural terrain it is the generator's height.
    let surf = if cfg.map_load.is_some() {
        highest_solid(world, eye.x.floor() as i32, eye.z.floor() as i32).unwrap_or(-1)
    } else {
        terrain_height(eye.x, eye.z)
    };
    // Feet should rest one voxel above the top solid block (surf) => surf + 1.
    let expect = surf + 1;
    let land_ok = (feet_y - expect).abs() <= 1;

    // The collision predicate under test: solid at the ground, empty in the sky.
    // Terrain has depth below the surface, so probe 2 blocks under it; a loaded map
    // may be a single-layer floor, so instead prove the surface block itself is solid.
    let ground_probe = if cfg.map_load.is_some() {
        surf as f32
    } else {
        surf as f32 - 2.0
    };
    let buried = body_collides(world, Vec3::new(eye.x, ground_probe + EYE_HEIGHT, eye.z));
    let sky = body_collides(world, Vec3::new(eye.x, surf as f32 + 40.0 + EYE_HEIGHT, eye.z));

    println!(
        "WALK_DEMO grounded feet_y={feet_y} surface={surf} expect_feet={expect} \
         land={} buried_solid={buried} sky_empty={} => {}",
        if land_ok { "PASS" } else { "FAIL" },
        !sky,
        if land_ok && buried && !sky { "PASS" } else { "FAIL" }
    );

    // Camera-boom collision: the spring-arm must pull in toward a wall (solid ground
    // below) yet extend fully into open air (empty sky above) — the same solid_at
    // grid test the aim raycast uses, so a wall can never come between cam & avatar.
    let boom_down = camera_boom(world, eye, Vec3::NEG_Y, BOOM_DIST);
    let boom_up = camera_boom(world, eye, Vec3::Y, BOOM_DIST);
    println!(
        "CAM_BOOM into_ground={boom_down:.2} into_sky={boom_up:.2} => {}",
        if boom_down < BOOM_DIST - 0.5 && boom_up >= BOOM_DIST { "PASS" } else { "FAIL" }
    );

    // The step-up proof below scans procedural terrain for a 1-block ledge; on a
    // loaded map there is no generator height to scan. Instead prove the map's *walls*
    // stop the body: find the nearest solid wall east of the player and drive move_body
    // straight into it — a working collision resolve keeps the body on the near side.
    if cfg.map_load.is_some() {
        let feet = surf + 1; // stand one voxel above the floor
        let (px, pz) = (eye.x.floor() as i32, eye.z.floor() as i32);
        let wall_x = (1..CHUNK).map(|dx| px + dx).find(|&wx| solid_at(world, wx, feet, pz));
        if let Some(wx) = wall_x {
            // Push +X for many steps with stepping OFF (the wall is taller than a step).
            let mut e = Vec3::new(eye.x, feet as f32 + EYE_HEIGHT, eye.z);
            for _ in 0..80 {
                let (np, _) = move_body(world, e, Vec3::new(0.3, 0.0, 0.0), false);
                e = np;
            }
            let front = e.x + PLAYER_HALF_W; // leading face of the body
            let blocked = front <= wx as f32; // never entered the wall voxel [wx, wx+1)
            println!(
                "WALK_WALL wall_x={wx} start_x={:.2} stopped_x={:.2} front={:.2} => {}",
                eye.x,
                e.x,
                front,
                if blocked { "PASS" } else { "FAIL(clipped through)" }
            );
        } else {
            println!("WALK_WALL no_wall_east => SKIP");
        }
        return;
    }

    // ---- Step-up proof: drive move_body across a real 1-block ledge and a
    // 2-block wall on the generated terrain, asserting it climbs the former and
    // refuses the latter. Runs against the same collision path the player uses.
    let side = world.chunks.keys().map(|(x, _)| *x).max().unwrap_or(0) + 1;
    let span = side * CHUNK;
    let mut ledge1: Option<(i32, i32)> = None; // (x,z) where surf(x+1) == surf(x)+1
    'scan: for z in 2..(span - 2) {
        for x in 2..(span - 2) {
            let h0 = terrain_height(x as f32, z as f32);
            let h1 = terrain_height((x + 1) as f32, z as f32);
            // Keep the whole body + a step of head-room inside the y=0 chunk.
            if h1 + 3 >= CHUNK {
                continue;
            }
            if h1 == h0 + 1 {
                ledge1 = Some((x, z));
                break 'scan;
            }
        }
    }

    // Place the body flush on column x's top, just shy of the +X boundary, then
    // push +X. With can_step it should climb the 1-block ledge; without it (the
    // airborne / no-assist path) the very same ledge must stop the body dead —
    // proving auto-step is deliberate and gated, not a free wall-climb.
    let probe = |x: i32, z: i32, can_step: bool| -> i32 {
        let feet0 = terrain_height(x as f32, z as f32) + 1;
        let eye = Vec3::new(x as f32 + 0.5, feet0 as f32 + EYE_HEIGHT, z as f32 + 0.5);
        let (np, _) = move_body(&world, eye, Vec3::new(0.6, 0.0, 0.0), can_step);
        (np.y - EYE_HEIGHT).round() as i32 - feet0
    };

    if let Some((x, z)) = ledge1 {
        let climb = probe(x, z, true);
        let blocked = probe(x, z, false);
        println!(
            "STEP_DEMO ledge x={x} z={z} assisted_climb={climb} gated_climb={blocked} => {}",
            if climb == 1 && blocked == 0 { "PASS" } else { "FAIL" }
        );
    } else {
        println!("STEP_DEMO ledge NONE-FOUND => SKIP");
    }
}

fn hud(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    world: Option<Res<World>>,
    bench: Res<Bench>,
    editor: Res<Editor>,
    fly: Query<&FlyCam>,
    mut q: Query<&mut Text, With<HudText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let chunks = world.as_ref().map(|w| w.chunks.len()).unwrap_or(0);
    let quads = world.as_ref().map(|w| w.total_quads).unwrap_or(0);
    let line = if bench.active {
        format!(
            "FPS {fps:.0}  |  chunks {chunks}  |  quads {quads}  |  BENCH ramp  |  best>=60fps: {} chunks",
            bench.max_60
        )
    } else {
        let movemode = fly
            .single()
            .map(|f| {
                if f.walking {
                    if f.grounded { "WALK" } else { "WALK·air" }
                } else {
                    "FLY"
                }
            })
            .unwrap_or("FLY");
        let editmode = match editor.mode {
            EditorMode::Edit => "EDIT",
            EditorMode::Play => "PLAY",
        };
        let status = if editor.status.is_empty() {
            String::new()
        } else {
            format!("  |  {}", editor.status)
        };
        format!(
            "FPS {fps:.0}  |  chunks {chunks}  |  quads {quads}  |  [{editmode}/{movemode}] Tab=mode F=fly/walk  |  L=break R=place[{}] 1-4=pick G=fill F5=save F9=load{status}",
            block_name(editor.selected)
        )
    };
    if let Ok(mut text) = q.single_mut() {
        text.0 = line.clone();
    }
    set_dom("fps", &format!("{fps:.0}"));
    set_dom("status", &line);
}

fn bench_ramp(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    mut bench: ResMut<Bench>,
    mut exit: MessageWriter<AppExit>,
) {
    if !bench.active || bench.finished {
        return;
    }
    let now = time.elapsed_secs();
    if now < WARMUP {
        bench.phase_start = now;
        return;
    }
    bench.samples.push(time.delta_secs() * 1000.0);

    if now - bench.phase_start < PHASE {
        return;
    }

    let mut ms = std::mem::take(&mut bench.samples);
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_ms = if ms.is_empty() { 0.0 } else { ms[ms.len() / 2] };
    let fps = if median_ms > 0.0 { 1000.0 / median_ms } else { 0.0 };
    let chunks = (bench.side * bench.side) as usize;
    let line = format!(
        "BENCH chunks={chunks} side={} fps={fps:.1} median_ms={median_ms:.2} quads={}",
        bench.side, world.total_quads
    );
    println!("{line}");
    bench.log.push_str(&line);
    bench.log.push('\n');
    let full_log = bench.log.clone();
    set_dom("bench", &full_log);

    if fps >= 60.0 {
        bench.max_60 = chunks;
    }

    let dipped = fps < 55.0;
    if bench.side as usize >= MAX_SIDE as usize || dipped {
        bench.finished = true;
        let summary = format!(
            "BENCH_RESULT max_60fps_chunks={} last_chunks={} last_fps={fps:.1}",
            bench.max_60, chunks
        );
        println!("{summary}");
        set_dom("result", &summary);
        #[cfg(not(target_arch = "wasm32"))]
        {
            exit.write(AppExit::Success);
        }
        return;
    }

    let new_side = bench.side + 1;
    for z in 0..new_side {
        for x in 0..new_side {
            spawn_chunk(&mut commands, &mut meshes, &mut world, x, z);
        }
    }
    bench.side = new_side;
    bench.phase_start = now;
    let _ = &mut exit;
}

fn screenshot_once(
    time: Res<Time>,
    mut commands: Commands,
    mut bench: ResMut<Bench>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = bench.shot.clone() else {
        return;
    };
    if bench.active {
        return;
    }
    let now = time.elapsed_secs();
    if !bench.took_shot && now > 3.2 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        bench.took_shot = true;
        println!("SHOT saved to {path}");
    }
    if bench.took_shot && now > 4.4 {
        exit.write(AppExit::Success);
    }
}
