//! Enemy-AI proof binary (Rose) — the chase clip.
//!
//! Renders Monanisa's enemy bodies (`enemies.rs`, `#[path]`-included
//! UNMODIFIED) driven by Rose's behaviour (`enemy_ai.rs`) against a scripted
//! player dummy, and captures one PNG every other frame so ffmpeg can
//! assemble the CEO-facing clip of enemies patrolling → alerting → pursuing
//! → telegraphing → striking.
//!
//! Exists for the same reason `char_shot_main.rs` does: `voxelforge_shot`
//! links `hero.rs` + `vfx.rs`, and the real `voxelforge` bin links
//! `combat.rs` (Kevin) + `main.rs` (Kevin) — neither of which this lane may
//! edit. This bin links `enemies.rs` + `enemy_ai.rs` only, so an enemy-AI
//! proof can never be blocked by (or block) another lane's mid-edit file.
//!
//! ```text
//!   VOXELFORGE_AIFRAMES=<dir>   where the numbered PNG frames go
//!                                (default `_enemyai_frames`)
//!   VOXELFORGE_AILOG=<path>     per-frame CSV trace: frame,enemy,state,x,z,dist
//!                                (default `<dir>/trace.csv`)
//! ```
//!
//! Assemble the clip afterwards with:
//!   ffmpeg -framerate 30 -i <dir>/f%04d.png -c:v libx264 -pix_fmt yuv420p enemy-chase.mp4
//! (capture cadence is every 2nd app frame, so 30fps output ≈ realtime if the
//! app holds 60fps; the CSV trace, not the clip duration, is the timing truth)

#[path = "enemies.rs"]
mod enemies;

#[path = "enemy_ai.rs"]
mod enemy_ai;

use bevy::camera::{Camera, ClearColorConfig, PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::AmbientLight;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::view::Msaa;
use bevy::window::PresentMode;
use std::fmt::Write as _;
use std::io::Write as _;

/// Which `Archetype` each of Monanisa's kinds maps to. This one match IS the
/// whole visual↔behaviour contract — in the real game this line happens in
/// the combat lane's spawn path, not here.
fn archetype_for(kind: enemies::EnemyKind) -> enemy_ai::Archetype {
    match kind {
        enemies::EnemyKind::Reaver => enemy_ai::Archetype::Swarm,
        enemies::EnemyKind::Sentinel => enemy_ai::Archetype::Bruiser,
        enemies::EnemyKind::Stalker => enemy_ai::Archetype::Pouncer,
    }
}

#[derive(Resource)]
struct Capture {
    dir: String,
    frame: u32,
    /// start a little late so the first captured frame isn't mid-settle
    start: u32,
    end: u32,
    took: u32,
    trace: std::fs::File,
}

/// The scripted player route — the "prey" side of the chase. Times are in
/// virtual seconds (`elapsed_secs`), phases chosen so the clip shows all four
/// behaviours: patrol-and-alert (walk INTO them), chase (flee), the swarm
/// catching up, and the pouncer orbiting between dashes.
#[derive(Resource)]
struct Route {
    phase: u32,
}

const PLAYER_SPEED: f32 = 3.8;

fn route_target(t: f32) -> (Vec3, &'static str) {
    if t < 2.0 {
        // stand — enemies are far, they patrol
        (Vec3::new(0.0, 0.0, 10.0), "stand")
    } else if t < 7.0 {
        // walk toward the pack — provoke the alert beats
        (Vec3::new(2.0, 0.0, -4.0), "approach")
    } else if t < 7.8 {
        // freeze — the classic horror beat: it noticed you and stopped
        (Vec3::new(2.0, 0.0, -4.0), "freeze")
    } else if t < 14.5 {
        // FLEE across the arena — the chase the clip exists to prove
        (Vec3::new(-16.0, 0.0, 12.0), "flee")
    } else {
        // cornered: stop, get surrounded/orbited/struck from all sides
        (Vec3::new(-16.0, 0.0, 12.0), "cornered")
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---- ground: dark arena plane ----
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x18, 0x1A, 0x1E),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(64.0, 1.0, 64.0))),
        MeshMaterial3d(ground),
        Transform::from_xyz(0.0, -0.5, 0.0),
        Visibility::default(),
    ));

    // ---- lighting: dim moonlight, same convention as the enemy shot's night
    // branch, so the emissive eyes are the loudest read in frame ----
    let dir = Vec3::new(0.35, -0.55, 0.45).normalize();
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.55, 0.62, 0.85),
            illuminance: 700.0,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.06,
            shadow_normal_bias: 1.4,
            ..default()
        },
        Transform::from_translation(-dir * 40.0).looking_to(dir, Vec3::Y),
    ));

    // ---- the player dummy: warm lantern-bearer, deliberately the ONLY warm
    // light in frame — the eye reads "that is what they hunt" ----
    let body = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x8A, 0x6F, 0x4D),
        perceptual_roughness: 0.85,
        ..default()
    });
    let lantern = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0xFF, 0xD9, 0x8A),
        emissive: LinearRgba::rgb(6.0, 3.6, 1.2),
        ..default()
    });
    commands.spawn((
        enemy_ai::AiPlayer,
        Transform::from_xyz(0.0, 0.0, 10.0),
        Visibility::default(),
        Name::new("Player (scripted)"),
    )).with_children(|p| {
        p.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.55, 0.95, 0.35))),
            MeshMaterial3d(body.clone()),
            Transform::from_xyz(0.0, 0.75, 0.0),
            Visibility::default(),
        ));
        p.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.35, 0.32, 0.35))),
            MeshMaterial3d(body),
            Transform::from_xyz(0.0, 1.42, 0.0),
            Visibility::default(),
        ));
        p.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.22, 0.28, 0.22))),
            MeshMaterial3d(lantern),
            Transform::from_xyz(0.42, 1.1, 0.1),
            Visibility::default(),
        ));
        p.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.78, 0.45),
                intensity: 90_000.0,
                range: 18.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(0.42, 1.1, 0.1),
        ));
    });

    // ---- the cast: 2 Reavers (Swarm), 1 Sentinel (Bruiser), 1 Stalker
    // (Pouncer) — spread so the first phase reads as patrol, not a wall ----
    let cast: [(enemies::EnemyKind, Vec3); 4] = [
        (enemies::EnemyKind::Reaver, Vec3::new(-7.0, 0.0, -6.0)),
        (enemies::EnemyKind::Reaver, Vec3::new(-4.0, 0.0, -9.0)),
        (enemies::EnemyKind::Sentinel, Vec3::new(9.0, 0.0, -8.0)),
        (enemies::EnemyKind::Stalker, Vec3::new(3.0, 0.0, -14.0)),
    ];
    for (i, (kind, at)) in cast.iter().enumerate() {
        let root = enemies::spawn_enemy(&mut commands, &mut meshes, &mut materials, *kind, *at, 0.0);
        enemy_ai::attach_mind(&mut commands, root, i as u32 + 1, archetype_for(*kind), *at);
    }

    // ---- camera: third-person follow behind the player, wide enough to keep
    // the converging pack in frame. AmbientLight rides the camera entity —
    // the pattern `enemies.rs`'s shot stage proved on this Bevy 0.19 ----
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.012, 0.014, 0.020)),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection { fov: 62f32.to_radians(), near: 0.05, ..default() }),
            Transform::from_xyz(0.0, 7.0, 18.0).looking_at(Vec3::new(0.0, 1.0, 10.0), Vec3::Y),
            Msaa::Off,
            Tonemapping::AcesFitted,
        ))
        .insert(AmbientLight {
            color: Color::srgb(0.10, 0.13, 0.22),
            brightness: 240.0,
            affects_lightmapped_meshes: false,
        });

    println!(
        "AIPROOF cast: reaver#1 reaver#2 (swarm), sentinel (bruiser), stalker (pouncer); player scripted 4 phases"
    );
}

/// Move the scripted player along `route_target`, facing the direction of
/// travel (the enemies key off its transform; this is the whole "input").
fn drive_player(mut player: Query<&mut Transform, (With<enemy_ai::AiPlayer>, Without<EnemyMind4Proof>)>, time: Res<Time>) {
    let Ok(mut ptf) = player.single_mut() else { return };
    let t = time.elapsed_secs();
    let (target, phase) = route_target(t);
    let mut step = target - ptf.translation;
    step.y = 0.0;
    let d = step.length();
    if d > 0.05 {
        let dir = step / d;
        let move_by = (PLAYER_SPEED * time.delta_secs()).min(d);
        ptf.translation += dir * move_by;
        let yaw = (-dir.x).atan2(-dir.z);
        ptf.rotation = Quat::from_axis_angle(Vec3::Y, yaw);
    }
    let _ = phase;
}

/// Alias so `drive_player`'s Without<> filter can exclude minds without
/// importing the component path twice.
type EnemyMind4Proof = enemy_ai::EnemyMind;

/// Smooth follow camera: behind + above the player, looking where the player
/// is headed, so the pursuit stays readable through direction changes.
fn follow_cam(
    player: Query<&Transform, With<enemy_ai::AiPlayer>>,
    mut cam: Query<&mut Transform, (With<Camera>, Without<enemy_ai::AiPlayer>)>,
    time: Res<Time>,
) {
    let Ok(ptf) = player.single() else { return };
    let Ok(mut ctf) = cam.single_mut() else { return };
    let fwd = ptf.forward();
    let want = ptf.translation - fwd * 7.5 + Vec3::Y * 4.2;
    let k = 1.0 - (-4.5 * time.delta_secs()).exp();
    ctf.translation = ctf.translation.lerp(want, k);
    ctf.look_at(ptf.translation + fwd * 2.0 + Vec3::Y * 0.8, Vec3::Y);
}

/// Capture every other frame from `start` to `end`, write the CSV trace, exit.
fn capture(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    minds: Query<(&Transform, &enemy_ai::EnemyMind)>,
    player: Query<&Transform, With<enemy_ai::AiPlayer>>,
    mut exit: MessageWriter<AppExit>,
    time: Res<Time>,
) {
    cap.frame += 1;
    let f = cap.frame;

    // CSV trace regardless of capture cadence — it is the numeric truth the
    // clip is a rendering of
    if let Ok(ptf) = player.single() {
        let mut line = String::new();
        for (tf, mind) in &minds {
            let _ = write!(
                line,
                "{},{},{},{},{:.2},{:.2},{:.2}\n",
                f,
                mind.id,
                mind.state.id(),
                mind.archetype.id(),
                tf.translation.x,
                tf.translation.z,
                tf.translation.distance(ptf.translation)
            );
        }
        let _ = cap.trace.write_all(line.as_bytes());
    }

    if f >= cap.start && f <= cap.end && f % 2 == 0 {
        let path = format!("{}/f{:04}.png", cap.dir, f);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
        cap.took += 1;
        if f % 120 == 0 {
            println!("AIPROOF frame {}/{} captured {} (t={:.1}s)", f, cap.end, cap.took, time.elapsed_secs());
        }
    }
    if f > cap.end {
        println!("AIPROOF DONE frames={} captured={} — assembling clip next", cap.frame, cap.took);
        exit.write(AppExit::Success);
    }
}

fn main() -> AppExit {
    let dir = std::env::var("VOXELFORGE_AIFRAMES").unwrap_or_else(|_| "_enemyai_frames".into());
    let trace_path = std::env::var("VOXELFORGE_AILOG").unwrap_or_else(|_| format!("{dir}/trace.csv"));
    let _ = std::fs::create_dir_all(&dir);
    let trace = std::fs::File::create(&trace_path).expect("create trace.csv");
    let mut trace = std::io::BufWriter::new(trace);
    let _ = writeln!(trace, "frame,enemy,state,archetype,x,z,dist");

    // Pin assets to the exe directory — same rationale as char_shot_main.rs.
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();

    // 22s of sim; capture (start..end] every 2nd frame → ~20s of 30fps clip
    // Named `capture_res`, NOT `capture`: the fn item `capture` must stay
    // reachable by name for `capture.after(follow_cam)` below — a local named
    // `capture` shadows it and rustc reads `.after` off the struct instead.
    let capture_res = Capture {
        dir,
        frame: 0,
        start: 60,
        end: 1320,
        took: 0,
        trace: trace.into_inner().expect("flush trace header"),
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: exe_dir.join("assets").to_string_lossy().to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Voxelforge — enemy AI proof".into(),
                    resolution: (1280u32, 720u32).into(),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(capture_res)
    .insert_resource(Route { phase: 0 })
    .add_plugins(enemy_ai::EnemyAiPlugin) // Arena + seeded AiRng + the AI system (in EnemyAiSet)
    .add_systems(Startup, setup)
    // The plugin above is the ONE place `enemy_ai` is added. This bin used to
    // add it a second time and then order against the system's type-set — two
    // instances made the ordering ambiguous and Bevy rejected the whole
    // Update schedule (the runlog's schedule.rs:566 panic: spawn ran, no mind
    // ever moved). Neighbours now order against `EnemyAiSet`, which cannot go
    // ambiguous. Run order: drive_player → enemy_ai → follow_cam → capture.
    .add_systems(Update, drive_player.before(enemy_ai::EnemyAiSet))
    .add_systems(Update, follow_cam.after(enemy_ai::EnemyAiSet))
    .add_systems(Update, capture.after(follow_cam));
    println!("AIPROOF bin: link=enemies.rs+enemy_ai.rs only — no combat.rs, no main.rs");
    app.run()
}
