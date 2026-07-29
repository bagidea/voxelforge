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

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PresentMode;

/// Hero-shot config — mirrors the fields `hero::setup_hero` reads. All env-driven
/// so framing/DOF/haze tune WITHOUT a recompile.
#[derive(Resource, Clone)]
pub struct Cfg {
    pub shot: Option<String>,
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
    }
}

#[derive(Resource)]
struct ShotState {
    path: Option<String>,
    took: bool,
}

fn main() {
    let cfg = read_cfg();
    let shot = cfg.shot.clone();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
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
    .insert_resource(ShotState { path: shot, took: false })
    .add_systems(Startup, hero::setup_hero)
    .add_systems(Update, screenshot_once);

    app.run();
}

/// Wait for TAA/PCSS/SSAO to accumulate (3.2s), grab the frame, then exit — same
/// timing as `main.rs::screenshot_once` so the look matches the terrain path.
fn screenshot_once(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<ShotState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = state.path.clone() else {
        return;
    };
    let now = time.elapsed_secs();
    if !state.took && now > 3.2 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        state.took = true;
        println!("SHOT saved to {path}");
    }
    if state.took && now > 4.4 {
        exit.write(AppExit::Success);
    }
}
