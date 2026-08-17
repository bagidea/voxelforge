//! Beach-dusk hero shot rig (Yamamoto) — `VOXELFORGE_BEACHSHOT=1`.
//!
//! Frames the golden-hour beach-cabin scene (`maps/beach_dusk.json`, loaded the
//! normal way via `VOXELFORGE_MAP_LOAD`) like a hero shot: a fixed camera pose
//! (same 7-float `VOXELFORGE_CAM` convention `hero.rs` reads, applied here to the
//! REAL gameplay `OrbitCam` instead of a second camera) plus the hand-built props
//! the map file can't express — water, campfire, boat, flower pots. See
//! `docs/hero-scene-beach-dusk.md` §5/§5.1 for the design this implements.
//!
//! Self-contained: does not touch `hero.rs`, `scene.rs` or `look.rs`.
//! `spawn_campfire` below is a smaller stand-alone copy of `scene.rs`'s (private
//! there, and scene.rs is another lane's file, currently mid-edit) — same
//! ring-of-stones idea, no shared code path.

use bevy::prelude::*;

use crate::{Cfg, FlyCam, OrbitCam};

/// `(ex,ey,ez, tx,ty,tz, fov_deg)` — `docs/hero-scene-beach-dusk.md` §3's starting
/// pose (beach-side, looking across the cabin toward the dock/water). Override
/// with `VOXELFORGE_CAM=ex,ey,ez,tx,ty,tz,fov` (same layout `hero.rs` reads)
/// without a recompile. `fov_deg` is accepted for parity with that convention but
/// not yet applied here (default `Camera3d` projection) — a possible follow-up if
/// the default FOV doesn't match the reference framing.
const DEFAULT_CAM: [f32; 7] = [18.0, 10.0, 6.0, 29.0, 4.0, 30.0, 60.0];

/// One-time dressing: hides the player avatar (this is a static hero shot, no
/// player in frame — same convention `hero.rs` uses for its kitchen shot) and
/// spawns the props `beach_dusk.json` cannot carry.
pub fn setup_beach_shot(
    mut commands: Commands,
    cfg: Res<Cfg>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    avatar: Query<Entity, With<FlyCam>>,
) {
    if !cfg.beachshot {
        return;
    }
    for e in &avatar {
        commands.entity(e).insert(Visibility::Hidden);
    }
    spawn_water(&mut commands, &mut meshes, &mut materials);
    spawn_campfire(&mut commands, &mut meshes, &mut materials, Vec3::new(23.0, 1.0, 14.0));
    spawn_boat(&mut commands, &mut meshes, &mut materials, Vec3::new(32.5, 1.2, 50.0));
    spawn_flower_pot(&mut commands, &mut meshes, &mut materials, Vec3::new(27.0, 1.0, 25.0));
    spawn_flower_pot(&mut commands, &mut meshes, &mut materials, Vec3::new(30.5, 1.0, 25.0));
}

/// Poses the REAL gameplay `OrbitCam` at a fixed hero-shot transform every frame
/// — the same trick `combat::lock_on_camera` uses to win the final word over
/// `fly_camera`'s per-frame orbit recompute (wired in `main.rs` as
/// `.after(fly_camera)`), so this stays put instead of snapping back to the
/// avatar-relative boom pose one frame after spawn.
pub fn pose_beach_camera(cfg: Res<Cfg>, mut cam_q: Query<&mut Transform, With<OrbitCam>>) {
    if !cfg.beachshot {
        return;
    }
    let [ex, ey, ez, tx, ty, tz, _fov_deg] = cfg.cam.unwrap_or(DEFAULT_CAM);
    let Ok(mut tf) = cam_q.single_mut() else {
        return;
    };
    *tf = Transform::from_xyz(ex, ey, ez).looking_at(Vec3::new(tx, ty, tz), Vec3::Y);
}

/// Reflective water quad over the trench footprint (`maps/beach_dusk.json` leaves
/// `z>40` as air on purpose — see the map doc §2). Low `perceptual_roughness`
/// under the existing IBL (`look::apply_ibl`, forced on with
/// `VOXELFORGE_LOOK_FORCE=1`) gets real reflections without a shader — see
/// `docs/hero-scene-beach-dusk.md` §5 option A. No `.wgsl` involved.
fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(32.0, 12.0)));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.05, 0.17, 0.22, 0.92),
        perceptual_roughness: 0.06,
        reflectance: 0.6,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(32.0, 0.4, 52.0),
    ));
}

/// A small ring-of-stones campfire — same idea as `scene.rs::spawn_campfire`
/// (crossed logs + emissive ember + point light), rebuilt here stand-alone since
/// that one is private and scene.rs is another lane's file. No flicker animation
/// (this is a still-image rig, not `--play`) — a static emissive block reads fine
/// in one frame.
fn spawn_campfire(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    foot: Vec3,
) {
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.42, 0.45),
        perceptual_roughness: 0.9,
        ..default()
    });
    let log = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.19, 0.10),
        perceptual_roughness: 0.85,
        ..default()
    });
    let ember = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.16),
        emissive: LinearRgba::rgb(6.0, 2.4, 0.5),
        ..default()
    });
    let stone_mesh = meshes.add(Cuboid::new(0.34, 0.28, 0.34));
    let log_mesh = meshes.add(Cuboid::new(1.05, 0.20, 0.20));
    let flame_mesh = meshes.add(Cuboid::new(0.36, 0.5, 0.36));

    commands
        .spawn((Transform::from_translation(foot), Visibility::default()))
        .with_children(|p| {
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                p.spawn((
                    Mesh3d(stone_mesh.clone()),
                    MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(a.cos() * 0.85, 0.14, a.sin() * 0.85)
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, a)),
                ));
            }
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                p.spawn((
                    Mesh3d(log_mesh.clone()),
                    MeshMaterial3d(log.clone()),
                    Transform::from_xyz(0.0, 0.16, 0.0)
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, a)),
                ));
            }
            p.spawn((
                Mesh3d(flame_mesh),
                MeshMaterial3d(ember),
                Transform::from_xyz(0.0, 0.45, 0.0),
            ));
            p.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.62, 0.28),
                    intensity: 260_000.0,
                    range: 22.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.9, 0.0),
            ));
        });
}

/// A small rowboat hull + mast, hand-built primitives (no boat asset exists in
/// this repo) — `docs/hero-scene-beach-dusk.md` §5.1 step 3. Floats beside the
/// dock, at the water plane's height.
fn spawn_boat(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
) {
    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.24, 0.13),
        perceptual_roughness: 0.75,
        ..default()
    });
    let mast_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.11),
        perceptual_roughness: 0.7,
        ..default()
    });
    let hull = meshes.add(Cuboid::new(1.3, 0.5, 3.2));
    let mast = meshes.add(Cylinder::new(0.05, 1.6));

    commands
        .spawn((Transform::from_translation(at), Visibility::default()))
        .with_children(|p| {
            p.spawn((Mesh3d(hull), MeshMaterial3d(hull_mat)));
            p.spawn((
                Mesh3d(mast),
                MeshMaterial3d(mast_mat),
                Transform::from_xyz(0.0, 1.05, 0.0),
            ));
        });
}

/// A terracotta pot with a small green canopy — hand-built primitives flanking
/// the cabin door, `docs/hero-scene-beach-dusk.md` §4 "flower pots" row.
fn spawn_flower_pot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
) {
    let terracotta = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.32, 0.20),
        perceptual_roughness: 0.85,
        ..default()
    });
    let foliage = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.46, 0.20),
        perceptual_roughness: 0.8,
        ..default()
    });
    let pot = meshes.add(Cylinder::new(0.28, 0.4));
    let bloom = meshes.add(Sphere::new(0.32));

    commands
        .spawn((Transform::from_translation(at), Visibility::default()))
        .with_children(|p| {
            p.spawn((
                Mesh3d(pot),
                MeshMaterial3d(terracotta),
                Transform::from_xyz(0.0, 0.2, 0.0),
            ));
            p.spawn((
                Mesh3d(bloom),
                MeshMaterial3d(foliage),
                Transform::from_xyz(0.0, 0.55, 0.0),
            ));
        });
}
