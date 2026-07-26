//! Voxelforge — Phase 0 go/no-go spike (Rust + Bevy 0.19 + wgpu).
//!
//! Proves: chunk gen -> greedy mesh -> texture-atlas StandardMaterial -> fly
//! camera, on both native (wgpu/DX12/Vulkan) and web (WebGPU). Ships a built-in
//! ramp benchmark that spawns more chunks every couple seconds and reports the
//! largest chunk count that still holds >= 60 FPS.

mod hero;
mod voxel;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::text::FontSize;
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow};
use std::collections::HashSet;

use voxel::{build_atlas, Chunk, CHUNK};

// ---------------------------------------------------------------------------
// Config (env vars on native, ?query params on web)
// ---------------------------------------------------------------------------

#[derive(Resource, Clone)]
struct Cfg {
    bench: bool,
    grid: usize,
    shot: Option<String>,
    /// Phase 1 hero look-shot scene instead of terrain.
    hero: bool,
    /// Force a present mode (root-causing the FPS cliff): one of
    /// vsync|novsync|fifo|mailbox|immediate. None => auto (novsync in bench).
    present: Option<String>,
    /// Cold-boot the ramp at this grid side instead of 1 (cliff isolation).
    start_side: i32,
    // Hero-shot tunables (env-driven so the shot re-frames without a recompile).
    cam: Option<[f32; 7]>, // ex,ey,ez, tx,ty,tz, fov_deg
    sun: Option<[f32; 3]>, // elevation_deg, azimuth_deg, illuminance
    dof: Option<[f32; 2]>, // focal_distance, aperture_f_stops
    fog: Option<f32>,      // volumetric density_factor
    exposure: Option<f32>, // camera ev100
    // Post color-grade (P0 blue-wash / saturation / micro-contrast): tempers the
    // frame AFTER tonemap so blue-wash / flat saturation / soft grain tune without
    // touching per-material colour. [temperature, post_saturation, contrast].
    grade: Option<[f32; 3]>,
    // Hero bounce/highlight knobs — env-driven so the 3 hardest gates
    // (G3 shadow floor, G5/G6 window roll-off) tune WITHOUT a recompile.
    ambient: Option<f32>,  // AmbientLight brightness (lux)
    emissive: Option<f32>, // scale on the window pane emissive
    dfog: Option<f32>,     // DistanceFog density
}

/// Parse "a,b,c" env into a fixed float array (all-or-nothing).
#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn read_cfg() -> Cfg {
    Cfg {
        bench: std::env::var("VOXELFORGE_BENCH").is_ok(),
        grid: std::env::var("VOXELFORGE_GRID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6),
        shot: std::env::var("VOXELFORGE_SHOT").ok().filter(|s| !s.is_empty()),
        hero: std::env::var("VOXELFORGE_HERO").is_ok(),
        present: std::env::var("VOXELFORGE_PRESENT").ok().filter(|s| !s.is_empty()),
        start_side: std::env::var("VOXELFORGE_START_SIDE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1)
            .max(1),
        cam: env_floats("VOXELFORGE_CAM"),
        sun: env_floats("VOXELFORGE_SUN"),
        dof: env_floats("VOXELFORGE_DOF"),
        fog: std::env::var("VOXELFORGE_FOG").ok().and_then(|v| v.parse().ok()),
        exposure: std::env::var("VOXELFORGE_EXPOSURE").ok().and_then(|v| v.parse().ok()),
        grade: env_floats("VOXELFORGE_GRADE"),
        ambient: std::env::var("VOXELFORGE_AMBIENT").ok().and_then(|v| v.parse().ok()),
        emissive: std::env::var("VOXELFORGE_EMISSIVE").ok().and_then(|v| v.parse().ok()),
        dfog: std::env::var("VOXELFORGE_DFOG").ok().and_then(|v| v.parse().ok()),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_cfg() -> Cfg {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    let bench = search.contains("bench");
    let grid = search
        .split(['?', '&'])
        .find_map(|kv| kv.strip_prefix("grid=").and_then(|v| v.parse().ok()))
        .unwrap_or(6);
    Cfg {
        bench,
        grid,
        shot: None,
        hero: search.contains("hero"),
        present: None,
        start_side: 1,
        cam: None,
        sun: None,
        dof: None,
        fog: None,
        exposure: None,
        grade: None,
        ambient: None,
        emissive: None,
        dfog: None,
    }
}

#[cfg(target_arch = "wasm32")]
fn set_dom(id: &str, text: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
    {
        el.set_text_content(Some(text));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_dom(_id: &str, _text: &str) {}

// ---------------------------------------------------------------------------

#[derive(Resource)]
struct World {
    material: Handle<StandardMaterial>,
    spawned: HashSet<(i32, i32)>,
    total_quads: usize,
}

#[derive(Component)]
struct FlyCam {
    yaw: f32,
    pitch: f32,
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

const WARMUP: f32 = 1.2;
const PHASE: f32 = 2.0;
const MAX_SIDE: i32 = 32; // up to 1024 chunks (ramp stops early once FPS dips <55)

fn main() {
    let cfg = read_cfg();

    // Present mode: explicit override wins (for root-causing the FPS cliff),
    // else uncapped in bench so we can read the TRUE GPU ceiling (VSync would
    // pin every reading at ~60 and hide all headroom).
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
            .set(ImagePlugin::default_nearest()),
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
    });

    if cfg.hero {
        // Phase 1 look-shot: kitchen + full post stack, then screenshot & exit.
        app.insert_resource(cfg)
            .add_systems(Startup, hero::setup_hero)
            .add_systems(Update, (fly_camera, screenshot_once));
    } else {
        app.insert_resource(cfg)
            .add_systems(Startup, setup)
            .add_systems(Update, (fly_camera, hud, bench_ramp, screenshot_once));
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<Cfg>,
    mut bench: ResMut<Bench>,
) {
    let atlas = images.add(build_atlas());
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(atlas),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        reflectance: 0.1,
        ..default()
    });

    let mut world = World {
        material,
        spawned: HashSet::new(),
        total_quads: 0,
    };

    // Sun.
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(60.0, 120.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Fly camera — vantage that frames the initial grid.
    // In bench mode start from bench.side (usually 1, but VOXELFORGE_START_SIDE
    // can cold-boot a big grid to test whether the FPS cliff is load- or ramp-triggered).
    let side = if bench.active { bench.side } else { cfg.grid as i32 };
    let c = side as f32 * CHUNK as f32 * 0.5;
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-CHUNK as f32 * 0.7, CHUNK as f32 * 1.6, -CHUNK as f32 * 0.7)
            .looking_at(Vec3::new(c, 8.0, c), Vec3::Y),
        FlyCam { yaw: 0.0, pitch: 0.0 },
        AmbientLight {
            brightness: 380.0,
            ..default()
        },
    ));

    // Initial chunks.
    for z in 0..side {
        for x in 0..side {
            spawn_chunk(&mut commands, &mut meshes, &mut world, x, z);
        }
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

    commands.insert_resource(world);
    bench.phase_start = 0.0;
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    x: i32,
    z: i32,
) {
    if !world.spawned.insert((x, z)) {
        return;
    }
    let chunk = Chunk::generate(x, z);
    let (mesh, quads) = chunk.greedy_mesh();
    world.total_quads += quads;
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(world.material.clone()),
        Transform::from_xyz((x * CHUNK) as f32, 0.0, (z * CHUNK) as f32),
    ));
}

fn fly_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut cam: Query<(&mut Transform, &mut FlyCam)>,
) {
    let Ok((mut tf, mut fly)) = cam.single_mut() else {
        return;
    };
    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };

    // grab / release cursor
    if mouse_btn.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }

    if cursor.grab_mode == CursorGrabMode::Locked {
        let mut delta = Vec2::ZERO;
        for ev in motion.read() {
            delta += ev.delta;
        }
        fly.yaw -= delta.x * 0.0025;
        fly.pitch = (fly.pitch - delta.y * 0.0025).clamp(-1.54, 1.54);
        tf.rotation = Quat::from_axis_angle(Vec3::Y, fly.yaw)
            * Quat::from_axis_angle(Vec3::X, fly.pitch);
    } else {
        motion.clear();
    }

    // movement
    let mut dir = Vec3::ZERO;
    let f = tf.forward();
    let r = tf.right();
    if keys.pressed(KeyCode::KeyW) {
        dir += *f;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= *f;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += *r;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= *r;
    }
    if keys.pressed(KeyCode::Space) {
        dir += Vec3::Y;
    }
    if keys.pressed(KeyCode::ShiftLeft) {
        dir -= Vec3::Y;
    }
    let speed = if keys.pressed(KeyCode::ControlLeft) {
        90.0
    } else {
        28.0
    };
    if dir != Vec3::ZERO {
        tf.translation += dir.normalize() * speed * time.delta_secs();
    }
}

fn hud(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    world: Option<Res<World>>,
    bench: Res<Bench>,
    mut q: Query<&mut Text, With<HudText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let chunks = world.as_ref().map(|w| w.spawned.len()).unwrap_or(0);
    let quads = world.as_ref().map(|w| w.total_quads).unwrap_or(0);
    let line = if bench.active {
        format!(
            "FPS {fps:.0}  |  chunks {chunks}  |  quads {quads}  |  BENCH ramp  |  best>=60fps: {} chunks",
            bench.max_60
        )
    } else {
        format!("FPS {fps:.0}  |  chunks {chunks}  |  quads {quads}  |  click=look WASD=move Ctrl=fast")
    };
    if let Ok(mut text) = q.single_mut() {
        text.0 = line.clone();
    }
    set_dom("fps", &format!("{fps:.0}"));
    set_dom("status", &line);
}

/// The ramp benchmark: warm up, then every PHASE seconds compute the median FPS
/// for the current chunk count, grow the grid, and remember the largest count
/// that held >= 60 FPS. On native, exits when done so a script can read stdout.
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

    // ---- close out this phase ----
    let mut ms = std::mem::take(&mut bench.samples);
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_ms = if ms.is_empty() {
        0.0
    } else {
        ms[ms.len() / 2]
    };
    let fps = if median_ms > 0.0 { 1000.0 / median_ms } else { 0.0 };
    let chunks = (bench.side * bench.side) as usize;
    let line = format!(
        "BENCH chunks={chunks} side={} fps={fps:.1} median_ms={median_ms:.2} quads={}",
        bench.side, world.total_quads
    );
    println!("{line}");
    bench.log.push_str(&line);
    bench.log.push('\n');
    let full_log = bench.log.clone();
    set_dom("bench", &full_log);

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
        set_dom("result", &summary);
        #[cfg(not(target_arch = "wasm32"))]
        {
            exit.write(AppExit::Success);
        }
        return;
    }

    // grow the grid by one ring and reframe the camera
    let new_side = bench.side + 1;
    for z in 0..new_side {
        for x in 0..new_side {
            spawn_chunk(&mut commands, &mut meshes, &mut world, x, z);
        }
    }
    bench.side = new_side;
    bench.phase_start = now;
    let _ = &mut exit; // silence unused on wasm
}

/// Optional: grab one screenshot a few seconds in, then exit (native only).
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
        return; // don't mix with the ramp
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
        exit.write(AppExit::Success);
    }
}
