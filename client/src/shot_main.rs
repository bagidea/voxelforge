//! ISOLATED hero-shot binary (Flamingo) — renders `hero::setup_hero` ONLY.
//!
//! Exists so the Golden Beauty-Shot can be rebuilt+rendered from the current
//! `hero.rs` WITHOUT touching `main.rs` while a teammate (Poppy) is mid-edit on
//! it (gameplay raycast/edit systems). Self-contained: its own `Cfg` + env parse
//! + screenshot timer; `#[path]`-includes the live `hero.rs`. Separate `[[bin]]`
//! (`voxelforge_shot`) → distinct exe, no clobber of `voxelforge.exe`.

// NOTE: `editor_ui` used to be pulled into this isolated shot bin purely to
// type-check it while main.rs was mid-edit. That verify-only hook is gone now —
// editor_ui (and its deps crate::import, crate::FlyCam) are wired into the real
// `voxelforge` binary via main.rs, so the main build is the type-check. This bin
// stays self-contained: it renders `hero.rs` only.

#[path = "hero.rs"]
mod hero;

// The VFX layer (`vfx.rs`) is deliberately bevy-only — it reaches into no other
// lane's module — so it compiles inside this isolated bin exactly as it will inside
// `voxelforge`. That is what makes it possible to build, run and photograph the VFX
// here without an edit to `main.rs` (kevin's lane, per docs/LANES.md).
#[path = "vfx.rs"]
mod vfx;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PresentMode;

/// Hero-shot config — mirrors the fields `hero::setup_hero` reads. All env-driven
/// so framing/DOF/haze tune WITHOUT a recompile.
#[derive(Resource, Clone)]
pub struct Cfg {
    pub shot: Option<String>,
    /// `VOXELFORGE_STRICT_EXIT=1`: exit code ≠0 when a gate FAILs (the overlap check).
    pub strict_exit: bool,
    pub cam: Option<[f32; 7]>, // ex,ey,ez, tx,ty,tz, fov_deg
    pub sun: Option<[f32; 3]>, // elevation_deg, azimuth_deg, illuminance
    pub dof: Option<[f32; 2]>, // focal_distance, aperture_f_stops
    pub fog: Option<f32>,      // volumetric density_factor
    pub exposure: Option<f32>, // camera ev100
    pub grade: Option<[f32; 3]>, // post grade: temperature, post_saturation, contrast
    pub ambient: Option<f32>,  // AmbientLight brightness (lux)
    pub emissive: Option<f32>, // scale on the window pane emissive
    pub dfog: Option<f32>,     // DistanceFog density
    pub soft: Option<f32>,     // PCSS soft_shadow_size
    // Look knobs hero.rs used to read from env itself; they live on Cfg now so
    // native and web go through ONE path (main.rs fills them from the query
    // string on wasm). Mirror main.rs::Cfg or hero.rs stops compiling here.
    pub wide: bool,
    pub fg_apron: bool,
    pub dust: Option<f32>,
    pub bluescale: Option<f32>,
    pub bounce: Option<f32>,
    pub bounce2: Option<f32>,
    pub shoulder: Option<f32>,
    pub ambcolor: Option<[f32; 3]>,
}

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

fn read_cfg() -> Cfg {
    Cfg {
        shot: std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty()),
        strict_exit: std::env::var("VOXELFORGE_STRICT_EXIT").is_ok(),
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
        wide: std::env::var("VOXELFORGE_WIDE").is_ok(),
        fg_apron: std::env::var("VOXELFORGE_FGAPRON").is_ok(),
        dust: std::env::var("VOXELFORGE_DUST").ok().and_then(|v| v.parse().ok()),
        bluescale: std::env::var("VOXELFORGE_BLUESCALE").ok().and_then(|v| v.parse().ok()),
        bounce: std::env::var("VOXELFORGE_BOUNCE").ok().and_then(|v| v.parse().ok()),
        bounce2: std::env::var("VOXELFORGE_BOUNCE2").ok().and_then(|v| v.parse().ok()),
        shoulder: std::env::var("VOXELFORGE_SHOULDER").ok().and_then(|v| v.parse().ok()),
        ambcolor: env_floats("VOXELFORGE_AMBCOLOR"),
    }
}

/// Frames the showcase runs before the grab, and before it quits. At the pinned
/// 1/60 s step these are the old 3.2 s / 4.4 s marks exactly — but counted, not
/// timed, so nothing about the grab depends on how fast the machine is.
const SHOWCASE_SHOT_FRAME: u32 = 192; // 3.2 s
const SHOWCASE_EXIT_FRAME: u32 = 264; // 4.4 s

#[derive(Resource)]
struct ShotState {
    path: Option<String>,
    took: bool,
    /// Frames drawn so far. Only meaningful on the showcase path.
    frame: u32,
    /// `Some(n)` ⇒ grab on frame `n` instead of at a wall-clock time. The golden
    /// kitchen shot leaves this `None` and keeps its original 3.2 s warm-up, so
    /// that path re-renders exactly as it always did.
    at_frame: Option<u32>,
}

fn main() -> AppExit {
    let cfg = read_cfg();
    let shot = cfg.shot.clone();

    // Pin assets to the exe directory — see the identical block in main.rs for
    // the full rationale (Bevy 0.19 `get_base_path()` CARGO_MANIFEST_DIR hijack).
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    let asset_path = exe_dir.join("assets");

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin { file_path: asset_path.to_string_lossy().to_string(), ..default() })
            .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxelforge — hero shot".into(),
                resolution: (1280u32, 720u32).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }),
    )
    // 4K directional shadow map → PCSS penumbra has enough texels (matches main.rs).
    .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
    .insert_resource(cfg)
    .insert_resource(ShotState { path: shot, took: false, frame: 0, at_frame: None })
    .insert_resource(hero::GateVerdict::default())
    .add_systems(Update, screenshot_once);

    // `VOXELFORGE_VFX=off|impact|dissolve|fire` swaps the locked golden KITCHEN for
    // the VFX showcase stage. Two scenes, one bin, and — importantly — the kitchen
    // path is untouched when the var is unset, so re-rendering the golden shot still
    // produces the same frame it always did.
    match vfx::VfxShot::from_env() {
        Some(which) => {
            // Drive the showcase clock off the FRAME COUNT, not off the wall clock.
            //
            // `ManualDuration` makes `Time` advance by exactly this much per frame no
            // matter how long the frame really took, so the beat timings, the husk
            // reel and the particle integration all land identically whether the box
            // is idle or has three other lanes' builds on it. That matters for a
            // before/after pair specifically: the two plates must be caught at the
            // same point of the beat, and CPU load is not allowed to be the thing
            // that decides where that point is.
            //
            // Note this is NOT the same as clamping `Time<Virtual>`'s `max_delta`.
            // A clamp only bounds frames that ran SLOWER than the step — the moment
            // the machine goes quiet and the app hits vsync at 60+ fps, a clamped
            // clock silently goes back to following the wall clock and the
            // determinism evaporates exactly when you stop watching for it.
            //
            // Scoped to the showcase branch: the golden kitchen shot keeps its
            // original wall-clock warm-up and re-renders as it always did.
            let step = std::time::Duration::from_secs_f64(1.0 / 60.0);

            let mute = vfx::VfxMute::from_env();
            app.add_plugins(vfx::VfxPlugin)
                .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(step))
                .insert_resource(which)
                .insert_resource(mute)
                .init_resource::<vfx::ShowcaseTimeline>()
                .add_systems(Startup, vfx::setup_showcase)
                .add_systems(Update, vfx::showcase_timeline);
            // Same reason: grab on a counted frame rather than the first frame past a
            // timestamp. A `now > 3.2` test overshoots by however big the last delta
            // was, which reintroduces per-run drift through the back door.
            if let Some(mut st) = app.world_mut().get_resource_mut::<ShotState>() {
                st.at_frame = Some(SHOWCASE_SHOT_FRAME);
            }
            println!("VFX showcase: {which:?} mute={} (fixed 1/60 step)", mute.0);
        }
        None => {
            app.add_systems(Startup, (hero::setup_hero, dup_probe).chain())
                // Overlap gate: prints `VOXEL_OVERLAPS=<n>` once, on the first
                // Update (Startup's spawns are applied by then).
                // `scripts/render_wide_hero.sh` fails the render unless that
                // reads 0 — two cubes in one cell z-fight, and which one you get
                // then depends on draw order, which is how the teal accent block
                // fell out of the old golden the moment dust motes were added.
                .add_systems(Update, hero::report_voxel_overlaps);
        }
    }

    app.add_systems(Last, check_gate_on_exit);
    app.run()
}

/// SELF-TEST for `hero::report_voxel_overlaps` (default OFF).
///
/// `VOXELFORGE_DUPPROBE=1` plants exactly one extra cube on top of a cube the hero
/// scene already spawned, so the overlap gate has to report `VOXEL_OVERLAPS=1`. It
/// lives here rather than in `hero.rs` for two reasons: this bin is native-only so a
/// direct `env::var` is honest here (hero.rs takes every knob through `Cfg` because
/// `env::var` always errs on wasm), and adding a debug-only field to `Cfg` would mean
/// editing `main.rs`, which is another lane's file.
///
/// The cell is the hero bowl's own base corner (6,3,3) — spawned on every path,
/// wide or narrow — so the probe never depends on a flag that might be off.
///
/// The planted cube gets its own material, not a bare mesh: the gate grades cells
/// holding two *different* materials (same-material coincidence is invisible, so
/// grading it would only add noise), and a material-less entity isn't in its query
/// at all. A probe that can't be seen by the gate it tests proves nothing.
fn dup_probe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    if std::env::var("VOXELFORGE_DUPPROBE").is_err() {
        return;
    }
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mat = mats.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 1.0),
        ..default()
    });
    commands.spawn((
        Mesh3d(cube),
        MeshMaterial3d(mat),
        Transform::from_xyz(6.5, 3.5, 3.5),
    ));
    println!("DUPPROBE: planted a conflicting cube at (6.5, 3.5, 3.5)");
}

/// Let TAA/PCSS/SSAO accumulate, grab the frame, then exit.
///
/// The kitchen path waits 3.2 s of wall clock, matching `main.rs::screenshot_once`
/// so the look still matches the terrain path it was graded against. The showcase
/// path waits 192 frames instead — same 3.2 s at its pinned 1/60 s step, but a
/// count rather than a deadline, so two plates of a pair cannot be caught at
/// different points of the beat just because the machine was busier for one of them.
fn screenshot_once(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<ShotState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = state.path.clone() else {
        return;
    };
    state.frame += 1;

    // Two clocks on purpose. The kitchen path keeps the wall-clock warm-up it was
    // graded on; the showcase path counts frames, because it runs on a fixed step
    // and a counted frame is the only grab that cannot drift between two plates.
    let (grab, quit) = match state.at_frame {
        Some(at) => (state.frame >= at, state.frame >= SHOWCASE_EXIT_FRAME),
        None => {
            let now = time.elapsed_secs();
            (now > 3.2, now > 4.4)
        }
    };

    if !state.took && grab {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        state.took = true;
        println!("SHOT saved to {path} (frame {})", state.frame);
    }
    if state.took && quit {
        // Exit code is decided via AppExit messages inside the ECS —
        // `check_gate_on_exit` in Last picks up GATE_FAILED and injects
        // an error exit that `should_exit()` finds before Success.
        exit.write(AppExit::Success);
    }
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
