//! Character / equipment shot binary (Flamingo).
//!
//! Renders `characters::setup_charshot` and nothing else. It exists for the same
//! reason `audio_proof_main.rs` does: `voxelforge_shot` `#[path]`-includes
//! `hero.rs` AND `vfx.rs`, so a teammate mid-edit in either one blocks the
//! character contact sheet from being rendered at all, even though the character
//! lane touches neither file. This bin links `characters.rs` + `equipment.rs`
//! only, so it cannot be stopped by a lane it does not use.
//!
//! The stage wiring here is character-for-character the same block that lives in
//! `shot_main.rs` (`VOXELFORGE_CHARSHOT` → `setup_charshot` + `charshot_timeline`,
//! with the single-grab path standing down for the swap stages). Both binaries
//! therefore render the same frames from the same tables; this one is just the
//! copy that always builds.
//!
//! ```text
//!   VOXELFORGE_CHARSHOT=<stage>   what to render — see characters::Stage
//!   VOXELFORGE_SHOT=<path.png>    where to save
//!   VOXELFORGE_CAM / _SUN / _AMBIENT / _EXPOSURE   the usual shot-lane overrides
//! ```

#[path = "characters.rs"]
mod characters;

#[path = "equipment.rs"]
mod equipment;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PresentMode;

/// Frames drawn before the single-still stages grab, and before they quit.
/// Counted rather than timed for the same reason the VFX showcase counts: a
/// wall-clock deadline overshoots by however big the last delta was, so two
/// plates of an A/B pair can be caught at different points of the settle just
/// because the machine was busier for one of them.
const STILL_SHOT_FRAME: u32 = 96;
const STILL_EXIT_FRAME: u32 = 150;

#[derive(Resource)]
struct StillShot {
    path: Option<String>,
    took: bool,
    frame: u32,
}

fn main() -> AppExit {
    let Some(stage) = characters::CharShot::from_env() else {
        eprintln!(
            "VOXELFORGE_CHARSHOT is unset. This bin renders the character stages only — \
             try VOXELFORGE_CHARSHOT=auren, =swap, =weapons, =line, =silhouette or =catalog."
        );
        return AppExit::error();
    };
    let path = std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty());

    // Make the output directory before the render, not after: `save_to_disk`
    // fails silently-ish inside an observer, and a missing parent dir is the
    // dullest possible way to lose a 20-second render.
    for p in path.iter() {
        if let Some(dir) = std::path::Path::new(p).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
    }

    // Pin assets to the exe directory — see the identical block in main.rs for the
    // full rationale (Bevy 0.19 `get_base_path()` CARGO_MANIFEST_DIR hijack).
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    let asset_path = exe_dir.join("assets");

    // Frame size is an env lever, not a constant. The gear sheet is judged BY EYE
    // against modern character mods, and 720p of a 2.5-block figure is ~460 px of
    // character — small enough that a chamfer, a pupil or a knuckle row simply is
    // not resolved, which would make the sculpt pass unjudgeable rather than bad.
    // The camera is untouched: `PerspectiveProjection::fov` is VERTICAL in Bevy, so
    // changing the aspect changes how much of the world is visible left/right and
    // never how tall the subject is in frame.
    let res = env_res().unwrap_or((1280, 720));

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_path.to_string_lossy().to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Voxelforge — character shot".into(),
                    resolution: (res.0, res.1).into(),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
    .insert_resource(stage)
    .init_resource::<characters::SwapRun>()
    .add_systems(Startup, characters::setup_charshot)
    .add_systems(Update, characters::charshot_timeline);

    // The swap stages take several captures in one process and drive their own
    // screenshots + exit; the still stages go through the one-grab path below.
    let still = if stage.owns_capture() { None } else { path };
    app.insert_resource(StillShot { path: still, took: false, frame: 0 })
        .add_systems(Update, still_shot);

    println!("CHARSHOT bin: stage={:?} owns_capture={}", stage.stage, stage.owns_capture());
    app.run()
}

/// `VOXELFORGE_RES=1440,1080` → `(1440, 1080)`. Anything unparseable is ignored
/// rather than defaulted-to-garbage: a typo'd resolution silently shooting a
/// 1×1 window is the sort of thing that costs an afternoon.
fn env_res() -> Option<(u32, u32)> {
    let raw = std::env::var("VOXELFORGE_RES").ok()?;
    let (w, h) = raw.split_once(&[',', 'x'][..])?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    (w >= 64 && h >= 64 && w <= 7680 && h <= 4320).then_some((w, h))
}

fn still_shot(
    mut commands: Commands,
    mut state: ResMut<StillShot>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = state.path.clone() else {
        return;
    };
    state.frame += 1;
    if !state.took && state.frame >= STILL_SHOT_FRAME {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
        state.took = true;
        println!("SHOT saved to {path} (frame {})", state.frame);
    }
    if state.took && state.frame >= STILL_EXIT_FRAME {
        exit.write(AppExit::Success);
    }
}
