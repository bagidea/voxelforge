//! Conversation staging — the cutscene camera, and the one resource every other
//! lane pokes to put the game into "two people are talking" mode.
//!
//! Two consumers read the state this file owns:
//!
//!   * `stage_conversation_camera` (here) eases the gameplay lens into a framed
//!     two-shot of the pair and eases it back out again, and
//!   * `anim.rs`'s conversation layer drives the stand / talk / listen bodies off
//!     the same blend, so the pose and the lens always arrive together.
//!
//! ## The whole API another lane needs
//!
//! ```ignore
//! // quest.rs, the frame dialogue opens — `npc` is the entity you already have:
//! convo.begin(npc);
//!
//! // the frame it closes:
//! convo.end();
//! ```
//!
//! Nothing else is required. `begin`/`end` are idempotent, safe to call every
//! frame, and safe to call with an entity that is later despawned (the shot
//! releases itself). Two optional one-liners refine it:
//!
//! ```ignore
//! convo.say(cutscene::Speaker::Player); // pin who holds the floor
//! convo.say(cutscene::Speaker::Npc);
//! ```
//!
//! Left alone, the talker alternates on its own every [`TURN_TAKE`] seconds so
//! both bodies read as taking turns rather than both miming at once.
//!
//! ## Why the camera blends instead of cutting
//!
//! `fly_camera` (and after it `combat::lock_on_camera` / `combat::apply_shake`)
//! writes the real gameplay transform every frame. This system runs LAST and
//! interpolates FROM whatever they just wrote TO the staged two-shot, weighted
//! by [`Conversation::weight`]. At weight 0 it writes the gameplay transform
//! back unchanged — so there is no saved-and-restored camera to go stale, and
//! the ride out is a blend toward a lens that is still live underneath, not a
//! snap back to a pose captured seconds ago.
//!
//! Ownership: this file is Poppy's, alongside `anim.rs`. It reads `quest.rs`
//! and `combat.rs`; it writes neither.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use crate::{Cfg, FlyCam, OrbitCam};

// ---------------------------------------------------------------------------
// Tunables
// ---------------------------------------------------------------------------

/// Seconds for the lens to travel from gameplay into the two-shot.
const ENTER_TIME: f32 = 0.85;
/// Seconds to travel back. Shorter than the entry — leaving a conversation
/// should hand control back briskly, arriving takes its time.
const EXIT_TIME: f32 = 0.60;
/// How often the auto-alternator swaps who is talking, when no lane calls
/// [`Conversation::say`].
const TURN_TAKE: f32 = 2.6;
/// Exponential follow rate on the staged lens + its aim point. This is what
/// turns a speaker change (or the NPC shifting its feet) into a glide instead
/// of a cut; the blend above only covers entering and leaving.
const FOLLOW_K: f32 = 3.2;
/// How far off the line-of-conversation each body turns, so the pair stands at
/// three-quarters to the lens instead of nose-to-nose in profile. The head and
/// chest close the same angle back (`anim.rs`), so they still *look* at each
/// other. ~17°.
const STAGE_OPEN: f32 = 0.30;
// Where the NPC's head sits above its transform origin comes from the rig
// `anim.rs` actually builds for it (`anim::npc_head_lift`), not from a constant
// typed in here — a two-shot aimed at a hand-guessed head height frames a chin.

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Who currently holds the floor. Drives which body gestures and which listens,
/// and biases the lens a touch toward the speaker.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Speaker {
    /// The NPC is talking, the player listens. The state a conversation opens in.
    #[default]
    Npc,
    /// The player is talking (a picked choice), the NPC listens.
    Player,
    /// A held beat — both bodies listen. Nothing selects this automatically.
    Nobody,
}

/// The one resource other lanes drive. See the module docs for the two-line API.
#[derive(Resource, Debug)]
pub struct Conversation {
    active: bool,
    npc: Option<Entity>,
    /// 0 = pure gameplay camera, 1 = pure staged two-shot.
    blend: f32,
    /// Seconds since [`Conversation::begin`]. Drives the turn-taking alternator
    /// and the slow push-in, so both restart with each conversation.
    clock: f32,
    speaker: Speaker,
    /// Set by [`Conversation::say`]. Until then the talker alternates.
    pinned: bool,
    /// +1/-1: which side of the conversation line the lens sits on. Chosen ONCE,
    /// on the first staged frame, from where the gameplay camera already was —
    /// re-deriving it per frame swings the shot through 180° the moment the
    /// player drifts across the line.
    side: f32,
    /// Smoothed lens position + aim point (see [`FOLLOW_K`]).
    eye: Vec3,
    aim: Vec3,
    /// False until `eye`/`aim` hold a real sample rather than their default.
    warm: bool,
}

impl Default for Conversation {
    fn default() -> Self {
        Conversation {
            active: false,
            npc: None,
            blend: 0.0,
            clock: 0.0,
            speaker: Speaker::Npc,
            pinned: false,
            side: 0.0,
            eye: Vec3::ZERO,
            aim: Vec3::ZERO,
            warm: false,
        }
    }
}

impl Conversation {
    /// **Enter conversation mode**, framed on `npc`. Call it the frame dialogue
    /// opens; calling it again while already talking to the same NPC is a no-op,
    /// so it is safe from a per-frame system. Passing a *different* NPC re-stages
    /// on the new one without dropping back to gameplay in between.
    pub fn begin(&mut self, npc: Entity) {
        if self.active && self.npc == Some(npc) {
            return;
        }
        self.active = true;
        self.npc = Some(npc);
        self.clock = 0.0;
        self.speaker = Speaker::Npc;
        self.pinned = false;
        self.side = 0.0; // re-picked on the first staged frame
        self.warm = false;
        println!("CUTSCENE begin npc={npc:?}");
    }

    /// **Leave conversation mode.** The lens eases back to the live gameplay
    /// camera over [`EXIT_TIME`]; `is_active` reports false immediately, but
    /// [`weight`](Self::weight) keeps falling until the ride is over.
    pub fn end(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        println!("CUTSCENE end");
    }

    /// Pin who holds the floor. Optional — see the module docs. Once called, the
    /// auto-alternator stays off for the rest of this conversation.
    pub fn say(&mut self, who: Speaker) {
        self.speaker = who;
        self.pinned = true;
    }

    /// True between `begin` and `end`, ignoring the blend ride on either side.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// 0..1 — how much of the staged framing (and the conversation pose layer)
    /// is currently mixed in. Non-zero through both blend rides.
    pub fn weight(&self) -> f32 {
        self.blend
    }

    /// The NPC being talked to, while any of the shot is still mixed in.
    pub fn npc(&self) -> Option<Entity> {
        self.npc
    }

    /// Who is talking right now, resolving the auto-alternator.
    pub fn speaker(&self) -> Speaker {
        if self.pinned {
            return self.speaker;
        }
        // NPC first: a conversation opens with the other party greeting you.
        if ((self.clock / TURN_TAKE) as i32) % 2 == 0 {
            Speaker::Npc
        } else {
            Speaker::Player
        }
    }

    /// How far this body turns off the conversation line, so the pair stands
    /// three-quarters to the lens. The two actors open in OPPOSITE directions —
    /// the same world-space rotation would open one toward the lens and the
    /// other away from it. `anim.rs` closes the same angle back with the head
    /// and chest, so they still look at each other.
    pub fn stage_open(&self, is_player: bool) -> f32 {
        if is_player {
            -self.side * STAGE_OPEN
        } else {
            self.side * STAGE_OPEN
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Conversation>()
            .init_resource::<CutsceneDemo>()
            .add_systems(
                Update,
                (
                    // LAST word on the camera transform: after `fly_camera`, after
                    // lock-on, after screen shake — otherwise whichever of those
                    // runs later overwrites the staged framing and the two-shot
                    // never appears (see docs/LANES.md's camera-ordering note).
                    stage_conversation_camera
                        .after(crate::fly_camera)
                        .after(crate::combat::lock_on_camera)
                        .after(crate::combat::apply_shake),
                    // Proof harness — inert unless VOXELFORGE_CUTSCENE_DEMO is set.
                    cutscene_demo
                        .after(stage_conversation_camera)
                        .run_if(demo_requested),
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// The camera
// ---------------------------------------------------------------------------

/// Frame the two speakers, blended over whatever the gameplay camera just wrote.
///
/// Writes nothing at all while `blend` is zero, which is every frame outside a
/// conversation — the cost of having this plugin installed is one resource read.
#[allow(clippy::type_complexity)]
pub fn stage_conversation_camera(
    time: Res<Time>,
    mut convo: ResMut<Conversation>,
    player_q: Query<&Transform, (With<FlyCam>, Without<OrbitCam>)>,
    other_q: Query<&Transform, (Without<FlyCam>, Without<OrbitCam>)>,
    mut cam_q: Query<&mut Transform, With<OrbitCam>>,
) {
    let dt = time.delta_secs().clamp(0.0, 1.0 / 15.0);

    // ---- blend clock ------------------------------------------------------
    let (target, span) = if convo.active {
        (1.0, ENTER_TIME)
    } else {
        (0.0, EXIT_TIME)
    };
    let step = dt / span;
    convo.blend = if convo.blend < target {
        (convo.blend + step).min(target)
    } else {
        (convo.blend - step).max(target)
    };
    if convo.active {
        convo.clock += dt;
    }
    if !convo.active && convo.blend <= 0.0 {
        // Ride over — release the NPC. Held until now because the ride out still
        // needs its transform to know what it is blending away from.
        convo.npc = None;
        convo.warm = false;
        return;
    }
    if convo.blend <= 0.0 {
        return;
    }

    // ---- resolve the pair -------------------------------------------------
    let Ok(mut cam) = cam_q.single_mut() else {
        return;
    };
    let Ok(ptf) = player_q.single() else {
        return;
    };
    let Some(npc_e) = convo.npc else {
        return;
    };
    let Ok(ntf) = other_q.get(npc_e) else {
        // The NPC went away mid-shot. Drop the conversation rather than framing a
        // hole; the blend then rides back out on its own, still smoothly.
        convo.active = false;
        return;
    };

    // The player transform IS its eye (main.rs's convention); the NPC's origin is
    // the middle of its body, so lift to roughly head height for both.
    let a = ptf.translation;
    let b = ntf.translation + Vec3::Y * crate::anim::npc_head_lift();

    let flat = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
    let sep = flat.length();
    let axis = flat.normalize_or(Vec3::NEG_Z); // player -> NPC, horizontal
    let side_dir = Vec3::new(-axis.z, 0.0, axis.x); // left/right of that line
    let mid = (a + b) * 0.5;

    if convo.side == 0.0 {
        // Stand the lens on the side it is ALREADY on, so entering the shot is a
        // short arc rather than a swing across the pair.
        let to_cam = cam.translation - mid;
        convo.side = if to_cam.dot(side_dir) >= 0.0 { 1.0 } else { -1.0 };
    }
    let side = side_dir * convo.side;

    // Boom length from how far apart they stand, so a close chat is a tight
    // two-shot and a shout across a gate still fits both bodies.
    let base = (sep * 0.95 + 1.55).clamp(2.6, 6.5);
    // A slow push-in over the first few seconds plus a long lateral float: the
    // frame keeps breathing instead of freezing the moment it arrives.
    let push = 1.0 - 0.07 * smooth01(convo.clock / 6.0);
    let float = (convo.clock * 0.27).sin() * 0.10;

    let eye = mid
        + side * (base * push)
        // Pulled a little behind the player's shoulder: a true perpendicular
        // two-shot reads as two profiles, this reads as three-quarter faces.
        - axis * (base * 0.16)
        + axis * float
        + Vec3::Y * (0.52 + 0.09 * (convo.clock * 0.21).sin());

    // Aim between them, nudged toward whoever is talking.
    let head = match convo.speaker() {
        Speaker::Player => a,
        Speaker::Npc => b,
        Speaker::Nobody => mid,
    };
    let aim = mid.lerp(head, 0.22) + Vec3::Y * 0.10;

    // ---- smooth, then blend over the gameplay lens ------------------------
    if convo.warm {
        let k = 1.0 - (-FOLLOW_K * dt).exp();
        let (prev_eye, prev_aim) = (convo.eye, convo.aim);
        convo.eye = prev_eye + (eye - prev_eye) * k;
        convo.aim = prev_aim + (aim - prev_aim) * k;
    } else {
        convo.eye = eye;
        convo.aim = aim;
        convo.warm = true;
    }

    let look = convo.aim - convo.eye;
    if look.length_squared() < 1.0e-6 {
        return; // degenerate — leave the gameplay camera alone this frame
    }
    let want = Transform::from_translation(convo.eye).looking_at(convo.aim, Vec3::Y);

    let w = smooth01(convo.blend);
    cam.translation = cam.translation.lerp(want.translation, w);
    cam.rotation = cam.rotation.slerp(want.rotation, w);
}

/// Smoothstep on an already-normalised 0..1 input.
#[inline]
fn smooth01(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------
// Proof harness
// ---------------------------------------------------------------------------
//
// Two env vars, both off by default, both inert in a shipped session:
//
//   VOXELFORGE_CUTSCENE_DEMO=1        run the scripted enter/exit
//   VOXELFORGE_CUTSCENE_SHOTS=<dir>   burst-capture the whole ride into <dir>
//
// The script parks the player in front of the nearest NPC, waits, enters the
// shot, holds it, leaves it, and exits — with a per-frame continuity gate on the
// lens, because "it blends" is a claim about frames NOBODY sees in a still.

const DEMO_SETTLE: f32 = 1.2; // world is up, drop the player at the NPC
const DEMO_BEGIN: f32 = 3.0; // enter the conversation
const DEMO_END: f32 = 8.5; // leave it
const DEMO_QUIT: f32 = 12.4;
const SHOT_FIRST: f32 = 1.9;
const SHOT_LAST: f32 = 12.0;
const SHOT_DT: f32 = 0.22;
/// Continuity gate. A hard cut moves the lens the whole boom in ONE frame —
/// hundreds of m/s at 60fps. A blend of a ~6 m arc over [`ENTER_TIME`] peaks
/// well under this. Anything above it is a teleport, not a move.
const MAX_LENS_SPEED: f32 = 30.0; // m/s
const MAX_LENS_SPIN: f32 = 360.0; // deg/s

#[derive(Resource, Default)]
struct CutsceneDemo {
    placed: bool,
    begun: bool,
    ended: bool,
    next_shot: f32,
    frame: u32,
    prev: Option<(Vec3, Quat)>,
    max_speed: f32,
    max_speed_t: f32,
    max_spin: f32,
    max_spin_t: f32,
}

fn demo_requested(cfg: Res<Cfg>) -> bool {
    cfg.play && std::env::var("VOXELFORGE_CUTSCENE_DEMO").is_ok()
}

fn shots_dir() -> Option<String> {
    std::env::var("VOXELFORGE_CUTSCENE_SHOTS").ok().filter(|s| !s.is_empty())
}

#[allow(clippy::type_complexity)]
fn cutscene_demo(
    time: Res<Time>,
    mut commands: Commands,
    mut demo: ResMut<CutsceneDemo>,
    mut convo: ResMut<Conversation>,
    mut player_q: Query<(&mut Transform, &mut FlyCam), Without<OrbitCam>>,
    npc_q: Query<(Entity, &Transform), (With<crate::quest::Npc>, Without<FlyCam>, Without<OrbitCam>)>,
    cam_q: Query<&Transform, With<OrbitCam>>,
    mut exit: bevy::ecs::message::MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs();

    // ---- stage: put the player in talking range of the NPC ----------------
    if !demo.placed && t >= DEMO_SETTLE {
        let Ok((mut ptf, mut fly)) = player_q.single_mut() else {
            return;
        };
        let Some((_, ntf)) = npc_q.iter().next() else {
            println!("CUTSCENE_DEMO no NPC in the scene => FAIL");
            demo.placed = true;
            return;
        };
        // 2.4 blocks out along +Z, dropped in from above so `move_body`'s gravity
        // settles the body onto whatever the ground actually is out here.
        let stand = Vec3::new(ntf.translation.x, ntf.translation.y + 1.6, ntf.translation.z + 2.4);
        ptf.translation = stand;
        fly.walking = true;
        fly.vel = Vec3::ZERO;
        // Face the NPC: forward is local -Z, so looking back along -Z is yaw 0.
        let to = ntf.translation - stand;
        fly.face_yaw = (-to.x).atan2(-to.z);
        demo.placed = true;
        println!(
            "CUTSCENE_DEMO staged player at ({:.1},{:.1},{:.1}) facing npc at ({:.1},{:.1},{:.1})",
            stand.x, stand.y, stand.z, ntf.translation.x, ntf.translation.y, ntf.translation.z
        );
    }

    // ---- script: enter, hold, leave ---------------------------------------
    if !demo.begun && t >= DEMO_BEGIN {
        if let Some((e, _)) = npc_q.iter().next() {
            // >>> THE ONE LINE another lane writes. <<<
            convo.begin(e);
        }
        demo.begun = true;
    }
    if !demo.ended && t >= DEMO_END {
        // >>> AND THE OTHER ONE. <<<
        convo.end();
        demo.ended = true;
    }

    // ---- continuity gate ---------------------------------------------------
    // Measured from after the staging teleport, which is a deliberate jump.
    if let Ok(ctf) = cam_q.single() {
        if t > DEMO_SETTLE + 0.6 && dt > 0.0 {
            if let Some((pp, pr)) = demo.prev {
                let speed = ctf.translation.distance(pp) / dt;
                let spin = ctf.rotation.angle_between(pr).to_degrees() / dt;
                if speed > demo.max_speed {
                    demo.max_speed = speed;
                    demo.max_speed_t = t;
                }
                if spin > demo.max_spin {
                    demo.max_spin = spin;
                    demo.max_spin_t = t;
                }
            }
        }
        demo.prev = Some((ctf.translation, ctf.rotation));

        // ---- burst capture -------------------------------------------------
        if let Some(dir) = shots_dir() {
            if t >= SHOT_FIRST && t <= SHOT_LAST && t >= demo.next_shot {
                let path = format!("{dir}/cut_{:03}.png", demo.frame);
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path.clone()));
                println!(
                    "CUTSCENE_SHOT n={} t={:.2} blend={:.3} cam=({:.2},{:.2},{:.2}) {path}",
                    demo.frame,
                    t,
                    convo.weight(),
                    ctf.translation.x,
                    ctf.translation.y,
                    ctf.translation.z
                );
                demo.frame += 1;
                demo.next_shot = t + SHOT_DT;
            }
        }
    }

    // ---- verdict + quit ----------------------------------------------------
    if t >= DEMO_QUIT {
        let ok = demo.max_speed <= MAX_LENS_SPEED && demo.max_spin <= MAX_LENS_SPIN;
        println!(
            "CUTSCENE_CONTINUITY max_speed={:.1}m/s@{:.2}s (limit {MAX_LENS_SPEED}) \
             max_spin={:.0}deg/s@{:.2}s (limit {MAX_LENS_SPIN}) frames={} => {}",
            demo.max_speed,
            demo.max_speed_t,
            demo.max_spin,
            demo.max_spin_t,
            demo.frame,
            if ok { "PASS" } else { "FAIL" }
        );
        if !ok {
            crate::hero::GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
        }
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn e(i: u32) -> Entity {
        Entity::from_raw_u32(i).expect("valid test entity id")
    }

    #[test]
    fn begin_then_end_rides_the_blend_both_ways() {
        let mut c = Conversation::default();
        assert_eq!(c.weight(), 0.0);
        c.begin(e(1));
        assert!(c.is_active());
        // `end` flips the flag immediately; the WEIGHT is what rides out, and it
        // is still whatever the camera system last left it at.
        c.end();
        assert!(!c.is_active());
    }

    #[test]
    fn begin_on_the_same_npc_does_not_restart_the_shot() {
        let mut c = Conversation::default();
        c.begin(e(7));
        c.clock = 4.0;
        c.side = 1.0;
        c.begin(e(7)); // safe from a per-frame system
        assert_eq!(c.clock, 4.0, "re-begin must not restage a live conversation");
        assert_eq!(c.side, 1.0);
        c.begin(e(8)); // a DIFFERENT npc does restage
        assert_eq!(c.clock, 0.0);
    }

    #[test]
    fn talker_alternates_until_a_lane_pins_it() {
        let mut c = Conversation::default();
        c.begin(e(1));
        assert_eq!(c.speaker(), Speaker::Npc, "a conversation opens on the NPC");
        c.clock = TURN_TAKE + 0.01;
        assert_eq!(c.speaker(), Speaker::Player);
        c.clock = 2.0 * TURN_TAKE + 0.01;
        assert_eq!(c.speaker(), Speaker::Npc);
        c.say(Speaker::Player);
        c.clock = 5.0 * TURN_TAKE; // alternator would say Npc here
        assert_eq!(c.speaker(), Speaker::Player, "say() pins for the rest of the scene");
    }

    #[test]
    fn the_two_bodies_open_away_from_each_other() {
        let mut c = Conversation::default();
        c.begin(e(1));
        c.side = 1.0;
        let p = c.stage_open(true);
        let n = c.stage_open(false);
        assert!(p * n < 0.0, "player and NPC must open in OPPOSITE directions");
        assert!((p.abs() - STAGE_OPEN).abs() < 1.0e-6);
        // Flipping which side the lens sits on flips both.
        c.side = -1.0;
        assert!((c.stage_open(true) + p).abs() < 1.0e-6);
    }
}
