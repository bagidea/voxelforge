//! Cross-quad vegetation with a wind vertex shader ("living grass").
//!
//! ## Why this module exists
//!
//! The atlas manifest catalogues seven vegetation sprites (`grass_tall`,
//! `flower_red`, `flower_pink`, `flower_white`, `foliage_bush`, `leaf_birch`,
//! `leaf_pine`) but, as of the 2026-08-18 PBR pass, deliberately gave them NO
//! `kinds` entry: "there is no cross-quad render mode yet". Nothing in Rust
//! references those tiles, and the whole client renders through bevy_pbr's
//! built-in `StandardMaterial` — this file is therefore the **first custom
//! vertex shader in the codebase**, and the wind lives in the vertex stage as
//! the Director asked: one shader, one uniform block, four requirements.
//!
//! ## The four requirements (mapped to the WGSL)
//!
//! * (1) wind FIELD  — `wind_field()` in `assets/shaders/foliage_wind.wgsl`:
//!   value noise advected by ONE whole-scene `wind_dir` so it travels downwind.
//! * (2) GUST        — `gust()`: a crest running along `wind_dir`, read as a
//!   ripple through the field.
//! * (3) per-plant phase/amp — `plant_phase()` / `plant_amp()`: a deterministic
//!   hash of the plant's WORLD cell, so neighbours never sway in lockstep.
//! * (4) bend ramp   — `bend = uv.y * uv.y` in `vertex()`: zero (and flat) at
//!   the root, max at the tip, so the plant is pinned to the ground.
//!
//! ## Wiring
//!
//! `FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageWindExt>`: the
//! base `StandardMaterial` keeps the whole PBR fragment (alpha-cutout, fog,
//! lighting) and `FoliageWindExt` swaps in the wind vertex shader. `#[uniform(100)]`
//! lands in the extension bind group (group 2), which the WGSL reads as
//! `@group(2) @binding(100)`.
//!
//! This module is self-contained: it links nothing else, so it can be built
//! through the isolated `voxelforge_foliage_proof` bin while other lanes own
//! `main.rs` / `look.rs`.

use bevy::asset::RenderAssetUsages;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{ShaderRef, ShaderType};

/// Wind state uploaded to the shader, one block per material. Field order and
/// types mirror `struct WindParams` in `assets/shaders/foliage_wind.wgsl`.
#[derive(ShaderType, Clone, Debug)]
pub struct FoliageWindUniform {
    /// Seconds since plugin start, advanced every frame.
    pub time: f32,
    /// Advection speed of the noise field (m/s).
    pub wind_speed: f32,
    /// Spatial frequency of the noise field.
    pub wind_scale: f32,
    /// The ONE wind direction for the whole scene (unit vector, xz plane).
    pub wind_dir: Vec2,
    /// Gust ripple amplitude, added on top of the field.
    pub gust_strength: f32,
    /// Spatial frequency of the travelling gust.
    pub gust_freq: f32,
    /// Gust crest speed along `wind_dir` (m/s).
    pub gust_speed: f32,
    /// Per-plant oscillation frequency (rad/s).
    pub sway_freq: f32,
    /// Max horizontal displacement at the tip (m).
    pub sway_amp: f32,
}

impl Default for FoliageWindUniform {
    fn default() -> Self {
        Self {
            time: 0.0,
            wind_speed: 0.8,
            wind_scale: 0.35,
            wind_dir: Vec2::new(1.0, 0.4).normalize(),
            gust_strength: 0.9,
            gust_freq: 0.6,
            gust_speed: 1.6,
            sway_freq: 1.7,
            sway_amp: 0.16,
        }
    }
}

/// The vertex-shader extension. The `#[uniform(100)]` slot is the extension
/// bind group (group 2) — see the WGSL `@group(2) @binding(100)`.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct FoliageWindExt {
    #[uniform(100)]
    pub wind: FoliageWindUniform,
}

impl MaterialExtension for FoliageWindExt {
    fn vertex_shader() -> ShaderRef {
        "shaders/foliage_wind.wgsl".into()
    }
}

/// The plant material: bevy_pbr's PBR fragment (alpha-cutout, fog, lighting)
/// with our wind vertex shader bolted on.
pub type FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageWindExt>;

/// Registers the material pipeline and the wind clock.
pub struct FoliagePlugin;

impl Plugin for FoliagePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FoliageMaterial>::default())
            .add_systems(Update, advance_wind);
    }
}

/// Advance the wind clock every frame and push it into every live material.
fn advance_wind(time: Res<Time>, mut mats: ResMut<Assets<FoliageMaterial>>) {
    let t = time.elapsed_secs();
    for mat in mats.iter_mut() {
        mat.extension.wind.time = t;
    }
}

/// Two perpendicular quads sharing a vertical axis, with `uv.y` = height along
/// the plant (0 at the root, 1 at the tip) — the channel the vertex shader
/// reads for the bend ramp. Faces ±Z and ±X so the billboard reads from any
/// horizontal angle.
pub fn cross_quad_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);

    let positions: Vec<[f32; 3]> = vec![
        // quad A — XY plane (faces ±Z)
        [-0.5, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [0.5, 1.0, 0.0],
        [-0.5, 1.0, 0.0],
        // quad B — ZY plane (faces ±X)
        [0.0, 0.0, -0.5],
        [0.0, 0.0, 0.5],
        [0.0, 1.0, 0.5],
        [0.0, 1.0, -0.5],
    ];
    let normals: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    let uvs: Vec<[f32; 2]> = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];
    let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Build a `FoliageMaterial` wearing `texture` as an alpha-cutout, double-sided
/// albedo, with the given wind state.
pub fn foliage_material(texture: Handle<Image>, wind: FoliageWindUniform) -> FoliageMaterial {
    ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            perceptual_roughness: 0.9,
            ..default()
        },
        extension: FoliageWindExt { wind },
    }
}
