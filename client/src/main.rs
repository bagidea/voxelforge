//! Voxelforge client — Phase 0 go/no-go spike (Rust + Bevy 0.19 + wgpu).
//!
//! Proves: chunk gen → greedy mesh (split per block type) → per-type
//! StandardMaterial with a repeating tile → fly
//! camera, native only (wgpu/DX12/Vulkan) — Steam is the target. Ships a built-in
//! ramp benchmark that spawns more chunks every couple seconds and reports the
//! largest chunk count that still holds >= 60 FPS.
//!
//! Chunk data and world-gen live in `voxelforge_sim` (shared with the server).
//! The greedy mesher and atlas live in `voxel` (Bevy-coupled, client-only).

mod anim;
mod audio;
mod beach_shot;
mod block_atlas;
mod block_shapes;
mod characters;
mod combat;
mod cutscene;
mod dialogue_ui;
mod dodge_parry;
mod editor;
mod editor_camera;
mod editor_config;
mod editor_ui;
mod enemies;
mod equipment;
mod foliage;
mod gizmo;
mod hero;
mod hud;
mod import;
mod inventory;
mod look;
mod main_menu;
mod mapfile;
mod quest;
mod save_game;
mod quest_chaos;
mod quest_rules;
mod scene;
mod settings_menu;
mod streaming;
mod vfx;
mod vfx_bridge;
mod voxel;
mod water;
// NOTE: no top-level `mod input_map;` — input_map.rs is already pulled in as a
// submodule of `editor_config` (`#[path="input_map.rs"] pub mod input_map;`).
// Declaring it here too would compile the file twice into two distinct type sets.

use bevy::camera::Exposure;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::ecs::system::ParamSet;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::text::FontSize;
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, Window};
use std::collections::HashMap;

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, ChunkPos, CHUNK_SIZE as CHUNK};
use voxelforge_sim::worldgen::{self, terrain_block, terrain_height};
use voxel::{
    atlas_material, build_atlas, build_block_materials, greedy_mesh_chunk, greedy_mesh_chunk_split,
};

use editor::AppState;

// ---------------------------------------------------------------------------
// Config (env vars on native, ?query params on web)
// ---------------------------------------------------------------------------

#[derive(Resource, Clone)]
pub(crate) struct Cfg {
    bench: bool,
    grid: usize,
    pub(crate) shot: Option<String>,
    /// `--play`: skip the editor entirely and boot straight into the playable scene
    /// (`scene::ScenePlugin` — map/campfire/spawn) in [`AppState::Play`].
    pub(crate) play: bool,
    /// `--play-demo`: the scripted headless proof of that scene (walk + respawn).
    /// Implies `--play`.
    pub(crate) play_demo: bool,
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
    /// Scripted combat demo (headless proof of attack→hit→stamina→dodge i-frames).
    combat_demo: bool,
    /// Scripted quest demo (headless proof of accept→complete→reward→next quest opens).
    pub(crate) quest_demo: bool,
    /// `menu` — the game entrance. True on a plain launch (no lane flags): boots to
    /// [`AppState::MainMenu`] with the play scene pre-loaded behind it. False for
    /// every scripted/bench/shot/play lane, which still lands in Editor/Play as before.
    pub(crate) menu: bool,
    /// `VOXELFORGE_SAVE_DEMO=save|continue` — the scripted end-to-end proof of the
    /// save/load loop (menu → New Game → walk → save → exit, or menu → Continue).
    pub(crate) save_demo: Option<String>,
    /// `VOXELFORGE_MENUSHOT=before|after` — headless menu capture. `before` shoots
    /// `docs/assets/menu/menu-a-before.png` (no save yet → Continue dimmed), `after`
    /// shoots `menu-b-after.png` (save present → Continue lit + slot data). Both
    /// boot to [`AppState::MainMenu`] and exit right after the frame lands.
    pub(crate) menushot: Option<String>,
    /// `--beauty-tour`: a ~30s scripted cinematic of the playable scene for the
    /// CEO to watch — outdoor noon → cool raking light → night firelight — then
    /// auto-exit. Implies `--play` and forces a HUD-free frame.
    pub(crate) beauty_tour: bool,
    /// `--beauty-tour-shots <dir>`: capture a still PNG at each tour stop into
    /// this directory (`beauty-tour-stop-{1,2,3}.png`).
    pub(crate) beauty_tour_shots: Option<String>,
    /// `--strict-exit`: exit code ≠0 when any gate FAILs. Without this flag the
    /// process always exits 0 even when a gate prints FAIL — the caller grades the
    /// log line itself. With it, the exit code IS the verdict.
    pub(crate) strict_exit: bool,
    /// Scripted editor proof (headless proof that a click places/breaks a voxel
    /// through the same `paint_at_cursor` path the interactive editor uses).
    editor_demo: bool,
    /// Load a saved map file at startup (world = file contents, no procedural terrain).
    pub(crate) map_load: Option<String>,
    /// Author-a-tiny-map demo: start blank, build a scene, save it here, then shoot.
    map_save: Option<String>,
    /// `VOXELFORGE_BEACHSHOT=1` — the beach-dusk hero shot rig (`beach_shot.rs`):
    /// hides the avatar, poses the real gameplay camera at a fixed hero-shot
    /// transform, and spawns water/campfire/boat/flower-pot props over a loaded
    /// `maps/beach_dusk.json`. See `docs/hero-scene-beach-dusk.md` §5.1.
    pub(crate) beachshot: bool,
    /// `VOXELFORGE_FPS_BENCH=<secs>` — headless steady-state frame-time sampler of
    /// the loaded play scene (the real map, not the bench grid). Warms up, samples
    /// `secs` of frame times, prints one `FPS_BENCH` line (median/mean/p95 ms, fps,
    /// chunk + quad counts) and exits. Gate for the map-density pass so a foliage
    /// change is priced in FPS, not just in draw-call count.
    pub(crate) fps_bench: Option<f32>,
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
    // ── Hero LOOK knobs ──────────────────────────────────────────────────────
    // These used to be read with `std::env::var` directly inside hero.rs; they
    // travel through Cfg now like every other knob, so one recipe drives the
    // whole locked look.
    wide: bool,            // WIDE establishing framing (VOXELFORGE_WIDE)
    fg_apron: bool,        // foreground counter apron (VOXELFORGE_FGAPRON / ?fgapron)
    dust: Option<f32>,     // dust-mote density scale (0 = off)
    bluescale: Option<f32>, // sun/ambient blue-leg scale
    bounce: Option<f32>,   // GI card 1 (floor bounce) intensity scale
    bounce2: Option<f32>,  // GI card 2 (dark lifter) intensity scale
    shoulder: Option<f32>, // WIDE highlight roll-off gain (1.0 = no shoulder)
    ambcolor: Option<[f32; 3]>, // warm-bounce fill colour r,g,b
}

/// Parse "a,b,c" env into a fixed float array (all-or-nothing).
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

/// True when the binary was launched with `flag` on the command line. The play
/// switches are real CLI flags (`cargo run --bin voxelforge -- --play`) rather than
/// env vars, because "start the game" is something a person types, not a recipe a
/// script exports — the `VOXELFORGE_*` twins below keep the scripted lanes working.
fn has_arg(flag: &str) -> bool {
    std::env::args().skip(1).any(|a| a == flag)
}

/// The value that follows `flag` on the command line, for flags that take an
/// argument (`--beauty-tour-shots <dir>`), if present.
fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

fn read_cfg() -> Cfg {
    // --play-demo is the scripted proof *of* --play, so it turns --play on too.
    let play_demo = has_arg("--play-demo") || std::env::var("VOXELFORGE_PLAY_DEMO").is_ok();
    // --combat-demo is the combat-loop proof (hit→kill→die→respawn). It also
    // turns --play on — needs the full scene (campsite, player, husk encounter).
    let combat_demo = has_arg("--combat-demo") || std::env::var("VOXELFORGE_COMBAT_DEMO").is_ok();
    // --quest-demo is the quest-loop proof (accept→complete→reward→next quest).
    // Also turns --play on — needs the full scene + NPCs + husk.
    let quest_demo = has_arg("--quest-demo") || std::env::var("VOXELFORGE_QUEST_DEMO").is_ok();
    // --beauty-tour is the CEO-facing cinematic tour (noon → cool → night, then
    // exit). Also turns --play on — needs the full scene + look stack + campfire.
    let beauty_tour = has_arg("--beauty-tour") || std::env::var("VOXELFORGE_BEAUTY_TOUR").is_ok();
    let beauty_tour_shots = arg_value("--beauty-tour-shots")
        .or_else(|| std::env::var("VOXELFORGE_BEAUTY_TOUR_SHOTS").ok())
        .filter(|s| !s.is_empty());

    // Hoisted so `menu` below can ask "did any OTHER lane fire?".
    let save_demo = std::env::var("VOXELFORGE_SAVE_DEMO").ok().filter(|s| !s.is_empty());
    let menushot = std::env::var("VOXELFORGE_MENUSHOT").ok().filter(|s| !s.is_empty());
    let bench = std::env::var("VOXELFORGE_BENCH").is_ok();
    let hero = std::env::var("VOXELFORGE_HERO").is_ok();
    let edit_demo = std::env::var("VOXELFORGE_EDIT_DEMO").is_ok();
    let walk_demo = std::env::var("VOXELFORGE_WALK_DEMO").is_ok();
    let editor_demo = std::env::var("VOXELFORGE_EDITOR_DEMO").is_ok();
    let shot = std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty());
    let map_load = std::env::var("VOXELFORGE_MAP_LOAD").ok().filter(|s| !s.is_empty());
    let map_save = std::env::var("VOXELFORGE_MAP_SAVE").ok().filter(|s| !s.is_empty());
    let beachshot = std::env::var("VOXELFORGE_BEACHSHOT").is_ok();
    let fps_bench = std::env::var("VOXELFORGE_FPS_BENCH")
        .ok()
        .and_then(|v| v.parse().ok());
    let play = play_demo
        || combat_demo
        || quest_demo
        || beauty_tour
        || has_arg("--play")
        || std::env::var("VOXELFORGE_PLAY").is_ok();

    // The game entrance = a plain launch (no other lane). `save_demo` and `menushot`
    // are themselves menu-mode lanes, so they force `menu` on regardless of the rest.
    let menu = save_demo.is_some()
        || menushot.is_some()
        || !(play
            || bench
            || hero
            || edit_demo
            || walk_demo
            || editor_demo
            || shot.is_some()
            || map_load.is_some()
            || map_save.is_some()
            || beachshot);

    Cfg {
        bench,
        grid: std::env::var("VOXELFORGE_GRID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6),
        shot,
        play,
        play_demo,
        hero,
        present: std::env::var("VOXELFORGE_PRESENT").ok().filter(|s| !s.is_empty()),
        start_side: std::env::var("VOXELFORGE_START_SIDE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1)
            .max(1),
        edit_demo,
        walk_demo,
        combat_demo,
        quest_demo,
        strict_exit: has_arg("--strict-exit") || std::env::var("VOXELFORGE_STRICT_EXIT").is_ok(),
        editor_demo,
        map_load,
        map_save,
        beachshot,
        fps_bench,
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
        wide: std::env::var("VOXELFORGE_WIDE").is_ok(),
        fg_apron: std::env::var("VOXELFORGE_FGAPRON").is_ok(),
        dust: std::env::var("VOXELFORGE_DUST").ok().and_then(|v| v.parse().ok()),
        bluescale: std::env::var("VOXELFORGE_BLUESCALE").ok().and_then(|v| v.parse().ok()),
        bounce: std::env::var("VOXELFORGE_BOUNCE").ok().and_then(|v| v.parse().ok()),
        bounce2: std::env::var("VOXELFORGE_BOUNCE2").ok().and_then(|v| v.parse().ok()),
        shoulder: std::env::var("VOXELFORGE_SHOULDER").ok().and_then(|v| v.parse().ok()),
        ambcolor: env_floats("VOXELFORGE_AMBCOLOR"),
        menu,
        save_demo,
        menushot,
        beauty_tour,
        beauty_tour_shots,
    }
}

// ---------------------------------------------------------------------------

/// One live chunk: its editable voxel data, the entity that *parents* its mesh
/// children, and the quad count it currently contributes (kept in sync so the
/// HUD total is correct after edits re-mesh a chunk).
///
/// `entity` carries no mesh of its own — see [`remesh_chunk_entity`].
pub(crate) struct ChunkSlot {
    pub(crate) data: ChunkData,
    pub(crate) entity: Entity,
    pub(crate) quads: usize,
}

#[derive(Resource)]
pub(crate) struct World {
    /// The one shared atlas material. Only the far LOD still wears it — see
    /// `streaming::build_lod_children`.
    pub(crate) material: Handle<StandardMaterial>,
    /// One material per (block type, face), indexed by `voxel::material_index`.
    /// This is what the split mesher's children wear, and it is why a merged
    /// quad now repeats its tile per block instead of stretching one atlas cell
    /// across it — and why a grass block has a top that is not its side.
    pub(crate) block_materials: Vec<Handle<StandardMaterial>>,
    /// Keyed by (chunk_x, chunk_z) — only the y=0 layer is spawned in Phase 0.
    pub(crate) chunks: HashMap<(i32, i32), ChunkSlot>,
    pub(crate) total_quads: usize,
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

/// Guards the first combat encounter so the Guard Husk + HUD spawn exactly once
/// per Play session (and are despawned again in `despawn_encounter` when the
/// editor sandbox is restored).
#[derive(Resource)]
struct Encounter {
    spawned: bool,
}

/// Fires the scripted "editor click places/breaks voxels" proof once (headless).
#[derive(Resource)]
struct EditorPaintDemo {
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
pub(crate) struct FlyCam {
    /// The direction the avatar is currently turned to face (smoothed toward the
    /// movement direction each frame). Drives the body mesh's rotation.
    pub(crate) face_yaw: f32,
    /// Vertical (+residual) velocity used by walk mode's gravity/jump; unused in fly.
    pub(crate) vel: Vec3,
    /// true = grounded walking body (gravity + AABB voxel collision); false =
    /// free noclip fly (EDIT mode building). Toggled live with F.
    pub(crate) walking: bool,
    /// Set the frame the body rests on a solid voxel below — gates the jump.
    pub(crate) grounded: bool,
}

/// The orbit camera — rides a spring-arm/boom behind + above the avatar (Roblox
/// style). Mouse drives `yaw`/`pitch`; `dist` is the boom length actually in use
/// this frame, pulled in from `want_dist` by `camera_boom` when a wall would
/// otherwise clip between camera and avatar.
#[derive(Component)]
pub(crate) struct OrbitCam {
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) dist: f32,
    /// The boom length the camera is *trying* to hold — [`BOOM_DIST`] in play.
    ///
    /// Separate from `dist` because `dist` is overwritten every frame with the
    /// collision-shortened result, so it can't also be the request: a value
    /// written into it is gone by the next frame. The look lane sets this from
    /// `VOXELFORGE_LOOK_CAM` to frame a proof shot (a long boom for a vista, a
    /// short one for a character close-up) without a second camera path.
    pub(crate) want_dist: f32,
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

/// Headless steady-state FPS sampler for the loaded play scene (the *real* map,
/// not the ramp's synthetic grid). Gated by `VOXELFORGE_FPS_BENCH=<secs>`: warms
/// up past `FPS_WARMUP`, collects `sample_secs` of frame times, prints one line
/// (median/mean/p95 ms, fps, chunk + quad counts) and exits. The quad count is
/// the draw-call proxy, so a foliage/instance change is priced two ways.
#[derive(Resource)]
struct FpsBench {
    sample_secs: f32,
    started: bool,
    start: f32,
    samples: Vec<f32>,
}

const WARMUP: f32 = 1.2;
const PHASE: f32 = 2.0;
const MAX_SIDE: i32 = 32; // up to 1024 chunks (ramp stops early once FPS dips <55)
/// FPS-sampler warmup matches `screenshot_once`'s 3.2s shot time, so the sampled
/// world is the same fully-settled scene the still-shot grades.
const FPS_WARMUP: f32 = 3.2;

fn main() -> AppExit {
    let mut cfg = read_cfg();

    // `--play` boots the game, so the scene — not the caller — decides which world
    // that is: Shiba's hand-built village once `maps/edhari.json` lands, procedural
    // terrain until then. An explicit VOXELFORGE_MAP_LOAD still wins.
    //
    // Menu mode (a plain launch, or the scripted save demo) pre-boots that same
    // playable scene BEHIND the main menu, so New Game / Continue drop straight in
    // instead of loading — the menu is an overlay on an already-ready world. `play`
    // is set true so `scene::ScenePlugin` and `quest::QuestPlugin` (both gated on
    // `cfg.play`) come alive; `boot_state` still routes the *state* to MainMenu.
    if (cfg.play || cfg.menu) && cfg.map_load.is_none() {
        cfg.map_load = scene::play_map();
    }
    if cfg.menu {
        cfg.play = true;
    }
    let cfg = cfg;
    // Captured before `cfg` is moved into `insert_resource` below; the beauty-tour
    // resource is built after that move and still needs the shots dir.
    let beauty_tour_shots = cfg.beauty_tour_shots.clone();

    // The beauty tour is a capture: hide every screen-space widget + gizmo via
    // scene.rs's existing `VOXELFORGE_NOHUD` sweep. Set BEFORE any plugin reads
    // it — `nohud_requested` caches the value in a OnceLock on first read, which
    // happens while `ScenePlugin` builds. Also pre-create the stills directory,
    // because `save_to_disk` (the image crate) does not mkdir -p.
    if cfg.beauty_tour {
        std::env::set_var("VOXELFORGE_NOHUD", "1");
        if let Some(dir) = &cfg.beauty_tour_shots {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("BEAUTY_TOUR warning: cannot create shots dir {dir}: {e}");
            }
        }
    }

    // Pin the asset root to the executable's directory so Bevy always finds the
    // assets build.rs copies into `target/<profile>/assets/`. Without this,
    // Bevy's `get_base_path()` resolves via `CARGO_MANIFEST_DIR` during
    // `cargo run`, redirecting to `client/assets/` which doesn't exist (the
    // assets are next to the exe, not inside the source tree). An absolute path
    // replaces `get_base_path()` entirely via Rust's Path::join behaviour.
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    let asset_path = exe_dir.join("assets");

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
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin { file_path: asset_path.to_string_lossy().to_string(), ..default() }),
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
    })
    .insert_resource(FpsBench {
        sample_secs: cfg.fps_bench.unwrap_or(0.0),
        started: false,
        start: 0.0,
        samples: Vec::new(),
    });

    if cfg.hero {
        // A 4K directional shadow map gives the PCSS penumbra enough texels to
        // stay smooth instead of stair-stepping (default is 2048).
        app.insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
            .insert_resource(cfg)
            .add_systems(Startup, hero::setup_hero)
            .add_systems(Update, (fly_camera, screenshot_once));
    } else {
        // ---- Editor shell ---------------------------------------------------
        // EditorPlugin owns AppState{Editor, Play} + the SelectedBlock resource +
        // the interactive build loop (raycast break/place in Editor). Round-2
        // assembly plugs the team's editor crates that have LANDED ON DISK in
        // beside it. Wired now (files present): editor_ui (Yamamoto), import
        // (Kevin), editor_config/InputConfigPlugin (Sun), and `scene` — the
        // playable world `--play` boots into (map load → spawn + campfire →
        // death/respawn loop). ScenePlugin is added unconditionally; every system
        // in it gates on `cfg.play`, so the editor/bench/shot lanes are untouched.
        // Interactive editor camera + transform gizmo are OUT of scope this round
        // (Director handed them to another owner) — no camera sub-plugin is wired
        // yet. In an interactive Editor session `fly_camera` is still gated off (see
        // `Scripted` + `not_interactive_editor` below), so the next owner just adds
        // their camera plugin against the Selection/SnapGrid contract in editor.rs.
        let scripted = cfg.bench
            || cfg.shot.is_some()
            || cfg.edit_demo
            || cfg.walk_demo
            || cfg.combat_demo
            || cfg.editor_demo
            || cfg.map_save.is_some()
            || cfg.save_demo.is_some()
            || cfg.play_demo || cfg.quest_demo
            // A --play session is a human at the controls, so it is NOT scripted —
            // but it never sits in AppState::Editor either, so the editor camera
            // still keeps its hands off (see `in_interactive_editor`).
            || (cfg.map_load.is_some() && !cfg.play);
        app.add_plugins((
            editor::EditorPlugin,
            editor_ui::EditorUiPlugin,
            import::ImportPlugin,
            editor_config::InputConfigPlugin,
            scene::ScenePlugin,
            // Procedural character animation (Poppy's lane). Builds a box-humanoid
            // rig as a CHILD of the avatar + each Husk and drives it from combat's
            // own timers. Gated on `cfg.play` like ScenePlugin, so bench/hero/editor
            // screenshots keep the capsule they were graded against.
            anim::AnimPlugin,
            cutscene::CutscenePlugin,
            audio::AudioPlugin,
            // Quest & dialogue engine (Poppy's lane). Gated to AppState::Play.
            quest::QuestPlugin,
            dialogue_ui::DialogueUiPlugin,
            // Editor camera (orbit/pan/zoom on the middle button) + transform
            // gizmo. Both gate on `in_interactive_editor` so scripted/headless
            // runs keep using the play-mode `fly_camera` for their screenshots.
            editor_camera::EditorCameraPlugin,
            gizmo::GizmoPlugin,
        ))
            .add_plugins((
                // Combat/ambient VFX (Flamingo's lane). `VfxPlugin` is inert until
                // something writes a message or carries a marker component, so it
                // costs the editor/bench/hero lanes nothing. `VfxBridgePlugin` is the
                // only thing that knows about combat: it reads the SfxEvent stream
                // combat already broadcasts and turns hits/deaths into Impact/Unravel,
                // so no other lane's file had to change. See vfx_bridge.rs.
                vfx::VfxPlugin,
                vfx_bridge::VfxBridgePlugin,
                // Combat "weight" layer (combat.rs) — hit-stop, knockback, camera
                // kick, Impact/Stagger/Dodge messages. Self-wiring; this line is all.
                combat::CombatFeelPlugin,
                // Combat "depth" layer (dodge_parry.rs) — frame-counted dodge
                // i-frames and the parry → poise-break → riposte chain, riding the
                // weight layer's ImpactWeight table. Self-wiring; this line is all.
                dodge_parry::DodgeParryPlugin,
            ))
            .add_plugins(look::LookPlugin)
            // Water surface shader (water.rs + assets/shaders/water.wgsl): moving
            // wave normals, Fresnel sky reflection, depth-graded colour. Swaps
            // only the `BlockId::WATER` child mesh's material and touches nothing
            // else; `VOXELFORGE_WATER=off` renders the old StandardMaterial water
            // out of this same binary, which is the A/B baseline.
            .add_plugins(water::WaterPlugin)
            // Foliage wind shader (foliage.rs + assets/shaders/foliage_wind.wgsl): the
            // seven vegetation sprites render as cross-quad billboards whose vertices
            // sway in a wind field + gust + per-plant phase. Registers the
            // FoliageMaterial pipeline + wind clock; the scatter lives in scene.rs.
            // `VOXELFORGE_FOLIAGE=off` skips the scatter out of this same binary.
            .add_plugins(foliage::FoliagePlugin)
            // The game entrance (main menu + real save/load). MainMenuPlugin is the
            // AppState::MainMenu overlay + action routing; SaveGamePlugin is the F6
            // quick-save. Both gate on their own run conditions so the editor / bench
            // / shot / play lanes are untouched.
            .add_plugins(main_menu::MainMenuPlugin)
            .add_plugins(save_game::SaveGamePlugin)
            .add_plugins(settings_menu::SettingsPlugin)
            .add_plugins(streaming::StreamingPlugin)
            // Combat HUD (Monanisa's lane, docs/hud-design.md Option A). Reads
            // combat::{Health,Stamina,LockOn} and quest::ObjectiveText only —
            // self-wiring, doesn't touch either file.
            .add_plugins(hud::HudPlugin)
            // Item bag + hotbar (inventory.rs): B dig / N place / H use / I bag /
            // 1-9 hotbar. Play-gated, and it frees the cursor while the bag is open
            // (fly_camera is gated off below so the two don't fight).
            .add_plugins(inventory::InventoryPlugin)
            .insert_resource(editor::Scripted(scripted))
            .insert_resource(cfg)
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
            .insert_resource(EditorPaintDemo { done: false })
            .insert_resource(Encounter { spawned: false })
            .insert_resource(hero::GateVerdict::default())
            // Combat layer (docs/combat-design.md §8) — all client-side gameplay.
            .insert_resource(combat::CombatIntent::default())
            .insert_resource(combat::LockOn::default())
            .insert_resource(combat::Shake::default())
            .insert_resource(combat::CombatDemo::default())
            // `player_combat` takes a `MessageWriter<PlayerDied>`, so the message
            // type MUST be registered or the very first Play frame panics on the
            // missing `Messages<PlayerDied>` resource. `scene::on_player_death` is
            // the reader — the death→campfire seam Kevin left for this side.
            .add_message::<combat::PlayerDied>()
            .add_systems(Startup, (setup, boot_state))
            // Beach-dusk hero shot rig (Yamamoto, `VOXELFORGE_BEACHSHOT=1`, see
            // beach_shot.rs). Runs after `setup` so the FlyCam/OrbitCam it dresses
            // already exist; no-ops when the env flag is unset. `pose_beach_camera`
            // runs after `fly_camera` for the same reason `combat::lock_on_camera`
            // does — it has to win the final word over the per-frame orbit recompute.
            .add_systems(Startup, beach_shot::setup_beach_shot.after(setup))
            .add_systems(Update, beach_shot::pose_beach_camera.after(fly_camera))
            // Sandbox↔Play encounter lifecycle: the default Editor state spawns no
            // husk; the Guard Husk + HUD come in on entering Play (once) and are
            // torn down on returning to the editor.
            .add_systems(OnEnter(AppState::Play), spawn_encounter)
            .add_systems(OnEnter(AppState::Editor), despawn_encounter)
            // edit_voxels runs before fly_camera so the click that grabs the
            // cursor is not also read as a break; edits happen from click #2 on.
            // In Editor state the equivalent build loop lives in EditorPlugin
            // (reads SelectedBlock); this play-mode path stays gated to Play so the
            // two never double-fire on one click.
            .add_systems(
                Update,
                (
                    // fly_camera also carries walk-physics, which the headless
                    // walk/edit/map-save proofs need while sitting in the default
                    // Editor state — so it runs everywhere EXCEPT an interactive
                    // editor session, where the editor camera owns the view.
                    // edit_voxels (L-click break / R-click place) is gated OFF
                    // during Play — the player fights, not builds. EditorPlugin
                    // owns the build loop in Editor state. The interactive
                    // systems below additionally gate off in MainMenu, so the
                    // entrance shows a still world behind the menu (the player
                    // body and camera are there, but nothing walks or draws HUD).
                    fly_camera
                        .run_if(editor::not_interactive_editor)
                        .run_if(settings_menu::settings_closed)
                        .run_if(main_menu::main_menu_closed)
                        // Bag open ⇒ cursor freed ⇒ stop walking/looking so the
                        // mouse drives the panel, not the camera.
                        .run_if(inventory::inventory_closed),
                    editor_controls.run_if(main_menu::main_menu_closed),
                    highlight_target.run_if(main_menu::main_menu_closed),
                    edit_demo,
                    walk_demo,
                    map_save_demo,
                    editor_paint_demo,
                    egui_save_load.run_if(main_menu::main_menu_closed),
                    hud.run_if(main_menu::main_menu_closed),
                    bench_ramp,
                    screenshot_once,
                    fps_bench_sampler,
                ),
            )
            // Combat systems. gather_input → player_combat → husk AI run before the
            // camera; lock-on + screen-shake run AFTER fly_camera so they override
            // the final camera transform. combat_demo scripts the headless proof.
            // The whole layer is gated to AppState::Play — the Editor builds the
            // scene with the fight held off; Enter drops you into Play and it wakes.
            .add_systems(
                Update,
                (
                    (
                        combat::gather_input,
                        combat::combat_demo.run_if(combat_demo_env_only),
                        combat::player_combat,
                        combat::husk_ai,
                        combat::husk_telegraph,
                        combat::hud_bars,
                    )
                        .chain()
                        .before(fly_camera)
                        .run_if(in_state(AppState::Play)),
                    combat::lock_on_camera
                        .after(fly_camera)
                        .run_if(in_state(AppState::Play)),
                    combat::apply_shake
                        .after(combat::lock_on_camera)
                        .run_if(in_state(AppState::Play)),
                ),
            )
            // Beauty tour — a scripted cinematic over the playable scene. Runs in
            // PostUpdate (after fly_camera / lock-on / shake, before propagation)
            // so its pose is the one that renders, exactly like scene.rs's `Cine`.
            .insert_resource(BeautyTour {
                start: None,
                shots_dir: beauty_tour_shots,
                shots_taken: 0,
                last_phase: -1,
                fill_base: Vec::new(),
            })
            .add_systems(
                PostUpdate,
                beauty_tour
                    .run_if(beauty_tour_run)
                    .before(TransformSystems::Propagate),
            );
    }

    app.add_systems(Last, check_gate_on_exit);
    app.run()
}

/// If `--strict-exit` is active and any gate printed FAIL, inject a non-zero
/// `AppExit` message so `fn main() -> AppExit` produces exit code ≠0.
/// Runs in `Last` so gate systems in `Update` have already set `GATE_FAILED`
/// by the time we look.
fn check_gate_on_exit(
    cfg: Res<Cfg>,
    mut exit: MessageWriter<AppExit>,
) {
    if cfg.strict_exit && hero::GATE_FAILED.load(std::sync::atomic::Ordering::Acquire) {
        exit.write(AppExit::error());
    }
}

/// Dead code: `combat_demo` now always forces `play=true` (the `Cfg::play`
/// boot derives from `combat_demo`), so this `combat_demo && !play` condition
/// is never true and `combat::combat_demo` is never reached. The path that
/// actually runs is `scene::combat_proof`, which scripts the heavy attack.
fn combat_demo_env_only(cfg: Res<Cfg>) -> bool {
    cfg.combat_demo && !cfg.play
}

/// Routes the boot state. Three doors, in priority order:
///
/// * **MainMenu** — `Cfg::menu` (a plain launch, or the save demo). The play scene is
///   already pre-loaded behind it (`main()` set `play`+`map_load`), so the menu is an
///   overlay on a ready world and New Game / Continue drop straight in.
/// * **Play** — combat/quest demos or `--play`. The combat systems are gated to
///   `AppState::Play`, so the headless combat proof must run *in Play*; `--play` takes
///   the same door as a product decision ("player wakes up directly in the world",
///   `docs/first-playable-loop.md` Act 0) — no menu, no Enter press through the editor.
/// * **Editor** — everything else: bench, shot, edit/walk/map-save/editor demos, and
///   an explicit `VOXELFORGE_MAP_LOAD` without `--play`. These previously *relied on*
///   `Editor` being the `#[default]` state; it no longer is (the product default is
///   MainMenu), so they are set explicitly here to keep every existing lane unchanged.
fn boot_state(cfg: Res<Cfg>, mut next: ResMut<NextState<AppState>>) {
    if cfg.menu {
        next.set(AppState::MainMenu);
    } else if cfg.combat_demo || cfg.quest_demo || cfg.play {
        next.set(AppState::Play);
    } else {
        next.set(AppState::Editor);
    }
}

/// First entry into Play spawns the Guard Husk + combat HUD once (and not on a
/// loaded map, which describes its own scene). `combat_demo` boots straight to
/// Play, so it reaches this hook and gets its husk too — the headless combat
/// proof is unchanged.
fn spawn_encounter(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<Cfg>,
    mut enc: ResMut<Encounter>,
    player: Query<&Transform, With<FlyCam>>,
) {
    if enc.spawned || cfg.map_load.is_some() {
        return;
    }
    let Ok(tf) = player.single() else {
        return;
    };
    enc.spawned = true;
    let dist = if cfg.combat_demo { 2.2 } else { 5.0 };
    // The tank: the Bone Sentinel, dead ahead (the first-playable guard).
    combat::spawn_guard_husk(
        &mut commands,
        &mut meshes,
        &mut materials,
        tf.translation.x,
        tf.translation.z - dist,
        None,
    );
    // Second enemy type: a Ghoul Reaver flanking to the player's right and a
    // little deeper — lighter and faster than the Sentinel, drops one potion.
    combat::spawn_husk_of_kind(
        &mut commands,
        &mut meshes,
        &mut materials,
        crate::enemies::EnemyKind::Reaver,
        tf.translation.x + 3.0,
        tf.translation.z - dist - 2.0,
        None,
    );
    hud::spawn_hud(&mut commands);
    println!("SPAWN_ENCOUNTER sentinel+reaver {dist} blocks in front of the player");
}

/// Returning to the editor tears down the encounter (husk + HUD bars + lock
/// reticle) and re-arms the spawn so the next Play session is a fresh fight. The
/// default Editor state fires this at boot with nothing to clear — a clean sandbox.
fn despawn_encounter(
    mut commands: Commands,
    mut enc: ResMut<Encounter>,
    enemies: Query<Entity, With<combat::Enemy>>,
    hbars: Query<Entity, With<combat::HealthBar>>,
    sbars: Query<Entity, With<combat::StaminaBar>>,
    reticles: Query<Entity, With<combat::LockReticle>>,
    hud_roots: Query<Entity, With<hud::HudRoot>>,
) {
    let mut n = 0;
    for e in enemies
        .iter()
        .chain(hbars.iter())
        .chain(sbars.iter())
        .chain(reticles.iter())
        .chain(hud_roots.iter())
    {
        commands.entity(e).despawn();
        n += 1;
    }
    enc.spawned = false;
    if n > 0 {
        println!("DESPAWN_ENCOUNTER cleared {n} combat entities (sandbox restored)");
    }
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

    // Two texture paths, on purpose. The atlas is now only the FAR LOD's
    // material (`atlas_material` instead of the old hand-rolled 0.95/0.1 chalk,
    // so the far ring is lit closer to the near ring and the LOD line is less of
    // a seam). Near chunks are meshed per block type and wear `block_materials`.
    let atlas = images.add(build_atlas());
    let material = materials.add(atlas_material(atlas));
    let block_materials = build_block_materials(&mut images, &mut materials);

    let mut world = World {
        material,
        block_materials,
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
    } else if bench.active {
        // Bench ramp manages its own grid — spawn the initial side exactly as before.
        side = bench.side;
        for z in 0..side {
            for x in 0..side {
                spawn_chunk(&mut commands, &mut meshes, &mut world, x, z);
            }
        }
    } else {
        // Streaming: seed a small ring around the origin so the player has ground
        // underfoot; the streaming plugin loads/unloads the rest as they move.
        let r = 3i32;
        for z in -r..=r {
            for x in -r..=r {
                spawn_chunk(&mut commands, &mut meshes, &mut world, x, z);
            }
        }
        side = 1; // Player spawns near origin; streaming fills the world outward.
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
    } else if loaded_map.is_some() {
        // Loaded map: hover a beat above its own floor (no procedural surface to stand on).
        (Vec3::new(c, ground as f32 + EYE_HEIGHT + 1.0, c), 0.0, false)
    } else {
        // Default: stand the avatar flush on Kevin's real generated surface. Pick a
        // column near the grid centre that is clear of trees/boulders so the body
        // spawns in open air, feet on the top face of the surface voxel (h+1), and
        // start it grounded-WALKing so gravity keeps it planted — never floating.
        let (sx, sz, h) = find_spawn(c.floor() as i32, c.floor() as i32);
        (
            Vec3::new(sx as f32 + 0.5, (h + 1) as f32 + EYE_HEIGHT, sz as f32 + 0.5),
            0.0,
            true,
        )
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
            // Combat kit (§8.1–3, §3): state machine, stamina, HP, poise.
            combat::player_bundle(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(body_mat),
                Transform::from_xyz(0.0, body_dy, 0.0),
                combat::PlayerBody,
            ));
            parent.spawn((
                Mesh3d(face_mesh),
                MeshMaterial3d(face_mat),
                Transform::from_xyz(0.0, body_dy + 0.55, -0.34),
                combat::PlayerBody,
            ));
        });

    // ---- Gameplay camera + the look stack (Flamingo's block) -----------------
    //
    // `VOXELFORGE_LOOK_CAM=yaw_deg,pitch_deg,dist` re-poses the boom at spawn.
    // Unset ⇒ exactly the pose this camera has always had. It exists so a look
    // proof can be shot from a chosen angle (vista / close-up / interior) through
    // the REAL gameplay camera rather than through a second, look-only camera
    // that would prove nothing about what the player sees.
    let (orbit_yaw, orbit_pitch, boom) = match env_floats::<3>("VOXELFORGE_LOOK_CAM") {
        Some([y, p, d]) => (y.to_radians(), p.to_radians(), d),
        None => (orbit_yaw, orbit_pitch, BOOM_DIST),
    };
    let cam = commands
        .spawn((
            Camera3d::default(),
            Transform::from_translation(eye + Vec3::new(0.0, PIVOT_UP, boom)),
            OrbitCam {
                yaw: orbit_yaw,
                pitch: orbit_pitch,
                dist: boom,
                want_dist: boom,
            },
            // Warm bounce fill. `look::apply_look_to_cameras` re-colours and
            // re-powers this for the live hour; these are the neutral values the
            // editor/bench lanes (where the look lane is off) keep.
            AmbientLight {
                brightness: 380.0,
                ..default()
            },
        ))
        .id();
    // THE LOOK. This camera used to spawn bare — a `Camera3d::default()` with no
    // tonemapper, no exposure, no grade, no bloom, no AO, no haze — and the whole
    // post stack arrived (or didn't) from `LookPlugin` one Update later. Dressing
    // it here means the camera-spawn site states what the camera looks like, and
    // the very first frames of a session are already graded.
    //
    // `look::base_camera_look()` is the tier-INDEPENDENT half (tonemap, grade,
    // exposure, emissive-only bloom, distance haze); `LookPlugin` layers TAA /
    // SSAO / shadow filter / god rays on top per `LookQuality` and owns the F7
    // runtime swap. Both come out of the same functions in look.rs, so this is
    // not a second copy of the look — see that module's header.
    //
    // Gated on `look::enabled_for` (the same predicate the plugin's systems use)
    // so the bench, the editor and every other lane's screenshot proof keep
    // rendering the frame they were graded against. See docs/look-contract.md.
    if look::enabled_for(&cfg) {
        commands.entity(cam).insert(look::base_camera_look());
    }

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

    // Objective tracker — top-right corner.
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::from(14.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 0.95, 0.80, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            right: Val::Px(12.0),
            ..default()
        },
        quest::ObjectiveTracker,
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

    // The first combat encounter (§4.1) is NO LONGER spawned here — the default
    // AppState::Editor is now a husk-free sandbox. `spawn_encounter` brings the
    // Guard Husk + combat HUD in on entering Play (so combat_demo, which boots to
    // Play, still gets it), and `despawn_encounter` clears them on returning to
    // the editor. See the OnEnter hooks registered in `main`.

    commands.insert_resource(world);
    bench.phase_start = 0.0;
}

/// Rebuild a chunk entity's drawable children — one child mesh per block type,
/// each wearing that type's own tile texture (sampled `Repeat`) and its own
/// surface response. Returns the chunk's total quad count.
///
/// The chunk entity itself carries no mesh; it is the transform/visibility
/// parent. Unloading, frustum culling and the editor still address a chunk as
/// ONE entity, while the draw calls underneath it are one per material — which
/// is the whole point. On the atlas path a greedy-merged 12×3 quad samples a
/// single atlas tile stretched across twelve blocks; here the UVs are measured
/// in blocks, so the tile repeats 12×3 times at real texel density.
///
/// `VOXELFORGE_ATLAS_MESH=1` pins near chunks back on the old single-atlas mesh.
/// It exists so the fix can be photographed against itself: same binary, same
/// seed, same map, same camera, same frame — the ONLY difference is the mesher.
/// Read once, because this sits inside every chunk re-mesh.
pub(crate) fn remesh_chunk_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &World,
    entity: Entity,
    chunk: &ChunkData,
) -> usize {
    // Queued before the spawns below, and command queues are FIFO, so the old
    // children are gone before the new ones land.
    commands.entity(entity).despawn_children();

    if atlas_mesh_forced() {
        let (mesh, quads) = greedy_mesh_chunk(chunk);
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(world.material.clone()),
            ChildOf(entity),
        ));
        return quads;
    }

    let mut quads = 0usize;
    for (key, mesh, n) in greedy_mesh_chunk_split(chunk) {
        // Shaped blocks (stair/slab/fence/pane/cross) are not cubes: the greedy
        // path would paint them as 6 full faces, so skip them here and emit
        // their real geometry in the shaped loop below.
        if key.block.is_shaped() {
            continue;
        }
        // A block id past the table can only come from a corrupt map file. The
        // atlas path renders it as the clamped edge tile rather than a hole, so
        // do the same here — a wrong texture beats missing geometry. The
        // fallback keeps the FACE, so a bad top still gets a top material.
        let material = world
            .block_materials
            .get(key.material_index())
            .unwrap_or(&world.block_materials[voxel::material_index(BlockId::STONE, key.face)]);
        quads += n;
        let child = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                ChildOf(entity),
            ))
            .id();
        // A shadow map is binary, so a mostly-clear pane would cast a solid
        // black shadow. See `voxel::casts_shadow`. `bevy::light`, not
        // `bevy::pbr` — the lighting components moved out of the PBR crate.
        if !voxel::casts_shadow(key.block) {
            commands.entity(child).insert(bevy::light::NotShadowCaster);
        }
        // Tag the liquid so `water::swap_water_material` can trade this child's
        // StandardMaterial for the wave/Fresnel/depth shader. Marked here rather
        // than matched on a material handle downstream: this is the only place
        // that still knows which BlockId the child was built from, and chunk
        // streaming respawns these children constantly.
        if key.block == BlockId::WATER {
            commands.entity(child).insert(water::WaterSurface);
        }
    }

    // The shaped palette — stair, slab, fence, pane, cross. Real per-block
    // geometry with chunk-local positions (the chunk entity carries the world
    // translation, the same convention the cube children above use).
    for (block, face, mesh) in block_shapes::emit_chunk_shapes(chunk, &|v| get_world_voxel(world, v)) {
        let material = world
            .block_materials
            .get(voxel::material_index(block, face))
            .unwrap_or(&world.block_materials[voxel::material_index(BlockId::STONE, face)]);
        let child = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                ChildOf(entity),
            ))
            .id();
        if !voxel::casts_shadow(block) {
            commands.entity(child).insert(bevy::light::NotShadowCaster);
        }
    }

    // Lamps emit real light, not just a glowing face — one warm shadowless
    // point light per LAMP block at the block centre. Chunk-local position;
    // the chunk entity carries the world translation (see `spawn_chunk_parent`).
    // Shadowless on purpose: a shadow map is binary, and a dozen shadow-casting
    // lamps would swamp the shadow pass for a soft indoor glow.
    for y in 0..CHUNK {
        for z in 0..CHUNK {
            for x in 0..CHUNK {
                if chunk.get(x, y, z) == BlockId::LAMP {
                    commands.spawn((
                        PointLight {
                            color: Color::srgb(1.0, 0.55, 0.25),
                            intensity: 40_000.0,
                            range: 9.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5),
                        ChildOf(entity),
                    ));
                }
            }
        }
    }

    quads
}

/// The A/B lever documented on [`remesh_chunk_entity`]. `OnceLock` because the
/// answer cannot change mid-run and every chunk edit would otherwise pay for an
/// env lookup.
fn atlas_mesh_forced() -> bool {
    static FORCED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FORCED.get_or_init(|| {
        let on = std::env::var("VOXELFORGE_ATLAS_MESH").is_ok_and(|v| v != "0");
        if on {
            println!("MESH_PATH atlas (VOXELFORGE_ATLAS_MESH) — pre-fix reference render");
        } else {
            println!("MESH_PATH split (one mesh + material per block type)");
        }
        on
    })
}

/// Spawn the parent entity every chunk hangs its per-type meshes off.
fn spawn_chunk_parent(commands: &mut Commands, x: i32, z: i32) -> Entity {
    commands
        .spawn((
            Transform::from_xyz((x * CHUNK) as f32, 0.0, (z * CHUNK) as f32),
            // Explicit because this entity has no `Mesh3d` to imply it, and the
            // children only inherit visibility through a parent that has it.
            Visibility::default(),
        ))
        .id()
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
    let entity = spawn_chunk_parent(commands, x, z);
    let quads = remesh_chunk_entity(commands, meshes, world, entity, &chunk);
    world.total_quads += quads;
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
    let entity = spawn_chunk_parent(commands, x, z);
    let quads = remesh_chunk_entity(commands, meshes, world, entity, &chunk);
    world.total_quads += quads;
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
    let Some(slot) = world.chunks.get(&key) else {
        return;
    };
    let entity = slot.entity;
    let was = slot.quads;
    // `world` and `&slot.data` are both shared reborrows of the same `&mut`, so
    // they coexist; the mutable updates come after they expire.
    let quads = remesh_chunk_entity(commands, meshes, world, entity, &slot.data);

    let delta = quads as isize - was as isize;
    if let Some(slot) = world.chunks.get_mut(&key) {
        slot.quads = quads;
    }
    world.total_quads = (world.total_quads as isize + delta).max(0) as usize;
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
                    // is_solid, not is_opaque: a saved map has to keep the glass.
                    if b.is_solid() {
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
/// the number of solid blocks saved.
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

/// Read + parse a map file.
fn load_map_file(path: &str) -> Result<mapfile::MapFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    mapfile::parse(&text)
}

/// Despawn every live chunk and rebuild the world from a map file — the shared
/// body of the F9 quick-load and the egui Load button. Returns the block count
/// the file listed.
fn reload_world(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    path: &str,
) -> Result<usize, String> {
    let map = load_map_file(path)?;
    for slot in world.chunks.values() {
        commands.entity(slot.entity).despawn();
    }
    world.chunks.clear();
    world.total_quads = 0;
    let nx = map.size.chunks_x.max(1);
    let nz = map.size.chunks_z.max(1);
    for z in 0..nz {
        for x in 0..nx {
            spawn_empty_chunk(commands, meshes, world, x, z);
        }
    }
    apply_map_blocks(commands, meshes, world, &map);
    Ok(map.blocks.len())
}

/// Fill an inclusive box of voxels with one block, re-meshing each touched chunk
/// once. The brush/fill tool and the scripted map-author demo both build through it.
/// Returns the number of voxels actually written.
pub(crate) fn box_fill(
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
pub(crate) fn highest_solid(world: &World, wx: i32, wz: i32) -> Option<i32> {
    (0..CHUNK).rev().find(|&y| solid_at(world, wx, y, wz))
}

/// Pick a spawn column near (cx,cz) whose surface is clear of features (trees,
/// boulders) so the avatar stands flush on real terrain instead of inside a trunk.
/// Spirals outward ring by ring (centre first) and returns (world_x, world_z,
/// surface_height); falls back to the centre column if nothing clear is found nearby.
/// Queries the shared worldgen directly (deterministic once the seed is set), so it
/// needs no `World` — the same height/feature source the chunks were meshed from.
pub(crate) fn find_spawn(cx: i32, cz: i32) -> (i32, i32, i32) {
    for r in 0i32..24 {
        for dz in -r..=r {
            for dx in -r..=r {
                if r > 0 && dx.abs() != r && dz.abs() != r {
                    continue; // only the outer ring at radius r (inner rings already tried)
                }
                let (wx, wz) = (cx + dx, cz + dz);
                let h = terrain_height(wx as f32, wz as f32);
                if h + 3 >= CHUNK {
                    continue; // keep the whole body + head-room inside the y=0 chunk
                }
                // The two voxels above the surface (feet + head) must both be empty air.
                let clear = (1..=2)
                    .all(|dy| !terrain_block(wx as f32, wz as f32, h + dy, h).is_solid());
                if clear {
                    return (wx, wz, h);
                }
            }
        }
    }
    (cx, cz, terrain_height(cx as f32, cz as f32))
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
pub(crate) fn fly_camera(
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
    let want = orbit.want_dist;
    let mut dist = match world.as_deref() {
        Some(world) => camera_boom(world, pivot, back, want),
        None => want,
    };
    // Post-collision safety: `camera_boom` only sweeps along the boom axis, so a
    // block flush against the lens from the side (avatar past a wall, boom swung
    // out) is invisible to it and can fill a third of the frame with one face.
    // Walk back toward the pivot until the lens centre is clear in all six
    // cardinal directions (A5).
    if let Some(world) = world.as_deref() {
        let clear = camera_lens_clear(world, pivot, back, dist);
        if clear < dist {
            dist = clear; // hard snap on side-collision — never trail into a block
        }
    }
    // Asymmetric spring-arm smoothing. `camera_boom` marches the lens along the
    // boom in fixed 0.1 steps against the voxel grid, so as the avatar drifts the
    // raw distance flickers between adjacent grid steps and the camera jitters.
    // Snap shut the instant a wall closes — the lens must never trail a collision
    // and clip back through it — but ease back *out* frame-rate-independently; the
    // slow release collapses the flicker without ever docking late. Hard snap on
    // pull-in (closing), exponential ease on release (opening).
    const BOOM_RELEASE_K: f32 = 8.0; // release time-constant: ~0.5s to near-full
    let prev = orbit.dist;
    let dist = if dist <= prev {
        dist
    } else {
        prev + (dist - prev) * (1.0 - (-BOOM_RELEASE_K * dt).exp())
    };
    orbit.dist = dist;
    ctf.translation = pivot + back * dist;
    ctf.rotation = cam_rot;
}

// ---------------------------------------------------------------------------
// Voxel editing — raycast pick, then break (→AIR) or place (selected block).
// ---------------------------------------------------------------------------

/// Human label for a block id (HUD + demo log).
///
/// Delegates to [`BlockId::name`] so the HUD always agrees with the sim
/// palette. The old local match covered only the 4 launch-day blocks and fell
/// through to `"air"` for everything else — including `LAMP`, `WOOD`,
/// `OBSIDIAN`, and the rest of the palette shipped since commit a3bb308.
fn block_name(b: BlockId) -> &'static str {
    b.name()
}

/// Is the world-space voxel (wx,wy,wz) solid? Only the y=0 chunk layer exists in
/// Phase 0, so anything outside 0..CHUNK vertically is empty air.
fn solid_at(world: &World, wx: i32, wy: i32, wz: i32) -> bool {
    solid_at_chunks(&world.chunks, wx, wy, wz)
}

/// Same as [`solid_at`] but works on a raw chunk map so tests can drive it without
/// constructing a full `World` (the material handle requires a running Bevy App).
#[inline]
fn solid_at_chunks(chunks: &HashMap<(i32, i32), ChunkSlot>, wx: i32, wy: i32, wz: i32) -> bool {
    if wy < 0 || wy >= CHUNK {
        return false;
    }
    let key = (wx.div_euclid(CHUNK), wz.div_euclid(CHUNK));
    let Some(slot) = chunks.get(&key) else {
        return false;
    };
    slot.data
        .get(wx.rem_euclid(CHUNK), wy, wz.rem_euclid(CHUNK))
        // is_solid, not is_opaque: you can see through a pane, not walk through it.
        .is_solid()
}

/// Read the block at a world voxel (AIR outside the y=0 chunk layer or in an
/// unloaded chunk). The read counterpart to [`set_world_voxel`], used by the
/// inventory dig loop to know *which* block it just picked up.
pub(crate) fn get_world_voxel(world: &World, voxel: IVec3) -> BlockId {
    if voxel.y < 0 || voxel.y >= CHUNK {
        return BlockId::AIR;
    }
    let key = (voxel.x.div_euclid(CHUNK), voxel.z.div_euclid(CHUNK));
    let Some(slot) = world.chunks.get(&key) else {
        return BlockId::AIR;
    };
    slot.data.get(
        voxel.x.rem_euclid(CHUNK),
        voxel.y,
        voxel.z.rem_euclid(CHUNK),
    )
}

// ---------------------------------------------------------------------------
// Player body — AABB vs voxels (walk mode: gravity, jump, no clipping through).
// ---------------------------------------------------------------------------

/// The player capsule approximated as an axis-aligned box, in voxel units.
pub(crate) const PLAYER_HALF_W: f32 = 0.3; // half of the 0.6-wide footprint
const PLAYER_HEIGHT: f32 = 1.8; // feet → crown
pub(crate) const EYE_HEIGHT: f32 = 1.62; // feet → camera (0.18 head clearance)
const GRAVITY: f32 = 28.0; // voxel/s²
const JUMP_SPEED: f32 = 9.0; // ~1.4-block hop
const TERMINAL: f32 = 55.0; // fall-speed clamp
const STEP_HEIGHT: f32 = 1.0; // auto-climb a single-block ledge while walking
const STEP_CLEAR: f32 = 0.2; // extra head-room probed above the ledge before stepping

// ---- Third-person orbit camera (spring-arm / boom) ------------------------
pub(crate) const BOOM_DIST: f32 = 6.5; // how far the camera sits behind the avatar (max)
const BOOM_MARGIN: f32 = 0.9; // keep the camera this far off a wall it pulls up to (was 0.35 — a wall at 0.35 fills >30% of the frame)
const BOOM_RADIUS: f32 = 0.7; // treat the lens as a disc this wide so walls beside the boom (corners, parallel faces) pull it in too — not just a wall dead on the boom axis (was 0.4 — missed blocks beside a wall-hugging camera)
pub(crate) const PIVOT_UP: f32 = 0.35; // lift the look-pivot a touch above the eye for framing
const PITCH_MIN: f32 = -1.35; // clamp: don't roll under the avatar
const PITCH_MAX: f32 = 1.20; // clamp: don't roll over the top
const TURN_RATE: f32 = 12.0; // how fast the avatar turns to face its movement (rad/s)

/// Does the player body — camera (eye) at `eye` — overlap any solid voxel? A tiny
/// epsilon inset stops a body that merely *touches* a block face from sticking.
fn body_collides(world: &World, eye: Vec3) -> bool {
    body_collides_chunks(&world.chunks, eye)
}

/// Same as [`body_collides`] but works on a raw chunk map so tests can drive it.
#[inline]
fn body_collides_chunks(chunks: &HashMap<(i32, i32), ChunkSlot>, eye: Vec3) -> bool {
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
                if solid_at_chunks(chunks, vx, vy, vz) {
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
    settle_down_chunks(&world.chunks, eye, max)
}

/// Same as [`settle_down`] but works on a raw chunk map so tests can drive it.
fn settle_down_chunks(chunks: &HashMap<(i32, i32), ChunkSlot>, eye: Vec3, max: f32) -> Vec3 {
    const STEP: f32 = 0.05;
    let mut y = eye.y;
    let mut dropped = 0.0;
    while dropped < max {
        let below = Vec3::new(eye.x, y - STEP, eye.z);
        if body_collides_chunks(chunks, below) {
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
    step_axis_chunks(&world.chunks, p, horiz, can_step)
}

/// Same as [`step_axis`] but works on a raw chunk map so tests can drive it.
fn step_axis_chunks(
    chunks: &HashMap<(i32, i32), ChunkSlot>,
    p: Vec3,
    horiz: Vec3,
    can_step: bool,
) -> Vec3 {
    let flat = p + horiz;
    if !body_collides_chunks(chunks, flat) {
        return flat;
    }
    if !can_step {
        return p;
    }
    // Room to stand a block higher, both in place and after the forward move?
    let lift = STEP_HEIGHT + STEP_CLEAR;
    let up = Vec3::new(p.x, p.y + lift, p.z);
    let up_fwd = up + horiz;
    if body_collides_chunks(chunks, up) || body_collides_chunks(chunks, up_fwd) {
        // Corner escape: the up check's AABB trailing edge can floor into a wall
        // the player is walking *away* from, falsely blocking the step-up when
        // the body is near a building corner. Retry with a forward bias of
        // PLAYER_HALF_W that shifts the trailing edge past the voxel boundary
        // so only voxels in the movement direction are tested. (Without this,
        // both X and Z get independently blocked at a corner — the body pins.)
        let bias = Vec3::new(
            if horiz.x != 0.0 { horiz.x.signum() * PLAYER_HALF_W } else { 0.0 },
            0.0,
            if horiz.z != 0.0 { horiz.z.signum() * PLAYER_HALF_W } else { 0.0 },
        );
        if bias != Vec3::ZERO {
            let up2 = up + bias;
            let up_fwd2 = up_fwd + bias;
            if !body_collides_chunks(chunks, up2) && !body_collides_chunks(chunks, up_fwd2) {
                let landed = settle_down_chunks(chunks, up_fwd2, lift);
                if landed.y < up_fwd2.y {
                    return landed;
                }
            }
        }
        return p; // ledge too tall or a ceiling in the way — stay blocked
    }
    // Only a ledge (solid within a step below) counts — never climb into open air.
    let landed = settle_down_chunks(chunks, up_fwd, lift);
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
///
/// The lens is treated as a disc of radius [`BOOM_RADIUS`] perpendicular to the
/// boom, not a single point: a wall the ray *centre* threads past still clips the
/// frustum at a corner or along a parallel face, so every step sweeps a ring of
/// sample points around the boom tip and pulls in the instant any of them would
/// enter a solid voxel.
fn camera_boom(world: &World, pivot: Vec3, dir: Vec3, want: f32) -> f32 {
    const STEP: f32 = 0.1;

    // Orthonormal basis in the plane perpendicular to the boom. Cross against world-Y
    // normally, but fall back to world-X when the boom is nearly vertical so the cross
    // product stays well-conditioned (covers the straight-up / straight-down probes).
    let ref_axis = if dir.dot(Vec3::Y).abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let right = dir.cross(ref_axis).normalize_or_zero();
    let upa = right.cross(dir).normalize_or_zero();
    // Eight points on the disc rim — cardinal plus diagonal. Axis-aligned voxel
    // corners always present at one of these angles, so nothing slips between samples.
    let diag = BOOM_RADIUS * std::f32::consts::FRAC_1_SQRT_2;
    let rim = [
        right * BOOM_RADIUS,
        -right * BOOM_RADIUS,
        upa * BOOM_RADIUS,
        -upa * BOOM_RADIUS,
        (right + upa) * diag,
        (right - upa) * diag,
        (-right + upa) * diag,
        (-right - upa) * diag,
    ];
    let solid = |p: Vec3| {
        solid_at(world, p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32)
    };

    let mut d = 0.0;
    while d < want {
        let base = pivot + dir * (d + BOOM_MARGIN);
        if solid(base) || rim.iter().any(|off| solid(base + *off)) {
            return d;
        }
        d += STEP;
    }
    want
}

/// After `camera_boom` settles on a distance along the boom axis, verify the
/// lens position itself isn't clipping into a block from the side or behind.
/// The boom check sweeps ahead of the lens; a block flush against the lens's
/// side (common when the avatar walks past a wall with the boom swung out) is
/// invisible to that axis and can fill a third of the frame with a single
/// brown face (A5). Walk the camera back toward the pivot until the lens
/// centre is clear in all six cardinal directions.
fn camera_lens_clear(world: &World, pivot: Vec3, dir: Vec3, mut dist: f32) -> f32 {
    const STEP: f32 = 0.1;
    const LENS_PAD: f32 = 0.25; // near-clip safety margin around the lens centre
    let solid = |p: Vec3| {
        solid_at(world, p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32)
    };
    while dist > 0.2 {
        let pos = pivot + dir * dist;
        let clear = !solid(pos)
            && !solid(pos + Vec3::X * LENS_PAD)
            && !solid(pos - Vec3::X * LENS_PAD)
            && !solid(pos + Vec3::Y * LENS_PAD)
            && !solid(pos - Vec3::Y * LENS_PAD)
            && !solid(pos + Vec3::Z * LENS_PAD)
            && !solid(pos - Vec3::Z * LENS_PAD);
        if clear {
            break;
        }
        dist -= STEP;
    }
    dist.max(0.1)
}

/// One raycast hit: the solid voxel struck and the empty cell just before it
/// (where a placed block lands).
pub(crate) struct RayHit {
    pub(crate) voxel: IVec3,
    pub(crate) prev: IVec3,
}

/// Amanatides & Woo voxel DDA: walk the grid from `origin` along `dir` until a
/// solid voxel is hit or `max_dist` is exceeded.
pub(crate) fn raycast_voxel(world: &World, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<RayHit> {
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
pub(crate) fn set_world_voxel(
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
    // Same tail as every other bulk edit — one re-mesh path, so a click and a map
    // load can't drift apart in how they rebuild a chunk's children.
    remesh_chunk(world, key, meshes, commands);
    true
}

/// Which side of an editor click: stamp a block, or carve air.
pub(crate) enum PaintOp {
    Place,
    Break,
}

/// The core of an editor click: cast a world ray from the camera through the
/// screen-space `cursor`, hit the voxel grid, then **place** (the empty cell in
/// front of the struck face) or **break** (the struck voxel). This is the exact
/// path the interactive `editor::editor_edit` and the headless `editor_paint_demo`
/// proof both drive, so a click and the proof edit the world through one function.
/// Returns the voxel cell it wrote to (`None` on a sky miss / off-grid).
pub(crate) fn paint_at_cursor(
    world: &mut World,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    cam: &Camera,
    cam_gt: &GlobalTransform,
    cursor: Vec2,
    op: PaintOp,
    block: BlockId,
) -> Option<IVec3> {
    let ray = cam.viewport_to_world(cam_gt, cursor).ok()?;
    let hit = raycast_voxel(world, ray.origin, ray.direction.as_vec3(), editor::EDIT_REACH)?;
    let (target, written) = match op {
        PaintOp::Break => (hit.voxel, BlockId::AIR),
        PaintOp::Place => (hit.prev, block),
    };
    set_world_voxel(world, target, written, meshes, commands);
    Some(target)
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
    mut selected_block: ResMut<editor::SelectedBlock>,
    mut fly_q: Query<&mut FlyCam>,
) {
    // 1-4 pick the held block, writing BOTH the legacy play-mode selector and the
    // shared SelectedBlock the editor palette / Editor-mode click read.
    if keys.just_pressed(KeyCode::Digit1) {
        editor.selected = BlockId::GRASS;
        selected_block.0 = BlockId::GRASS;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        editor.selected = BlockId::DIRT;
        selected_block.0 = BlockId::DIRT;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        editor.selected = BlockId::STONE;
        selected_block.0 = BlockId::STONE;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        editor.selected = BlockId::SAND;
        selected_block.0 = BlockId::SAND;
    }

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
        match reload_world(&mut commands, &mut meshes, &mut world, &path) {
            Ok(n) => {
                println!("MAP_LOAD(F9) ok blocks={n} path={path}");
                editor.status = format!("loaded {n} blocks ← {path}");
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

/// egui toolbar Save/Load → the same disk path F5/F9 use. Reads the messages
/// `editor_ui`'s toolbar emits and writes/reads `Editor.map_path`, so the on-screen
/// buttons are no longer dead — they round-trip a real map file.
fn egui_save_load(
    mut save_ev: MessageReader<editor_ui::SaveRequest>,
    mut load_ev: MessageReader<editor_ui::LoadRequest>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    mut editor: ResMut<Editor>,
) {
    for _ in save_ev.read() {
        let path = editor.map_path.clone();
        match save_world_to(&world, &path) {
            Ok(n) => {
                println!("MAP_SAVE(egui) ok blocks={n} path={path}");
                editor.status = format!("saved {n} blocks → {path}");
            }
            Err(e) => {
                eprintln!("MAP_SAVE(egui) FAIL {e}");
                editor.status = format!("save FAILED: {e}");
            }
        }
    }
    for _ in load_ev.read() {
        let path = editor.map_path.clone();
        match reload_world(&mut commands, &mut meshes, &mut world, &path) {
            Ok(n) => {
                println!("MAP_LOAD(egui) ok blocks={n} path={path}");
                editor.status = format!("loaded {n} blocks ← {path}");
            }
            Err(e) => {
                eprintln!("MAP_LOAD(egui) FAIL {e}");
                editor.status = format!("load FAILED: {e}");
            }
        }
    }
}

/// Headless proof that the editor builds voxels by clicking: drives the SAME
/// `paint_at_cursor` (screen→world raycast → `set_world_voxel`) the interactive
/// `editor::editor_edit` uses, in the default Editor state. Proves (a) the
/// sandbox holds no enemy husk, (b) a "click" places a voxel, (c) a "click"
/// removes one — then leaves a small brick pillar for the screenshot run.
fn editor_paint_demo(
    time: Res<Time>,
    cfg: Res<Cfg>,
    mut demo: ResMut<EditorPaintDemo>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    enemies: Query<Entity, With<combat::Enemy>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    mut editor: ResMut<Editor>,
    mut exit: MessageWriter<AppExit>,
    mut verdict: ResMut<hero::GateVerdict>,
) {
    if !cfg.editor_demo || demo.done {
        return;
    }
    // Let the camera GlobalTransform + window size settle before raycasting.
    if time.elapsed_secs() < 1.0 {
        return;
    }
    demo.done = true;

    // (a) The default Editor state is a husk-free sandbox.
    let husk_count = enemies.iter().count();

    let Ok(window) = windows.single() else {
        return;
    };
    let (w, h) = (window.width(), window.height());
    let Ok((cam, cam_gt)) = cam_q.single() else {
        return;
    };
    let center = Vec2::new(w * 0.5, h * 0.5);

    // (b) PLACE via the screen→world paint path a click uses.
    let quads_before = world.total_quads;
    let placed = paint_at_cursor(
        &mut world,
        &mut meshes,
        &mut commands,
        cam,
        cam_gt,
        center,
        PaintOp::Place,
        BlockId::STONE,
    );
    let quads_after_place = world.total_quads;
    let place_ok = placed.is_some() && quads_after_place > quads_before;

    // (c) BREAK that same voxel through the same path: re-project it to screen
    // and "click" it, proving a click removes a block.
    let mut break_ok = false;
    let mut quads_after_break = quads_after_place;
    if let Some(v) = placed {
        let p = v.as_vec3() + Vec3::splat(0.5);
        if let Some(screen) = cam.world_to_viewport(cam_gt, p).ok() {
            let removed = paint_at_cursor(
                &mut world,
                &mut meshes,
                &mut commands,
                cam,
                cam_gt,
                screen,
                PaintOp::Break,
                BlockId::STONE,
            );
            quads_after_break = world.total_quads;
            break_ok = removed.is_some() && quads_after_break < quads_after_place;
        }
    }

    // Visible marker for the screenshot run: a 4-tall brick pillar where the
    // camera is looking, built through the same `set_world_voxel` path.
    if let Some(base) = placed {
        for dy in 0..4 {
            let y = (base.y + dy).clamp(0, CHUNK as i32 - 1);
            set_world_voxel(
                &mut world,
                IVec3::new(base.x, y, base.z),
                BlockId::BRICK,
                &mut meshes,
                &mut commands,
            );
        }
    }

    editor.status = format!("editor demo: place={} break={}", place_ok, break_ok);
    let sandbox_ok = husk_count == 0;
    let pass = sandbox_ok && place_ok && break_ok;
    if !pass {
        verdict.failed = true;
        hero::GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
    }
    println!(
        "EDITOR_DEMO husk_in_sandbox={} screen_place quads {quads_before}->{quads_after_place} ({}) \
         break ->{quads_after_break} ({}) => {}",
        husk_count,
        if place_ok { "PASS" } else { "FAIL" },
        if break_ok { "PASS" } else { "FAIL" },
        if sandbox_ok && place_ok && break_ok { "PASS" } else { "FAIL" }
    );

    // Exit now for a pure headless proof; defer to the screenshot path otherwise.
    // The real exit code is decided after app.run() via the GATE_FAILED static.
    if cfg.shot.is_none() {
        exit.write(AppExit::Success);
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
    mut verdict: ResMut<hero::GateVerdict>,
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

    let land_pass = land_ok && buried && !sky;
    if !land_pass {
        verdict.failed = true;
        hero::GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
    }
    println!(
        "WALK_DEMO grounded feet_y={feet_y} surface={surf} expect_feet={expect} \
         land={} buried_solid={buried} sky_empty={} => {}",
        if land_ok { "PASS" } else { "FAIL" },
        !sky,
        if land_pass { "PASS" } else { "FAIL" }
    );

    // Camera-boom collision: the spring-arm must pull in toward a wall (solid ground
    // below) yet extend fully into open air (empty sky above) — the same solid_at
    // grid test the aim raycast uses, so a wall can never come between cam & avatar.
    let boom_down = camera_boom(world, eye, Vec3::NEG_Y, BOOM_DIST);
    let boom_up = camera_boom(world, eye, Vec3::Y, BOOM_DIST);
    let boom_pass = boom_down < BOOM_DIST - 0.5 && boom_up >= BOOM_DIST;
    if !boom_pass {
        verdict.failed = true;
        hero::GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
    }
    println!(
        "CAM_BOOM into_ground={boom_down:.2} into_sky={boom_up:.2} => {}",
        if boom_pass { "PASS" } else { "FAIL" }
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
            if !blocked {
                verdict.failed = true;
                hero::GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
            }
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
        let step_pass = climb == 1 && blocked == 0;
        if !step_pass {
            verdict.failed = true;
            hero::GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
        }
        println!(
            "STEP_DEMO ledge x={x} z={z} assisted_climb={climb} gated_climb={blocked} => {}",
            if step_pass { "PASS" } else { "FAIL" }
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
    selected_block: Res<editor::SelectedBlock>,
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
        // The block an Editor-mode click stamps: the egui palette (and 1-4 keys)
        // write the shared SelectedBlock, so show that.
        let held = block_name(selected_block.0);
        let ctrls = match editor.mode {
            EditorMode::Edit => format!(
                "L=place[{held}] R=break | MMB=orbit Shift+MMB=pan Wheel=zoom F=fly"
            ),
            EditorMode::Play => format!(
                "B=dig N=place[{held}] H=use I=bag 1-9=slot (L/R=attack/block) (cursor: click to lock)"
            ),
        };
        let status = if editor.status.is_empty() {
            String::new()
        } else {
            format!("  |  {}", editor.status)
        };
        format!(
            "FPS {fps:.0}  |  chunks {chunks}  |  quads {quads}  |  [{editmode}/{movemode}] {ctrls}  |  Tab=mode G=fill F5=map F6=save F9=load{status}"
        )
    };
    if let Ok(mut text) = q.single_mut() {
        text.0 = line;
    }
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
        // Exit code decided after app.run() via the GATE_FAILED static.
        exit.write(AppExit::Success);
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

// ---------------------------------------------------------------------------
// Beauty tour — a ~30s scripted cinematic for the CEO to watch the world
// ---------------------------------------------------------------------------
//
// `--beauty-tour` boots the playable scene (Edhari map → campfire → look stack)
// and flies the REAL gameplay camera along a placed path through three moods —
// outdoor noon → cool raking light → night firelight — then exits on its own.
// `--beauty-tour-shots <dir>` captures a still PNG at each of the three stops.
//
// No second render path: this overrides the OrbitCam transform in PostUpdate
// (after fly_camera, before propagation) and re-colours the look stack's own
// sun / directional fills / ambient / sky / IBL / exposure per-phase. The look
// stack is apply-once (guarded by `LookApplied` / `LookLightApplied` markers),
// so it sets those lights once at boot and never fights the per-frame changes.
// The HUD is hidden through scene.rs's existing `VOXELFORGE_NOHUD` sweep (set in
// `main` before any plugin reads the cached flag).

/// One authored lighting mood. Colours are sRGB, matched against the look
/// lane's `Hour` constants. `fill_scale` dims the look lane's directional fill
/// rig (sky / bounce / rim) without this lane naming those private types — they
/// are every `DirectionalLight` with `shadow_maps_enabled == false`.
struct TourLight {
    elev_deg: f32,
    azim_deg: f32,
    illuminance: f32,
    key: [f32; 3],
    sky: [f32; 3],
    sky_gain: f32,
    ambient: [f32; 3],
    ambient_lux: f32,
    fill_scale: f32,
    ibl_nits: f32,
    ev100: f32,
}

/// Outdoor noon — bright overhead key, warm-neutral, deep noon sky.
const TOUR_NOON: TourLight = TourLight {
    elev_deg: 62.0,
    azim_deg: 205.0,
    illuminance: 26_000.0,
    key: [1.00, 0.99, 0.92],
    sky: [0.34, 0.60, 0.93],
    sky_gain: 3.2,
    ambient: [0.96, 0.95, 0.90],
    ambient_lux: 1400.0,
    fill_scale: 1.0,
    ibl_nits: 330.0,
    ev100: 10.3,
};

/// Cool angled light — low raking key from a shifted azimuth, blue-leaning.
const TOUR_COOL: TourLight = TourLight {
    elev_deg: 20.0,
    azim_deg: 135.0,
    illuminance: 16_000.0,
    key: [0.85, 0.90, 1.00],
    sky: [0.36, 0.58, 0.92],
    sky_gain: 2.4,
    ambient: [0.78, 0.86, 1.00],
    ambient_lux: 900.0,
    fill_scale: 0.7,
    ibl_nits: 280.0,
    ev100: 10.3,
};

/// Night firelight — mirrors `look::Hour::NIGHT` so the campfire + lamp blocks
/// are the only warm sources left in frame.
const TOUR_NIGHT: TourLight = TourLight {
    elev_deg: -8.0,
    azim_deg: 205.0,
    illuminance: 260.0,
    key: [0.55, 0.66, 0.95],
    sky: [0.03, 0.05, 0.12],
    sky_gain: 1.0,
    ambient: [0.42, 0.52, 0.78],
    ambient_lux: 42.0,
    fill_scale: 0.03,
    ibl_nits: 14.0,
    ev100: 7.5,
};

/// Camera path: `(seconds-since-tour-start, eye, aim)`. Consecutive keyframes at
/// the same pose are a HOLD; the pairs that differ are a smoothstep dolly. This
/// is what makes each of the three stops read as a "stop" rather than a pan.
const TOUR_KEYFRAMES: &[(f32, [f32; 3], [f32; 3])] = &[
    // Outdoor noon — high establishing vista from the south rim.
    (0.0, [32.5, 16.0, 58.0], [32.5, 2.0, 30.0]),
    (5.0, [32.5, 16.0, 58.0], [32.5, 2.0, 30.0]),
    // Cool angled light — raking across the lamp colonnade from the west.
    (8.0, [20.0, 9.0, 22.0], [34.0, 4.0, 18.0]),
    (15.0, [20.0, 9.0, 22.0], [34.0, 4.0, 18.0]),
    // Night firelight — fireside, the campfire warm in the foreground.
    (18.0, [31.0, 6.0, 34.0], [32.5, 1.0, 29.0]),
    (25.0, [31.0, 6.0, 34.0], [32.5, 1.0, 29.0]),
    // Gentle push-in toward the fire as the tour ends.
    (28.0, [31.5, 5.0, 32.0], [32.5, 1.0, 29.0]),
    (30.0, [31.5, 5.0, 32.0], [32.5, 1.0, 29.0]),
];

/// Seconds of wall-clock the tour idles (camera parked on the noon vista) before
/// it starts moving, so the map finishes streaming and the look stack settles.
const TOUR_SETTLE: f32 = 3.0;

/// Still captures, seconds since tour start — one per stop, mid-hold.
const TOUR_SHOT_TIMES: [f32; 3] = [3.0, 12.0, 22.0];

/// Wall-clock second the tour exits.
const TOUR_EXIT_AT: f32 = 31.0;

/// Tour bookkeeping. `start` anchors the timeline to the first frame the system
/// runs; `fill_base` snapshots each directional fill's boot illuminance so the
/// per-frame `fill_scale` never compounds.
#[derive(Resource)]
struct BeautyTour {
    start: Option<f32>,
    shots_dir: Option<String>,
    shots_taken: usize,
    last_phase: i32,
    fill_base: Vec<(Entity, f32)>,
}

fn beauty_tour_run(cfg: Res<Cfg>) -> bool {
    cfg.beauty_tour
}

fn tour_lerp(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}

fn tour_lerp3(a: [f32; 3], b: [f32; 3], u: f32) -> [f32; 3] {
    [tour_lerp(a[0], b[0], u), tour_lerp(a[1], b[1], u), tour_lerp(a[2], b[2], u)]
}

fn tour_lerp_light(a: &TourLight, b: &TourLight, u: f32) -> TourLight {
    TourLight {
        elev_deg: tour_lerp(a.elev_deg, b.elev_deg, u),
        azim_deg: tour_lerp(a.azim_deg, b.azim_deg, u),
        illuminance: tour_lerp(a.illuminance, b.illuminance, u),
        key: tour_lerp3(a.key, b.key, u),
        sky: tour_lerp3(a.sky, b.sky, u),
        sky_gain: tour_lerp(a.sky_gain, b.sky_gain, u),
        ambient: tour_lerp3(a.ambient, b.ambient, u),
        ambient_lux: tour_lerp(a.ambient_lux, b.ambient_lux, u),
        fill_scale: tour_lerp(a.fill_scale, b.fill_scale, u),
        ibl_nits: tour_lerp(a.ibl_nits, b.ibl_nits, u),
        ev100: tour_lerp(a.ev100, b.ev100, u),
    }
}

fn tour_smoothstep(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Unit vector pointing FROM the sky TO the scene — the same convention as
/// `look::Hour::sun_dir()`, so a `DirectionalLight`'s transform looks along it.
fn tour_sun_dir(elev_deg: f32, azim_deg: f32) -> Vec3 {
    let e = elev_deg.to_radians();
    let a = azim_deg.to_radians();
    Vec3::new(a.sin() * e.cos(), -e.sin(), a.cos() * e.cos()).normalize()
}

/// Camera pose at `t` seconds since tour start — smoothstep between the two
/// keyframes bracketing `t`.
fn tour_camera(t: f32) -> (Vec3, Vec3) {
    let n = TOUR_KEYFRAMES.len();
    if t <= TOUR_KEYFRAMES[0].0 {
        return (
            Vec3::from(TOUR_KEYFRAMES[0].1),
            Vec3::from(TOUR_KEYFRAMES[0].2),
        );
    }
    for i in 0..n.saturating_sub(1) {
        let (t0, e0, a0) = TOUR_KEYFRAMES[i];
        let (t1, e1, a1) = TOUR_KEYFRAMES[i + 1];
        if t <= t1 {
            let u = tour_smoothstep((t - t0) / (t1 - t0).max(1e-3));
            return (
                tour_lerp3(e0, e1, u).into(),
                tour_lerp3(a0, a1, u).into(),
            );
        }
    }
    let last = TOUR_KEYFRAMES[n - 1];
    (Vec3::from(last.1), Vec3::from(last.2))
}

/// Lighting mood at `t` seconds since tour start. Two crossfade windows move
/// noon→cool (t 5→6.5) and cool→night (t 15→16.5); elsewhere a phase is held.
fn tour_light(t: f32) -> TourLight {
    let (a, b, u) = if t < 5.0 {
        (&TOUR_NOON, &TOUR_NOON, 0.0)
    } else if t < 6.5 {
        (&TOUR_NOON, &TOUR_COOL, tour_smoothstep((t - 5.0) / 1.5))
    } else if t < 15.0 {
        (&TOUR_COOL, &TOUR_COOL, 0.0)
    } else if t < 16.5 {
        (&TOUR_COOL, &TOUR_NIGHT, tour_smoothstep((t - 15.0) / 1.5))
    } else {
        (&TOUR_NIGHT, &TOUR_NIGHT, 0.0)
    };
    tour_lerp_light(a, b, u)
}

#[allow(clippy::too_many_arguments)]
fn beauty_tour(
    time: Res<Time>,
    mut tour: ResMut<BeautyTour>,
    mut commands: Commands,
    // `cam_q` and `lights_q` both write `&mut Transform`, which Bevy's schedule
    // validation rejects as a conflicting access pair (B0001) — it is checked at
    // schedule build time, before any `run_if`, so it panics on EVERY boot, not
    // just `--beauty-tour`. A `ParamSet` fuses the two into one parameter, which
    // is exactly the disjoint-TMut-writes shape Bevy accepts.
    mut trans_q: ParamSet<(
        Query<&mut Transform, With<OrbitCam>>,
        Query<(Entity, &mut DirectionalLight, &mut Transform)>,
    )>,
    mut ambient_q: Query<&mut AmbientLight, With<OrbitCam>>,
    mut clear: ResMut<ClearColor>,
    mut ibl_q: Query<&mut EnvironmentMapLight, With<OrbitCam>>,
    mut exposure_q: Query<&mut Exposure, With<OrbitCam>>,
    mut phase_writer: MessageWriter<audio::BeautyTourPhase>,
    mut exit: MessageWriter<AppExit>,
    enemies: Query<Entity, With<combat::Enemy>>,
    hbars: Query<Entity, With<combat::HealthBar>>,
    sbars: Query<Entity, With<combat::StaminaBar>>,
    reticles: Query<Entity, With<combat::LockReticle>>,
    hud_roots: Query<Entity, With<hud::HudRoot>>,
) {
    // Strip the combat encounter (Guard Husk + HUD bars): a tour is scenery, not
    // a fight. `VOXELFORGE_NOHUD` hides the Node UI; this removes the 3D husk the
    // Edhari boot drops right in the middle of the night-fire framing.
    for e in enemies
        .iter()
        .chain(hbars.iter())
        .chain(sbars.iter())
        .chain(reticles.iter())
        .chain(hud_roots.iter())
    {
        commands.entity(e).despawn();
    }

    let now = time.elapsed_secs();
    let start = *tour.start.get_or_insert(now + TOUR_SETTLE);
    let t = (now - start).max(0.0);

    // ---- camera (PostUpdate: after fly_camera, before propagation) --------
    let (eye, aim) = tour_camera(t);
    if let Ok(mut tf) = trans_q.p0().single_mut() {
        *tf = Transform::from_translation(eye).looking_at(aim, Vec3::Y);
    }

    // ---- lighting ----------------------------------------------------------
    let light = tour_light(t);
    let dir = tour_sun_dir(light.elev_deg, light.azim_deg);
    for (e, mut dl, mut tf) in &mut trans_q.p1() {
        if dl.shadow_maps_enabled {
            // The sun — point it, colour it, power it.
            dl.illuminance = light.illuminance;
            dl.color = Color::srgb(light.key[0], light.key[1], light.key[2]);
            tf.translation = -dir * 200.0;
            tf.look_to(dir, Vec3::Y);
        } else {
            // A directional fill (sky / bounce / rim) — dim to the phase's scale,
            // snapshotting its boot illuminance once so the factor never compounds.
            let base = match tour.fill_base.iter().find(|(fe, _)| *fe == e) {
                Some((_, b)) => *b,
                None => {
                    let b = dl.illuminance;
                    tour.fill_base.push((e, b));
                    b
                }
            };
            dl.illuminance = base * light.fill_scale;
        }
    }
    if let Ok(mut ambient) = ambient_q.single_mut() {
        ambient.color = Color::srgb(light.ambient[0], light.ambient[1], light.ambient[2]);
        ambient.brightness = light.ambient_lux;
    }
    let sky = Color::srgb(light.sky[0], light.sky[1], light.sky[2]).to_linear();
    clear.0 = Color::linear_rgb(
        sky.red * light.sky_gain,
        sky.green * light.sky_gain,
        sky.blue * light.sky_gain,
    );
    if let Ok(mut env) = ibl_q.single_mut() {
        env.intensity = light.ibl_nits;
    }
    if let Ok(mut exp) = exposure_q.single_mut() {
        exp.ev100 = light.ev100;
    }

    // ---- phase-matched ambience (message → audio.rs) ------------------------
    let phase = if t >= 15.0 {
        audio::BeautyTourPhase::Night
    } else if t >= 5.0 {
        audio::BeautyTourPhase::Cool
    } else {
        audio::BeautyTourPhase::Noon
    };
    let phase_idx = match phase {
        audio::BeautyTourPhase::Noon => 0,
        audio::BeautyTourPhase::Cool => 1,
        audio::BeautyTourPhase::Night => 2,
    };
    if phase_idx != tour.last_phase {
        tour.last_phase = phase_idx;
        phase_writer.write(phase);
        println!("BEAUTY_TOUR phase={phase:?} t={t:.1}");
    }

    // ---- stills at the three stops ------------------------------------------
    let shot_path = tour.shots_dir.as_ref().and_then(|dir| {
        let idx = tour.shots_taken;
        if idx < TOUR_SHOT_TIMES.len() && t >= TOUR_SHOT_TIMES[idx] {
            Some(format!("{dir}/beauty-tour-stop-{}.png", idx + 1))
        } else {
            None
        }
    });
    if let Some(path) = shot_path {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        tour.shots_taken += 1;
        println!("BEAUTY_TOUR shot {} -> {path}", tour.shots_taken);
    }

    // ---- auto-exit ----------------------------------------------------------
    if t >= TOUR_EXIT_AT {
        println!("BEAUTY_TOUR done t={t:.1} => exit");
        exit.write(AppExit::Success);
    }
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
        // Exit code decided after app.run() via the GATE_FAILED static.
        exit.write(AppExit::Success);
    }
}

/// `VOXELFORGE_FPS_BENCH=<secs>`: sample the loaded play scene's steady-state
/// frame time and print one line, then exit. Unlike `bench_ramp` (which grows a
/// synthetic grid), this samples the *real* map at whatever camera
/// `VOXELFORGE_CINE` pinned — so running it with the same env as a still-shot
/// gives an FPS number for the same frame composition the edge metric grades.
fn fps_bench_sampler(
    time: Res<Time>,
    world: Option<Res<World>>,
    mut bench: ResMut<FpsBench>,
    mut exit: MessageWriter<AppExit>,
) {
    if bench.sample_secs <= 0.0 {
        return;
    }
    let now = time.elapsed_secs();
    if now < FPS_WARMUP {
        return;
    }
    if !bench.started {
        bench.started = true;
        bench.start = now;
    }
    bench.samples.push(time.delta_secs() * 1000.0);
    if now - bench.start < bench.sample_secs {
        return;
    }

    let mut ms = std::mem::take(&mut bench.samples);
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = ms.len();
    let median_ms = if n > 0 { ms[n / 2] } else { 0.0 };
    let mean_ms = if n > 0 { ms.iter().sum::<f32>() / n as f32 } else { 0.0 };
    let p95_idx = if n > 0 { ((n as f32 * 0.95) as usize).min(n - 1) } else { 0 };
    let p95_ms = if n > 0 { ms[p95_idx] } else { 0.0 };
    let fps = if median_ms > 0.0 { 1000.0 / median_ms } else { 0.0 };
    let chunks = world.as_ref().map(|w| w.chunks.len()).unwrap_or(0);
    let quads = world.as_ref().map(|w| w.total_quads).unwrap_or(0);
    println!(
        "FPS_BENCH frames={n} median_ms={median_ms:.2} mean_ms={mean_ms:.2} p95_ms={p95_ms:.2} fps={fps:.1} chunks={chunks} quads={quads}"
    );
    exit.write(AppExit::Success);
}

// ---------------------------------------------------------------------------
// tests — corner-escape & step-up physics (no Bevy App needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Entity;
    use std::collections::HashMap;
    use voxelforge_sim::block::BlockId;
    use voxelforge_sim::chunk::{ChunkData, ChunkPos};

    /// Build a chunk slot whose `entity` is a placeholder (never rendered).
    fn slot_empty(pos: ChunkPos) -> ChunkSlot {
        ChunkSlot {
            data: ChunkData::empty(pos),
            entity: Entity::PLACEHOLDER,
            quads: 0,
        }
    }

    /// Place a single solid block into a chunk map, creating the chunk on demand.
    fn place(chunks: &mut HashMap<(i32, i32), ChunkSlot>, wx: i32, wy: i32, wz: i32, id: BlockId) {
        let cx = wx.div_euclid(CHUNK);
        let cz = wz.div_euclid(CHUNK);
        let slot = chunks
            .entry((cx, cz))
            .or_insert_with(|| slot_empty(ChunkPos::new(cx, 0, cz)));
        slot.data.set(wx.rem_euclid(CHUNK), wy, wz.rem_euclid(CHUNK), id);
    }

    /// Fill a column from y=0..=height (inclusive) at (wx,wz) with STONE.
    fn wall_column(
        chunks: &mut HashMap<(i32, i32), ChunkSlot>,
        wx: i32,
        wz: i32,
        height: i32,
    ) {
        for wy in 0..=height {
            place(chunks, wx, wy, wz, BlockId::STONE);
        }
    }

    /// Build a wall of `STONE` blocks along one axis.
    /// `axis` = 'x' → wall runs in Z direction at a fixed X (`fixed`); each block
    /// is placed at (`fixed`, 0..=height, z0..z1).
    /// `axis` = 'z' → wall runs in X direction at a fixed Z.
    fn wall(
        chunks: &mut HashMap<(i32, i32), ChunkSlot>,
        axis: char,
        fixed: i32,
        z0: i32,
        z1: i32,
        height: i32,
    ) {
        match axis {
            'x' => {
                for z in z0..=z1 {
                    wall_column(chunks, fixed, z, height);
                }
            }
            'z' => {
                for x in z0..=z1 {
                    wall_column(chunks, x, fixed, height);
                }
            }
            _ => panic!("axis must be 'x' or 'z'"),
        }
    }

    // ── helpers that mirror the production call chain for compact assertions ──

    /// Run `move_body` on a raw chunk map (no `World` needed).
    fn move_body_chunks(
        chunks: &HashMap<(i32, i32), ChunkSlot>,
        eye: Vec3,
        delta: Vec3,
        can_step: bool,
    ) -> (Vec3, bool) {
        let mut p = eye;
        p = step_axis_chunks(chunks, p, Vec3::new(delta.x, 0.0, 0.0), can_step);
        p = step_axis_chunks(chunks, p, Vec3::new(0.0, 0.0, delta.z), can_step);
        let ty = Vec3::new(p.x, p.y + delta.y, p.z);
        let mut grounded = false;
        if body_collides_chunks(chunks, ty) {
            if delta.y < 0.0 {
                grounded = true;
            }
        } else {
            p = ty;
        }
        (p, grounded)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test 1 — corner escape: moving away from a wall that falsely blocks `up`
    // ═══════════════════════════════════════════════════════════════════════
    //
    // Layout (top-down, Y axis up):
    //
    //    ██                ██ = stone wall at x = 0  (z 0..5, height 2)
    //    ██  P →             P  = player eye ≈ (1.0, 1.62, 1.0)
    //    ██  ■                ■  = ground block at (1, 0, 1)
    //    ██
    //
    // Body spans x∈{0,1}, z∈{0,1} at eye.x=1.0.  The wall at x=0 is BEHIND
    // the player (trailing edge).  Walking +X is blocked by the ground block
    // at (1,0,1) which triggers the step-up path.  The `up` check at the
    // lifted position still touches x=0 (the wall behind), which is a FALSE
    // collision.  Without corner-escape the body pins.  With it, the forward
    // bias shifts the trailing x-edge past x=0 so `up2` clears the wall,
    // and the body steps onto the block at (1,0,1).
    #[test]
    fn corner_escape_unblocks_behind_wall() {
        let mut chunks: HashMap<(i32, i32), ChunkSlot> = HashMap::new();

        // Wall at x=0 (z 0..5, height 2) — the "false blocker" behind the player.
        wall(&mut chunks, 'x', 0, 0, 5, 2);
        // Ground block at (1, 0, 1) — blocks the flat move & becomes the step-up ledge.
        place(&mut chunks, 1, 0, 1, BlockId::STONE);

        // Player eye at (1.0, 1.62, 1.0). Body AABB x[0.701, 1.299] → {0,1};
        // z[0.701, 1.299] → {0,1}.  The wall at x=0, z=1 blocks `up`.
        let eye = Vec3::new(1.0, 1.62, 1.0);

        // Move +X 0.5 — the corner-escape bias (+0.3 X) must unstick the body.
        let result = step_axis_chunks(&chunks, eye, Vec3::new(0.5, 0.0, 0.0), true);

        // Body must have moved forward (stepped onto the ledge).
        assert!(
            result.x > eye.x + 0.1,
            "corner-escape failed — body did not move forward\n  eye={eye:?} → result={result:?}",
        );
    }

    // Same setup but with diagonal input (+X, +Z).  `move_body` calls
    // step_axis for X then Z; corner-escape frees the body from the corner
    // so at least one axis can move — without it both axes pin.
    #[test]
    fn corner_escape_diagonal_unblocks() {
        let mut chunks: HashMap<(i32, i32), ChunkSlot> = HashMap::new();

        // Two wall columns meeting at (0, 0) — a tight building corner.
        wall_column(&mut chunks, 0, 0, 2); // column at x=0, z=0
        // Ground block for the X-axis step-up ledge.
        place(&mut chunks, 1, 0, 1, BlockId::STONE);

        let eye = Vec3::new(1.0, 1.62, 1.0);
        let (result, _grounded) = move_body_chunks(&chunks, eye, Vec3::new(0.5, 0.0, 0.5), true);

        // The body must have moved overall (not pinned at the corner).
        assert!(
            (result.x - eye.x).abs() > 0.01 || (result.z - eye.z).abs() > 0.01,
            "corner-escape failed — body pinned with diagonal input\n  eye={eye:?} → result={result:?}",
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test 2 — false-pass prevention: bias does NOT tunnel through a wall
    // ═══════════════════════════════════════════════════════════════════════
    //
    // The corner-escape bias must NOT let the body phase through a solid wall.
    // A 3-block-tall wall at x = 1 (height 3 → can't step over) blocks flat.
    // A false block at (0, 1, 0) behind the player triggers the corner-escape
    // path, but the bias (+0.3 X) can't push the body's trailing x-edge past
    // x=0 → up2 still collides with the false block → step_axis returns p.
    #[test]
    fn bias_into_wall_is_blocked() {
        let mut chunks: HashMap<(i32, i32), ChunkSlot> = HashMap::new();

        // Tall wall at x = 1 (z = -1..1, height 3 — cannot step over).
        wall(&mut chunks, 'x', 1, -1, 1, 3);
        // False block at (0, 1, 0) — triggers corner-escape by blocking `up`,
        // but persists in `up2` because bias (+0.3) can't shift past x=0.
        place(&mut chunks, 0, 1, 0, BlockId::STONE);

        // Body at eye.x=0.5: AABB x∈{0} only, so (0, 1, 0) blocks `up`.
        let eye = Vec3::new(0.5, 1.62, 0.0);
        // horiz.x=0.5 brings flat into x=1 wall → flat blocked.
        let result = step_axis_chunks(&chunks, eye, Vec3::new(0.5, 0.0, 0.0), true);

        // Must NOT tunnel through — the body stays at the original position.
        assert_eq!(
            result, eye,
            "false-pass: body tunnelled through a wall\n  eye={eye:?} → result={result:?}",
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Test 3 — single-wall normal step-up (original behaviour unchanged)
    // ═══════════════════════════════════════════════════════════════════════
    //
    // A lone 1-block-tall ledge at x = 1 (y=0 only).  The player walks +X into
    // it and auto-steps onto the block.  This is the classic step-up path; the
    // corner-escape logic must not interfere.
    #[test]
    fn single_wall_normal_step_up() {
        let mut chunks: HashMap<(i32, i32), ChunkSlot> = HashMap::new();

        // One-block ledge at x=1 (z=-1..1, height 0 → y=0 only, step-uppable).
        wall(&mut chunks, 'x', 1, -1, 1, 0);

        let eye = Vec3::new(0.5, 1.62, 0.0);
        let result = step_axis_chunks(&chunks, eye, Vec3::new(0.5, 0.0, 0.0), true);

        // Must have stepped UP onto the ledge (y increased).
        assert!(
            result.y > eye.y + 0.1,
            "normal step-up failed — body did not climb the ledge\n  eye={eye:?} → result={result:?}",
        );

        // Must have moved forward in X.
        assert!(
            result.x > eye.x + 0.1,
            "normal step-up failed — no forward movement\n  eye={eye:?} → result={result:?}",
        );
    }
}
