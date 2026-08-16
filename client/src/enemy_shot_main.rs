//! Enemy design shot binary (Monanisa).
//!
//! Renders `enemies::setup_enemyshot` and nothing else. Same reason
//! `char_shot_main.rs` exists: `voxelforge_shot` (`shot_main.rs`) `#[path]`-
//! includes `hero.rs`, `vfx.rs` AND `characters.rs`, so a teammate mid-edit
//! in any one of those blocks the enemy contact sheet from rendering at all,
//! even though the enemy lane touches none of them — and `shot_main.rs`
//! itself is mid-edit on this branch right now. This bin links `enemies.rs`
//! only, so it cannot be stopped by a lane it does not use.
//!
//! The stage wiring here is the same block `shot_main.rs` uses for
//! `VOXELFORGE_ENEMYSHOT` (`EnemyShot::from_env()` → `setup_enemyshot`); this
//! one just always builds.
//!
//! ```text
//!   VOXELFORGE_ENEMYSHOT=<stage>   what to render — see enemies::EnemyShot
//!   VOXELFORGE_SHOT=<path.png>     where to save
//!   VOXELFORGE_RES=<w,h>           frame size (default 1280,720)
//!   VOXELFORGE_CAM / _SUN / _SUN_NIGHT / _AMBIENT / _EXPOSURE   the usual overrides
//! ```

#[path = "enemies.rs"]
mod enemies;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PresentMode;

/// Frames drawn before the grab, and before the process quits. Counted
/// rather than timed for the same reason `char_shot_main.rs` counts: a
/// wall-clock deadline overshoots by however big the last delta was, which
/// would put an A/B pair (e.g. `sentinel:day` vs `before`) at different
/// points of the settle just because the machine was busier for one of them.
const STILL_SHOT_FRAME: u32 = 96;
const STILL_EXIT_FRAME: u32 = 150;

#[derive(Resource)]
struct StillShot {
    path: Option<String>,
    took: bool,
    frame: u32,
}

fn main() -> AppExit {
    let Some(stage) = enemies::EnemyShot::from_env() else {
        eprintln!(
            "VOXELFORGE_ENEMYSHOT is unset. This bin renders the enemy design stages only — \
             try VOXELFORGE_ENEMYSHOT=before, =line, =sentinel, =reaver or =stalker \
             (append :day, :night or :sil)."
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

    // Pin assets to the exe directory — see the identical block in main.rs
    // for the full rationale (Bevy 0.19 `get_base_path()` CARGO_MANIFEST_DIR
    // hijack).
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    let asset_path = exe_dir.join("assets");

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
                    title: "Voxelforge — enemy shot".into(),
                    resolution: (res.0, res.1).into(),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
    .insert_resource(stage)
    .add_systems(Startup, enemies::setup_enemyshot);

    app.insert_resource(StillShot { path, took: false, frame: 0 }).add_systems(Update, still_shot);

    println!("ENEMYSHOT bin: stage={stage:?}");
    app.run()
}

/// `VOXELFORGE_RES=1440,1080` → `(1440, 1080)`. Anything unparseable is
/// ignored rather than defaulted-to-garbage: a typo'd resolution silently
/// shooting a 1×1 window is the sort of thing that costs an afternoon.
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
