//! OUTDOOR block-atlas showcase binary (Poppy) — renders `block_atlas.rs` ONLY.
//!
//! Why this exists at all: the atlas ships wired into `hero.rs`, and the hero
//! scene is a KITCHEN — four walls, a ceiling and a fake sky painted on an
//! emissive pane. There is no frame in that bin where a block is seen under real
//! sky, so the one thing the atlas is for (a voxel WORLD that reads as material
//! instead of coloured cubes) cannot be photographed from it.
//!
//! So this bin builds the smallest honest outdoor stage — grass field, sand path,
//! a plaster/stone cottage with a tiled roof, an oak with a leaf canopy — out of
//! the SAME `block_atlas::load` + `block_atlas::cube_mesh` calls `hero.rs` makes.
//! It links `block_atlas.rs` and nothing else, so it cannot be blocked by (or
//! block) any other lane's file, and it needs no edit to `main.rs`.
//!
//! ## The before/after pair
//!
//! One binary, one scene, one changed env bit — the same A/B discipline the look
//! lane uses:
//!
//! ```text
//! VOXELFORGE_ATLAS_MODE=off    VOXELFORGE_SHOT=before.png   # flat colour cubes
//! VOXELFORGE_ATLAS_MODE=albedo VOXELFORGE_SHOT=after.png    # the art set's pixels
//! ```
//!
//! The BEFORE plate is not a strawman: every kind's flat colour below is the
//! measured mean sRGB of its own 16×16 tile, so the two plates carry the same
//! palette and the only difference in the frame is texture detail. If they had
//! different hues the pair would be proving a repaint, not an atlas.

// The atlas itself. Bevy + serde + image only — reaches into no other module,
// which is exactly what lets this bin stand alone.
#[path = "block_atlas.rs"]
mod block_atlas;

use bevy::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{AmbientLight, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::view::Msaa;
use bevy::window::PresentMode;

use block_atlas::{AtlasMode, BlockAtlas};

/// Frame the grab happens on, and the frame the app quits on.
///
/// Counted, not timed: the two plates of a pair must be caught at the same point
/// no matter how loaded the box is when each one runs. At the pinned 1/60 s step
/// these are 1.5 s and 2.5 s, which is plenty for a static scene (no TAA history
/// to settle here — MSAA is off and nothing moves).
const SHOT_FRAME: u32 = 90;
const EXIT_FRAME: u32 = 150;

#[derive(Resource)]
struct ShotState {
    path: Option<String>,
    took: bool,
    frame: u32,
}

fn main() -> AppExit {
    let shot = std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty());

    // Pin assets to the exe directory — same reason as `shot_main.rs`. The atlas
    // tiles themselves are read with `std::fs` (see `block_atlas::DEFAULT_ATLAS_DIR`,
    // relative to the WORKING directory) and not through Bevy's asset server, so
    // run this from the repo root or pass `VOXELFORGE_ATLAS_DIR`.
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
                        title: "Voxelforge — block atlas (outdoor)".into(),
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
        .add_systems(Startup, setup_stage)
        .add_systems(Update, screenshot_once)
        .run()
}

// ---------------------------------------------------------------------------
// palette
// ---------------------------------------------------------------------------

/// Flat colour per kind — the MEASURED mean sRGB of that kind's own tile.
///
/// Printed by a one-liner over `assets/textures/blocks/*.png` (mean of every
/// texel, /255). Kept here rather than computed at runtime on purpose: with the
/// atlas switched OFF there are no tiles to average, and a BEFORE plate that
/// needed the art set present would not be a before plate.
///
/// Where a kind wears two tiles, this is the face the camera mostly sees: the
/// field reads by its grass TOP, the trunk by its log SIDE.
fn flat_color(kind: &str) -> Color {
    match kind {
        "grass" => Color::srgb(0.314, 0.513, 0.229),
        "sand" => Color::srgb(0.855, 0.773, 0.584),
        "stone" => Color::srgb(0.440, 0.461, 0.485),
        "plaster" => Color::srgb(0.735, 0.773, 0.801),
        "plank" => Color::srgb(0.610, 0.466, 0.324),
        "floorboard" => Color::srgb(0.538, 0.493, 0.453),
        "log" => Color::srgb(0.331, 0.248, 0.192),
        "roof_tile" => Color::srgb(0.437, 0.236, 0.175),
        "leaves" => Color::srgb(0.168, 0.417, 0.160),
        "glass" => Color::srgb(0.622, 0.782, 0.840),
        _ => Color::srgb(0.8, 0.0, 0.8), // unmapped kind screams magenta
    }
}

/// Every kind the stage places, in manifest order.
const KINDS: [&str; 10] = [
    "grass",
    "sand",
    "stone",
    "plaster",
    "plank",
    "floorboard",
    "log",
    "roof_tile",
    "leaves",
    "glass",
];

/// One material + one mesh per kind.
///
/// This is the same binding `hero.rs::BlockSkins` does, minus the handle→kind
/// indirection: hero has to key off a material handle because its ~60 `put`
/// calls name palette materials, whereas this stage is authored in kinds
/// directly, so a plain map is the whole of it.
struct Blocks {
    mesh: std::collections::HashMap<String, Handle<Mesh>>,
    mat: std::collections::HashMap<String, Handle<StandardMaterial>>,
    /// Untextured unit cube — what every kind gets when the atlas is off.
    plain: Handle<Mesh>,
}

impl Blocks {
    fn build(
        atlas: Option<&BlockAtlas>,
        tex: Option<&Handle<Image>>,
        meshes: &mut Assets<Mesh>,
        mats: &mut Assets<StandardMaterial>,
    ) -> Self {
        let plain = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let mut out = Blocks {
            mesh: Default::default(),
            mat: Default::default(),
            plain: plain.clone(),
        };
        for kind in KINDS {
            let mut m = StandardMaterial {
                base_color: flat_color(kind),
                perceptual_roughness: if kind == "glass" { 0.25 } else { 0.9 },
                metallic: 0.0,
                reflectance: 0.1,
                ..default()
            };
            match (atlas, tex) {
                (Some(atlas), Some(tex)) => match atlas.face_uv(kind) {
                    Some(uv) => {
                        m.base_color_texture = Some(tex.clone());
                        // Albedo mode: the tile IS the colour, so the authored
                        // flat colour has to drop out or the art is tinted twice.
                        if atlas.mode == AtlasMode::Albedo {
                            m.base_color = Color::WHITE;
                        }
                        out.mesh
                            .insert(kind.to_string(), meshes.add(block_atlas::cube_mesh(uv)));
                    }
                    None => println!("ATLAS_STAGE missing kind {kind:?} — left flat"),
                },
                _ => {}
            }
            out.mat.insert(kind.to_string(), mats.add(m));
        }
        out
    }

    fn mesh_for(&self, kind: &str) -> Handle<Mesh> {
        self.mesh.get(kind).cloned().unwrap_or_else(|| self.plain.clone())
    }
}

// ---------------------------------------------------------------------------
// the stage
// ---------------------------------------------------------------------------

fn setup_stage(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let atlas = match block_atlas::load(None) {
        Ok(a) => a,
        Err(e) => {
            // Loud, and NOT fatal: a missing tile must not silently turn the
            // after-plate into a second before-plate.
            println!("ATLAS load failed ({e}) — flat materials kept");
            None
        }
    };
    let tex = atlas.as_ref().map(|a| images.add(a.image.clone()));
    println!(
        "ATLAS_STAGE mode={:?} textured={}",
        AtlasMode::from_env(),
        atlas.is_some()
    );
    let blocks = Blocks::build(atlas.as_ref(), tex.as_ref(), &mut meshes, &mut mats);

    // `put` is the whole authoring vocabulary — name a kind and a cell. Cells are
    // one world unit, cube centres sit on the half, so a block at (0,0,0) fills
    // x 0..1 / y 0..1 / z 0..1 exactly like the chunk mesher's cells do.
    //
    // The scene is authored into a cell list first and spawned in one pass at the
    // end, rather than spawning as it goes: `fill` calls `put`, and two closures
    // that both hold `Commands` cannot nest.
    // A single cell is `fill` with both ends of every span equal — one closure, so
    // there is only ever one live borrow of the list.
    let mut cells: Vec<(&'static str, i32, i32, i32)> = Vec::new();
    let mut fill = |kind: &'static str, x0: i32, x1: i32, y0: i32, y1: i32, z0: i32, z1: i32| {
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    cells.push((kind, x, y, z));
                }
            }
        }
    };

    // ---- ground: grass field with a sand path running to the door --------
    // Only the TOP layer is spawned: nothing under it is ever visible and 20×20
    // of hidden cubes is 400 draw calls bought for no pixels.
    for x in -10..10 {
        for z in -10..10 {
            let on_path = (2..=4).contains(&x) && z >= 2;
            fill(if on_path { "sand" } else { "grass" }, x, x, 0, 0, z, z);
        }
    }

    // ---- cottage ---------------------------------------------------------
    // Stone footing, plaster walls, oak-log corner posts, plank door, glass
    // window, tiled hip roof. Small enough to read whole in one frame, and it
    // puts six different kinds against each other at a shared edge — which is
    // where a packed atlas fails if the gutter is wrong (a wrong tile bleeds in
    // along the seam), so the stage is also the bleed test.
    let (x0, x1, z0, z1) = (-1, 5, -4, 1);
    fill("stone", x0, x1, 1, 1, z0, z1); // footing slab ring is fine as a full pad
    // walls y 2..4, hollow
    for y in 2..=4 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let edge = x == x0 || x == x1 || z == z0 || z == z1;
                if !edge {
                    continue;
                }
                let corner = (x == x0 || x == x1) && (z == z0 || z == z1);
                let door = (3..=4).contains(&x) && z == z1 && y <= 3;
                // Window on the +X wall specifically: that and +Z are the two the
                // camera sees, and a kind photographed on a wall facing away from
                // the lens is a kind this stage did not actually prove.
                let window = x == x1 && (-3..=-2).contains(&z) && y == 3;
                if door {
                    continue; // opening
                }
                let kind = if corner {
                    "log"
                } else if window {
                    "glass"
                } else {
                    "plaster"
                };
                fill(kind, x, x, y, y, z, z);
            }
        }
    }
    // plank door leaf, set one cell back in the opening
    fill("plank", 3, 4, 2, 3, z1, z1);
    // floorboard deck: a porch step OUTSIDE the door, not the interior floor — an
    // interior floor is a kind the frame never sees.
    fill("floorboard", 2, 5, 1, 1, z1 + 1, z1 + 2);
    // hip roof: two tiled courses stepping in, then a ridge
    fill("roof_tile", x0, x1, 5, 5, z0, z1);
    fill("roof_tile", x0 + 1, x1 - 1, 6, 6, z0 + 1, z1 - 1);
    fill("roof_tile", x0 + 2, x1 - 2, 7, 7, z0 + 2, z1 - 2);

    // ---- oak: log trunk + leaf canopy ------------------------------------
    fill("log", -6, -6, 1, 4, 4, 4);
    fill("leaves", -8, -4, 5, 6, 2, 6);
    fill("leaves", -7, -5, 7, 7, 3, 5);

    // ---- a low sand dune + stone outcrop, so the field is not empty ------
    fill("sand", 7, 9, 1, 1, -8, -5);
    fill("sand", 8, 9, 2, 2, -7, -6);
    fill("stone", -9, -7, 1, 2, -8, -6);

    // ---- spawn the authored cells ----------------------------------------
    for (kind, x, y, z) in &cells {
        commands.spawn((
            Mesh3d(blocks.mesh_for(kind)),
            MeshMaterial3d(blocks.mat[*kind].clone()),
            Transform::from_xyz(*x as f32 + 0.5, *y as f32 + 0.5, *z as f32 + 0.5),
        ));
    }
    println!("ATLAS_STAGE blocks={}", cells.len());

    // ---- sun: late-afternoon key, low enough to throw long block shadows -
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

    // ---- camera: 3/4 outdoor establishing ---------------------------------
    // Sky is the clear colour, not a dome: this stage exists to photograph BLOCK
    // faces, and a real sky/fog stack would put the look lane's grade between the
    // camera and the thing being measured.
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
        Transform::from_xyz(13.0, 9.5, 13.0).looking_at(Vec3::new(0.5, 3.0, -1.5), Vec3::Y),
        Msaa::Off,
        AmbientLight {
            // Sky fill: cool and modest, so the shaded block faces still read
            // their own albedo instead of being washed to one hue.
            color: Color::srgb(0.62, 0.72, 0.88),
            brightness: 900.0,
            affects_lightmapped_meshes: false,
        },
        Exposure { ev100: 10.5 },
        Tonemapping::AcesFitted,
    ));
}

/// Grab on a counted frame, then quit.
fn screenshot_once(
    mut commands: Commands,
    mut state: ResMut<ShotState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = state.path.clone() else {
        return;
    };
    state.frame += 1;
    if !state.took && state.frame >= SHOT_FRAME {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        state.took = true;
        println!("SHOT saved to {path} (frame {})", state.frame);
    }
    if state.took && state.frame >= EXIT_FRAME {
        exit.write(AppExit::Success);
    }
}
