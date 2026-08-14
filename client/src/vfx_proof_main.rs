//! ISOLATED VFX proof binary (kevin, VFX lane) — renders `vfx::setup_showcase`
//! ONLY, linking the live `vfx.rs` verbatim and nothing else.
//!
//! WHY A SEPARATE BIN. The `voxelforge` main bin does not compile right now:
//! two lanes that are *not* mine are mid-edit and broken — `main_menu.rs` and
//! `combat.rs` (docs/LANES.md). `voxelforge_shot` would render the VFX stage, but
//! it also `#[path]`-includes `hero.rs`, `characters.rs`, `equipment.rs` and
//! `enemies.rs` — four other lanes' files — so any one of those breaking stops the
//! VFX pairs from rendering at all. This bin links exactly one lane's file, so it
//! can only be blocked by a breakage in `vfx.rs` itself, which is mine to fix.
//!
//! WHY ZERO SHIMS. `audio_proof_main.rs` (yamamoto) needed local shims because
//! `audio.rs` reaches `crate::combat` / `crate::quest` / `crate::scene`. `vfx.rs`
//! deliberately reaches into no other module — its contract is "bevy + messages +
//! marker components" (see its doc header) — so `#[path = "vfx.rs"] mod vfx;` is
//! the whole integration. No shim, no copy, no edit: the real file, verbatim.
//!
//! WHAT THIS ADDS. The one thing `vfx.rs` does not do is own the render pipeline
//! or the screenshot grab — that lives in the binary (here), lifted from
//! `shot_main.rs`'s VFX branch so the stage is graded through the identical post
//! stack. The stage itself (`setup_showcase` + `showcase_timeline`) fires the four
//! effects on a fixed clock — impact burst, hit flash, ambient dust, weapon trail —
//! and this bin photographs it at a *counted* frame (see the determinism note
//! below) instead of a wall-clock deadline.

#[path = "vfx.rs"]
mod vfx;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PresentMode;

/// Frames the showcase runs before the grab, and before it quits. At the pinned
/// 1/60 s step these are the same 3.2 s / 4.4 s marks `shot_main.rs` uses — but
/// counted, not timed, so two plates of a before/after pair cannot be caught at
/// different points of the beat just because the box was busier for one of them.
const SHOWCASE_SHOT_FRAME: u32 = 192; // 3.2 s
const SHOWCASE_EXIT_FRAME: u32 = 264; // 4.4 s

#[derive(Resource)]
struct ShotState {
    path: Option<String>,
    took: bool,
    frame: u32,
}

fn main() -> AppExit {
    let shot = std::env::var("VOXELFORGE_SHOT")
        .ok()
        .filter(|s| !s.is_empty());

    // Pin assets to the exe directory — see the identical block in main.rs for the
    // full rationale (Bevy 0.19 `get_base_path()` CARGO_MANIFEST_DIR hijack).
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
                    title: "Voxelforge — VFX proof".into(),
                    resolution: (1280u32, 720u32).into(),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
    )
    // 4K directional shadow map → PCSS penumbra has enough texels (matches main.rs).
    .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
    .insert_resource(ShotState { path: shot, took: false, frame: 0 })
    .add_systems(Update, screenshot_once);

    // This bin exists only for the VFX stage, so there is no golden-kitchen
    // fallback: the stage always runs, defaulting to `Impact` when the env var is
    // unset. The clock is a fixed 1/60 s manual step (NOT a clamped virtual clock)
    // so the beat timings, the husk reel and the particle integration all land
    // identically whether the box is idle or carrying other lanes' builds — the
    // property a before/after pair actually depends on. See `shot_main.rs` for the
    // full argument.
    let step = std::time::Duration::from_secs_f64(1.0 / 60.0);
    let which = vfx::VfxShot::from_env().unwrap_or(vfx::VfxShot::Impact);
    let mute = vfx::VfxMute::from_env();

    app.add_plugins(vfx::VfxPlugin)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(step))
        .insert_resource(which)
        .insert_resource(mute)
        .init_resource::<vfx::ShowcaseTimeline>()
        .add_systems(Startup, vfx::setup_showcase)
        .add_systems(Update, vfx::showcase_timeline);

    println!("VFX proof: {which:?} mute={} (fixed 1/60 step, grab @ frame {SHOWCASE_SHOT_FRAME})", mute.0);

    app.run()
}

/// Let the post stack accumulate, grab the frame on a counted step, then exit.
/// A `now > 3.2` test overshoots by however big the last delta was; a counted
/// frame on a fixed step cannot drift between two plates.
fn screenshot_once(
    mut commands: Commands,
    mut state: ResMut<ShotState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = state.path.clone() else {
        return;
    };
    state.frame += 1;

    let grab = state.frame >= SHOWCASE_SHOT_FRAME;
    let quit = state.frame >= SHOWCASE_EXIT_FRAME;

    if !state.took && grab {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        state.took = true;
        println!("SHOT saved to {path} (frame {})", state.frame);
    }
    if state.took && quit {
        exit.write(AppExit::Success);
    }
}
