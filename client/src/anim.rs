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
//!   ⑦ conversation— a real standing/talking/listening layer for dialogue, driven
//!                   by `cutscene::Conversation` so the pose and the cutscene
//!                   lens blend in and out on the same clock. Story NPCs
//!                   (`quest::Npc`) get a rig of their own here too — see
//!                   [`RigLook`] for why that is not a third [`Actor`].
//!
//! **It cannot move a hitbox.** Every transform written here belongs to a child of
//! the actor. Combat reads only the actor's own `Transform.translation` (melee range,
//! cones, the Husk's `surface_y` clamp) and the root is never touched — so the fight
//! plays out identically with the rig on or off. `scripts/prove_combat.sh` runs the
//! full loop with this plugin live.
//!
//! Ownership: this file + `scene.rs`. `combat.rs` is read-only from here — the rig
//! observes its state, never writes it.

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::combat::{self, CombatState, Enemy, Health, HuskState, PlayerCombat};
use crate::cutscene::{Conversation, Speaker};
use crate::{Cfg, FlyCam, EYE_HEIGHT};

// ===========================================================================
// Animation timing events — the message stream anim fires for other lanes.
// ===========================================================================
//
// anim.rs is the ONLY writer of these. They carry the *visual timing* of each
// move — the exact frame the rig reaches a phase — NOT the gameplay resolution.
// That is combat.rs's own `ImpactEvent` / `DodgeEvent` / `StaggerEvent`, which
// fire only when a hit actually lands / a roll is actually paid for / poise
// breaks. The split is deliberate and documented in docs/anim-events.md:
//
//   * combat's events answer "did it connect / count?"   — gameplay truth.
//   * these events answer "where is the body in the move?" — sync the VFX spark,
//     the SFX whoosh and the camera pop to the PICTURE, hit or whiff.
//
// Every timing below is anchored to the SAME constants combat publishes
// (`LIGHT_ACTIVE`, `DODGE_IFRAMES`, `PARRY_WINDOW`, ...) via `player_beat` /
// `husk_beat`, so an anim event and the gameplay window that shares it cannot
// drift apart. Subscribe without touching this file:
//   ```ignore
//   fn on_contact(mut swings: MessageReader<anim::AnimSwing>) {
//       for s in swings.read() {
//           if s.phase == anim::SwingPhase::Contact { /* spawn the spark */ }
//       }
//   }
//   ```

/// Which leg of an attack the rig is in: anticipation -> contact -> follow-through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwingPhase {
    /// Wind-up: the blade is cocked back (the telegraph). `beat.t < active.0`.
    Windup,
    /// Contact: the hitbox is open and the blade is at its fastest. This is the
    /// frame a spark / impact SFX keys off — fired once on entry, hit or whiff.
    Contact,
    /// Follow-through: the swing has spent its energy and is recovering.
    Recover,
}

/// The rig reached an attack phase. `combo` is 1..=3 for chained lights, 0 for a
/// heavy / charged / husk swing (see `player_beat` / `husk_beat`).
#[derive(Clone, Copy, Debug)]
pub struct AnimSwing {
    pub actor: Actor,
    pub phase: SwingPhase,
    pub combo: u8,
}
impl Message for AnimSwing {}

/// Which edge of the dodge i-frame window the rig crossed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DodgePhase {
    /// The roll began — i-frames are now live (`beat.t` entered the i-frame band).
    IframeStart,
    /// The i-frames expired; the recovery leg (still tumbling, no longer safe).
    IframeEnd,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimDodge {
    pub actor: Actor,
    pub phase: DodgePhase,
}
impl Message for AnimDodge {}

/// Which edge of the parry receive window the rig crossed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParryPhase {
    /// The guard flashed into the parry pose — the receive window is open.
    Open,
    /// The window closed; a late input is now a plain block, not a deflect.
    Close,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimParry {
    pub actor: Actor,
    pub phase: ParryPhase,
}
impl Message for AnimParry {}

/// A flinch / hit-reaction fired — additive recoil on the rig the frame HP
/// dropped. Pairs with, but is separate from, combat's `StaggerEvent` (poise
// break): every stagger is a hit, not every hit is a stagger.
#[derive(Clone, Copy, Debug)]
pub struct AnimHit {
    pub actor: Actor,
}
impl Message for AnimHit {}

/// Which foot struck the ground. Twice per stride cycle, alternating — the audio
/// lane's footfall sync. Only fires while actually locomoting (the stride phase
/// is advanced by distance travelled, so a standing body emits none).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Foot {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimFootstep {
    pub actor: Actor,
    pub foot: Foot,
}
impl Message for AnimFootstep {}

/// A hard landing just hit the ground — the same edge that triggers the
/// landing-squash pose (`rig.land = LAND_TIME` below), fired as a message so
/// the VFX lane can throw dust/debris at the exact frame the fall lands
/// instead of guessing it from footsteps. Only fires on a real fall (the
/// squash gate already requires `rig.prev_fall < -2.0`); an ordinary step
/// never crosses it.
#[derive(Clone, Copy, Debug)]
pub struct AnimLand {
    pub actor: Actor,
    /// 0.25..=1.0 — how hard, mirrors `rig.land_amp` (`|fall speed| / LAND_FULL`,
    /// clamped). Scale dust count/speed on this so a stumble and a hard drop
    /// don't throw the same amount of ground debris.
    pub amp: f32,
}
impl Message for AnimLand {}

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
/// sighting instead of strobing. Also doubles as the release-blend window: the
/// pose weight eases out across this same span instead of holding at full
/// strength and vanishing the frame after — see the `Action::Guard` /
/// `Action::Parry` arms in `animate_rigs`.
const GUARD_LATCH: f32 = 0.14;
/// Same idea for stagger: how long the poise-break silhouette takes to blend
/// back into whatever the body does next, instead of snapping straight to
/// locomotion the instant `CombatState::Stagger` ends.
const STAGGER_FADE: f32 = 0.20;
/// Cloak spring (see [`CLOAK_MOTION`]): stiffness and damping of the damped
/// spring the cloak hinge chases its target angle with. Soft/underdamped on
/// purpose — cloth should overshoot and settle, a stiff spring just looks like
/// a rigid plate on a hinge.
const CLOAK_SPRING: f32 = 26.0;
const CLOAK_DAMP: f32 = 6.0;
/// The cloak's own motion, expressed through [`SecondaryMotionSpec`] — see that
/// type's doc comment. Kept as the exact numbers the cloak shipped with; this
/// is just the first `SecondaryPart` built through the now-generic path.
const CLOAK_MOTION: SecondaryMotionSpec = SecondaryMotionSpec {
    base_pitch: 0.30,
    loco_gain: 0.30,
    fwd_gain: 1.4,
    sway_gain: -0.55,
    turn_gain: -0.30,
    spring: CLOAK_SPRING,
    damp: CLOAK_DAMP,
};

/// The villager shawl. Same spring path as the cloak, tuned quieter: it hangs
/// heavier (a shorter, wider panel), reacts less to speed because a story NPC
/// mostly stands, and keeps just enough turn/sway gain that a shoulder shrug
/// during a conversation still ripples through it.
const SHAWL_MOTION: SecondaryMotionSpec = SecondaryMotionSpec {
    base_pitch: 0.16,
    loco_gain: 0.14,
    fwd_gain: 0.6,
    sway_gain: -0.26,
    turn_gain: -0.34,
    spring: CLOAK_SPRING * 0.8,
    damp: CLOAK_DAMP,
};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Builds and drives the procedural rigs. Gated on [`Cfg::play`] exactly like
/// [`crate::scene::ScenePlugin`], so the bench, the hero shot and every editor
/// screenshot keep the plain capsule they were graded against.
pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        // The animation-timing message stream other lanes subscribe to. anim.rs
        // is the sole writer (see the events section + docs/anim-events.md).
        app.add_message::<AnimSwing>()
            .add_message::<AnimDodge>()
            .add_message::<AnimParry>()
            .add_message::<AnimHit>()
            .add_message::<AnimFootstep>()
            .add_message::<AnimLand>()
            // `animate_rigs` reads it for the conversation layer. `CutscenePlugin`
            // owns it, but `init_resource` is idempotent and this makes the rig
            // driver independent of plugin registration ORDER in main.rs.
            .init_resource::<Conversation>()
            .add_systems(Startup, init_rig_assets)
            .add_systems(
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

/// Which body a rig WEARS — proportions, palette, and whether it carries a blade.
///
/// Deliberately separate from [`Actor`], which says what a rig *does*. A villager
/// is a player-shaped body in a different palette with no weapon; giving it an
/// `Actor` variant instead would ripple into every lane that matches on `Actor`
/// (`vfx_bridge`'s blade table, its footstep-dust arms) for a body that never
/// swings, never runs and never bleeds. `animate_rigs` keeps every anim event
/// off a `Villager` rig for the same reason.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RigLook {
    Player,
    Husk,
    /// Story NPCs (`quest::Npc`) — Elder Maren and whoever follows her.
    Villager,
}

/// Marker on the ACTOR entity: its rig has been built, don't build a second one.
#[derive(Component)]
pub struct Rigged;

/// Marker on every joint of a rig, so the joint query is provably disjoint from
/// the actor / rig-root queries.
#[derive(Component)]
pub struct Joint;

/// Marks the weapon mesh entity riding in a rig's `hand_r` joint — the blade
/// that is actually drawn, as opposed to any other lane's own placeholder
/// (e.g. `combat::HuskArm`, which `attach_rigs` hides the moment a rig lands
/// on that actor). Other lanes (the VFX bridge) attach effects — a swing
/// trail — to this entity so the ribbon follows the blade the player really
/// sees, not a proxy with its own, slightly different arc.
#[derive(Component, Clone, Copy)]
pub struct RigWeapon {
    pub actor: Actor,
}

/// Which rig joint an extra part hangs off — mirrors the joints [`build_rig`]
/// creates. See [`extra_parts`]'s doc comment for the attachment-spec format
/// new art (armor, props, cloth, hair) plugs in through, without touching the
/// hand-tuned core skeleton `build_rig` already builds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoneName {
    Hips,
    Torso,
    Head,
    ShoulderL,
    ElbowL,
    ShoulderR,
    ElbowR,
    HandR,
    HipL,
    KneeL,
    HipR,
    KneeR,
}

/// One extra decorative piece described as DATA — size, offset, colour, which
/// bone it hangs off — instead of a hand-edited field in `Parts`/`build_rig`.
/// A bone can carry any number of these, and any of them can opt into
/// [`SecondaryMotionSpec`] to lag the body like the cloak instead of riding
/// rigidly. See [`extra_parts`] for where a list of these gets consumed.
#[derive(Clone)]
struct PartSpec {
    bone: BoneName,
    /// Width/height/depth of the box, in blocks.
    size: Vec3,
    /// Bone-local position. For a rigid part this is the mesh centre. For a
    /// secondary-motion part this is where the hinge sits — `mesh_offset`
    /// below then positions the mesh under/past that hinge (the same
    /// hang-past-the-joint convention limbs use, see `build_rig`'s doc).
    offset: Vec3,
    /// Only read when `secondary` is `Some`.
    mesh_offset: Vec3,
    color: Color,
    roughness: f32,
    metallic: f32,
    secondary: Option<SecondaryMotionSpec>,
}

/// Parameters for the damped-spring "lags the body a beat" motion the cloak
/// pioneered (see `CLOAK_SPRING`/`CLOAK_DAMP`/`CLOAK_MOTION`). `base_pitch` is
/// a static rest droop applied on top of the spring; `loco_gain`/`fwd_gain`
/// drive the pitch target off locomotion weight and forward lunge, and
/// `sway_gain`/`turn_gain` drive the roll target off stride phase and turn
/// rate — the same body signals the cloak already reads, just re-weighted per
/// part so a second cloth panel or a hair mass can have its own amplitude.
#[derive(Clone, Copy)]
struct SecondaryMotionSpec {
    base_pitch: f32,
    loco_gain: f32,
    fwd_gain: f32,
    sway_gain: f32,
    turn_gain: f32,
    spring: f32,
    damp: f32,
}

/// Live spring state for one secondary-motion part — the cloak's hinge, plus
/// whatever `extra_parts` adds. Driven generically in `animate_rigs`.
struct SecondaryPart {
    anchor: Entity,
    base_pos: Vec3,
    spec: SecondaryMotionSpec,
    pitch: f32,
    pitch_vel: f32,
    roll: f32,
    roll_vel: f32,
}

/// The rig root — one child entity per actor, holding the joint handles and all
/// the animation state. Everything below it is pure decoration.
#[derive(Component)]
pub struct Rig {
    actor: Actor,
    /// The body this rig wears — see [`RigLook`]. Drives proportions, palette
    /// and the event mute on story NPCs.
    look: RigLook,
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
    /// Every secondary-motion part hanging off this rig — the player's cloak
    /// (Husk carries none; `docs/character-bible.md` §1/§2), plus whatever
    /// `extra_parts` adds. Built once in `build_rig`, driven in `animate_rigs`.
    secondary: Vec<SecondaryPart>,

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
    /// Progress through the parry action, 0 → 1, held across the latch tail so
    /// the recovery pose does not snap back to "window open" as the beat fades.
    parry_t: f32,
    /// Stagger release-blend window (see [`STAGGER_FADE`]).
    stagger_fade: f32,
    /// Death progress, 0 → 1. Reset when the actor comes back alive.
    death: f32,
    /// Eased 0 → 1: is this body the one currently talking? See `conversation()`.
    convo_talk: f32,
    // --- animation-event edge state (see the events section above) -----------
    /// The swing phase this rig was in last frame, so a phase change fires its
    /// `AnimSwing` exactly once on the frame it crosses.
    prev_swing: Option<SwingPhase>,
    /// Was the rig inside the dodge i-frame window last frame?
    in_iframe: bool,
    /// Was the rig inside the parry receive window last frame?
    in_parry: bool,
    /// Stride phase at the previous frame, for footstep crossing detection.
    prev_phase: f32,
}

/// Native-only screenshot hook — see `pose_override()` and docs/anim-events.md
/// capture section. Set with VOXELFORGE_ANIM_POSE=attack|dodge|parry|clash.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OverridePose {
    Attack,
    Dodge,
    Parry,
    /// Player and Husk both held at their own contact frame, so the two blades
    /// read as meeting instead of two actors posed independently. The other
    /// three poses only ever touch the player rig; this one also drives the
    /// nearest `Actor::Husk` rig via `override_husk_beat()`.
    Clash,
}

/// Read VOXELFORGE_ANIM_POSE once (cached). Returns None on wasm and in every
/// normal play session, so gameplay is untouched — this only ever redirects the
/// player rig's pose for an isolated capture run.
fn pose_override() -> Option<OverridePose> {
    static LOCK: std::sync::OnceLock<Option<OverridePose>> = std::sync::OnceLock::new();
    *LOCK.get_or_init(|| match std::env::var("VOXELFORGE_ANIM_POSE").ok().as_deref() {
        Some("attack") | Some("swing") | Some("strike") => Some(OverridePose::Attack),
        Some("dodge") | Some("roll") => Some(OverridePose::Dodge),
        Some("parry") | Some("guard") => Some(OverridePose::Parry),
        Some("clash") => Some(OverridePose::Clash),
        _ => None,
    })
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
    npc: Parts,
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
    /// The player's half-cloak slab (see [`Rig::secondary`]). Allocated for
    /// both actors for `Parts`' sake, but only ever spawned on the player.
    cloak: Handle<Mesh>,
    cloth: Handle<StandardMaterial>,
    trim: Handle<StandardMaterial>,
    skin: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    /// Extra decorative pieces baked from [`extra_parts`] — armor, props,
    /// hair. Empty until that function returns something; `build_rig` spawns
    /// whatever is here without needing to know it's there.
    extra: Vec<ExtraPart>,
}

/// A [`PartSpec`] with its mesh/material already allocated — built once in
/// [`init_rig_assets`] via [`build_extra_parts`], the same way the hand-tuned
/// core pieces are, so `build_rig` only ever spawns handles, never allocates.
struct ExtraPart {
    bone: BoneName,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    offset: Vec3,
    mesh_offset: Vec3,
    secondary: Option<SecondaryMotionSpec>,
}

/// Bake a list of [`PartSpec`]s into [`ExtraPart`]s (allocates their mesh +
/// a dedicated material each — the list is short, so no sharing/caching).
fn build_extra_parts(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    specs: Vec<PartSpec>,
) -> Vec<ExtraPart> {
    specs
        .into_iter()
        .map(|s| ExtraPart {
            bone: s.bone,
            mesh: meshes.add(Cuboid::new(s.size.x, s.size.y, s.size.z)),
            material: materials.add(StandardMaterial {
                base_color: s.color,
                perceptual_roughness: s.roughness,
                metallic: s.metallic,
                ..default()
            }),
            offset: s.offset,
            mesh_offset: s.mesh_offset,
            secondary: s.secondary,
        })
        .collect()
}

/// Extra decorative pieces for one actor — armor plates, pouches, a second
/// cloak panel, hair — layered on top of the hand-tuned 11-piece core
/// skeleton `build_rig` builds. Empty today: this is the hook the art pass's
/// pieces land in, not a placeholder that still needs wiring — `build_rig`
/// already consumes whatever this returns, rigid or secondary-motion, one or
/// a hundred entries, on any bone. To add a piece once sizes/offsets/colours
/// are in hand:
///
/// ```ignore
/// vec![PartSpec {
///     bone: BoneName::Torso,
///     size: Vec3::new(0.50, 0.20, 0.30),   // width, height, depth (blocks)
///     offset: Vec3::new(0.0, 0.10, 0.16),  // bone-local position
///     mesh_offset: Vec3::ZERO,             // only used if `secondary` is Some
///     color: Color::srgb(0.55, 0.50, 0.45),
///     roughness: 0.6,
///     metallic: 0.2,
///     secondary: None,                     // Some(spec) = cloth/hair that lags
/// }]
/// ```
fn extra_parts(look: RigLook) -> Vec<PartSpec> {
    match look {
        RigLook::Player => vec![
            // --- Hair: tousled voxel-cluster mass, NOT a flat slab (character-bible
            // §1 fix #2 — see docs/assets/characters/auren-hero-concept.png and the
            // before/after at auren-hero-concept-before-after-2026-08-06-hairfix.png).
            // 4 overlapping boxes on BoneName::Head so it reads as clustered volume
            // instead of one brick; deliberately asymmetric L/R (bigger lump stage
            // right) per the CEO's "asymmetry that makes the silhouette read" note.
            // Offsets are in the same head-local frame as the existing head cube
            // (head_off=0.18) and face plate (face_y=0.20, face_z=-0.16) two lines
            // up in `Dims::of` — local -Z is forward/face side, +Z is the back of
            // the skull.
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.32, 0.16, 0.30),   // crown mass
                offset: Vec3::new(0.0, 0.36, 0.03),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),  // #2A1B12, character-bible §1
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.28, 0.14, 0.22),   // nape/back mass, lower + further back
                offset: Vec3::new(0.0, 0.24, 0.15),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.09, 0.10, 0.12),   // stage-left clump, smaller
                offset: Vec3::new(-0.16, 0.28, 0.02),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.13, 0.14, 0.16),   // stage-right clump, bigger (asymmetry)
                offset: Vec3::new(0.15, 0.30, 0.00),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            // --- Belt ember-pouch (character-bible §1 palette: housing #C88A4A,
            // glow #F4B860 -> #FFD98A). Hangs on ONE hip only (not centred on the
            // buckle) — that off-centre placement is part of the asymmetry read too.
            // NOT wired to true emissive/bloom yet — see docs/auren-extra-parts-spec.md
            // open note #1 (needs an `Option<LinearRgba>` emissive field or a
            // vfx.rs glowing-coal effect anchored to the pouch bone).
            PartSpec {
                bone: BoneName::Hips,
                size: Vec3::new(0.10, 0.12, 0.08),   // housing
                offset: Vec3::new(0.14, -0.10, -0.10),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.78, 0.54, 0.29),  // #C88A4A
                roughness: 0.55,
                metallic: 0.1,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Hips,
                size: Vec3::new(0.05, 0.06, 0.03),   // inset "coal" face
                offset: Vec3::new(0.14, -0.10, -0.135),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(1.0, 0.85, 0.54),  // #FFD98A
                roughness: 0.3,
                metallic: 0.0,
                secondary: None,
            },
            // --- Cloak clasp (art-order-2026-08-09 A6 fix #2 — docs/note-to-yamamoto-
            // teal-accent-2026-08-10.md). Everything on this rig so far is one hue
            // family (warm gold/brown, character-bible §1's palette) — measured on a
            // real gate3 boot frame, hero hue 30.2 deg vs background 33.4 deg, only
            // 3.3 deg apart, so the silhouette separates by VALUE only, not HUE. This
            // is the look-bible.md:132 "Accent 2 (cool)" `#4FC9D6` teal, the one point
            // the bible allows to cut against the warm 85%. Placed at the SAME torso-
            // local point `cloak_anchor`'s `base_pos` computes (sh_x*0.7, sh_y*0.95,
            // 0.08)) — where the cloak fastens over the shoulder — because that is the
            // one spot on Auren's body actually facing the third-person camera; a
            // front chest placement would sit on the player's back-to-camera side and
            // never render. Housing reuses the rig's own `trim` leather-brown so this
            // does not add a 7th distinct material; only the gem inset is new.
            // z re-derived 2026-08-10 (docs/VERDICT-...A6...-2026-08-10.md: "clasp
            // fully occluded"). The old z=0.08/0.095 put both parts INSIDE the cloak
            // slab: cloak (line ~764) is `Cuboid(0.30, 0.62, 0.05)` skinned with no
            // z-offset onto `cloak_anchor`, whose `base_pos.z` is 0.08 (line ~915) —
            // same torso-local space these offsets are in. So the cloak's own front
            // (camera-facing) face sits at 0.08 + 0.05/2 = 0.105, and the old gem
            // front face (0.095 + 0.02/2 = 0.105) only reached flush with that
            // surface — zero clearance, fully hidden by the slab in front of it.
            //
            // z re-derived AGAIN 2026-08-11 (docs/VERDICT-a6-teal-accent-2026-08-11.md):
            // the first re-derive (housing 0.1225 / gem 0.1375, commit 4d24d30) shot
            // 0/3.48M matching px in gate3 boot/walk/combat, same as before — it only
            // cleared the CLOAK's own 0.05-thick slab and never checked the clasp's
            // own bone. `bone: BoneName::Torso` skins both parts directly onto the
            // TORSO joint (no `secondary`, so no cloak_anchor involved) — same joint
            // the torso body mesh itself hangs off (line ~764: `Cuboid(0.46, 0.58,
            // 0.28)`, skinned with `Transform::from_xyz(0.0, neck_y*0.5, 0.0)` — no
            // z-offset), so the torso's own BACK face sits at 0.28/2 = 0.14 in this
            // same frame — 0.035 further out than the cloak's 0.105, and past the
            // housing's old 0.1225-0.14 span (0.1225 + housing_half_z 0.0175 = 0.14
            // exactly: flush with the torso's own body, not clear of it) and all but
            // 0.0075 of the gem's old 0.1275-0.1475 span. The clasp's XY (0.1785,
            // 0.475) sits inside the torso box's own footprint (half-x 0.23, y-range
            // 0-0.58) at every z, so this is the real occluder, not the cloak — and
            // it was silently below the fix radar because it isn't `cloak_anchor`.
            // Confirmed empirically, not just by the numbers: binary-diffed
            // `target/release/voxelforge.exe` for the literal 0.1225/0.1375 f32
            // constants (each present exactly once, so the 4d24d30 build DID ship),
            // then projected the clasp's known torso-local point through the same
            // camera math `grade_character.py`'s `analytic_bbox` uses and cropped
            // the predicted screen pixel in the boot frame — flat, featureless cloak
            // colour, no seam, no bump, at 5x zoom.
            //
            // Fix: seat the housing's REAR face past the real occluder — max(cloak
            // front 0.105, torso back 0.14) = 0.14 — with an explicit 0.01 clearance
            // margin (not flush: flush-against-a-surface is exactly how the ORIGINAL
            // 0.095-vs-0.105 bug happened, so this deliberately doesn't repeat it).
            // housing_z = 0.14 + 0.01 + housing_half_z 0.0175 = 0.1675. Gem keeps its
            // original 0.015 proud-of-housing gap on top of that: 0.1675 + 0.015 =
            // 0.1825. NOT build-verified past the pixel-projection crop above — see
            // docs/VERDICT-a6-teal-accent-2026-08-11.md; build/reshoot is a follow-up.
            PartSpec {
                bone: BoneName::Torso,
                size: Vec3::new(0.09, 0.08, 0.035),  // bezel housing
                offset: Vec3::new(0.1785, 0.475, 0.1675),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.34, 0.20, 0.13),  // matches `trim`
                roughness: 0.5,
                metallic: 0.15,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Torso,
                size: Vec3::new(0.045, 0.05, 0.02),   // teal gem face
                offset: Vec3::new(0.1785, 0.475, 0.1825),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.310, 0.788, 0.839),  // #4FC9D6, look-bible §Accent 2
                roughness: 0.15,
                metallic: 0.0,
                secondary: None,
            },
        ],
        // The Husk is armor and stone; the villager's one moving shape is the
        // shawl `build_rig` hangs off the cloak anchor, not a listed extra.
        RigLook::Husk | RigLook::Villager => Vec::new(),
    }
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
    fn of(look: RigLook) -> Self {
        match look {
            // 1.80 crown-to-floor, matching PLAYER_HEIGHT.
            RigLook::Player => Dims {
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
            RigLook::Husk => Dims {
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
            // ~1.65 crown-to-floor — an elder stands a head shorter and slighter
            // than Auren, so the two-shot reads as two different people rather
            // than one body in two palettes.
            RigLook::Villager => Dims {
                hip_y: 0.78,
                hip_x: 0.115,
                thigh: 0.40,
                shin: 0.37,
                sh_y: 0.46,
                sh_x: 0.225,
                upper: 0.27,
                fore: 0.26,
                neck_y: 0.54,
                head_off: 0.17,
                face_y: 0.19,
                face_z: -0.15,
                foot_h: 0.045,
                bob: 0.045,
                sway: 0.026,
            },
        }
    }

    fn leg(&self) -> f32 {
        self.thigh + self.shin
    }
}

/// Feet-relative offset from the ACTOR entity's own origin down to the rig root.
///
/// Three different conventions meet here and every one of them is somebody
/// else's: the avatar's origin is its EYE (main.rs's `move_body` contract), the
/// Husk's is already its feet (combat.rs), and a story NPC's is the centre of the
/// 1.4-tall body box `quest::spawn_npcs` places it with — so its soles sit half
/// that box below the origin. Spelled once so `build_rig` and `animate_rigs`
/// cannot drift apart on it.
#[inline]
fn root_offset(look: RigLook) -> f32 {
    match look {
        RigLook::Player => -EYE_HEIGHT,
        RigLook::Husk => 0.0,
        RigLook::Villager => -0.7,
    }
}

/// Height of this body's head above its ACTOR entity's origin — what an eye line
/// between two bodies is measured on. Derived from the rig `build_rig` actually
/// builds (root offset + hips + neck + the head cube's own centre), so it cannot
/// drift out of step with the proportions the way a hand-typed constant would.
/// `cutscene.rs` aims its two-shot with this.
pub fn head_lift(look: RigLook) -> f32 {
    let d = Dims::of(look);
    root_offset(look) + d.hip_y + d.neck_y + d.head_off
}

/// The villager head lift, for lanes that hold an NPC entity but not a [`RigLook`].
pub fn npc_head_lift() -> f32 {
    head_lift(RigLook::Villager)
}

// ---------------------------------------------------------------------------
// Asset setup
// ---------------------------------------------------------------------------

fn init_rig_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pd = Dims::of(RigLook::Player);
    let hd = Dims::of(RigLook::Husk);
    let vd = Dims::of(RigLook::Villager);

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
        // A flat slab, wide as the shoulders and long enough to trail past the
        // hip line (character-bible §1's read-point) once it hangs from the
        // cloak hinge.
        cloak: meshes.add(Cuboid::new(0.30, 0.62, 0.05)),
        cloth: materials.add(StandardMaterial {
            base_color: Color::srgb(0.420, 0.290, 0.180), // #6B4A2E, character-bible §1 tunic
            perceptual_roughness: 0.85,
            ..default()
        }),
        trim: materials.add(StandardMaterial {
            base_color: Color::srgb(0.34, 0.20, 0.13),
            perceptual_roughness: 0.65,
            ..default()
        }),
        skin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.62, 0.47),
            perceptual_roughness: 0.50,
            ..default()
        }),
        steel: materials.add(StandardMaterial {
            base_color: Color::srgb(0.725, 0.663, 0.549), // #B9A98C, character-bible §1 sword blade
            perceptual_roughness: 0.18,
            metallic: 0.0,
            reflectance: 0.7,
            ..default()
        }),
        extra: build_extra_parts(&mut meshes, &mut materials, extra_parts(RigLook::Player)),
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
        // Unused — the Husk wears no cloak (armor and stone, not cloth; see
        // character-bible §2). Allocated only so `Parts` stays one struct.
        cloak: meshes.add(Cuboid::new(0.30, 0.62, 0.05)),
        cloth: materials.add(StandardMaterial {
            base_color: Color::srgb(0.725, 0.663, 0.549), // #B9A98C, character-bible §2 armor plate
            perceptual_roughness: 0.70,
            metallic: 0.0, // stone, not metal-flake
            ..default()
        }),
        trim: materials.add(StandardMaterial {
            base_color: Color::srgb(0.541, 0.478, 0.361), // #8A7A5C, character-bible §2 crumbling edges
            perceptual_roughness: 0.90,
            ..default()
        }),
        skin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.27, 0.23),
            perceptual_roughness: 0.85,
            ..default()
        }),
        steel: materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.51, 0.46),
            perceptual_roughness: 0.20,
            metallic: 0.0,
            reflectance: 0.7,
            ..default()
        }),
        extra: build_extra_parts(&mut meshes, &mut materials, extra_parts(RigLook::Husk)),
    };

    // Story NPCs. The palette is deliberately the slate-blue quest.rs already
    // spawned Maren's placeholder body in (`srgb(0.35,0.45,0.55)`) — swapping a
    // box for a rig should not also recolour the character in anyone's shot.
    // `weapon` is allocated but never spawned (see `build_rig`): villagers carry
    // nothing, and a `RigWeapon` on one would put a swing trail on an elder.
    let npc = Parts {
        pelvis: meshes.add(Cuboid::new(0.36, 0.18, 0.24)),
        torso: meshes.add(Cuboid::new(0.41, 0.53, 0.25)),
        head: meshes.add(Cuboid::new(0.33, 0.32, 0.31)),
        face: meshes.add(Cuboid::new(0.22, 0.09, 0.06)),
        upper_arm: meshes.add(Cuboid::new(0.13, vd.upper, 0.14)),
        forearm: meshes.add(Cuboid::new(0.11, vd.fore, 0.12)),
        thigh: meshes.add(Cuboid::new(0.17, vd.thigh, 0.18)),
        shin: meshes.add(Cuboid::new(0.15, vd.shin, 0.16)),
        foot: meshes.add(Cuboid::new(0.17, 0.08, 0.26)),
        weapon: meshes.add(Cuboid::new(0.05, 0.40, 0.05)),
        // A shawl rather than a half-cloak: shorter, wider, hangs off both
        // shoulders. Same secondary-motion path the player's cloak rides.
        cloak: meshes.add(Cuboid::new(0.40, 0.44, 0.05)),
        cloth: materials.add(StandardMaterial {
            base_color: Color::srgb(0.350, 0.450, 0.550), // quest.rs's Maren slate
            perceptual_roughness: 0.90,
            ..default()
        }),
        trim: materials.add(StandardMaterial {
            base_color: Color::srgb(0.255, 0.330, 0.415),
            perceptual_roughness: 0.85,
            ..default()
        }),
        skin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.780, 0.640, 0.545), // older, cooler than Auren's
            perceptual_roughness: 0.60,
            ..default()
        }),
        steel: materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.46, 0.50),
            perceptual_roughness: 0.55,
            metallic: 0.0,
            reflectance: 0.4,
            ..default()
        }),
        extra: build_extra_parts(&mut meshes, &mut materials, extra_parts(RigLook::Villager)),
    };

    commands.insert_resource(RigAssets { player, husk, npc });
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
    look: RigLook,
    parent: Entity,
) -> (Entity, Rig) {
    let d = Dims::of(look);
    let p = match look {
        RigLook::Player => &assets.player,
        RigLook::Husk => &assets.husk,
        RigLook::Villager => &assets.npc,
    };

    // The rig root sits at the actor's FEET. The avatar's own origin is its eye, the
    // Husk's is already its feet, and a story NPC's is the middle of the body box
    // quest.rs spawned it with — hence the three offsets.
    let root_dy = root_offset(look);
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

    // Anchor joints for the cloak + any `extra_parts` secondary pieces are
    // built here, while `joint` is still the only closure alive that captures
    // `commands`. `skin` (below) captures it too — the two can't coexist — so
    // every entity that needs `joint` has to exist before `skin` is born. The
    // MESH side of these parts is attached further down, once `skin` exists.
    let bone_of = |name: BoneName| -> Entity {
        match name {
            BoneName::Hips => hips,
            BoneName::Torso => torso,
            BoneName::Head => head,
            BoneName::ShoulderL => sh_l,
            BoneName::ElbowL => el_l,
            BoneName::ShoulderR => sh_r,
            BoneName::ElbowR => el_r,
            BoneName::HandR => hand_r,
            BoneName::HipL => hip_l,
            BoneName::KneeL => knee_l,
            BoneName::HipR => hip_r,
            BoneName::KneeR => knee_r,
        }
    };
    // Auren's half-cloak hangs off ONE shoulder (asymmetric, character-bible §1);
    // a villager's shawl sits centred across both. The Husk wears neither.
    let cloak_anchor = match look {
        RigLook::Player => {
            let base_pos = Vec3::new(d.sh_x * 0.7, d.sh_y * 0.95, 0.08);
            Some((joint(base_pos.x, base_pos.y, base_pos.z, torso), base_pos))
        }
        RigLook::Villager => {
            let base_pos = Vec3::new(0.0, d.sh_y * 0.98, 0.07);
            Some((joint(base_pos.x, base_pos.y, base_pos.z, torso), base_pos))
        }
        RigLook::Husk => None,
    };
    // Only the secondary-motion extra parts need a joint; rigid ones ride
    // directly on their bone and are meshed straight from `p.extra` below.
    let extra_anchors: Vec<Option<Entity>> = p
        .extra
        .iter()
        .map(|part| {
            part.secondary
                .map(|_| joint(part.offset.x, part.offset.y, part.offset.z, bone_of(part.bone)))
        })
        .collect();

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
    // Husk-only per character-bible §2 (LOCKED): the Husk face is a blank, featureless
    // slab — no visor slit. Everyone who TALKS gets the directional face-accent
    // block: in a two-shot it is the only thing that says which way a head is
    // turned, and the conversation layer spends its whole budget turning heads.
    if look != RigLook::Husk {
        skin(
            &p.face,
            &p.trim,
            Transform::from_xyz(0.0, d.face_y, d.face_z),
            head,
        );
    }
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
        // Player boots are leather (trim), not the sword's chrome steel — a traveller's
        // boots, not polished metal. The Husk's armored boots stay on steel.
        let boot_mat = if look == RigLook::Husk { &p.steel } else { &p.trim };
        skin(
            &p.foot,
            boot_mat,
            Transform::from_xyz(0.0, -d.shin + d.foot_h, -0.06),
            knee,
        );
    }
    // The half-cloak (character-bible §1): the one shape on Auren's silhouette
    // that's SUPPOSED to move on its own — "the only shape on the body that
    // moves... what the eye locks onto in motion". Hung off the torso, over
    // one shoulder (asymmetric per the bible), trailing down past the hip.
    // `animate_rigs` drives it with a damped spring so it arrives a beat late
    // instead of moving rigidly with the body — that lag IS the secondary
    // motion. The Husk gets none (armor and stone, not cloth — §2). The
    // anchor joint itself was already built above (before `skin` existed);
    // this is just the mesh + spring-state bookkeeping.
    let mut secondary: Vec<SecondaryPart> = Vec::new();
    if let Some((anchor, base_pos)) = cloak_anchor {
        // Hang the slab half its own length below the hinge, so the hinge swings
        // it instead of spinning it about its middle.
        let (drop, motion) = match look {
            RigLook::Villager => (-0.22, SHAWL_MOTION),
            _ => (-0.31, CLOAK_MOTION),
        };
        skin(&p.cloak, &p.trim, Transform::from_xyz(0.0, drop, 0.0), anchor);
        secondary.push(SecondaryPart {
            anchor,
            base_pos,
            spec: motion,
            pitch: 0.0,
            pitch_vel: 0.0,
            roll: 0.0,
            roll_vel: 0.0,
        });
    }

    // Extra parts from the art pass (see `extra_parts`'s doc comment for the
    // spec format) — empty until pieces land, so this is a no-op today and
    // changes nothing about the silhouette above. `extra_anchors` (built
    // above, alongside the cloak's) already has the joint for every
    // secondary-motion entry, in the same order as `p.extra`.
    for (part, anchor) in p.extra.iter().zip(extra_anchors.iter().copied()) {
        match (part.secondary, anchor) {
            (Some(spec), Some(anchor)) => {
                skin(&part.mesh, &part.material, Transform::from_translation(part.mesh_offset), anchor);
                secondary.push(SecondaryPart {
                    anchor,
                    base_pos: part.offset,
                    spec,
                    pitch: 0.0,
                    pitch_vel: 0.0,
                    roll: 0.0,
                    roll_vel: 0.0,
                });
            }
            _ => {
                skin(&part.mesh, &part.material, Transform::from_translation(part.offset), bone_of(part.bone));
            }
        }
    }

    // Weapon in the right hand: blade forward-and-down at rest, so the whole arc is
    // a rotation of the grip rather than a teleport.
    // A villager carries nothing. It matters beyond the silhouette: `RigWeapon`
    // is what `vfx_bridge` hangs a swing trail off, so an armed elder would trail
    // a ribbon the first time any lane fired an `AnimSwing`.
    if look != RigLook::Villager {
        let blade_len = match look {
            RigLook::Husk => 1.05,
            _ => 0.86,
        };
        let weapon_entity = commands
            .spawn((
                Mesh3d(p.weapon.clone()),
                MeshMaterial3d(p.steel.clone()),
                Transform::from_xyz(0.0, -blade_len * 0.5 + 0.06, -0.04),
                ChildOf(hand_r),
                RigWeapon { actor },
            ))
            .id();
        // Proof marker for docs/anim-events.md's checklist — one line per rig built
        // (edge-triggered by `attach_rigs`'/`spawn_husk_corpse`'s `Without<Rigged>`/
        // `Without<Dying>` filters, never per-frame), so this is cheap to leave in.
        println!("ANIM_RIG_WEAPON spawn actor={actor:?} entity={weapon_entity:?}");
    }

    let rig = Rig {
        actor,
        look,
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
        secondary,
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
        parry_t: 0.0,
        stagger_fade: 0.0,
        death: 0.0,
        convo_talk: 0.0,
        prev_swing: None,
        in_iframe: false,
        in_parry: false,
        prev_phase: 0.0,
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
    // `Without<EnemyBody>`: enemies spawned through `combat::spawn_husk_of_kind`
    // carry one of `enemies.rs`'s hand-tooled silhouettes (Monanisa's lane) on
    // their own children — dressing them with the generic Husk rig would hide
    // the authored body and render the old guard over it. The rig still owns
    // any enemy without an `EnemyBody` (legacy spawns, demos).
    enemy_q: Query<
        (Entity, &Transform),
        (
            With<Enemy>,
            Without<Rigged>,
            Without<crate::enemies::EnemyBody>,
        ),
    >,
    npc_q: Query<(Entity, &Transform), (With<crate::quest::Npc>, Without<Enemy>, Without<Rigged>)>,
    child_q: Query<&Children>,
    mut vis_q: Query<&mut Visibility, With<Mesh3d>>,
) {
    let Some(assets) = assets else {
        return;
    };

    let mut dress = |entity: Entity,
                     tf: &Transform,
                     actor: Actor,
                     look: RigLook,
                     commands: &mut Commands| {
        // Blank the placeholder meshes BEFORE the rig's own children exist — the
        // spawn above is deferred, so `Children` here only holds the old boxes.
        if let Ok(children) = child_q.get(entity) {
            for c in children.iter() {
                if let Ok(mut v) = vis_q.get_mut(c) {
                    *v = Visibility::Hidden;
                }
            }
        }
        let (root, mut rig) = build_rig(commands, &assets, actor, look, entity);
        rig.prev_pos = tf.translation;
        rig.face_yaw = yaw_of(tf);
        commands.entity(root).insert(rig);
        commands.entity(entity).insert(Rigged);
    };

    for (e, tf) in player_q.iter() {
        dress(e, tf, Actor::Player, RigLook::Player, &mut commands);
    }
    for (e, tf) in enemy_q.iter() {
        dress(e, tf, Actor::Husk, RigLook::Husk, &mut commands);
    }
    // Story NPCs. Their placeholder body is a mesh on the ACTOR entity itself,
    // not a child — so the `Visibility::Hidden` sweep above cannot reach it, and
    // hiding the entity would hide the rig hanging off it too. Drop the mesh
    // handle instead: the entity keeps its transform, the rig renders, and
    // quest.rs's spawn is untouched (it runs once, in PostStartup).
    for (e, tf) in npc_q.iter() {
        dress(e, tf, Actor::Player, RigLook::Villager, &mut commands);
        commands
            .entity(e)
            .remove::<Mesh3d>()
            .remove::<MeshMaterial3d<StandardMaterial>>();
        println!("ANIM_RIG_NPC dressed entity={e:?} look=Villager");
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
        let (rig_root, mut rig) = build_rig(&mut commands, &assets, Actor::Husk, RigLook::Husk, root);
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

// `Debug` is what `assert_eq!(a.action, Action::Swing)` in this file's own tests
// needs to print a mismatch. Without it `cargo test` cannot build the bin's test
// target AT ALL (E0277 ×6), which takes every other module's tests down with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    npc_q: Query<
        &Transform,
        (
            With<crate::quest::Npc>,
            Without<Rig>,
            Without<Joint>,
            Without<FlyCam>,
            Without<Enemy>,
            Without<Corpse>,
        ),
    >,
    convo: Res<Conversation>,
    mut rig_q: Query<
        (&ChildOf, &mut Rig, &mut Transform),
        (Without<Joint>, Without<FlyCam>, Without<Enemy>, Without<Corpse>),
    >,
    mut joint_q: Query<&mut Transform, JointFilter>,
    mut swings: MessageWriter<AnimSwing>,
    mut dodges: MessageWriter<AnimDodge>,
    mut parries: MessageWriter<AnimParry>,
    mut hits: MessageWriter<AnimHit>,
    mut steps: MessageWriter<AnimFootstep>,
    mut lands: MessageWriter<AnimLand>,
) {
    let dt = time.delta_secs().clamp(1.0 / 240.0, 1.0 / 15.0);
    let elapsed = time.elapsed_secs();

    // ---- conversation staging (cutscene.rs) ---------------------------------
    // Both bodies in a dialogue need the OTHER body's position, so the pair is
    // resolved ONCE here rather than re-queried per rig. `None` outside a
    // conversation — which is every frame of ordinary play, so the whole layer
    // below costs one resource read and one branch.
    let staged: Option<(Vec3, Vec3)> = if convo.weight() > 0.0 {
        match (
            player_q.iter().next(),
            convo.npc().and_then(|e| npc_q.get(e).ok()),
        ) {
            (Some((ptf, ..)), Some(ntf)) => Some((ptf.translation, ntf.translation)),
            _ => None,
        }
    } else {
        None
    };

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
            } else if let Ok(tf) = npc_q.get(owner) {
                // A story NPC stands where quest.rs put it: no controller, no combat
                // state, no health bar. It holds its heading until the conversation
                // block below hands it one — that is the ONLY thing that turns it.
                (tf.translation, yaw_of(tf), rig.face_yaw, true, 0.0, Beat::none(), None)
            } else {
                continue;
            };

        // Capture hook: hold one move's canonical frame so a PNG of that pose can
        // be grabbed (see docs/anim-events.md capture). Every pose but `Clash`
        // only ever touches the player rig; `Clash` also parks the Husk at its
        // own contact frame so the shot reads as blades meeting, not two actors
        // posed independently.
        let mut beat = beat;
        // Never on a villager: it wears `Actor::Player` for the pose tables but
        // carries no blade, so an `attack` capture would have it swing at air.
        if let Some(op) = pose_override().filter(|_| rig.look != RigLook::Villager) {
            match rig.actor {
                Actor::Player => beat = override_beat(op),
                Actor::Husk if op == OverridePose::Clash => beat = override_husk_beat(),
                Actor::Husk => {}
            }
        }

        // ---- conversation: where this body should be pointing -----------------
        // Both actors turn onto their partner, opened a few degrees off the line
        // of conversation (in OPPOSITE directions) so the pair stands three-
        // quarters to the lens rather than in profile. `conversation()` closes
        // that same angle back with the chest and head, so they still look at
        // each other — the body is angled, the gaze is not.
        let mut want_yaw = want_yaw;
        let mut convo_beat: Option<(f32, f32)> = None; // (residual yaw, eye-line pitch)
        if let Some((ppos, npos)) = staged {
            let is_player = rig.look == RigLook::Player;
            if is_player || convo.npc() == Some(owner) {
                let (mine, theirs) = if is_player { (ppos, npos) } else { (npos, ppos) };
                let (my_lift, their_lift) = if is_player {
                    (head_lift(RigLook::Player), head_lift(RigLook::Villager))
                } else {
                    (head_lift(RigLook::Villager), head_lift(RigLook::Player))
                };
                let to = theirs - mine;
                let flat = Vec3::new(to.x, 0.0, to.z);
                let reach = flat.length();
                if reach > 0.05 {
                    let open = convo.stage_open(is_player);
                    want_yaw = (-flat.x).atan2(-flat.z) + open;
                    let eye_dy = (theirs.y + their_lift) - (mine.y + my_lift);
                    convo_beat = Some((-open, eye_dy.atan2(reach)));
                }
            }
        }
        // Who holds the floor, eased — the talker alternates, and arms that
        // snapped between gesturing and hanging would pop on every swap.
        let want_talk = match (convo_beat.is_some(), convo.speaker()) {
            (true, Speaker::Player) => (rig.look == RigLook::Player) as u8 as f32,
            (true, Speaker::Npc) => (rig.look == RigLook::Villager) as u8 as f32,
            _ => 0.0,
        };
        rig.convo_talk += (want_talk - rig.convo_talk) * (4.5 * dt).min(1.0);

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
                lands.write(AnimLand { actor: rig.actor, amp: rig.land_amp });
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
                // Flinch event: the rig took a knock this frame (distinct from a
                // poise-break StaggerEvent — every stagger is a hit, not vice versa).
                hits.write(AnimHit { actor: rig.actor });
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
            rig.parry_t = beat.t;
            GUARD_LATCH
        } else {
            (rig.parry - dt).max(0.0)
        };
        // Stagger's own release-blend window — same idea as guard/parry above,
        // just a separate timer/duration since a poise-break recovery reads
        // better with a slightly longer settle than the guard debounce needs.
        rig.stagger_fade = if beat.action == Action::Stagger {
            STAGGER_FADE
        } else {
            (rig.stagger_fade - dt).max(0.0)
        };

        // ---- death -----------------------------------------------------------
        if beat.action == Action::Dead || rig.death > 0.0 {
            rig.death = (rig.death + dt / 0.85).min(1.0);
        }

        // =====================================================================
        // Animation-timing events — edge-detected, one message per crossing.
        // (See the events section header + docs/anim-events.md for the contract.)
        // =====================================================================
        // Swing: Windup -> Contact -> Recover, each fired once on entry.
        let cur_swing = swing_phase_of(&beat);
        if cur_swing != rig.prev_swing {
            if let Some(phase) = cur_swing {
                swings.write(AnimSwing { actor: rig.actor, phase, combo: beat.combo });
            }
            rig.prev_swing = cur_swing;
        }

        // Dodge i-frame window is the first IFRAMES/(IFRAMES+RECOVERY) of beat.t.
        let iframe_end = combat::DODGE_IFRAMES / (combat::DODGE_IFRAMES + combat::DODGE_RECOVERY);
        let now_iframe = beat.action == Action::Dodge && beat.t < iframe_end;
        if now_iframe && !rig.in_iframe {
            dodges.write(AnimDodge { actor: rig.actor, phase: DodgePhase::IframeStart });
        } else if !now_iframe && rig.in_iframe {
            dodges.write(AnimDodge { actor: rig.actor, phase: DodgePhase::IframeEnd });
        }
        rig.in_iframe = now_iframe;

        // Parry receive window is the first PARRY_WINDOW_FRAC of beat.t.
        let now_parry = beat.action == Action::Parry && beat.t < PARRY_WINDOW_FRAC;
        if now_parry && !rig.in_parry {
            parries.write(AnimParry { actor: rig.actor, phase: ParryPhase::Open });
        } else if !now_parry && rig.in_parry {
            parries.write(AnimParry { actor: rig.actor, phase: ParryPhase::Close });
        }
        rig.in_parry = now_parry;

        // Footfalls: twice per stride cycle (one per foot), only while actually
        // locomoting and free of a committed action. Phase advances by distance
        // travelled, so a planted body crosses no boundary and stays silent.
        // A villager fires NO anim events at all — see [`RigLook`]. It wears
        // `Actor::Player` for the pose tables, and an `AnimFootstep { Player }`
        // from an elder at a gate would put the avatar's dust under her feet.
        if speed > IDLE_SPEED
            && matches!(beat.action, Action::None | Action::Guard)
            && pose_override().is_none()
            && rig.look != RigLook::Villager
        {
            if phase_crossed(rig.prev_phase, rig.phase, std::f32::consts::PI) {
                steps.write(AnimFootstep { actor: rig.actor, foot: Foot::Left });
            }
            if phase_crossed(rig.prev_phase, rig.phase, 0.0) {
                steps.write(AnimFootstep { actor: rig.actor, foot: Foot::Right });
            }
        }
        rig.prev_phase = rig.phase;

        // =====================================================================
        // Pose
        // =====================================================================
        let d = Dims::of(rig.look);
        let mut pose = Pose::default();

        locomotion(&mut pose, &rig, &d, loco, run, elapsed);
        turn_in_place(&mut pose, &rig, turn);
        airborne(&mut pose, air_w);

        // ⑦ Conversation. Sits above locomotion (a body that walks off mid-line
        // fades the layer out with `loco`) and below the action match, so drawing
        // a blade during a dialogue still overrides it completely.
        if let Some((face, tilt)) = convo_beat {
            let cw = convo.weight() * (1.0 - loco) * (1.0 - air_w);
            conversation(&mut pose, &rig, cw, face, tilt, elapsed);
        }

        // Attacks/guards override the upper body and bias the stance.
        let action = if rig.death > 0.0 {
            Action::Dead
        } else if beat.action == Action::Swing || beat.action == Action::Dodge {
            beat.action
        } else if rig.stagger_fade > 0.0 {
            // Gated on the fade timer, not `beat.action` directly, so the
            // silhouette keeps rendering (at fading weight) through its own
            // release blend after combat has already left Stagger.
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
        let mut root_fwd = 0.0f32;
        match action {
            Action::Swing => {
                apply_key(&mut pose, swing_pose(rig.actor, &beat));
                // Weight the whole body into the swing, on top of the limb pose:
                // a small gather (rise + pull-back) through the wind-up, then the
                // root sinks and drives forward through the strike, easing back
                // upright across the recovery leg.
                let (offset, lean) = swing_root_motion(rig.actor, swing_k(&beat));
                root_lift += offset.y;
                root_fwd += offset.z;
                root_extra = Quat::from_axis_angle(Vec3::X, lean);
            }
            Action::Guard => {
                // Full weight while actually held — `rig.guard` is re-latched to
                // GUARD_LATCH every frame Block is down, so this stays at 1.0 —
                // easing to 0 only once the button has really been let go, over
                // the same window that used to just debounce the per-frame
                // Block->Idle->Block bounce. Before this the key held at full
                // strength right up to the frame `rig.guard` hit exactly 0, then
                // vanished — the arms snapped straight back to the walk cycle.
                let release = ease_out(rig.guard / GUARD_LATCH);
                apply_key(&mut pose, Key { weight: GUARD_KEY.weight * release, ..GUARD_KEY });
            }
            Action::Parry => {
                let release = ease_out(rig.parry / GUARD_LATCH);
                let mut k = parry_key(rig.parry_t);
                k.weight *= release;
                apply_key(&mut pose, k);
            }
            Action::Stagger => {
                let release = ease_out(rig.stagger_fade / STAGGER_FADE);
                stagger(&mut pose, elapsed, release);
            }
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

        // ---- secondary motion: cloth/hair that lags the body ------------------
        // One damped spring per part in `rig.secondary` (the cloak, plus whatever
        // `extra_parts` adds), each chasing a target driven by whatever the body
        // is doing — running streams it back, turning swings it, a swing's own
        // lunge drags it along — so it arrives a beat LATE. That lag is the
        // entire difference between "a prop glued to the back" and cloth. See
        // [`SecondaryMotionSpec`] for what each gain multiplies.
        let phase_sin = rig.phase.sin();
        let turn_norm = (rig.yaw_rate / TURN_FULL).clamp(-1.0, 1.0);
        for part in &mut rig.secondary {
            let spec = part.spec;
            let target_pitch = spec.loco_gain * loco * (0.5 + 0.5 * run) + spec.fwd_gain * root_fwd.abs();
            let target_roll = spec.sway_gain * phase_sin * loco + spec.turn_gain * turn_norm;
            spring(&mut part.pitch, &mut part.pitch_vel, target_pitch, spec.spring, spec.damp, dt);
            spring(&mut part.roll, &mut part.roll_vel, target_roll, spec.spring, spec.damp, dt);
            set(
                &mut joint_q,
                part.anchor,
                part.base_pos,
                pitch(spec.base_pitch + part.pitch) * roll(part.roll),
            );
        }

        // ---- write the rig ---------------------------------------------------
        let root_dy = root_offset(rig.look);
        root_tf.translation = Vec3::new(0.0, root_dy + root_lift, root_fwd);
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

/// ⑦ Conversation — the stand / talk / listen layer.
///
/// A dialogue is the easiest place in a game for a body to look dead: locomotion
/// parks at `loco == 0` and every joint holds the same breathing idle until the
/// text box closes. This lays a real STANCE over that idle and then plays one of
/// two behaviours on top of it.
///
///   * **stance** — contrapposto. Weight loads onto one leg, that hip lifts and
///     its knee straightens while the free leg softens, and the load crosses
///     over on a long uneven cycle. This is what separates *standing* from
///     *placed*, and it runs whether the actor is talking or listening.
///   * **talking** — the LEFT arm gestures (the right one is carrying a sword,
///     which is exactly what a person does), the chest leans in a touch, and the
///     head punctuates on the beat of the gesture.
///   * **listening** — arms hang, and the head acknowledges in short bursts
///     rather than nodding metronomically.
///
/// `w`     0..1 — the camera's own conversation blend (`cutscene.rs`), already
///         de-weighted by locomotion and airtime, so pose and lens arrive
///         together and a body that walks off mid-line simply leaves the scene.
/// `face`  the yaw the chest and head must close to look AT the partner, after
///         the root has been turned to stand three-quarters to the lens.
/// `tilt`  eye-line pitch: signed angle up to the partner's head. An elder is a
///         head shorter than Auren, and both of them should show it.
///
/// Every cycle below is a sum of sines whose periods do not divide one another,
/// so the stance never lands back on the same frame inside any shot length — a
/// visible loop is the single thing that reads as "animation" instead of a
/// person.
fn conversation(pose: &mut Pose, rig: &Rig, w: f32, face: f32, tilt: f32, elapsed: f32) {
    let w = w.clamp(0.0, 1.0);
    if w <= 0.001 {
        return;
    }
    let talk = rig.convo_talk.clamp(0.0, 1.0);
    let t = elapsed;

    let breath = (t * 1.15).sin();
    // +1 = weight over the right leg, -1 = over the left.
    let load = 0.62 * (t * 0.41).sin() + 0.38 * (t * 0.17 + 1.1).sin();
    let free_l = load.max(0.0); // weight right ⇒ the LEFT leg is the free one
    let free_r = (-load).max(0.0);
    // Gesture drive, and the two nod shapes.
    let g = 0.55 * (t * 2.10).sin() + 0.30 * (t * 3.70 + 0.6).sin() + 0.15 * (t * 1.27).sin();
    let talk_nod = (t * 2.10).sin().max(0.0).powi(2);
    let listen_nod = smoothstep(0.55, 0.95, (t * 0.37).sin()) * (t * 5.0).sin();

    // ---- lower body: the weight shift --------------------------------------
    pose.hips_pos.x += 0.030 * load * w;
    pose.hips_pos.y -= 0.018 * w;
    pose.hips = pose.hips.slerp(roll(-0.075 * load) * yaw(0.15 * face), w);
    pose.hip_l = pose.hip_l.slerp(pitch(0.11 * free_l) * roll(0.05 * load), w);
    pose.hip_r = pose.hip_r.slerp(pitch(0.11 * free_r) * roll(0.05 * load), w);
    pose.knee_l = pose.knee_l.slerp(pitch(-0.13 - 0.17 * free_l), w);
    pose.knee_r = pose.knee_r.slerp(pitch(-0.13 - 0.17 * free_r), w);

    // ---- spine + head: close the opened angle, hold the eye line ------------
    // hips 0.15 + torso 0.45 + head 0.40 = the whole residual, spread down the
    // chain the way a real turn distributes: least at the pelvis, most at the neck.
    let torso_lean = -0.045 - 0.035 * talk + 0.018 * breath;
    pose.torso = pose.torso.slerp(yaw(0.45 * face) * pitch(torso_lean), w);
    let head_pitch =
        tilt * 0.85 - 0.09 * talk_nod * talk + 0.11 * listen_nod * (1.0 - talk);
    pose.head = pose
        .head
        .slerp(yaw(0.40 * face + 0.05 * (t * 0.63).sin()) * pitch(head_pitch), w);

    // ---- arms ---------------------------------------------------------------
    // The off hand does the talking; the sword hand stays where `locomotion`'s
    // resting grip put it, just settled (shoulder eased back, elbow softened).
    let idle_l = pitch(-0.16 - 0.05 * breath) * roll(-0.14);
    let gest_l = pitch(-0.62 - 0.40 * g) * roll(0.30 + 0.22 * g) * yaw(0.10 * g);
    pose.sh_l = pose.sh_l.slerp(idle_l.slerp(gest_l, talk), w);
    pose.el_l = pose
        .el_l
        .slerp(pitch(0.42 + 0.55 * talk + 0.45 * talk * g.abs()), w);
    pose.sh_r = pose.sh_r.slerp(pitch(-0.11 - 0.04 * breath) * roll(0.12), w);
    pose.el_r = pose.el_r.slerp(pitch(0.30 + 0.10 * talk), w);
    pose.hand = pose.hand.slerp(pitch(-0.95) * roll(0.10), w);
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

/// Stagger — poise broken. A loose, off-balance wobble that decays on its own
/// clock. `blend` is the release-fade weight (see [`STAGGER_FADE`]): 1.0 while
/// actually staggered, easing to 0.0 across the tail instead of cutting
/// straight back to whatever `locomotion`/`turn_in_place` already wrote this
/// frame the instant `CombatState::Stagger` ends.
fn stagger(pose: &mut Pose, elapsed: f32, blend: f32) {
    let blend = blend.clamp(0.0, 1.0);
    if blend <= 0.001 {
        return;
    }
    let w = (elapsed * 11.0).sin();
    pose.torso = pose.torso.slerp(pitch(0.30 + 0.10 * w) * yaw(0.22 * w) * pose.torso, blend);
    pose.head = pose.head.slerp(pitch(0.28) * yaw(-0.26 * w) * pose.head, blend);
    pose.sh_l = pose.sh_l.slerp(pitch(-0.70) * roll(-0.75) * pose.sh_l, blend);
    pose.sh_r = pose.sh_r.slerp(pitch(-0.55) * roll(0.85) * pose.sh_r, blend);
    pose.hip_r = pose.hip_r.slerp(pitch(-0.40) * pose.hip_r, blend);
    pose.knee_l = pose.knee_l.slerp(pitch(-0.55) * pose.knee_l, blend);
    pose.hips_pos.y -= 0.10 * blend;
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

    let k = swing_k(beat);
    if k < 1.0 {
        mix(&neutral, &cock, k)
    } else if k < 2.0 {
        mix(&cock, &follow, k - 1.0)
    } else {
        // Follow-through: the blade doesn't just decelerate into neutral, it
        // carries a LITTLE past rest first — the overshoot a real swing's
        // momentum has once the strike is spent — then springs back. The angle
        // and the blend-weight are deliberately on two different curves: the
        // weight (`ease_in`, slow-then-fast) stays high through most of the
        // leg so the overshoot is still visible, and only collapses to 0 right
        // at the very end; the angle (`ease_out_back`) is what actually swings
        // past zero and settles. Using one curve for both (the old code) meant
        // the weight had already faded past the point the overshoot happens,
        // so a wider overshoot silently disappeared — see `swing_starts_and_
        // ends_neutral` for the boundary this still has to hit exactly.
        let u = (k - 2.0).clamp(0.0, 1.0);
        let mut key = mix(&follow, &neutral, ease_out_back(u));
        key.weight = follow.weight + (neutral.weight - follow.weight) * ease_in(u);
        key
    }
}

/// The swing's progress, 0→3: `0..1` wind-up, `1..2` the strike itself (spanning
/// exactly `beat.active`), `2..3` recovery. Shared by [`swing_pose`] (limb angles)
/// and [`swing_root_motion`] (the body's weight) so the two can never drift apart.
fn swing_k(beat: &Beat) -> f32 {
    let (a0, a1) = beat.active;
    let t = beat.t.clamp(0.0, 1.0);
    if t < a0 {
        ease_out(t / a0.max(1e-3))
    } else if t < a1 {
        1.0 + ease_io((t - a0) / (a1 - a0).max(1e-3))
    } else {
        2.0 + ease_out((t - a1) / (1.0 - a1).max(1e-3))
    }
}

/// The RIG ROOT's own weight through a swing — on top of whatever [`swing_pose`]
/// already did to the limbs. A real cut is not thrown from the shoulder alone: the
/// body gathers (rises, pulls back) through the wind-up, then the whole mass drops
/// and drives forward through the strike, and eases back upright across recovery.
/// Purely cosmetic — this moves the rig root, a child of the actor; the actor's own
/// transform (what combat reads) is never touched.
///
/// Returns (translation offset, extra forward/back lean in radians — positive
/// leans back, matching the sign the authored [`Key::torso_pitch`] values use).
fn swing_root_motion(actor: Actor, k: f32) -> (Vec3, f32) {
    let mag = match actor {
        Actor::Player => 1.0,
        // The Husk is bigger and its overhead carries more mass — the weight
        // transfer reads proportionally larger.
        Actor::Husk => 1.6,
    };
    // (rise, pull-back, lean-back) gathered by the top of the wind-up →
    // (sink, drive-forward, lean-forward) at full extension, at the end of the
    // strike leg → eased back out to rest across recovery.
    let gather = (0.016 * mag, 0.03 * mag, 0.06 * mag);
    let strike = (-0.095 * mag, -0.14 * mag, -0.15 * mag);

    let (lift, back, lean) = if k < 1.0 {
        let u = ease_out(k);
        (gather.0 * u, gather.1 * u, gather.2 * u)
    } else if k < 2.0 {
        let u = ease_io(k - 1.0);
        (
            gather.0 + (strike.0 - gather.0) * u,
            gather.1 + (strike.1 - gather.1) * u,
            gather.2 + (strike.2 - gather.2) * u,
        )
    } else {
        // Same overshoot-and-settle as `swing_pose`'s recovery leg, applied to
        // the root's own weight shift — the body rocks very slightly past
        // upright after a big swing before it settles, instead of gliding
        // straight back to neutral.
        let ov = ease_out_back((k - 2.0).clamp(0.0, 1.0));
        (strike.0 * (1.0 - ov), strike.1 * (1.0 - ov), strike.2 * (1.0 - ov))
    };

    (Vec3::new(0.0, lift, back), lean)
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

/// Where the 12-frame receive window ends, as a fraction of the whole parry
/// action. Everything after this point is the part of the parry that can only
/// hurt you.
const PARRY_WINDOW_FRAC: f32 = combat::PARRY_WINDOW / crate::dodge_parry::PARRY_STATE_LEN;

/// Blade dropping out of the parry — arms down, guard open, weight forward onto
/// the wrong foot. This is what "you committed and the window has shut" looks
/// like from across the arena.
const PARRY_SPENT_KEY: Key = Key {
    sh_pitch: -0.20,
    sh_yaw: 0.30,
    sh_roll: -0.10,
    elbow: 0.25,
    grip_pitch: 0.35,
    grip_roll: 0.20,
    off_pitch: -0.15,
    off_roll: 0.10,
    off_elbow: 0.35,
    torso_yaw: -0.10,
    torso_pitch: 0.14,
    hips_yaw: -0.06,
    head_yaw: 0.04,
    head_pitch: 0.10,
    lunge: 0.16,
    crouch: 0.02,
    stance: 0.30,
    weight: 0.85,
};

/// The parry, posed across its own timeline rather than as one frozen shape.
///
/// The window is a *frame count* now (`dodge_parry::PARRY_WINDOW_FRAMES`), and a
/// mechanic the player cannot see the edge of is a mechanic they cannot learn.
/// So the blade snaps out over the first fraction of the window, holds while the
/// window is live, and then visibly falls out of guard the instant it shuts —
/// the same information the log line carries, drawn on the body.
fn parry_key(t: f32) -> Key {
    let t = t.clamp(0.0, 1.0);
    if t <= PARRY_WINDOW_FRAC {
        // Snap out fast (the flick is the read), then hold the live pose.
        let snap = (t / (PARRY_WINDOW_FRAC * 0.35)).min(1.0);
        mix(&GUARD_KEY, &PARRY_KEY, ease_io(snap))
    } else {
        let spent = ((t - PARRY_WINDOW_FRAC) / (1.0 - PARRY_WINDOW_FRAC)).clamp(0.0, 1.0);
        mix(&PARRY_KEY, &PARRY_SPENT_KEY, ease_io(spent))
    }
}

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
        // The parry now runs on a real clock — a 12-frame receive window inside a
        // 0.70 s commitment (`dodge_parry::PARRY_STATE_LEN`) — so the pose gets
        // the timeline instead of a single frozen shape.
        CombatState::Parry => Beat {
            action: Action::Parry,
            t: (pc.timer / crate::dodge_parry::PARRY_STATE_LEN).clamp(0.0, 1.0),
            active: (0.0, PARRY_WINDOW_FRAC),
            combo: 0,
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

/// Which swing phase a [`Beat`] is in, or None when it is not attacking. The
/// CONTACT band is exactly `beat.active` — the same window combat opens its
/// hitbox across — so the `AnimSwing(Contact)` event lands inside the gameplay
/// active frames, never beside them.
fn swing_phase_of(beat: &Beat) -> Option<SwingPhase> {
    if beat.action != Action::Swing {
        return None;
    }
    Some(if beat.t < beat.active.0 {
        SwingPhase::Windup
    } else if beat.t < beat.active.1 {
        SwingPhase::Contact
    } else {
        SwingPhase::Recover
    })
}

/// The canonical frame of a move, for the capture hook (VOXELFORGE_ANIM_POSE).
/// Each held pose sits at the moment that move reads most clearly on a still:
///
///   * Attack  — the CONTACT frame (mid-LIGHT_ACTIVE): blade mid-strike.
///   * Dodge   — beat.t = 0.5: a quarter-turned tuck, unmistakably a roll.
///   * Parry   — the centre of the receive window: guard flashed up to deflect.
///   * Clash   — the player half is identical to Attack; see
///     `override_husk_beat()` for the Husk's half of the same beat.
fn override_beat(p: OverridePose) -> Beat {
    match p {
        OverridePose::Attack | OverridePose::Clash => {
            let a0 = combat::LIGHT_ACTIVE.0 / combat::LIGHT_TIME;
            let a1 = combat::LIGHT_ACTIVE.1 / combat::LIGHT_TIME;
            Beat {
                action: Action::Swing,
                t: 0.5 * (a0 + a1),
                active: (a0, a1),
                combo: 1,
            }
        }
        OverridePose::Dodge => Beat {
            action: Action::Dodge,
            t: 0.5,
            active: (0.3, 0.6),
            combo: 0,
        },
        OverridePose::Parry => Beat {
            action: Action::Parry,
            t: 0.5 * PARRY_WINDOW_FRAC,
            active: (0.0, PARRY_WINDOW_FRAC),
            combo: 0,
        },
    }
}

/// The Husk's half of `Clash`: held at its own contact frame — the midpoint of
/// the `(wind, 0.80)` active window `husk_beat` uses for `Swing1`/`Swing2` —
/// so its blade is up and moving at the same instant the player's is.
fn override_husk_beat() -> Beat {
    const WIND: f32 = 0.62;
    const STRIKE: f32 = 0.80;
    Beat {
        action: Action::Swing,
        t: 0.5 * (WIND + STRIKE),
        active: (WIND, STRIKE),
        combo: 0,
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
        HuskState::Feint => {
            // The wind-up is pulled back down without ever committing — the arm
            // retreats from the raised pose to neutral over `HUSK_FEINT_RECOVER`,
            // ending exactly where the next Telegraph starts (t=0), so the tell
            // reads as "changed its mind", not a stutter.
            let k = (e.timer / combat::HUSK_FEINT_RECOVER).min(1.0);
            Beat {
                action: Action::Swing,
                t: wind * (1.0 - k),
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
        HuskState::Patrol
        | HuskState::Chase
        | HuskState::Alert
        | HuskState::Reposition
        | HuskState::Search
        | HuskState::LungeWind
        | HuskState::LungeDash => {
            // Movement / tactics states (added with the perception pass in
            // combat.rs): no arm-arc beat yet. The melee telegraph pose lives in
            // combat.rs's `husk_telegraph` (arm-mesh raise) and covers these
            // states' tells; dedicated beats for the lunge crouch belong to the
            // animation lane if wanted later.
            Beat::none()
        }
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

/// Did the stride `phase` advance past `target` this frame? `phase` only ever
/// moves forward (it is advanced by distance travelled), so this is a one-sided
/// crossing test — used to fire a footfall exactly once per foot per cycle.
#[inline]
fn phase_crossed(prev: f32, cur: f32, target: f32) -> bool {
    let delta = wrap_tau(cur - prev);
    if delta <= 0.0 {
        return false;
    }
    wrap_tau(target - prev) < delta
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

/// Slow start, fast finish — the mirror of [`ease_out`].
#[inline]
fn ease_in(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

/// Like [`ease_out`], but overshoots slightly above 1.0 before settling back —
/// "the motion doesn't just decelerate into rest, it carries past it and
/// springs back", the follow-through/overlap read a plain `ease_out` can't
/// produce. `c1` is well below the textbook ~1.7 so this stays a subtle settle,
/// not a cartoon bounce.
#[inline]
fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.4;
    const C3: f32 = C1 + 1.0;
    let x = t.clamp(0.0, 1.0) - 1.0;
    1.0 + C3 * x * x * x + C1 * x * x
}

/// Semi-implicit-Euler damped spring: `value` chases `target`, `vel` carries the
/// momentum between calls. Used for the cloak hinge — anything driven this way
/// arrives at its target a beat AFTER the thing driving it changes, which is
/// what makes it read as cloth lagging the body instead of a rigid attachment.
#[inline]
fn spring(value: &mut f32, vel: &mut f32, target: f32, k: f32, damping: f32, dt: f32) {
    *vel += (target - *value) * k * dt;
    *vel *= (1.0 - damping * dt).clamp(0.0, 1.0);
    *value += *vel * dt;
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
            rhythm: combat::HuskRhythm::Straight,
            combo_no: 0,
            feinted: false,
            lunge_cd: 0.0,
            strafe_dir: 1.0,
            logged_state: state,
            kind: crate::enemies::EnemyKind::Sentinel,
            last_seen: Vec3::ZERO,
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

    // ---- animation-event timing (docs/anim-events.md contract) -------------

    fn beat(action: Action, t: f32, active: (f32, f32)) -> Beat {
        Beat { action, t, active, combo: 1 }
    }

    #[test]
    fn swing_phase_splits_anticipation_contact_followthrough() {
        let active = (0.34, 0.63);
        assert_eq!(swing_phase_of(&beat(Action::Swing, 0.10, active)), Some(SwingPhase::Windup));
        assert_eq!(swing_phase_of(&beat(Action::Swing, 0.50, active)), Some(SwingPhase::Contact));
        assert_eq!(swing_phase_of(&beat(Action::Swing, 0.90, active)), Some(SwingPhase::Recover));
        assert_eq!(swing_phase_of(&beat(Action::Swing, active.0, active)), Some(SwingPhase::Contact));
        assert_eq!(swing_phase_of(&beat(Action::Swing, active.1, active)), Some(SwingPhase::Recover));
        assert_eq!(swing_phase_of(&beat(Action::None, 0.5, active)), None);
    }

    #[test]
    fn light_contact_band_lands_inside_the_gameplay_hitbox() {
        let a0 = combat::LIGHT_ACTIVE.0 / combat::LIGHT_TIME;
        let a1 = combat::LIGHT_ACTIVE.1 / combat::LIGHT_TIME;
        let active = (a0, a1);
        assert_eq!(swing_phase_of(&beat(Action::Swing, a0, active)), Some(SwingPhase::Contact));
        assert_eq!(swing_phase_of(&beat(Action::Swing, a1 - 1e-4, active)), Some(SwingPhase::Contact));
        assert_eq!(swing_phase_of(&beat(Action::Swing, a0 - 1e-3, active)), Some(SwingPhase::Windup));
    }

    #[test]
    fn attack_event_fires_each_phase_exactly_once() {
        let active = (0.34, 0.63);
        let mut seen = Vec::new();
        let mut prev: Option<SwingPhase> = None;
        let n = 400;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let cur = swing_phase_of(&beat(Action::Swing, t, active));
            if cur != prev {
                if let Some(p) = cur {
                    seen.push(p);
                }
                prev = cur;
            }
        }
        assert_eq!(seen, vec![SwingPhase::Windup, SwingPhase::Contact, SwingPhase::Recover]);
    }

    #[test]
    fn dodge_iframe_window_matches_combat() {
        let iframe_end = combat::DODGE_IFRAMES / (combat::DODGE_IFRAMES + combat::DODGE_RECOVERY);
        let in_band = beat(Action::Dodge, iframe_end - 1e-4, (0.3, 0.6));
        let past = beat(Action::Dodge, iframe_end + 1e-4, (0.3, 0.6));
        assert!(in_band.action == Action::Dodge && in_band.t < iframe_end);
        assert!(past.action == Action::Dodge && !(past.t < iframe_end));
        assert!((iframe_end - 10.0 / 22.0).abs() < 1e-4);
    }

    #[test]
    fn parry_window_matches_combat() {
        let expect = combat::PARRY_WINDOW / crate::dodge_parry::PARRY_STATE_LEN;
        assert!((PARRY_WINDOW_FRAC - expect).abs() < 1e-4);
        let ob = override_beat(OverridePose::Parry);
        assert!(ob.t > 0.0 && ob.t < PARRY_WINDOW_FRAC);
    }

    #[test]
    fn override_beat_holds_each_canonical_frame() {
        let a = override_beat(OverridePose::Attack);
        assert_eq!(a.action, Action::Swing);
        let a0 = combat::LIGHT_ACTIVE.0 / combat::LIGHT_TIME;
        let a1 = combat::LIGHT_ACTIVE.1 / combat::LIGHT_TIME;
        assert!(a.t > a0 && a.t < a1);
        assert_eq!(swing_phase_of(&a), Some(SwingPhase::Contact));
        assert_eq!(override_beat(OverridePose::Dodge).t, 0.5);
    }

    #[test]
    fn clash_holds_both_actors_at_their_own_contact_frame() {
        let player = override_beat(OverridePose::Clash);
        assert_eq!(player.action, Action::Swing);
        assert_eq!(swing_phase_of(&player), Some(SwingPhase::Contact));

        let husk = override_husk_beat();
        assert_eq!(husk.action, Action::Swing);
        assert_eq!(swing_phase_of(&husk), Some(SwingPhase::Contact));
    }

    #[test]
    fn phase_crossed_fires_once_per_foot_per_cycle() {
        let tau = std::f32::consts::TAU;
        let mut left = 0;
        let mut right = 0;
        let mut prev = 0.0f32;
        for _ in 1..=60 {
            let cur = wrap_tau(prev + tau / 60.0);
            if phase_crossed(prev, cur, std::f32::consts::PI) {
                left += 1;
            }
            if phase_crossed(prev, cur, 0.0) {
                right += 1;
            }
            prev = cur;
        }
        assert_eq!(left, 1);
        assert_eq!(right, 1);
        assert!(!phase_crossed(1.0, 1.0, std::f32::consts::PI));
    }
}
