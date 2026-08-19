//! Foliage wind proof binary — renders `foliage.rs` ONLY.
//!
//! Why this exists at all: the whole client renders through `StandardMaterial`
//! and the seven vegetation sprites have no `kinds` entry / no render mode yet,
//! so there is no frame anywhere in the game where a plant sways in wind. This
//! bin builds the smallest honest stage — a field of cross-quad grass on a
//! ground plane under a late-afternoon sun — wearing `foliage::FoliageMaterial`,
//! so the wind vertex shader (field + gust + per-plant phase + bend ramp) has
//! something to move.
//!
//! It links `foliage.rs` and nothing else, so it cannot be blocked by (or
//! block) any other lane's file, and it needs no edit to `main.rs`.
//!
//! ## Capture
//!
//! ```text
//! VOXELFORGE_SHOT=foliage_wind.png  voxelforge_foliage_proof.exe
//! ```
//!
//! The wind clock runs on `TimeUpdateStrategy::ManualDuration` at a pinned
//! 1/60 s step, so the sway is deterministic frame-to-frame: the same binary
//! shot twice at the same frame gives the same sway.

#[path = "foliage.rs"]
mod foliage;

use bevy::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{AmbientLight, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::view::Msaa;
use bevy::window::PresentMode;

use foliage::{foliage_material, cross_quad_mesh, FoliageMaterial, FoliagePlugin, FoliageWindUniform};

/// Frame the grab happens on, and the frame the app quits on (same counted-not-
/// timed discipline as `atlas_shot_main.rs`).
const SHOT_FRAME: u32 = 90;
/// Frames between the grab and the quit, so the async screenshot write lands
/// before the process exits (the grab is never the last thing the app does).
const EXIT_MARGIN: u32 = 60;

/// Capture frame, overridable via `VOXELFORGE_FOLIAGE_SHOT_FRAME`. The sway
/// harness uses this to grab two wind times from one binary (e.g. frame 90
/// vs 174) instead of rebuilding for each shot.
fn shot_frame() -> u32 {
    std::env::var("VOXELFORGE_FOLIAGE_SHOT_FRAME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(SHOT_FRAME)
}

/// Sway amplitude override via `VOXELFORGE_FOLIAGE_SWAY_AMP`. 0.0 zeroes the
/// sway so the harness can shoot a control pair that MUST not move.
fn sway_amp_override() -> Option<f32> {
    std::env::var("VOXELFORGE_FOLIAGE_SWAY_AMP")
        .ok()
        .and_then(|v| v.parse().ok())
}

/// The field's wind state, with sway amplitude overridable via env so the
/// harness can shoot a `sway_amp = 0.0` control arm (grass must not move).
fn wind_uniform() -> FoliageWindUniform {
    let mut wind = FoliageWindUniform::default();
    if let Some(amp) = sway_amp_override() {
        wind.sway_amp = amp;
    }
    wind
}

#[derive(Resource)]
struct ShotState {
    path: Option<String>,
    took: bool,
    frame: u32,
}

fn main() -> AppExit {
    let shot = std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty());

    // Pin assets to the exe directory, same as `atlas_shot_main.rs`, so the
    // shader + sprite resolve from `<exe>/assets`.
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    let asset_path = exe_dir.join("assets");

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_path.to_string_lossy().to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Voxelforge — foliage wind".into(),
                        resolution: (1280u32, 720u32).into(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .insert_resource(ShotState {
            path: shot,
            took: false,
            frame: 0,
        })
        .add_plugins(FoliagePlugin)
        .add_systems(Startup, setup_stage)
        .add_systems(Update, screenshot_once)
        .run()
}

// ---------------------------------------------------------------------------
// the stage
// ---------------------------------------------------------------------------

fn setup_stage(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<FoliageMaterial>>,
    // The ground plane is plain PBR, so it needs the OTHER material store —
    // `mats` only holds `ExtendedMaterial<StandardMaterial, FoliageWindExt>`.
    mut ground_mats: ResMut<Assets<StandardMaterial>>,
) {
    let tex = asset_server.load("textures/blocks/vegetation/grass_tall.png");
    let mesh = meshes.add(cross_quad_mesh());
    let mat = mats.add(foliage_material(tex, wind_uniform()));

    // A field of cross-quads. Each plant is its own entity with its own world
    // transform — the shader derives its phase/amplitude from that world cell,
    // so the field never sways in lockstep.
    let half = 12;
    let spacing = 0.8;
    let mut n = 0usize;
    for ix in -half..=half {
        for iz in -half..=half {
            let x = ix as f32 * spacing;
            let z = iz as f32 * spacing;
            // Deterministic height jitter so the field reads as grass, not a
            // uniform grid of identical blades.
            let scale = 0.6 + 0.7 * height_jitter(ix, iz);
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(scale)),
            ));
            n += 1;
        }
    }
    println!("FOLIAGE plants={n} wind_dir={:?}", FoliageWindUniform::default().wind_dir);

    // Ground plane the field sits on.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 1.0, 40.0))),
        MeshMaterial3d(ground_mats.add(StandardMaterial {
            base_color: Color::srgb(0.314, 0.513, 0.229),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    // Sun: late-afternoon key, same recipe as the atlas stage.
    let dir = Vec3::new(-0.55, -0.62, -0.56).normalize();
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.94, 0.84),
            illuminance: 22_000.0,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.10,
            shadow_normal_bias: 1.8,
            ..default()
        },
        Transform::from_translation(-dir * 60.0).looking_to(dir, Vec3::Y),
    ));

    // Camera: 3/4 establishing over the field.
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.46, 0.66, 0.86)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 50.0_f32.to_radians(),
            near: 0.05,
            ..default()
        }),
        Transform::from_xyz(14.0, 10.0, 14.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        Msaa::Off,
        AmbientLight {
            color: Color::srgb(0.62, 0.72, 0.88),
            brightness: 900.0,
            affects_lightmapped_meshes: false,
        },
        Exposure { ev100: 10.5 },
        Tonemapping::AcesFitted,
    ));
}

/// Deterministic [0,1) per grid cell — a stable height jitter, not a random one.
fn height_jitter(x: i32, z: i32) -> f32 {
    let h = (x.wrapping_mul(374_761_393) ^ z.wrapping_mul(668_265_263)) as u32;
    (h % 1000) as f32 / 1000.0
}

/// Grab on a counted frame, then quit (same as `atlas_shot_main.rs`).
fn screenshot_once(
    mut commands: Commands,
    mut state: ResMut<ShotState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = state.path.clone() else {
        return;
    };
    state.frame += 1;
    let shot = shot_frame();
    if !state.took && state.frame >= shot {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        state.took = true;
        println!("SHOT saved to {path} (frame {})", state.frame);
    }
    if state.took && state.frame >= shot + EXIT_MARGIN {
        exit.write(AppExit::Success);
    }
}
