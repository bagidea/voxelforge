//! Procedural character animation — the layer that turns the avatar and the Guard
//! Husk from sliding boxes into bodies that walk, turn, swing, flinch and fall.
//!
//! There is no skeletal source to play back: `assets/models` holds one `.vox`
//! sample and nothing rigged, so every pose here is *computed*, not sampled. The
//! plugin builds a 13-joint humanoid rig out of boxes as a CHILD of each actor and
//! drives it from the same numbers combat already publishes:
//!
//!   ① locomotion  — idle → walk → run blended on the actor's REAL speed, with the
//!                   stride phase advanced by distance travelled (no foot skating).
//!   ② turning     — the rig leads the root with a torso/hips twist, and shuffles
//!                   its feet when it pivots on the spot (turn-in-place).
//!   ③ attacks     — arcs keyed to [`combat`]'s own timers, so the blade is fastest
//!                   exactly inside `LIGHT_ACTIVE` / `HEAVY_ACTIVE` / `CHARGED_ACTIVE`
//!                   and the Husk's overhead lands on `HuskState::Swing1/2`.
//!   ④ hit react   — an additive recoil fired by a drop in `Health.cur`.
//!   ⑤ death       — the player folds and falls; a dying Husk hands its pose to a
//!                   detached corpse rig (combat despawns the real one the same frame).
//!   ⑥ weight      — head/torso bob + lateral sway while walking, and a spring
//!                   squash-and-stretch on landing.
//!
//! **It cannot move a hitbox.** Every transform written here belongs to a child of
//! the actor. Combat reads only the actor's own `Transform.translation` (melee range,
//! cones, the Husk's `surface_y` clamp) and the root is never touched — so the fight
//! plays out identically with the rig on or off. `scripts/prove_combat.sh` runs the
//! full loop with this plugin live.
//!
//! Ownership: this file + `scene.rs`. `combat.rs` is read-only from here — the rig
//! observes its state, never writes it.

use bevy::prelude::*;

use crate::combat::{self, CombatState, Enemy, Health, HuskState, PlayerCombat};
use crate::{Cfg, FlyCam, EYE_HEIGHT};

// ---------------------------------------------------------------------------
// Tunables
// ---------------------------------------------------------------------------

/// Speed (blocks/s) below which the actor is standing still.
const IDLE_SPEED: f32 = 0.25;
/// Speed at which the walk cycle is at full weight.
const WALK_SPEED: f32 = 1.2;
/// Speed band over which walk blends into run (the avatar walks at 6, sprints at 10).
const RUN_LO: f32 = 2.5;
const RUN_HI: f32 = 6.0;
/// Metres of ground covered by one full stride cycle, walking / running.
const STRIDE_WALK: f32 = 1.30;
const STRIDE_RUN: f32 = 2.10;
/// Speeds above this are clipped — a teleport (respawn, map load) must not spin
/// the legs like a windmill for a frame.
const SPEED_CLIP: f32 = 14.0;
/// How fast the measured speed follows the truth (exp smoothing, per second).
const SPEED_LAG: f32 = 12.0;

/// Yaw rate (rad/s) that counts as a full turn-in-place.
const TURN_FULL: f32 = 2.5;
/// Hit-reaction length.
const HIT_TIME: f32 = 0.36;
/// Landing-squash length.
const LAND_TIME: f32 = 0.32;
/// Fall speed that produces a full-strength landing squash.
const LAND_FULL: f32 = 13.0;
/// Time a fallen body lies on the ground before it is cleared.
const CORPSE_TIME: f32 = 6.0;
/// Guard/parry poses are re-entered every frame while the button is held (combat
/// bounces Block→Idle→Block), so the pose is latched for this long past the last
/// sighting instead of strobing.
const GUARD_LATCH: f32 = 0.14;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Builds and drives the procedural rigs. Gated on [`Cfg::play`] exactly like
/// [`crate::scene::ScenePlugin`], so the bench, the hero shot and every editor
/// screenshot keep the plain capsule they were graded against.
pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_rig_assets).add_systems(
            Update,
            (
                // Rigs are attached the frame after their actor appears (the player
                // in `setup`, husks on entering Play), so this just runs every frame
                // and picks up whatever is new.
                attach_rigs,
                // Must see the Husk's Health *before* `husk_ai` despawns it, or the
                // body vanishes mid-air with no fall.
                spawn_husk_corpse.before(combat::husk_ai),
                // After the controller so the root transform is final for this frame
                // — speed and facing are measured from it.
                animate_rigs.after(crate::fly_camera).after(combat::husk_ai),
                tick_corpses,
            )
                .run_if(playing),
        );
    }
}

/// Run condition: a `--play` session (the `--combat-demo` flag sets this too).
fn playing(cfg: Res<Cfg>) -> bool {
    cfg.play
}

// ---------------------------------------------------------------------------
// Rig data
// ---------------------------------------------------------------------------

/// Which body the rig is wearing. Proportions, materials and the attack arcs all
/// key off this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Actor {
    Player,
    Husk,
}

/// Marker on the ACTOR entity: its rig has been built, don't build a second one.
#[derive(Component)]
pub struct Rigged;

/// Marker on every joint of a rig, so the joint query is provably disjoint from
/// the actor / rig-root queries.
#[derive(Component)]
pub struct Joint;

/// The rig root — one child entity per actor, holding the joint handles and all
/// the animation state. Everything below it is pure decoration.
#[derive(Component)]
pub struct Rig {
    actor: Actor,
    // Joints, top-down. Arms/legs are two-bone so knees and elbows really bend.
    hips: Entity,
    torso: Entity,
    head: Entity,
    sh_l: Entity,
    el_l: Entity,
    sh_r: Entity,
    el_r: Entity,
    hand_r: Entity,
    hip_l: Entity,
    knee_l: Entity,
    hip_r: Entity,
    knee_r: Entity,

    /// Stride phase in radians, advanced by distance travelled (never by time) so
    /// the feet stay planted at any speed.
    phase: f32,
    /// Smoothed horizontal speed of the actor root.
    speed: f32,
    prev_pos: Vec3,
    /// World yaw the rig is pointing at, and how fast it is changing.
    face_yaw: f32,
    yaw_rate: f32,
    /// Phase of the turn-in-place shuffle.
    turn_phase: f32,
    /// Seconds airborne (player only — the Husk is clamped to its surface).
    air: f32,
    /// Countdown of the hit-reaction recoil, and which side it came from.
    hit: f32,
    hit_side: f32,
    prev_hp: f32,
    /// Landing squash countdown + strength.
    land: f32,
    land_amp: f32,
    /// Fall speed carried from the previous frame (the frame we land, it's already 0).
    prev_fall: f32,
    /// Guard/parry latch (see [`GUARD_LATCH`]).
    guard: f32,
    parry: f32,
    /// Death progress, 0 → 1. Reset when the actor comes back alive.
    death: f32,
}

/// A detached Husk body. `husk_ai` despawns the real enemy on the frame its HP hits
/// zero, so the fall is played by this stand-in, spawned one system earlier.
#[derive(Component)]
pub struct Corpse {
    age: f32,
}

/// Set on an enemy whose corpse has already been handed off — one body per death.
#[derive(Component)]
pub struct Dying;

/// Meshes + materials shared by every rig, built once at startup so a corpse can be
/// spawned mid-frame without touching `Assets`.
#[derive(Resource)]
struct RigAssets {
    player: Parts,
    husk: Parts,
}

struct Parts {
    pelvis: Handle<Mesh>,
    torso: Handle<Mesh>,
    head: Handle<Mesh>,
    face: Handle<Mesh>,
    upper_arm: Handle<Mesh>,
    forearm: Handle<Mesh>,
    thigh: Handle<Mesh>,
    shin: Handle<Mesh>,
    foot: Handle<Mesh>,
    weapon: Handle<Mesh>,
    cloth: Handle<StandardMaterial>,
    trim: Handle<StandardMaterial>,
    skin: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
}

/// Skeleton proportions, in blocks, measured from the FEET.
#[derive(Clone, Copy)]
struct Dims {
    hip_y: f32,
    hip_x: f32,
    thigh: f32,
    shin: f32,
    /// Shoulder height + spread, measured from the hips (torso-local).
    sh_y: f32,
    sh_x: f32,
    upper: f32,
    fore: f32,
    /// Neck height above the hips (torso-local).
    neck_y: f32,
    /// Head-cube centre above the neck, and where the face plate sits on it — both
    /// measured, not derived, so the plate never floats off the front of the skull.
    head_off: f32,
    face_y: f32,
    face_z: f32,
    /// Half the foot box's height. Lifts the foot so its SOLE is flush with y=0
    /// instead of sinking through the floor the body is standing on.
    foot_h: f32,
    /// Vertical bob amplitude at a full-weight walk.
    bob: f32,
    /// Lateral hip sway amplitude at a full-weight walk.
    sway: f32,
}

impl Dims {
    fn of(actor: Actor) -> Self {
        match actor {
            // 1.80 crown-to-floor, matching PLAYER_HEIGHT.
            Actor::Player => Dims {
                hip_y: 0.84,
                hip_x: 0.125,
                thigh: 0.44,
                shin: 0.40,
                sh_y: 0.50,
                sh_x: 0.255,
                upper: 0.30,
                fore: 0.28,
                neck_y: 0.58,
                head_off: 0.18,
                face_y: 0.20,
                face_z: -0.16,
                foot_h: 0.045,
                bob: 0.055,
                sway: 0.030,
            },
            // ~2.38 crown-to-floor — the same reading silhouette the box Husk had.
            Actor::Husk => Dims {
                hip_y: 1.00,
                hip_x: 0.20,
                thigh: 0.52,
                shin: 0.48,
                sh_y: 0.70,
                sh_x: 0.44,
                upper: 0.44,
                fore: 0.41,
                neck_y: 0.88,
                head_off: 0.27,
                face_y: 0.30,
                face_z: -0.24,
                foot_h: 0.065,
                bob: 0.070,
                sway: 0.045,
            },
        }
    }

    fn leg(&self) -> f32 {
        self.thigh + self.shin
    }
}

// ---------------------------------------------------------------------------
// Asset setup
// ---------------------------------------------------------------------------

fn init_rig_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pd = Dims::of(Actor::Player);
    let hd = Dims::of(Actor::Husk);

    // The avatar keeps the orange it has always worn, so the play-mode screenshots
    // still read as "the same guy" — just with limbs now.
    let player = Parts {
        pelvis: meshes.add(Cuboid::new(0.40, 0.20, 0.26)),
        torso: meshes.add(Cuboid::new(0.46, 0.58, 0.28)),
        head: meshes.add(Cuboid::new(0.36, 0.34, 0.34)),
        face: meshes.add(Cuboid::new(0.24, 0.10, 0.06)),
        upper_arm: meshes.add(Cuboid::new(0.15, pd.upper, 0.16)),
        forearm: meshes.add(Cuboid::new(0.13, pd.fore, 0.14)),
        thigh: meshes.add(Cuboid::new(0.19, pd.thigh, 0.20)),
        shin: meshes.add(Cuboid::new(0.17, pd.shin, 0.18)),
        foot: meshes.add(Cuboid::new(0.19, 0.09, 0.30)),
        weapon: meshes.add(Cuboid::new(0.07, 0.86, 0.14)),
        cloth: materials.add(StandardMaterial {
            base_color: Color::srgb(0.92, 0.38, 0.16),
            perceptual_roughness: 0.72,
            ..default()
        }),
        trim: materials.add(StandardMaterial {
            base_color: Color::srgb(0.34, 0.20, 0.13),
            perceptual_roughness: 0.80,
            ..default()
        }),
        skin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.62, 0.47),
            perceptual_roughness: 0.68,
            ..default()
        }),
        steel: materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.75, 0.80),
            perceptual_roughness: 0.30,
            metallic: 0.75,
            ..default()
        }),
    };

    let husk = Parts {
        pelvis: meshes.add(Cuboid::new(0.68, 0.26, 0.44)),
        torso: meshes.add(Cuboid::new(0.88, 0.86, 0.58)),
        head: meshes.add(Cuboid::new(0.50, 0.48, 0.50)),
        face: meshes.add(Cuboid::new(0.34, 0.08, 0.06)),
        upper_arm: meshes.add(Cuboid::new(0.26, hd.upper, 0.26)),
        forearm: meshes.add(Cuboid::new(0.23, hd.fore, 0.23)),
        thigh: meshes.add(Cuboid::new(0.30, hd.thigh, 0.30)),
        shin: meshes.add(Cuboid::new(0.27, hd.shin, 0.27)),
        foot: meshes.add(Cuboid::new(0.30, 0.13, 0.44)),
        weapon: meshes.add(Cuboid::new(0.10, 1.05, 0.26)),
        cloth: materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.34, 0.40),
            perceptual_roughness: 0.55,
            metallic: 0.30,
            ..default()
        }),
        trim: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.19, 0.24),
            perceptual_roughness: 0.50,
            ..default()
        }),
        skin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.25, 0.24),
            perceptual_roughness: 0.85,
            ..default()
        }),
        steel: materials.add(StandardMaterial {
            base_color: Color::srgb(0.46, 0.44, 0.42),
            perceptual_roughness: 0.45,
            metallic: 0.60,
            ..default()
        }),
    };

    commands.insert_resource(RigAssets { player, husk });
}

// ---------------------------------------------------------------------------
// Rig construction
// ---------------------------------------------------------------------------

/// Spawn the joint tree under `parent`. Returns the rig-root entity and the [`Rig`]
/// that belongs ON it — the caller inserts it once `prev_pos`/`face_yaw` are seeded.
///
/// Limb meshes hang HALF THEIR LENGTH below their joint, so rotating the joint
/// swings the limb about the shoulder/hip/knee instead of about its own middle —
/// that offset is the whole reason the rig reads as a body and not as floating bricks.
fn build_rig(
    commands: &mut Commands,
    assets: &RigAssets,
    actor: Actor,
    parent: Entity,
) -> (Entity, Rig) {
    let d = Dims::of(actor);
    let p = match actor {
        Actor::Player => &assets.player,
        Actor::Husk => &assets.husk,
    };

    // The rig root sits at the actor's FEET. The avatar's own origin is its eye, the
    // Husk's is already its feet — hence the two offsets.
    let root_dy = match actor {
        Actor::Player => -EYE_HEIGHT,
        Actor::Husk => 0.0,
    };
    let root = commands
        .spawn((
            Transform::from_xyz(0.0, root_dy, 0.0),
            Visibility::default(),
            ChildOf(parent),
        ))
        .id();

    let mut joint = |x: f32, y: f32, z: f32, of: Entity| -> Entity {
        commands
            .spawn((
                Transform::from_xyz(x, y, z),
                Visibility::default(),
                Joint,
                ChildOf(of),
            ))
            .id()
    };

    let hips = joint(0.0, d.hip_y, 0.0, root);
    let torso = joint(0.0, 0.0, 0.0, hips);
    let head = joint(0.0, d.neck_y, 0.0, torso);
    let sh_l = joint(-d.sh_x, d.sh_y, 0.0, torso);
    let el_l = joint(0.0, -d.upper, 0.0, sh_l);
    let sh_r = joint(d.sh_x, d.sh_y, 0.0, torso);
    let el_r = joint(0.0, -d.upper, 0.0, sh_r);
    let hand_r = joint(0.0, -d.fore, 0.0, el_r);
    let hip_l = joint(-d.hip_x, 0.0, 0.0, hips);
    let knee_l = joint(0.0, -d.thigh, 0.0, hip_l);
    let hip_r = joint(d.hip_x, 0.0, 0.0, hips);
    let knee_r = joint(0.0, -d.thigh, 0.0, hip_r);

    let mut skin = |mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, t: Transform, of: Entity| {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            t,
            ChildOf(of),
        ));
    };

    skin(&p.pelvis, &p.trim, Transform::from_xyz(0.0, -0.05, 0.0), hips);
    skin(
        &p.torso,
        &p.cloth,
        Transform::from_xyz(0.0, d.neck_y * 0.5, 0.0),
        torso,
    );
    skin(
        &p.head,
        &p.skin,
        Transform::from_xyz(0.0, d.head_off, 0.0),
        head,
    );
    // The dark face block keeps the old "which way am I looking" read (local -Z).
    skin(
        &p.face,
        &p.trim,
        Transform::from_xyz(0.0, d.face_y, d.face_z),
        head,
    );
    skin(
        &p.upper_arm,
        &p.cloth,
        Transform::from_xyz(0.0, -d.upper * 0.5, 0.0),
        sh_l,
    );
    skin(
        &p.forearm,
        &p.skin,
        Transform::from_xyz(0.0, -d.fore * 0.5, 0.0),
        el_l,
    );
    skin(
        &p.upper_arm,
        &p.cloth,
        Transform::from_xyz(0.0, -d.upper * 0.5, 0.0),
        sh_r,
    );
    skin(
        &p.forearm,
        &p.skin,
        Transform::from_xyz(0.0, -d.fore * 0.5, 0.0),
        el_r,
    );
    for (hip, knee) in [(hip_l, knee_l), (hip_r, knee_r)] {
        skin(
            &p.thigh,
            &p.trim,
            Transform::from_xyz(0.0, -d.thigh * 0.5, 0.0),
            hip,
        );
        skin(
            &p.shin,
            &p.trim,
            Transform::from_xyz(0.0, -d.shin * 0.5, 0.0),
            knee,
        );
        // Sole flush with the ground plane, toe protruding forward (-Z).
        skin(
            &p.foot,
            &p.steel,
            Transform::from_xyz(0.0, -d.shin + d.foot_h, -0.06),
            knee,
        );
    }
    // Weapon in the right hand: blade forward-and-down at rest, so the whole arc is
    // a rotation of the grip rather than a teleport.
    let blade_len = match actor {
        Actor::Player => 0.86,
        Actor::Husk => 1.05,
    };
    skin(
        &p.weapon,
        &p.steel,
        Transform::from_xyz(0.0, -blade_len * 0.5 + 0.06, -0.04),
        hand_r,
    );

    let rig = Rig {
        actor,
        hips,
        torso,
        head,
        sh_l,
        el_l,
        sh_r,
        el_r,
        hand_r,
        hip_l,
        knee_l,
        hip_r,
        knee_r,
        phase: 0.0,
        speed: 0.0,
        prev_pos: Vec3::ZERO,
        face_yaw: 0.0,
        yaw_rate: 0.0,
        turn_phase: 0.0,
        air: 0.0,
        hit: 0.0,
        hit_side: 1.0,
        prev_hp: f32::MAX,
        land: 0.0,
        land_amp: 0.0,
        prev_fall: 0.0,
        guard: 0.0,
        parry: 0.0,
        death: 0.0,
    };
    (root, rig)
}

/// Attach a rig to any actor that doesn't have one yet, and hide the placeholder
/// meshes it shipped with (the avatar's capsule + face block, the Husk's three
/// boxes). Hiding — not despawning — keeps `combat::husk_telegraph`'s `HuskArm`
/// query intact; it goes on writing a transform nobody renders.
#[allow(clippy::type_complexity)]
fn attach_rigs(
    mut commands: Commands,
    assets: Option<Res<RigAssets>>,
    player_q: Query<(Entity, &Transform), (With<FlyCam>, With<PlayerCombat>, Without<Rigged>)>,
    enemy_q: Query<(Entity, &Transform), (With<Enemy>, Without<Rigged>)>,
    child_q: Query<&Children>,
    mut vis_q: Query<&mut Visibility, With<Mesh3d>>,
) {
    let Some(assets) = assets else {
        return;
    };

    let mut dress = |entity: Entity, tf: &Transform, actor: Actor, commands: &mut Commands| {
        // Blank the placeholder meshes BEFORE the rig's own children exist — the
        // spawn above is deferred, so `Children` here only holds the old boxes.
        if let Ok(children) = child_q.get(entity) {
            for c in children.iter() {
                if let Ok(mut v) = vis_q.get_mut(c) {
                    *v = Visibility::Hidden;
                }
            }
        }
        let (root, mut rig) = build_rig(commands, &assets, actor, entity);
        rig.prev_pos = tf.translation;
        rig.face_yaw = yaw_of(tf);
        commands.entity(root).insert(rig);
        commands.entity(entity).insert(Rigged);
    };

    for (e, tf) in player_q.iter() {
        dress(e, tf, Actor::Player, &mut commands);
    }
    for (e, tf) in enemy_q.iter() {
        dress(e, tf, Actor::Husk, &mut commands);
    }
}

// ---------------------------------------------------------------------------
// Corpses
// ---------------------------------------------------------------------------

/// A Husk at 0 HP is despawned by `husk_ai` in the very next system, children and
/// all. Copy its pose onto a free-standing corpse rig here so the body actually
/// falls instead of blinking out.
///
/// The [`Dying`] marker is belt-and-braces: `husk_ai` clears the enemy the same
/// frame, but it bails out early when there is no player, and a dead husk left
/// standing must not spit a fresh corpse every tick.
fn spawn_husk_corpse(
    mut commands: Commands,
    assets: Option<Res<RigAssets>>,
    dying_q: Query<(Entity, &Transform, &Health), (With<Enemy>, With<Rigged>, Without<Dying>)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, tf, hp) in dying_q.iter() {
        if !hp.dead() {
            continue;
        }
        commands.entity(entity).insert(Dying);
        let root = commands
            .spawn((*tf, Visibility::default(), Corpse { age: 0.0 }))
            .id();
        let (rig_root, mut rig) = build_rig(&mut commands, &assets, Actor::Husk, root);
        rig.prev_pos = tf.translation;
        rig.face_yaw = yaw_of(tf);
        rig.death = 0.001; // already dying — the fall starts on frame one
        commands.entity(rig_root).insert(rig);
    }
}

fn tick_corpses(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Corpse)>) {
    let dt = time.delta_secs();
    for (e, mut c) in q.iter_mut() {
        c.age += dt;
        if c.age > CORPSE_TIME {
            commands.entity(e).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// The animation driver
// ---------------------------------------------------------------------------

/// What the actor is doing right now, and how far through it is. Everything the
/// rig needs to pick a pose, read straight off combat's own state machines.
struct Beat {
    action: Action,
    /// Normalised progress through the action, 0..1.
    t: f32,
    /// Fraction of the action at which the hitbox opens / closes. The blade is at
    /// its fastest exactly across this window.
    active: (f32, f32),
    combo: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    None,
    Swing,
    Dodge,
    Guard,
    Parry,
    Stagger,
    Dead,
}

impl Beat {
    fn none() -> Self {
        Beat {
            action: Action::None,
            t: 0.0,
            active: (0.3, 0.6),
            combo: 0,
        }
    }
}

#[allow(clippy::type_complexity)]
fn animate_rigs(
    time: Res<Time>,
    player_q: Query<
        (&Transform, &FlyCam, &PlayerCombat, &Health),
        (With<FlyCam>, Without<Rig>, Without<Joint>, Without<Enemy>),
    >,
    enemy_q: Query<
        (&Transform, &Enemy, &Health),
        (With<Enemy>, Without<Rig>, Without<Joint>, Without<FlyCam>),
    >,
    corpse_q: Query<&Transform, (With<Corpse>, Without<Rig>, Without<Joint>)>,
    mut rig_q: Query<
        (&ChildOf, &mut Rig, &mut Transform),
        (Without<Joint>, Without<FlyCam>, Without<Enemy>, Without<Corpse>),
    >,
    mut joint_q: Query<&mut Transform, JointFilter>,
) {
    let dt = time.delta_secs().clamp(1.0 / 240.0, 1.0 / 15.0);
    let elapsed = time.elapsed_secs();

    for (parent, mut rig, mut root_tf) in rig_q.iter_mut() {
        let owner = parent.0;

        // ---- read the actor -------------------------------------------------
        let (pos, root_yaw, want_yaw, grounded, fall, beat, hp) =
            if let Ok((tf, fly, pc, hp)) = player_q.get(owner) {
                let y = yaw_of(tf);
                (
                    tf.translation,
                    y,
                    y,
                    fly.grounded || !fly.walking,
                    fly.vel.y,
                    player_beat(pc),
                    Some(hp.cur),
                )
            } else if let Ok((tf, e, hp)) = enemy_q.get(owner) {
                // The Husk's root yaw is combat's own bookkeeping and does not point
                // its mesh anywhere in particular, so the rig aims itself: down its
                // travel direction while it walks, and it holds that heading while it
                // stands and winds up.
                let y = yaw_of(tf);
                let moved = tf.translation - rig.prev_pos;
                let flat = Vec3::new(moved.x, 0.0, moved.z);
                let want = if flat.length() > 0.004 {
                    (-flat.x).atan2(-flat.z)
                } else {
                    rig.face_yaw
                };
                (tf.translation, y, want, true, 0.0, husk_beat(e), Some(hp.cur))
            } else if let Ok(tf) = corpse_q.get(owner) {
                // A corpse keeps the heading it died with — it is not turning anywhere.
                let face = rig.face_yaw;
                (tf.translation, yaw_of(tf), face, true, 0.0, Beat::none(), None)
            } else {
                continue;
            };

        // ---- speed, stride phase, facing ------------------------------------
        let moved = Vec3::new(pos.x - rig.prev_pos.x, 0.0, pos.z - rig.prev_pos.z);
        rig.prev_pos = pos;
        let raw = (moved.length() / dt).min(SPEED_CLIP);
        rig.speed += (raw - rig.speed) * (SPEED_LAG * dt).min(1.0);
        let speed = rig.speed;

        let run = smoothstep(RUN_LO, RUN_HI, speed);
        let loco = smoothstep(IDLE_SPEED, WALK_SPEED, speed);
        let stride = STRIDE_WALK + (STRIDE_RUN - STRIDE_WALK) * run;
        rig.phase = wrap_tau(rig.phase + (speed / stride) * std::f32::consts::TAU * dt);

        let prev_face = rig.face_yaw;
        rig.face_yaw = wrap_pi(prev_face + wrap_pi(want_yaw - prev_face));
        let yaw_rate = wrap_pi(rig.face_yaw - prev_face) / dt;
        rig.yaw_rate += (yaw_rate - rig.yaw_rate) * (8.0 * dt).min(1.0);
        // Turn-in-place only counts while the actor is NOT travelling — a moving
        // body turns with its stride, a planted one has to shuffle.
        let turn = (rig.yaw_rate.abs() / TURN_FULL).clamp(0.0, 1.0) * (1.0 - loco);
        rig.turn_phase = wrap_tau(rig.turn_phase + rig.yaw_rate.abs() * 2.2 * dt);

        // ---- airborne / landing ---------------------------------------------
        if grounded {
            if rig.air > 0.05 && rig.prev_fall < -2.0 {
                rig.land = LAND_TIME;
                rig.land_amp = (rig.prev_fall.abs() / LAND_FULL).clamp(0.25, 1.0);
            }
            rig.air = 0.0;
        } else {
            rig.air += dt;
        }
        rig.prev_fall = fall;
        rig.land = (rig.land - dt).max(0.0);
        let air_w = smoothstep(0.06, 0.25, rig.air);

        // ---- hit reaction ---------------------------------------------------
        if let Some(cur) = hp {
            if cur < rig.prev_hp - 0.001 && rig.prev_hp != f32::MAX && cur > 0.0 {
                rig.hit = HIT_TIME;
                rig.hit_side = -rig.hit_side;
            }
            // A respawn refills HP; clear the death fall with it.
            if cur > rig.prev_hp + 0.001 {
                rig.death = 0.0;
            }
            rig.prev_hp = cur;
        }
        rig.hit = (rig.hit - dt).max(0.0);

        // ---- guard latches ---------------------------------------------------
        rig.guard = if beat.action == Action::Guard {
            GUARD_LATCH
        } else {
            (rig.guard - dt).max(0.0)
        };
        rig.parry = if beat.action == Action::Parry {
            GUARD_LATCH
        } else {
            (rig.parry - dt).max(0.0)
        };

        // ---- death -----------------------------------------------------------
        if beat.action == Action::Dead || rig.death > 0.0 {
            rig.death = (rig.death + dt / 0.85).min(1.0);
        }

        // =====================================================================
        // Pose
        // =====================================================================
        let d = Dims::of(rig.actor);
        let mut pose = Pose::default();

        locomotion(&mut pose, &rig, &d, loco, run, elapsed);
        turn_in_place(&mut pose, &rig, turn);
        airborne(&mut pose, air_w);

        // Attacks/guards override the upper body and bias the stance.
        let action = if rig.death > 0.0 {
            Action::Dead
        } else if beat.action == Action::Swing || beat.action == Action::Dodge {
            beat.action
        } else if beat.action == Action::Stagger {
            Action::Stagger
        } else if rig.parry > 0.0 {
            Action::Parry
        } else if rig.guard > 0.0 {
            Action::Guard
        } else {
            Action::None
        };

        let mut root_extra = Quat::IDENTITY;
        let mut root_lift = 0.0f32;
        match action {
            Action::Swing => apply_key(&mut pose, swing_pose(rig.actor, &beat)),
            Action::Guard => apply_key(&mut pose, GUARD_KEY),
            Action::Parry => apply_key(&mut pose, PARRY_KEY),
            Action::Stagger => stagger(&mut pose, elapsed),
            Action::Dodge => {
                // A forward roll: the whole rig tumbles about its own X axis while
                // the limbs tuck. The root translation is combat's (§2.2 glide) —
                // this only spins the body that rides it.
                let t = beat.t.clamp(0.0, 1.0);
                root_extra = Quat::from_axis_angle(Vec3::X, -std::f32::consts::TAU * ease_io(t));
                root_lift = (t * std::f32::consts::PI).sin() * d.leg() * 0.35;
                apply_key(&mut pose, tuck_key(t));
            }
            Action::Dead => {
                let t = ease_io(rig.death);
                root_extra = Quat::from_axis_angle(Vec3::X, -1.52 * t)
                    * Quat::from_axis_angle(Vec3::Y, 0.35 * t);
                root_lift = -d.hip_y * 0.55 * t;
                apply_key(&mut pose, fallen_key(t));
            }
            Action::None => {}
        }

        // Additive on top of everything: recoil, then the landing spring.
        hit_react(&mut pose, &rig);
        let squash = landing(&mut pose, &rig, &d);

        // ---- write the rig ---------------------------------------------------
        let root_dy = match rig.actor {
            Actor::Player => -EYE_HEIGHT,
            Actor::Husk => 0.0,
        };
        root_tf.translation = Vec3::new(0.0, root_dy + root_lift, 0.0);
        root_tf.rotation = Quat::from_axis_angle(Vec3::Y, wrap_pi(rig.face_yaw - root_yaw)) * root_extra;
        root_tf.scale = squash;

        set(&mut joint_q, rig.hips, pose.hips_pos + Vec3::Y * d.hip_y, pose.hips);
        set(&mut joint_q, rig.torso, Vec3::ZERO, pose.torso);
        set(&mut joint_q, rig.head, Vec3::Y * d.neck_y, pose.head);
        set(&mut joint_q, rig.sh_l, Vec3::new(-d.sh_x, d.sh_y, 0.0), pose.sh_l);
        set(&mut joint_q, rig.el_l, Vec3::new(0.0, -d.upper, 0.0), pose.el_l);
        set(&mut joint_q, rig.sh_r, Vec3::new(d.sh_x, d.sh_y, 0.0), pose.sh_r);
        set(&mut joint_q, rig.el_r, Vec3::new(0.0, -d.upper, 0.0), pose.el_r);
        set(&mut joint_q, rig.hand_r, Vec3::new(0.0, -d.fore, 0.0), pose.hand);
        set(&mut joint_q, rig.hip_l, Vec3::new(-d.hip_x, 0.0, 0.0), pose.hip_l);
        set(&mut joint_q, rig.knee_l, Vec3::new(0.0, -d.thigh, 0.0), pose.knee_l);
        set(&mut joint_q, rig.hip_r, Vec3::new(d.hip_x, 0.0, 0.0), pose.hip_r);
        set(&mut joint_q, rig.knee_r, Vec3::new(0.0, -d.thigh, 0.0), pose.knee_r);
    }
}

/// Filter shared by the joint query and the [`set`] helper — spelled once so the two
/// can never drift apart (a mismatch is a compile error nobody enjoys reading).
type JointFilter = (
    With<Joint>,
    Without<Rig>,
    Without<FlyCam>,
    Without<Enemy>,
    Without<Corpse>,
);

fn set(q: &mut Query<&mut Transform, JointFilter>, e: Entity, pos: Vec3, rot: Quat) {
    if let Ok(mut t) = q.get_mut(e) {
        t.translation = pos;
        t.rotation = rot;
    }
}

// ---------------------------------------------------------------------------
// Pose model
// ---------------------------------------------------------------------------

/// One frame of the rig, joint by joint. Built additively: locomotion lays down
/// the base, actions overwrite the parts they own, reactions add on top.
#[derive(Clone, Copy)]
struct Pose {
    hips_pos: Vec3,
    hips: Quat,
    torso: Quat,
    head: Quat,
    sh_l: Quat,
    el_l: Quat,
    sh_r: Quat,
    el_r: Quat,
    hand: Quat,
    hip_l: Quat,
    knee_l: Quat,
    hip_r: Quat,
    knee_r: Quat,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            hips_pos: Vec3::ZERO,
            hips: Quat::IDENTITY,
            torso: Quat::IDENTITY,
            head: Quat::IDENTITY,
            sh_l: Quat::IDENTITY,
            el_l: Quat::IDENTITY,
            sh_r: Quat::IDENTITY,
            el_r: Quat::IDENTITY,
            hand: Quat::IDENTITY,
            hip_l: Quat::IDENTITY,
            knee_l: Quat::IDENTITY,
            hip_r: Quat::IDENTITY,
            knee_r: Quat::IDENTITY,
        }
    }
}

/// ① Idle → walk → run, blended on real speed. Phase comes from distance travelled,
/// so the contact pose lands at the same point on the ground at every speed.
/// ⑥ lives here too: the vertical bob, the lateral sway and the counter-bobbing head.
fn locomotion(pose: &mut Pose, rig: &Rig, d: &Dims, loco: f32, run: f32, elapsed: f32) {
    let ph = rig.phase;
    let s = ph.sin();

    // --- idle: slow breath, arms hanging with a touch of sway.
    let idle = 1.0 - loco;
    let breath = (elapsed * 1.35).sin();
    let idle_torso = pitch(0.020 * breath) * yaw(0.020 * (elapsed * 0.7).sin());
    let idle_arm = 0.09 + 0.035 * breath;

    // --- stride amplitudes ramp from a walk into a run.
    let leg_a = (0.42 + 0.55 * run) * loco;
    let arm_a = (0.34 + 0.62 * run) * loco;
    let knee_a = (0.75 + 0.65 * run) * loco;
    let lean = -(0.05 + 0.26 * run) * loco;

    // Hips: dip twice per cycle (once per foot-fall) and sway once.
    let bob = -d.bob * (1.0 - (2.0 * ph).cos()) * 0.5 * (1.0 + run) * loco;
    let sway = d.sway * s * loco;
    pose.hips_pos = Vec3::new(sway, bob, 0.0);
    pose.hips = yaw(0.10 * s * loco) * roll(-0.06 * s * loco);

    // Shoulders counter-rotate against the hips; the spine leans into the speed.
    pose.torso = (yaw(-0.17 * s * loco) * pitch(lean)).slerp(idle_torso, idle);

    // The head resists the bob and the twist — the classic "eyes stay level" trick.
    pose.head = pitch(-lean * 0.55 - 0.10 * (2.0 * ph).cos() * loco) * yaw(0.12 * s * loco);

    // Legs: hip swings, knee only ever folds backwards (negative), peaking just
    // after the foot leaves the ground.
    let hip_l = leg_a * s;
    let hip_r = leg_a * (ph + std::f32::consts::PI).sin();
    pose.hip_l = pitch(hip_l);
    pose.hip_r = pitch(hip_r);
    pose.knee_l = pitch(-knee_a * (ph + 1.15).sin().max(0.0));
    pose.knee_r = pitch(-knee_a * (ph + 1.15 + std::f32::consts::PI).sin().max(0.0));

    // Arms swing opposite their leg, elbows keep a permanent soft bend.
    pose.sh_l = pitch(-arm_a * s - idle_arm * idle * 0.2) * roll(-idle_arm * idle);
    pose.sh_r = pitch(arm_a * s - idle_arm * idle * 0.2) * roll(idle_arm * idle);
    pose.el_l = pitch(0.14 + 0.55 * arm_a * (-s).max(0.0));
    pose.el_r = pitch(0.14 + 0.55 * arm_a * s.max(0.0));
    // Resting grip: the blade hangs from the fist, so with the arm down an IDENTITY
    // wrist would drive the point straight through the floor. Angle it back instead
    // — the "carried at the side, ready" read every action game uses.
    pose.hand = pitch(-0.90) * roll(0.12);
}

/// ② Turn-in-place. A planted body cannot rotate rigidly: the head leads, the chest
/// follows, the hips come last, and the feet cross-step to catch up.
fn turn_in_place(pose: &mut Pose, rig: &Rig, turn: f32) {
    if turn <= 0.001 {
        return;
    }
    let dir = rig.yaw_rate.signum();
    let step = rig.turn_phase.sin();

    pose.head = yaw(0.42 * turn * dir) * pose.head;
    pose.torso = yaw(0.26 * turn * dir) * pose.torso;
    pose.hips = yaw(-0.18 * turn * dir) * pose.hips;

    // One foot pivots while the other steps around it, alternating on turn_phase.
    let a = 0.34 * turn * step;
    pose.hip_l = pitch(a * dir) * roll(-0.10 * turn * dir) * pose.hip_l;
    pose.hip_r = pitch(-a * dir) * roll(-0.10 * turn * dir) * pose.hip_r;
    pose.knee_l = pitch(-0.42 * turn * step.max(0.0)) * pose.knee_l;
    pose.knee_r = pitch(-0.42 * turn * (-step).max(0.0)) * pose.knee_r;
    pose.hips_pos.y -= 0.02 * turn;
}

/// Legs tuck and arms rise while off the ground, so a jump does not look like a
/// standing pose being levitated.
fn airborne(pose: &mut Pose, w: f32) {
    if w <= 0.001 {
        return;
    }
    pose.hip_l = pose.hip_l.slerp(pitch(0.42), w);
    pose.hip_r = pose.hip_r.slerp(pitch(0.18), w);
    pose.knee_l = pose.knee_l.slerp(pitch(-0.95), w);
    pose.knee_r = pose.knee_r.slerp(pitch(-0.55), w);
    pose.sh_l = pose.sh_l.slerp(pitch(-0.55) * roll(-0.45), w);
    pose.sh_r = pose.sh_r.slerp(pitch(-0.35) * roll(0.45), w);
    pose.torso = pose.torso.slerp(pitch(-0.14), w);
}

/// ④ Hit reaction — a sharp recoil that rings down over [`HIT_TIME`]. Additive, so
/// it reads whether the actor is standing, walking or mid-combo.
fn hit_react(pose: &mut Pose, rig: &Rig) {
    if rig.hit <= 0.0 {
        return;
    }
    let k = rig.hit / HIT_TIME; // 1 → 0
    let e = k * k * (k * 13.0).cos(); // snap, then ring out
    let side = rig.hit_side;

    pose.torso = pitch(0.34 * e) * yaw(0.22 * e * side) * pose.torso;
    pose.head = pitch(0.46 * e) * yaw(-0.30 * e * side) * pose.head;
    pose.sh_l = roll(-0.55 * e.abs()) * pose.sh_l;
    pose.sh_r = roll(0.55 * e.abs()) * pose.sh_r;
    pose.hips_pos.y -= 0.06 * e.abs();
    pose.knee_l = pitch(-0.30 * e.abs()) * pose.knee_l;
    pose.knee_r = pitch(-0.30 * e.abs()) * pose.knee_r;
}

/// ⑥ Landing squash — a damped spring on the rig's scale plus a knee absorb. Returns
/// the root scale so the caller can write it in one go.
fn landing(pose: &mut Pose, rig: &Rig, d: &Dims) -> Vec3 {
    if rig.land <= 0.0 {
        return Vec3::ONE;
    }
    let u = 1.0 - rig.land / LAND_TIME; // 0 → 1
    let q = rig.land_amp * (-9.0 * u).exp() * (u * 19.0).cos();

    pose.hips_pos.y -= d.hip_y * 0.30 * q.max(0.0);
    pose.knee_l = pitch(-1.05 * q.max(0.0)) * pose.knee_l;
    pose.knee_r = pitch(-1.05 * q.max(0.0)) * pose.knee_r;
    pose.hip_l = pitch(0.42 * q.max(0.0)) * pose.hip_l;
    pose.hip_r = pitch(0.42 * q.max(0.0)) * pose.hip_r;
    pose.torso = pitch(-0.26 * q.max(0.0)) * pose.torso;

    Vec3::new(1.0 + 0.16 * q, 1.0 - 0.24 * q, 1.0 + 0.16 * q)
}

/// Stagger — poise broken. A loose, off-balance wobble that decays on its own clock.
fn stagger(pose: &mut Pose, elapsed: f32) {
    let w = (elapsed * 11.0).sin();
    pose.torso = pitch(0.30 + 0.10 * w) * yaw(0.22 * w) * pose.torso;
    pose.head = pitch(0.28) * yaw(-0.26 * w) * pose.head;
    pose.sh_l = pitch(-0.70) * roll(-0.75) * pose.sh_l;
    pose.sh_r = pitch(-0.55) * roll(0.85) * pose.sh_r;
    pose.hip_r = pitch(-0.40) * pose.hip_r;
    pose.knee_l = pitch(-0.55) * pose.knee_l;
    pose.hips_pos.y -= 0.10;
}

// ---------------------------------------------------------------------------
// Keyframed action poses
// ---------------------------------------------------------------------------

/// One authored pose. Angles are radians; `lunge` pushes the hips along local -Z,
/// `crouch` drops them, `stance` folds both knees.
#[derive(Clone, Copy, Default)]
struct Key {
    sh_pitch: f32,
    sh_yaw: f32,
    sh_roll: f32,
    elbow: f32,
    grip_pitch: f32,
    grip_roll: f32,
    off_pitch: f32,
    off_roll: f32,
    off_elbow: f32,
    torso_yaw: f32,
    torso_pitch: f32,
    hips_yaw: f32,
    head_yaw: f32,
    head_pitch: f32,
    lunge: f32,
    crouch: f32,
    stance: f32,
    /// 0 = keep the locomotion pose, 1 = fully this key. Lets a swing own the arms
    /// while the legs keep walking.
    weight: f32,
}

fn mix(a: &Key, b: &Key, t: f32) -> Key {
    let l = |x: f32, y: f32| x + (y - x) * t;
    Key {
        sh_pitch: l(a.sh_pitch, b.sh_pitch),
        sh_yaw: l(a.sh_yaw, b.sh_yaw),
        sh_roll: l(a.sh_roll, b.sh_roll),
        elbow: l(a.elbow, b.elbow),
        grip_pitch: l(a.grip_pitch, b.grip_pitch),
        grip_roll: l(a.grip_roll, b.grip_roll),
        off_pitch: l(a.off_pitch, b.off_pitch),
        off_roll: l(a.off_roll, b.off_roll),
        off_elbow: l(a.off_elbow, b.off_elbow),
        torso_yaw: l(a.torso_yaw, b.torso_yaw),
        torso_pitch: l(a.torso_pitch, b.torso_pitch),
        hips_yaw: l(a.hips_yaw, b.hips_yaw),
        head_yaw: l(a.head_yaw, b.head_yaw),
        head_pitch: l(a.head_pitch, b.head_pitch),
        lunge: l(a.lunge, b.lunge),
        crouch: l(a.crouch, b.crouch),
        stance: l(a.stance, b.stance),
        weight: l(a.weight, b.weight),
    }
}

/// Blend a key over whatever locomotion already wrote, by its own `weight`.
fn apply_key(pose: &mut Pose, k: Key) {
    let w = k.weight.clamp(0.0, 1.0);
    if w <= 0.001 {
        return;
    }
    pose.sh_r = pose
        .sh_r
        .slerp(pitch(k.sh_pitch) * yaw(k.sh_yaw) * roll(k.sh_roll), w);
    pose.el_r = pose.el_r.slerp(pitch(k.elbow), w);
    pose.hand = pose
        .hand
        .slerp(pitch(k.grip_pitch) * roll(k.grip_roll), w);
    pose.sh_l = pose
        .sh_l
        .slerp(pitch(k.off_pitch) * roll(k.off_roll), w);
    pose.el_l = pose.el_l.slerp(pitch(k.off_elbow), w);

    pose.torso = pose
        .torso
        .slerp(yaw(k.torso_yaw) * pitch(k.torso_pitch), w * 0.9);
    pose.hips = pose.hips.slerp(yaw(k.hips_yaw), w * 0.9);
    pose.head = pose
        .head
        .slerp(yaw(k.head_yaw) * pitch(k.head_pitch), w * 0.85);

    // The lower body only takes the lunge/crouch/stance bias — it keeps walking.
    pose.hips_pos.z -= k.lunge * w;
    pose.hips_pos.y -= k.crouch * w;
    pose.knee_l = pitch(-k.stance * w) * pose.knee_l;
    pose.knee_r = pitch(-k.stance * w) * pose.knee_r;
    pose.hip_l = pitch(k.stance * 0.45 * w) * pose.hip_l;
    pose.hip_r = pitch(k.stance * 0.45 * w) * pose.hip_r;
}

/// ③ The swing. `beat.active` is combat's own hitbox window, normalised — the blade
/// travels from cocked to follow-through across exactly that span, so the frame the
/// damage lands is the frame the arc is fastest.
///
/// Three legs, sampled by `k ∈ [0,3]`:
///   0→1 wind-up   (ease-out: snappy anticipation that settles into the cock)
///   1→2 STRIKE    (ease-in-out over `active`)
///   2→3 recovery  (ease-out back to neutral)
fn swing_pose(actor: Actor, beat: &Beat) -> Key {
    let (cock, follow) = match actor {
        Actor::Husk => (HUSK_COCK, HUSK_FOLLOW),
        Actor::Player => match beat.combo {
            // The three-hit light chain alternates its arc so a combo reads as one
            // continuous sentence rather than the same swing three times.
            1 => (LIGHT1_COCK, LIGHT1_FOLLOW),
            2 => (LIGHT2_COCK, LIGHT2_FOLLOW),
            3 => (LIGHT3_COCK, LIGHT3_FOLLOW),
            _ => (HEAVY_COCK, HEAVY_FOLLOW),
        },
    };
    let neutral = Key {
        weight: 0.0,
        ..Default::default()
    };

    let (a0, a1) = beat.active;
    let t = beat.t.clamp(0.0, 1.0);
    let k = if t < a0 {
        ease_out(t / a0.max(1e-3))
    } else if t < a1 {
        1.0 + ease_io((t - a0) / (a1 - a0).max(1e-3))
    } else {
        2.0 + ease_out((t - a1) / (1.0 - a1).max(1e-3))
    };

    if k < 1.0 {
        mix(&neutral, &cock, k)
    } else if k < 2.0 {
        mix(&cock, &follow, k - 1.0)
    } else {
        mix(&follow, &neutral, k - 2.0)
    }
}

// --- authored keys ---------------------------------------------------------

/// Light 1: a flat horizontal cut, right shoulder to left hip.
const LIGHT1_COCK: Key = Key {
    sh_pitch: -0.55,
    sh_yaw: 1.10,
    sh_roll: -0.35,
    elbow: 1.45,
    grip_pitch: -0.30,
    grip_roll: 0.45,
    off_pitch: 0.30,
    off_roll: -0.45,
    off_elbow: 0.85,
    torso_yaw: 0.52,
    torso_pitch: 0.08,
    hips_yaw: 0.22,
    head_yaw: -0.22,
    head_pitch: -0.05,
    lunge: -0.06,
    crouch: 0.03,
    stance: 0.22,
    weight: 1.0,
};
const LIGHT1_FOLLOW: Key = Key {
    sh_pitch: 0.38,
    sh_yaw: -1.25,
    sh_roll: 0.28,
    elbow: 0.32,
    grip_pitch: 0.25,
    grip_roll: -0.25,
    off_pitch: -0.25,
    off_roll: 0.55,
    off_elbow: 0.35,
    torso_yaw: -0.62,
    torso_pitch: -0.14,
    hips_yaw: -0.28,
    head_yaw: 0.20,
    head_pitch: 0.10,
    lunge: 0.32,
    crouch: 0.09,
    stance: 0.42,
    weight: 1.0,
};

/// Light 2: the backhand return, left to right.
const LIGHT2_COCK: Key = Key {
    sh_pitch: 0.10,
    sh_yaw: -1.15,
    sh_roll: 0.40,
    elbow: 1.30,
    grip_pitch: -0.20,
    grip_roll: -0.50,
    off_pitch: 0.20,
    off_roll: 0.40,
    off_elbow: 0.90,
    torso_yaw: -0.50,
    torso_pitch: 0.06,
    hips_yaw: -0.20,
    head_yaw: 0.24,
    head_pitch: -0.04,
    lunge: -0.05,
    crouch: 0.04,
    stance: 0.24,
    weight: 1.0,
};
const LIGHT2_FOLLOW: Key = Key {
    sh_pitch: -0.15,
    sh_yaw: 1.30,
    sh_roll: -0.30,
    elbow: 0.28,
    grip_pitch: 0.30,
    grip_roll: 0.35,
    off_pitch: -0.30,
    off_roll: -0.55,
    off_elbow: 0.30,
    torso_yaw: 0.66,
    torso_pitch: -0.10,
    hips_yaw: 0.30,
    head_yaw: -0.22,
    head_pitch: 0.08,
    lunge: 0.30,
    crouch: 0.08,
    stance: 0.40,
    weight: 1.0,
};

/// Light 3: the finisher — a downward diagonal that ends the chain low.
const LIGHT3_COCK: Key = Key {
    sh_pitch: -2.20,
    sh_yaw: 0.45,
    sh_roll: -0.20,
    elbow: 1.55,
    grip_pitch: -0.55,
    grip_roll: 0.20,
    off_pitch: -0.35,
    off_roll: -0.30,
    off_elbow: 1.00,
    torso_yaw: 0.30,
    torso_pitch: 0.26,
    hips_yaw: 0.12,
    head_yaw: -0.10,
    head_pitch: -0.22,
    lunge: -0.10,
    crouch: 0.0,
    stance: 0.18,
    weight: 1.0,
};
const LIGHT3_FOLLOW: Key = Key {
    sh_pitch: 1.05,
    sh_yaw: -0.35,
    sh_roll: 0.15,
    elbow: 0.18,
    grip_pitch: 0.45,
    grip_roll: -0.15,
    off_pitch: 0.30,
    off_roll: 0.40,
    off_elbow: 0.25,
    torso_yaw: -0.34,
    torso_pitch: -0.48,
    hips_yaw: -0.16,
    head_yaw: 0.12,
    head_pitch: 0.26,
    lunge: 0.42,
    crouch: 0.18,
    stance: 0.58,
    weight: 1.0,
};

/// Heavy / charged: a full overhead. The long `active` window in combat gives this
/// a slow, readable wind-up and a slam you can see coming — which is the point (§2.4).
const HEAVY_COCK: Key = Key {
    sh_pitch: -2.55,
    sh_yaw: 0.20,
    sh_roll: -0.15,
    elbow: 1.70,
    grip_pitch: -0.70,
    grip_roll: 0.10,
    off_pitch: -0.50,
    off_roll: -0.25,
    off_elbow: 1.20,
    torso_yaw: 0.18,
    torso_pitch: 0.34,
    hips_yaw: 0.10,
    head_yaw: -0.06,
    head_pitch: -0.30,
    lunge: -0.16,
    crouch: 0.02,
    stance: 0.20,
    weight: 1.0,
};
const HEAVY_FOLLOW: Key = Key {
    sh_pitch: 1.25,
    sh_yaw: -0.18,
    sh_roll: 0.10,
    elbow: 0.12,
    grip_pitch: 0.55,
    grip_roll: -0.10,
    off_pitch: 0.45,
    off_roll: 0.35,
    off_elbow: 0.20,
    torso_yaw: -0.22,
    torso_pitch: -0.60,
    hips_yaw: -0.12,
    head_yaw: 0.06,
    head_pitch: 0.34,
    lunge: 0.58,
    crouch: 0.26,
    stance: 0.72,
    weight: 1.0,
};

/// The Husk's two-handed overhead. Bigger and slower than the player's — its whole
/// silhouette has to telegraph for `HUSK_TELEGRAPH` (0.8 s) before it commits.
const HUSK_COCK: Key = Key {
    sh_pitch: -2.70,
    sh_yaw: 0.10,
    sh_roll: -0.30,
    elbow: 1.35,
    grip_pitch: -0.60,
    grip_roll: 0.0,
    off_pitch: -2.30,
    off_roll: 0.35,
    off_elbow: 1.20,
    torso_yaw: 0.10,
    torso_pitch: 0.40,
    hips_yaw: 0.06,
    head_yaw: 0.0,
    head_pitch: -0.34,
    lunge: -0.20,
    crouch: 0.0,
    stance: 0.22,
    weight: 1.0,
};
const HUSK_FOLLOW: Key = Key {
    sh_pitch: 1.35,
    sh_yaw: -0.08,
    sh_roll: 0.20,
    elbow: 0.10,
    grip_pitch: 0.50,
    grip_roll: 0.0,
    off_pitch: 1.10,
    off_roll: -0.25,
    off_elbow: 0.25,
    torso_yaw: -0.14,
    torso_pitch: -0.66,
    hips_yaw: -0.08,
    head_yaw: 0.0,
    head_pitch: 0.40,
    lunge: 0.62,
    crouch: 0.30,
    stance: 0.80,
    weight: 1.0,
};

/// Guard — weapon up across the body, weight back, knees loaded.
const GUARD_KEY: Key = Key {
    sh_pitch: -0.85,
    sh_yaw: -0.55,
    sh_roll: 0.30,
    elbow: 1.35,
    grip_pitch: -1.25,
    grip_roll: 0.0,
    off_pitch: -0.95,
    off_roll: 0.55,
    off_elbow: 1.45,
    torso_yaw: 0.30,
    torso_pitch: 0.10,
    hips_yaw: 0.16,
    head_yaw: -0.12,
    head_pitch: -0.05,
    lunge: -0.08,
    crouch: 0.10,
    stance: 0.46,
    weight: 1.0,
};

/// Parry — the same guard, flicked outward. Short and sharp (§2.5: 12 frames).
const PARRY_KEY: Key = Key {
    sh_pitch: -1.05,
    sh_yaw: 0.85,
    sh_roll: -0.45,
    elbow: 0.55,
    grip_pitch: -0.80,
    grip_roll: 0.60,
    off_pitch: -0.60,
    off_roll: 0.35,
    off_elbow: 1.10,
    torso_yaw: -0.30,
    torso_pitch: -0.06,
    hips_yaw: -0.14,
    head_yaw: 0.10,
    head_pitch: 0.04,
    lunge: 0.10,
    crouch: 0.06,
    stance: 0.34,
    weight: 1.0,
};

/// Mid-roll tuck — limbs pulled in, tightest at the apex.
fn tuck_key(t: f32) -> Key {
    let w = (t * std::f32::consts::PI).sin();
    Key {
        sh_pitch: -1.35 * w,
        sh_roll: 0.55 * w,
        elbow: 1.85 * w,
        grip_pitch: -0.60 * w,
        off_pitch: -1.35 * w,
        off_roll: -0.55 * w,
        off_elbow: 1.85 * w,
        torso_pitch: 0.55 * w,
        head_pitch: -0.45 * w,
        crouch: 0.24 * w,
        stance: 1.40 * w,
        weight: 0.55 + 0.45 * w,
        ..Default::default()
    }
}

/// ⑤ Fallen — knees give first, then the body goes over and the arms drop out.
fn fallen_key(t: f32) -> Key {
    Key {
        sh_pitch: -0.45 * t,
        sh_roll: 0.80 * t,
        elbow: 0.35 * t,
        grip_pitch: 0.60 * t,
        off_pitch: -0.30 * t,
        off_roll: -0.90 * t,
        off_elbow: 0.30 * t,
        torso_pitch: 0.42 * t,
        head_pitch: 0.55 * t,
        head_yaw: 0.22 * t,
        crouch: 0.30 * t,
        stance: 1.15 * t,
        weight: t,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Reading combat's state machines
// ---------------------------------------------------------------------------

/// Map [`PlayerCombat`] onto a [`Beat`]. Every duration and every active window is
/// combat's own constant — nothing is re-timed here, so the arc can never drift out
/// of sync with the hitbox.
fn player_beat(pc: &PlayerCombat) -> Beat {
    let norm = |t: f32, len: f32, w: (f32, f32)| (t / len, (w.0 / len, w.1 / len));
    match pc.state {
        CombatState::Light => {
            let (t, a) = norm(pc.timer, combat::LIGHT_TIME, combat::LIGHT_ACTIVE);
            Beat {
                action: Action::Swing,
                t,
                active: a,
                combo: pc.combo.clamp(1, 3),
            }
        }
        CombatState::Heavy => {
            let (t, a) = norm(pc.timer, combat::HEAVY_TIME, combat::HEAVY_ACTIVE);
            Beat {
                action: Action::Swing,
                t,
                active: a,
                combo: 0,
            }
        }
        CombatState::Charged => {
            let (t, a) = norm(pc.timer, combat::CHARGED_TIME, combat::CHARGED_ACTIVE);
            Beat {
                action: Action::Swing,
                t,
                active: a,
                combo: 0,
            }
        }
        CombatState::Dodge => Beat {
            action: Action::Dodge,
            t: pc.timer / (combat::DODGE_IFRAMES + combat::DODGE_RECOVERY),
            active: (0.3, 0.6),
            combo: 0,
        },
        CombatState::Block => Beat {
            action: Action::Guard,
            ..Beat::none()
        },
        CombatState::Parry => Beat {
            action: Action::Parry,
            ..Beat::none()
        },
        CombatState::Stagger => Beat {
            action: Action::Stagger,
            ..Beat::none()
        },
        CombatState::Dead => Beat {
            action: Action::Dead,
            ..Beat::none()
        },
        CombatState::Idle => {
            // Holding the heavy button past CHARGE_HOLD commits a charged attack, so
            // show the wind-up while it charges — the tell the design asks for (§2.4).
            if pc.charge > 0.0 {
                let t = (pc.charge / combat::CHARGE_HOLD).min(1.0) * 0.45;
                Beat {
                    action: Action::Swing,
                    t,
                    active: (0.5, 0.8),
                    combo: 0,
                }
            } else {
                Beat::none()
            }
        }
    }
}

/// Map [`HuskState`] onto a [`Beat`]. The Husk's overhead is one continuous arc
/// stretched across four AI states: Telegraph winds it, Swing lands it, Gap re-cocks
/// it for the second hit, Recover puts it away.
fn husk_beat(e: &Enemy) -> Beat {
    // Telegraph occupies the wind-up leg, the swing occupies the strike leg.
    let wind = 0.62;
    match e.state {
        HuskState::Telegraph => Beat {
            action: Action::Swing,
            t: (e.timer / combat::HUSK_TELEGRAPH).min(1.0) * wind,
            active: (wind, 0.80),
            combo: 0,
        },
        HuskState::Swing1 | HuskState::Swing2 => {
            let k = (e.timer / combat::HUSK_ACTIVE).min(1.0);
            Beat {
                action: Action::Swing,
                t: wind + (0.80 - wind) * k,
                active: (wind, 0.80),
                combo: 0,
            }
        }
        HuskState::Gap => {
            // Snap back to the cocked pose for the second half of the combo.
            let k = (e.timer / combat::HUSK_GAP).min(1.0);
            Beat {
                action: Action::Swing,
                t: 0.80 - (0.80 - wind) * k,
                active: (wind, 0.80),
                combo: 0,
            }
        }
        HuskState::Recover => Beat {
            action: Action::Swing,
            t: 0.80 + (1.0 - 0.80) * (e.timer / combat::HUSK_COMBO_PAUSE).min(1.0),
            active: (wind, 0.80),
            combo: 0,
        },
        HuskState::Staggered => Beat {
            action: Action::Stagger,
            ..Beat::none()
        },
        HuskState::Dead => Beat {
            action: Action::Dead,
            ..Beat::none()
        },
        HuskState::Patrol | HuskState::Chase => Beat::none(),
    }
}

// ---------------------------------------------------------------------------
// Small maths
// ---------------------------------------------------------------------------

#[inline]
fn pitch(a: f32) -> Quat {
    Quat::from_axis_angle(Vec3::X, a)
}
#[inline]
fn yaw(a: f32) -> Quat {
    Quat::from_axis_angle(Vec3::Y, a)
}
#[inline]
fn roll(a: f32) -> Quat {
    Quat::from_axis_angle(Vec3::Z, a)
}

/// Y-rotation of a transform, in radians. Both actors are yaw-only, so this is exact.
#[inline]
fn yaw_of(tf: &Transform) -> f32 {
    tf.rotation.to_euler(EulerRot::YXZ).0
}

#[inline]
fn wrap_pi(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut x = (a + std::f32::consts::PI) % tau;
    if x < 0.0 {
        x += tau;
    }
    x - std::f32::consts::PI
}

#[inline]
fn wrap_tau(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let x = a % tau;
    if x < 0.0 {
        x + tau
    } else {
        x
    }
}

#[inline]
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[inline]
fn ease_io(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// ---------------------------------------------------------------------------
// Tests — the pure pose maths, provable without a GPU.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swing_peaks_inside_the_active_window() {
        // The blade must travel from cocked to follow-through ACROSS the hitbox
        // window and nowhere else — that is the whole contract with combat.rs.
        let beat = |t: f32| Beat {
            action: Action::Swing,
            t,
            active: (
                combat::LIGHT_ACTIVE.0 / combat::LIGHT_TIME,
                combat::LIGHT_ACTIVE.1 / combat::LIGHT_TIME,
            ),
            combo: 1,
        };
        let a0 = combat::LIGHT_ACTIVE.0 / combat::LIGHT_TIME;
        let a1 = combat::LIGHT_ACTIVE.1 / combat::LIGHT_TIME;

        let at_cock = swing_pose(Actor::Player, &beat(a0));
        let at_end = swing_pose(Actor::Player, &beat(a1));
        // Cocked and follow-through are the two authored extremes.
        assert!((at_cock.sh_yaw - LIGHT1_COCK.sh_yaw).abs() < 1e-3);
        assert!((at_end.sh_yaw - LIGHT1_FOLLOW.sh_yaw).abs() < 1e-3);

        // Angular travel inside the window dwarfs the travel outside it.
        let travel = |a: f32, b: f32| {
            (swing_pose(Actor::Player, &beat(b)).sh_yaw
                - swing_pose(Actor::Player, &beat(a)).sh_yaw)
                .abs()
        };
        let inside = travel(a0, a1);
        let before = travel(0.0, a0);
        let after = travel(a1, 1.0);
        assert!(inside > before, "inside={inside} before={before}");
        assert!(inside > after, "inside={inside} after={after}");
    }

    #[test]
    fn swing_starts_and_ends_neutral() {
        let b = Beat {
            action: Action::Swing,
            t: 0.0,
            active: (0.34, 0.63),
            combo: 1,
        };
        assert!(swing_pose(Actor::Player, &b).weight.abs() < 1e-4);
        let b_end = Beat { t: 1.0, ..b };
        assert!(swing_pose(Actor::Player, &b_end).weight.abs() < 1e-4);
    }

    #[test]
    fn husk_beat_is_monotonic_through_its_combo() {
        // Telegraph → Swing1 must never step backwards, or the arm would snap back
        // mid-attack and the tell would lie about when the hit lands.
        let mk = |state: HuskState, timer: f32| Enemy {
            state,
            timer,
            patrol_origin: Vec3::ZERO,
            patrol_dir: 1.0,
            facing: 0.0,
            hitstop: 0.0,
            hit_applied: false,
            surface_y: 0.0,
        };
        let wind_end = husk_beat(&mk(HuskState::Telegraph, combat::HUSK_TELEGRAPH)).t;
        let swing_start = husk_beat(&mk(HuskState::Swing1, 0.0)).t;
        let swing_end = husk_beat(&mk(HuskState::Swing1, combat::HUSK_ACTIVE)).t;
        assert!((wind_end - swing_start).abs() < 1e-4);
        assert!(swing_end > swing_start);
    }

    #[test]
    fn stride_phase_is_distance_locked() {
        // Two speeds, same distance covered → the same phase. This is what stops
        // the feet skating when the blend moves from walk to run.
        let advance = |speed: f32, dt: f32, steps: usize| {
            let mut ph = 0.0f32;
            let run = smoothstep(RUN_LO, RUN_HI, speed);
            let stride = STRIDE_WALK + (STRIDE_RUN - STRIDE_WALK) * run;
            for _ in 0..steps {
                ph += (speed / stride) * std::f32::consts::TAU * dt;
            }
            ph
        };
        // Same speed, half the timestep, twice the steps → identical phase.
        let a = advance(4.0, 1.0 / 60.0, 60);
        let b = advance(4.0, 1.0 / 120.0, 120);
        assert!((a - b).abs() < 1e-4, "a={a} b={b}");
    }

    #[test]
    fn locomotion_blends_off_at_a_standstill() {
        assert_eq!(smoothstep(IDLE_SPEED, WALK_SPEED, 0.0), 0.0);
        assert_eq!(smoothstep(IDLE_SPEED, WALK_SPEED, 5.0), 1.0);
        assert_eq!(smoothstep(RUN_LO, RUN_HI, 0.0), 0.0);
        assert_eq!(smoothstep(RUN_LO, RUN_HI, 20.0), 1.0);
    }

    #[test]
    fn wrap_pi_keeps_turns_short() {
        // Turning from +179° to -179° is a 2° step, not a 358° spin.
        let from = 3.10_f32;
        let to = -3.10_f32;
        assert!(wrap_pi(to - from).abs() < 0.1);
    }
}
