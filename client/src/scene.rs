//! Scene assembly — the world `--play` actually drops you into.
//!
//! Everything else in the client is *tooling*: the editor shell builds a world, the
//! hero branch renders a look-shot, each scripted demo proves one system. This is the
//! seam that turns those parts into a game you can launch and play, in the order
//! `docs/first-playable-loop.md` Act 0 asks for:
//!
//! 1. **World** — Shiba's hand-built [`PLAY_MAP`] when it lands on disk; until then a
//!    flattened campsite clearing on Kevin's procedural terrain, so nothing blocks.
//! 2. **Body** — the avatar stands flush on a real surface, already WALKing (never a
//!    floating body), with the third-person orbit camera on its boom behind it.
//! 3. **Campfire** — lit, three blocks in front of where you wake up, so the first
//!    thing you want to do is walk toward it.
//! 4. **Death is a door** — Kevin's [`combat::PlayerDied`] → fade to black → fade back
//!    in at the fire with full HP, enemies reset. No death screen.
//!
//! It is deliberately additive: every system here is gated on `--play`, so the bench,
//! the shots and all the existing headless proofs behave exactly as before.

use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::CHUNK_SIZE as CHUNK;

use crate::combat;
use crate::editor::AppState;
use crate::{
    box_fill, find_spawn, highest_solid, Cfg, FlyCam, OrbitCam, World, BOOM_DIST, EYE_HEIGHT,
    PIVOT_UP,
};

/// The hand-built Village of Edhari (`docs/first-playable-loop.md` Act 0). Shiba owns
/// this file; the scene loads it the moment it appears and falls back to procedural
/// terrain until then — so a missing map is a *plainer* game, never a broken one.
pub const PLAY_MAP: &str = "maps/edhari.json";

/// The map `--play` should boot, or `None` to use procedural terrain. Native only:
/// the browser has no filesystem, so the web build always takes the fallback.
#[cfg(not(target_arch = "wasm32"))]
pub fn play_map() -> Option<String> {
    std::path::Path::new(PLAY_MAP)
        .exists()
        .then(|| PLAY_MAP.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn play_map() -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Tunables
// ---------------------------------------------------------------------------

/// Radius (in voxels) of the flattened clearing cut around the campfire on
/// procedural terrain, so the fire and the avatar stand on a level floor.
const CLEARING_R: i32 = 4;
/// Head-room carved above the clearing (kills any tree/boulder over the campsite).
const CLEARING_HEAD: i32 = 6;
/// How far in front of the spawn the fire is lit ("a campfire already lit 3 blocks away").
const FIRE_AHEAD: f32 = 3.0;
/// Fade-to-black / fade-back-in duration on death (design doc: 0.5 s each way).
const FADE: f32 = 0.5;
/// How long "Rest. Try again." stays on screen after a respawn.
const NOTICE: f32 = 3.0;
/// Camera pitch the player wakes up with (looking slightly down at their own back).
const WAKE_PITCH: f32 = -0.25;

// ---------------------------------------------------------------------------
// World state
// ---------------------------------------------------------------------------

/// The campsite: where the player wakes up and where every death returns them.
/// `eye` is the avatar's Transform (eye height, the convention `move_body` uses).
#[derive(Resource, Debug, Clone, Copy)]
pub struct Campsite {
    pub eye: Vec3,
    pub yaw: f32,
    /// Foot of the fire itself — the thing the player walks toward.
    pub fire: Vec3,
}

/// Marks the campfire's flame + light so it can flicker.
#[derive(Component)]
struct Flame {
    /// Phase offset so the light and the mesh do not pulse in lock-step.
    phase: f32,
}

/// Full-screen black plate used for the death fade.
#[derive(Component)]
struct DeathFade;

/// The "Rest. Try again." line.
#[derive(Component)]
struct RestNotice;

/// Death → respawn timeline state.
#[derive(Resource, Default)]
struct Death {
    /// Seconds since the player died; `None` while alive.
    since: Option<f32>,
    /// Set once the mid-fade teleport has happened (so it fires exactly once).
    moved: bool,
    /// Countdown left on the notice line.
    notice: f32,
}

/// Bookkeeping for the scripted `--play-demo` proof.
#[derive(Resource, Default)]
struct PlayProof {
    start: Option<Vec3>,
    /// Camera yaw at the moment the proof takes the controls, so the log can show
    /// the *look* moved too and not just the body.
    start_yaw: Option<f32>,
    /// The one left-click that captures the cursor (mouse-look is off until then).
    clicked: bool,
    walk_logged: bool,
    killed: bool,
    respawn_logged: bool,
}

/// Bookkeeping for the scripted `--combat-demo` proof of the full combat loop:
/// hit → HP drops → kill the husk → die → respawn at the campfire.
#[derive(Resource, Default)]
struct CombatProof {
    phase: u8,
    /// Player HP at phase 0 (before any action).
    player_hp0: Option<f32>,
    /// Husk HP at phase 0.
    husk_hp0: Option<f32>,
    /// Husk HP after the first hit lands.
    husk_hp1: Option<f32>,
    /// true once the husk's HP has dropped (proving hit detection).
    husk_hit: bool,
    /// true once the husk has been despawned or its HP reached 0.
    husk_dead: bool,
    /// true once the player died and the event was seen.
    died: bool,
    /// true once the respawn has completed.
    respawned: bool,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Boots the playable scene. Everything is gated on [`Cfg::play`], so adding the
/// plugin unconditionally changes nothing for the editor/bench/shot runs.
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Death>()
            .init_resource::<PlayProof>()
            .init_resource::<CombatProof>()
            // PostStartup, not Startup: `setup` spawns the world + avatar with
            // `Commands`, and those are only applied once the Startup schedule ends.
            // By PostStartup the `World` resource and the avatar entity really exist.
            .add_systems(PostStartup, boot_scene.run_if(playing))
            .add_systems(
                Update,
                (
                    flicker_flame,
                    // The scripted proof presses W through the real `ButtonInput`
                    // resource, so it must land before the controller reads it.
                    play_proof.before(crate::fly_camera).run_if(play_demo),
                    // --combat-demo: the full combat-loop proof (hit→kill→die→respawn).
                    combat_proof.before(crate::fly_camera).run_if(combat_demo_run),
                    // Kevin's combat layer only ever drives Health to 0 and fires
                    // PlayerDied; the whole respawn loop hangs off that one message.
                    on_player_death.run_if(in_state(AppState::Play)),
                    respawn_at_campfire.after(on_player_death),
                )
                    .run_if(playing),
            );
    }
}

/// Run condition: this launch is a `--play` session.
fn playing(cfg: Res<Cfg>) -> bool {
    cfg.play
}

/// Run condition: the scripted headless play proof.
fn play_demo(cfg: Res<Cfg>) -> bool {
    cfg.play_demo
}

/// Run condition: the `--combat-demo` combat-loop proof. Both `cfg.combat_demo` and
/// `cfg.play` must be true (the CLI flag sets both; the env var only sets the former
/// and does not boot the full scene).
fn combat_demo_run(cfg: Res<Cfg>) -> bool {
    cfg.combat_demo && cfg.play
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

/// Pick the campsite, stand the avatar on it, light the fire, and hang the death
/// plate. Runs once, after `setup` has built the world and spawned the avatar.
#[allow(clippy::too_many_arguments)]
fn boot_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<Cfg>,
    mut world: ResMut<World>,
    mut player_q: Query<(&mut Transform, &mut FlyCam), Without<OrbitCam>>,
    mut cam_q: Query<(&mut Transform, &mut OrbitCam)>,
) {
    let from_map = cfg.map_load.is_some();
    let side = world_side(&world);
    let c = side * CHUNK / 2;

    // ---- Where does the player wake up? --------------------------------------
    // A loaded map is authored ground truth — scan *its* blocks for a standable
    // column near the middle and never touch the geometry (Shiba owns the level).
    // Procedural terrain is scenery, so we flatten a clearing into it for the fire.
    let (sx, sz, surface) = match map_spawn(&world, c, c) {
        Some(found) => found,
        None => {
            let (x, z, h) = find_spawn(c, c);
            clear_campsite(&mut commands, &mut meshes, &mut world, x, z, h);
            (x, z, h)
        }
    };

    // Feet rest on the *top face* of the surface voxel (surface + 1).
    let feet = (surface + 1) as f32;
    let eye = Vec3::new(sx as f32 + 0.5, feet + EYE_HEIGHT, sz as f32 + 0.5);
    // Yaw 0 faces -Z (the controller's convention), so the fire goes straight ahead
    // and the player wakes up already looking at it.
    let yaw = 0.0;
    let fire_xz = Vec3::new(eye.x, 0.0, eye.z - FIRE_AHEAD);
    let fire_y = highest_solid(&world, fire_xz.x.floor() as i32, fire_xz.z.floor() as i32)
        .map(|h| (h + 1) as f32)
        .unwrap_or(feet);
    let camp = Campsite {
        eye,
        yaw,
        fire: Vec3::new(fire_xz.x, fire_y, fire_xz.z),
    };

    spawn_campfire(&mut commands, &mut meshes, &mut materials, camp.fire);
    spawn_death_plate(&mut commands);
    place_player(&mut player_q, &mut cam_q, &camp);
    commands.insert_resource(camp);

    // Ground invariant — the one check that catches "avatar falls through the world"
    // at its source instead of 3 s later in a screenshot. The spawn column MUST have
    // its topmost solid voxel exactly at `surface`; if it doesn't, the feet were
    // computed from a height that belongs to some *other* column and the body is
    // standing over air. That is precisely how the swapped `(x, h, z)` return from
    // `map_spawn` shipped: it compiled, booted, printed SCENE_READY, and only then
    // dropped the player to y ≈ -48. Cheap, loud, and graded by prove_playable.sh.
    let under = highest_solid(&world, sx, sz);
    if under == Some(surface) {
        println!("SPAWN_GROUND col=({sx},{sz}) top={surface} feet={feet} => PASS");
    } else {
        println!(
            "SPAWN_GROUND col=({sx},{sz}) top={under:?} expected={surface} \
             — feet at {feet} stand over AIR => FAIL"
        );
    }

    println!(
        "SCENE_READY source={} spawn=({:.1},{:.1},{:.1}) surface={surface} campfire=({:.1},{:.1},{:.1})",
        if from_map { PLAY_MAP } else { "procedural+clearing" },
        eye.x,
        eye.y,
        eye.z,
        camp.fire.x,
        camp.fire.y,
        camp.fire.z
    );
}

/// The world's extent in chunks, as `setup` actually spawned it.
fn world_side(world: &World) -> i32 {
    let mx = world.chunks.keys().map(|(x, _)| *x).max().unwrap_or(0) + 1;
    let mz = world.chunks.keys().map(|(_, z)| *z).max().unwrap_or(0) + 1;
    mx.max(mz)
}

/// Find a standable column in a **loaded map**: spiral out from its centre for the
/// first column with a solid top and room for the body above it. `None` when the map
/// is empty there (or when no map is loaded at all), which sends the caller to the
/// procedural fallback.
fn map_spawn(world: &World, cx: i32, cz: i32) -> Option<(i32, i32, i32)> {
    for r in 0..(CHUNK * 2) {
        for dz in -r..=r {
            for dx in -r..=r {
                if r > 0 && dx.abs() != r && dz.abs() != r {
                    continue; // outer ring only — the inner ones were already tried
                }
                let (x, z) = (cx + dx, cz + dz);
                // `highest_solid` returns the topmost solid voxel, so everything above
                // it is air by definition — only the head-room bound needs checking.
                let Some(h) = highest_solid(world, x, z) else {
                    continue;
                };
                if h + 3 < CHUNK {
                    // (x, z, height) — the same order `find_spawn` returns, so both
                    // branches feed `(sx, sz, surface)` the same way.
                    return Some((x, z, h));
                }
            }
        }
    }
    None
}

/// Cut a level clearing into procedural terrain so the campsite reads as a place
/// rather than a hillside: solid ground up to the spawn height, and air above it.
fn clear_campsite(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &mut World,
    x: i32,
    z: i32,
    h: i32,
) {
    let (lo, hi) = (
        IVec3::new(x - CLEARING_R, 0, z - CLEARING_R),
        IVec3::new(x + CLEARING_R, 0, z + CLEARING_R),
    );
    // Fill any hollow under the pad with dirt, cap it with grass, clear the air above.
    box_fill(
        commands,
        meshes,
        world,
        IVec3::new(lo.x, 0, lo.z),
        IVec3::new(hi.x, (h - 1).max(0), hi.z),
        BlockId::DIRT,
    );
    box_fill(
        commands,
        meshes,
        world,
        IVec3::new(lo.x, h, lo.z),
        IVec3::new(hi.x, h, hi.z),
        BlockId::GRASS,
    );
    box_fill(
        commands,
        meshes,
        world,
        IVec3::new(lo.x, h + 1, lo.z),
        IVec3::new(hi.x, h + CLEARING_HEAD, hi.z),
        BlockId::AIR,
    );
}

/// Stand the avatar on the campsite and swing the boom camera in behind it. Called
/// at boot and again on every respawn, so "wake up" and "come back" look identical.
fn place_player(
    player_q: &mut Query<(&mut Transform, &mut FlyCam), Without<OrbitCam>>,
    cam_q: &mut Query<(&mut Transform, &mut OrbitCam)>,
    camp: &Campsite,
) {
    if let Ok((mut tf, mut fly)) = player_q.single_mut() {
        tf.translation = camp.eye;
        tf.rotation = Quat::from_axis_angle(Vec3::Y, camp.yaw);
        fly.face_yaw = camp.yaw;
        // Grounded from frame one — `--play` is a game, not a build session.
        fly.walking = true;
        fly.grounded = true;
        fly.vel = Vec3::ZERO;
    }
    if let Ok((mut ctf, mut orbit)) = cam_q.single_mut() {
        orbit.yaw = camp.yaw;
        orbit.pitch = WAKE_PITCH;
        orbit.dist = BOOM_DIST;
        let rot = Quat::from_axis_angle(Vec3::Y, orbit.yaw) * Quat::from_axis_angle(Vec3::X, orbit.pitch);
        ctf.translation = camp.eye + Vec3::Y * PIVOT_UP + (rot * Vec3::Z) * BOOM_DIST;
        ctf.rotation = rot;
    }
}

// ---------------------------------------------------------------------------
// The campfire
// ---------------------------------------------------------------------------

/// A ring of stones, three crossed logs, a warm emissive flame and the point light
/// that sells it. Entity-based (not voxels) so it sits the same on Shiba's authored
/// map as on the procedural fallback, and a respawn never has to rebuild geometry.
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
    let flame_lo = meshes.add(Cuboid::new(0.42, 0.42, 0.42));
    let flame_hi = meshes.add(Cuboid::new(0.22, 0.30, 0.22));

    commands
        .spawn((Transform::from_translation(foot), Visibility::default()))
        .with_children(|p| {
            // Ring of eight stones.
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                p.spawn((
                    Mesh3d(stone_mesh.clone()),
                    MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(a.cos() * 0.85, 0.14, a.sin() * 0.85)
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, a)),
                ));
            }
            // Three logs crossed over the pit.
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                p.spawn((
                    Mesh3d(log_mesh.clone()),
                    MeshMaterial3d(log.clone()),
                    Transform::from_xyz(0.0, 0.16, 0.0)
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, a)),
                ));
            }
            // Flame: two stacked emissive blocks + the light that actually lifts the
            // scene. Both flicker (see `flicker_flame`).
            p.spawn((
                Mesh3d(flame_lo),
                MeshMaterial3d(ember.clone()),
                Transform::from_xyz(0.0, 0.42, 0.0),
                Flame { phase: 0.0 },
            ));
            p.spawn((
                Mesh3d(flame_hi),
                MeshMaterial3d(ember),
                Transform::from_xyz(0.0, 0.74, 0.0),
                Flame { phase: 1.7 },
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
                Flame { phase: 0.9 },
            ));
        });
}

/// Two overlapping sines — enough irregularity to read as fire without a noise field.
fn flicker_flame(
    time: Res<Time>,
    mut flames: Query<(&Flame, Option<&mut PointLight>, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (f, light, mut tf) in flames.iter_mut() {
        let w = ((t * 9.0 + f.phase).sin() * 0.6 + (t * 13.7 + f.phase).sin() * 0.4) * 0.5 + 1.0;
        match light {
            Some(mut l) => l.intensity = 260_000.0 * (0.82 + 0.18 * w),
            None => tf.scale = Vec3::splat(0.88 + 0.16 * w),
        }
    }
}

// ---------------------------------------------------------------------------
// Death is a door, not a wall
// ---------------------------------------------------------------------------

/// The black plate + the one line of text the design doc allows on death.
fn spawn_death_plate(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        DeathFade,
    ));
    commands.spawn((
        Text::new("Rest. Try again."),
        TextFont {
            font_size: bevy::text::FontSize::from(28.0),
            ..default()
        },
        TextColor(Color::srgba(0.94, 0.90, 0.84, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(46.0),
            left: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-96.0),
                ..default()
            },
            ..default()
        },
        RestNotice,
    ));
}

/// The hook Kevin's combat layer feeds: `PlayerDied` in, respawn timeline armed.
/// Nothing else in the client needs to know how dying works — drive `Health` to 0
/// (or write the message directly) and the loop below takes it from there.
fn on_player_death(mut died: MessageReader<combat::PlayerDied>, mut death: ResMut<Death>) {
    let fired = died.read().count();
    if fired > 0 && death.since.is_none() {
        death.since = Some(0.0);
        death.moved = false;
        println!("PLAYER_DIED → fade to black, respawn at campfire");
    }
}

/// Fade out → put the player back at the fire with a full kit and the encounter
/// reset → fade back in with one line of text. No death screen, no reload.
#[allow(clippy::too_many_arguments)]
fn respawn_at_campfire(
    time: Res<Time>,
    camp: Option<Res<Campsite>>,
    mut death: ResMut<Death>,
    mut commands: Commands,
    mut player_q: Query<(&mut Transform, &mut FlyCam), Without<OrbitCam>>,
    mut cam_q: Query<(&mut Transform, &mut OrbitCam)>,
    players: Query<Entity, With<FlyCam>>,
    mut enemies: Query<
        (&mut Transform, &mut combat::Enemy, &mut combat::Health),
        (Without<FlyCam>, Without<OrbitCam>),
    >,
    mut fade_q: Query<&mut BackgroundColor, With<DeathFade>>,
    mut notice_q: Query<&mut TextColor, With<RestNotice>>,
) {
    let dt = time.delta_secs();

    // The notice fades itself out whether or not a respawn is in flight.
    if death.notice > 0.0 {
        death.notice = (death.notice - dt).max(0.0);
        if let Ok(mut c) = notice_q.single_mut() {
            c.0.set_alpha(death.notice.min(1.0));
        }
    }

    let Some(camp) = camp else { return };
    let Some(t0) = death.since else { return };
    let t = t0 + dt;
    death.since = Some(t);

    // Mid-point of the fade — the screen is fully black, so move everything now.
    if t >= FADE && !death.moved {
        death.moved = true;
        place_player(&mut player_q, &mut cam_q, &camp);
        if let Ok(e) = players.single() {
            // A fresh combat kit: full HP, full stamina, poise reset, state Idle.
            // Uses Kevin's own constructor, so the respawned player is exactly the
            // player `setup` spawns — no duplicated balance numbers here.
            commands.entity(e).insert(combat::player_bundle());
        }
        // "Enemy fully respawns" — heal every husk and send it back to its patrol.
        let mut reset = 0;
        for (mut etf, mut enemy, mut hp) in enemies.iter_mut() {
            hp.cur = hp.max;
            enemy.state = combat::HuskState::Patrol;
            enemy.timer = 0.0;
            enemy.hitstop = 0.0;
            enemy.hit_applied = false;
            etf.translation = enemy.patrol_origin;
            reset += 1;
        }
        death.notice = NOTICE;
        println!(
            "RESPAWN at campfire ({:.1},{:.1},{:.1}) hp=full enemies_reset={reset}",
            camp.eye.x, camp.eye.y, camp.eye.z
        );
    }

    // 0 → black over FADE, black → 0 over the next FADE, then we're alive again.
    let alpha = if t < FADE {
        t / FADE
    } else {
        (1.0 - (t - FADE) / FADE).max(0.0)
    };
    if let Ok(mut bg) = fade_q.single_mut() {
        bg.0.set_alpha(alpha);
    }
    if t >= FADE * 2.0 {
        death.since = None;
    }
}

// ---------------------------------------------------------------------------
// Scripted proof (`--play-demo`)
// ---------------------------------------------------------------------------

/// Headless proof that `--play` is a *game*, not a still frame. Every button it
/// "presses" goes through the real input resources — `ButtonInput<KeyCode>`,
/// `ButtonInput<MouseButton>` and the `MouseMotion` message stream — so the actual
/// controller, mouse-look, gravity and voxel-collision path runs and nothing is
/// simulated on the side. `play_proof` is ordered `.before(fly_camera)`, so the
/// controller reads these in the very same frame a player's would land.
///
/// Beats: click to capture the cursor → hold W (+ a D strafe) while dragging the
/// mouse to orbit the camera → log the body *and* look delta → then (when no
/// screenshot is pending) kill the player to prove the campfire respawn loop.
#[allow(clippy::too_many_arguments)]
fn play_proof(
    time: Res<Time>,
    cfg: Res<Cfg>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: bevy::ecs::message::MessageWriter<MouseMotion>,
    mut proof: ResMut<PlayProof>,
    camp: Option<Res<Campsite>>,
    mut player_q: Query<(&Transform, &FlyCam, &mut combat::Health)>,
    orbit_q: Query<&OrbitCam>,
    mut exit: bevy::ecs::message::MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    let Ok((tf, fly, mut hp)) = player_q.single_mut() else {
        return;
    };
    let yaw_now = orbit_q.single().map(|o| o.yaw).unwrap_or(0.0);
    // Let the first frames settle (the boom camera and gravity both need a tick).
    if t < 1.5 {
        return;
    }
    if proof.start.is_none() {
        proof.start = Some(tf.translation);
        proof.start_yaw = Some(yaw_now);
    }
    // One left-click, once. `fly_camera` reads it as "capture the cursor" and only
    // then does mouse-look turn on; `edit_voxels` runs first and bails while the
    // cursor is still free, so this click can never break a block.
    if !proof.clicked {
        proof.clicked = true;
        mouse.press(MouseButton::Left);
    }

    // ---- beat 1: walk forward for ~1.9 s (the screenshot lands mid-stride) ----
    if t < 3.4 {
        keys.press(KeyCode::KeyW);
        // Mid-walk, add a strafe and swing the camera around — the same two events
        // a player makes when they look where they are going.
        if (2.2..3.0).contains(&t) {
            keys.press(KeyCode::KeyD);
            motion.write(MouseMotion {
                delta: Vec2::new(-9.0, 0.0),
            });
        }
        return;
    }
    if !proof.walk_logged {
        proof.walk_logged = true;
        let from = proof.start.unwrap_or(tf.translation);
        let to = tf.translation;
        let moved = Vec2::new(to.x - from.x, to.z - from.z).length();
        let yaw0 = proof.start_yaw.unwrap_or(yaw_now);
        let turned = (yaw_now - yaw0).abs();
        println!(
            "PLAY_LOOK yaw {:.3} -> {:.3} rad (turned {:.1}°) => {}",
            yaw0,
            yaw_now,
            turned.to_degrees(),
            if turned > 0.1 { "PASS" } else { "FAIL" }
        );
        println!(
            "PLAY_WALK from=({:.2},{:.2},{:.2}) to=({:.2},{:.2},{:.2}) moved={moved:.2} \
             grounded={} => {}",
            from.x,
            from.y,
            from.z,
            to.x,
            to.y,
            to.z,
            fly.grounded,
            if moved > 1.0 && fly.grounded { "PASS" } else { "FAIL" }
        );
        // A screenshot run stops here: `screenshot_once` owns the shot and the exit,
        // and a fade-to-black death would black out the frame it is about to take.
        if cfg.shot.is_some() {
            return;
        }
    }
    if cfg.shot.is_some() {
        return;
    }

    // ---- beat 2: die, and come back at the fire ------------------------------
    if !proof.killed && t >= 3.6 {
        proof.killed = true;
        let max = hp.max;
        hp.damage(max);
        println!("PLAY_KILL forced hp -> 0 (drives combat::PlayerDied)");
        return;
    }
    // The respawn completes at kill + FADE; check well after the fade-in ends.
    if proof.killed && !proof.respawn_logged && t >= 5.0 {
        proof.respawn_logged = true;
        let at = tf.translation;
        let (home, dist) = match camp.as_deref() {
            Some(c) => (c.eye, at.distance(c.eye)),
            None => (Vec3::ZERO, f32::INFINITY),
        };
        let ok = dist < 1.0 && (hp.cur - hp.max).abs() < f32::EPSILON;
        println!(
            "PLAY_RESPAWN at=({:.2},{:.2},{:.2}) campfire=({:.2},{:.2},{:.2}) dist={dist:.2} \
             hp={:.0}/{:.0} => {}",
            at.x,
            at.y,
            at.z,
            home.x,
            home.y,
            home.z,
            hp.cur,
            hp.max,
            if ok { "PASS" } else { "FAIL" }
        );
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// Scripted combat-loop proof (`--combat-demo`)
// ---------------------------------------------------------------------------

/// Headless proof of the full combat loop: walk to the husk → attack until it dies
/// → get killed (or force-death) → respawn at the campfire. All inputs flow through
/// real `ButtonInput<KeyCode>` so `gather_input` + `player_combat` + `husk_ai` run
/// exactly as they would for a human player. Self-grades with `COMBAT_* => PASS/FAIL`.
#[allow(clippy::too_many_arguments)]
fn combat_proof(
    time: Res<Time>,
    _cfg: Res<Cfg>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut proof: ResMut<CombatProof>,
    camp: Option<Res<Campsite>>,
    mut player_q: Query<(&Transform, &FlyCam, &mut combat::Health)>,
    enemies: Query<(&combat::Enemy, &combat::Health), (With<combat::Enemy>, Without<FlyCam>)>,
    mut exit: bevy::ecs::message::MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    let Ok((tf, _fly, mut hp)) = player_q.single_mut() else {
        return;
    };
    let husk = enemies.iter().next();
    let husk_hp = husk.map(|(_, h)| h.cur).unwrap_or(-1.0);
    let husk_alive = husk.map(|(_, h)| !h.dead()).unwrap_or(false);

    // ---- settle frames: let the scene boot ----------------------------------
    if t < 1.5 {
        return;
    }

    // ---- phase 0: snapshot initial state ------------------------------------
    if proof.phase == 0 {
        proof.player_hp0 = Some(hp.cur);
        proof.husk_hp0 = Some(husk_hp);
        proof.phase = 1;
        return;
    }

    // ---- phase 1: walk toward the husk (~2 s of W) --------------------------
    if proof.phase == 1 {
        if t < 3.5 {
            keys.press(KeyCode::KeyW);
            // Press D briefly to strafe a little — proves the body is moving
            // under input, not just falling forward.
            if (2.2..2.8).contains(&t) {
                keys.press(KeyCode::KeyD);
            }
            return;
        }
        proof.phase = 2;
        return;
    }

    // ---- phase 2: first light attack (t ≈ 3.5) ------------------------------
    if proof.phase == 2 {
        keys.press(KeyCode::KeyX); // light attack
        proof.phase = 3;
        return;
    }
    // ---- phase 3: check husk HP a beat later (t ≈ 4.2) ----------------------
    if proof.phase == 3 && t >= 4.2 {
        let delta = proof.husk_hp0.unwrap_or(80.0) - husk_hp;
        proof.husk_hit = delta > 0.5;
        proof.husk_hp1 = Some(husk_hp);
        if proof.husk_hit {
            println!(
                "COMBAT_HIT husk_hp {:.0}->{:.0} (delta={delta:.1}) => PASS",
                proof.husk_hp0.unwrap_or(80.0), husk_hp
            );
        } else {
            println!(
                "COMBAT_HIT husk_hp {:.0}->{:.0} (delta={delta:.1}) => FAIL",
                proof.husk_hp0.unwrap_or(80.0), husk_hp
            );
        }
        if husk_alive {
            proof.phase = 4; // keep attacking to kill it
        } else {
            proof.husk_dead = true; // already dead
            proof.phase = 9; // skip to player death
        }
        return;
    }

    // ---- phase 4-8: keep attacking until husk dies ---------------------------
    if proof.phase == 4 {
        keys.press(KeyCode::KeyX); // light attack
        proof.phase = 5;
        return;
    }
    if proof.phase == 5 && t >= 5.0 {
        keys.press(KeyCode::KeyX); // light attack
        proof.phase = 6;
        return;
    }
    if proof.phase == 6 && t >= 5.7 {
        keys.press(KeyCode::KeyC); // heavy attack (45 dmg — husk should die)
        proof.phase = 7;
        return;
    }
    if proof.phase == 7 && t >= 6.0 {
        // One more light to finish it if the heavy didn't.
        keys.press(KeyCode::KeyX);
        proof.phase = 8;
        return;
    }
    if proof.phase == 8 && t >= 6.8 {
        proof.husk_dead = husk_alive;
        let final_hp = husk_hp;
        println!(
            "COMBAT_KILL husk_hp={:.0} dead={} => {}",
            final_hp, proof.husk_dead,
            if proof.husk_dead { "PASS" } else { "FAIL" }
        );
        proof.phase = 9;
        return;
    }

    // ---- phase 9: kill the player → trigger respawn loop ---------------------
    if proof.phase == 9 && t >= 7.2 {
        if !proof.died {
            let max = hp.max;
            hp.damage(max); // force HP to 0 → PlayerDied fires next frame
            proof.died = true;
            println!("COMBAT_DEATH hp forced to 0");
            return;
        }
        // PlayerDied → Respawn in flight. The death is gated at a 0.5 s fade.
        proof.phase = 10;
        return;
    }

    // ---- phase 10: wait for respawn to complete (fade = 0.5+0.5 = 1.0 s) ----
    if proof.phase == 10 {
        if t < 8.5 {
            return; // still fading / respawning
        }
        if !proof.respawned {
            proof.respawned = true;
            let at = tf.translation;
            let (home, dist) = match camp.as_deref() {
                Some(c) => (c.eye, at.distance(c.eye)),
                None => (Vec3::ZERO, f32::INFINITY),
            };
            let hp_ok = (hp.cur - hp.max).abs() < 1.0; // within 1 HP of full
            let pos_ok = dist < 2.0; // near the campfire
            let ok = hp_ok && pos_ok;
            println!(
                "COMBAT_RESPAWN at=({:.1},{:.1},{:.1}) campfire=({:.1},{:.1},{:.1}) \
                 dist={dist:.2} hp={:.0}/{:.0} => {}",
                at.x, at.y, at.z,
                home.x, home.y, home.z,
                hp.cur, hp.max,
                if ok { "PASS" } else { "FAIL" }
            );
            if !hp_ok {
                println!("COMBAT_RESPAWN detail: hp not restored (expected {:.0})", hp.max);
            }
            if !pos_ok {
                println!("COMBAT_RESPAWN detail: too far from campfire (dist={dist:.2})");
            }
            exit.write(AppExit::Success);
        }
    }
}
