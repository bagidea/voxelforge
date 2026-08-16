//! Enemy behaviour / AI — Rose's lane (2026-08-14).
//!
//! The bodies live in `enemies.rs` (Monanisa); the hit→die→respawn loop lives
//! in `combat.rs` (Kevin). This module is the third leg: the *behaviour* that
//! drives any body — patrol, the alert beat, pursuit, and an honest
//! telegraphed attack. It is deliberately decoupled from both files:
//!
//! - **No `crate::` reference** — same self-containment rule as `enemies.rs`
//!   and `characters.rs`, so an isolated proof bin can `#[path]`-include this
//!   file plus the bodies and render a chase clip without touching `combat.rs`.
//! - **Archetype, not EnemyKind** — behaviour is tuned against `Archetype`
//!   (Swarm / Bruiser / Pouncer), not against any visual kind. The caller
//!   attaches a mind to whatever entity it wants moved (Monanisa's new bodies,
//!   the legacy husk, a grey box). Mapping `enemies::EnemyKind` → `Archetype`
//!   is one match line in the wiring code, which belongs to the combat lane.
//! - **Motion only, no damage** — a Strike is a committed lunge with a printed
//!   telegraph; deciding what a connect does is `combat.rs`'s call. This keeps
//!   the lane fence honest: behaviour proposes, combat disposes.
//!
//! Design contract per archetype (full rationale: docs/enemy-behavior.md):
//!
//! | Archetype | read | aggro | pursuit | attack |
//! |---|---|---|---|---|
//! | Swarm  (reaver)   | many, fast, erratic | wide, skittish | straight seek + zigzag, faster than the player | short windup, quick hit-and-hop |
//! | Bruiser (sentinel)| slow, never forgets | narrow, permanent | relentless slow advance, holds a threat distance | LONG telegraph, committed cleave-lunge |
//! | Pouncer(stalker)  | patient predator | widest | orbits at a stalk radius, circles, waits | crouch freeze → pounce dash, overshoots, re-orbits |
//!
//! Every archetype shares the same detect→alert beat (freeze + face the player
//! before moving — the "it noticed me" read `combat.rs` §4.1 established) and
//! the same honest-telegraph rule: the strike direction locks at the END of
//! the windup and cannot turn, so a dodge is real.
//!
//! The telegraph is also POSed, not just timed. A pose channel rides the same
//! system (see `pose_for`): alert flinch → windup coil that leans AWAY from the
//! prey and squashes → whip into a stretched lean-in strike → damped recover
//! wobble. It is a pure function of state + time-in-state — no RNG, no writes
//! to translation — so behaviour output is bit-identical with or without it;
//! only the body read changes.

use bevy::prelude::*;
use fastrand::Rng;

/// Marker for the thing the minds hunt. In the proof bin this sits on a
/// scripted dummy; when the combat lane wires this module into the real game
/// it inserts this component on the player entity and nothing else changes.
#[derive(Component)]
pub struct AiPlayer;

/// Seeded RNG resource — the proof clip must be reproducible, so no global
/// `fastrand::rng()` (unseeded, process-random) anywhere in this module.
#[derive(Resource)]
pub struct AiRng(pub Rng);

impl Default for AiRng {
    fn default() -> Self {
        // fixed seed = reproducible proof clips
        AiRng(Rng::with_seed(0x52505345)) // "RPSE" — Rose's proof seed
    }
}

/// Flat-arena bounds. The proof world is a plane; the real game replaces this
/// with `combat.rs`'s ground query at wiring time. Kept as a resource so the
/// clamp is one knob, not a hard-coded constant in three places.
#[derive(Resource)]
pub struct Arena {
    pub half: f32,
}
impl Default for Arena {
    fn default() -> Self {
        Arena { half: 28.0 }
    }
}

/// Behaviour class. Independent of `enemies::EnemyKind` on purpose — see the
/// module header. The intended mapping is Reaver→Swarm, Sentinel→Bruiser,
/// Stalker→Pouncer, but that line lives in wiring code, not here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Archetype {
    /// Fast, erratic, hits and hops away. Dangerous because there are many.
    Swarm,
    /// Slow, relentless, never loses aggro, hits like a truck after a long
    /// readable windup. Dangerous because it does not stop coming.
    Bruiser,
    /// Circles at a stalk radius, waits for the commit, then dashes through
    /// the target and re-orbits. Dangerous because it punishes standing still.
    Pouncer,
}

impl Archetype {
    pub fn id(self) -> &'static str {
        match self {
            Archetype::Swarm => "swarm",
            Archetype::Bruiser => "bruiser",
            Archetype::Pouncer => "pouncer",
        }
    }
}

/// One archetype's full tuning table. Everything the state machine consults
/// lives here so a balance pass is a table edit, not a logic hunt.
pub struct Tuning {
    pub patrol_speed: f32,
    pub aggro: f32,
    pub deaggro: f32, // Bruiser: INFINITY — it never forgets
    pub alert_beat: f32,
    pub chase_speed: f32,
    pub keep_dist: f32,   // Bruiser only: holds here while telegraphing
    pub strike_range: f32,
    pub windup: f32,      // the honest telegraph — 0 for Pouncer (uses crouch)
    pub strike_speed: f32,
    pub strike_time: f32,
    pub recover: f32,
    pub orbit_radius: f32, // Pouncer only
    pub orbit_speed: f32,  // Pouncer only
    pub crouch: f32,       // Pouncer only: pre-pounce freeze
    pub pounce_from: f32,  // Pouncer only: dist under which it stops circling
}

const TUNING: &[Tuning] = &[
    // Swarm — Reaver. Faster than the player's flee speed so a chase that
    // starts inside aggro always ends in contact; that is the archetype's
    // whole threat model.
    Tuning {
        patrol_speed: 1.2,
        aggro: 12.0,
        deaggro: 22.0,
        alert_beat: 0.40,
        chase_speed: 4.4,
        keep_dist: 0.0,
        strike_range: 1.5,
        windup: 0.25,
        strike_speed: 6.0,
        strike_time: 0.22,
        recover: 0.7,
        orbit_radius: 0.0,
        orbit_speed: 0.0,
        crouch: 0.0,
        pounce_from: 0.0,
    },
    // Bruiser — Sentinel. aggro 9 is tight (you must earn its attention) but
    // deaggro is infinite: once it has seen you, the slow advance does not
    // stop. windup 0.65s is the longest telegraph in the set — readable on
    // purpose; its counterplay is the dodge, and `combat.rs` already owns one.
    Tuning {
        patrol_speed: 0.8,
        aggro: 9.0,
        deaggro: f32::INFINITY,
        alert_beat: 0.55,
        chase_speed: 1.7,
        keep_dist: 2.1,
        strike_range: 2.5,
        windup: 0.65,
        strike_speed: 5.0,
        strike_time: 0.28,
        recover: 1.1,
        orbit_radius: 0.0,
        orbit_speed: 0.0,
        crouch: 0.0,
        pounce_from: 0.0,
    },
    // Pouncer — Stalker. Widest aggro (it is a predator, it watches), but the
    // approach is an orbit at 6.5, not a beeline. It commits only on the
    // crouch→dash: crouch 0.45s freeze (the readable beat), then a 9 u/s dash
    // that locks direction at the crouch's end and overshoots past the player
    // — landing behind you is the scariest place it can be.
    Tuning {
        patrol_speed: 1.4,
        aggro: 14.0,
        deaggro: 24.0,
        alert_beat: 0.45,
        chase_speed: 2.7, // orbit speed; the real approach is the dash
        keep_dist: 0.0,
        strike_range: 0.0,
        windup: 0.0,
        strike_speed: 9.0,
        strike_time: 0.35,
        recover: 1.2,
        orbit_radius: 6.5,
        orbit_speed: 2.7,
        crouch: 0.45,
        pounce_from: 5.5,
    },
];

pub fn tuning(a: Archetype) -> &'static Tuning {
    match a {
        Archetype::Swarm => &TUNING[0],
        Archetype::Bruiser => &TUNING[1],
        Archetype::Pouncer => &TUNING[2],
    }
}

// ---------------------------------------------------------------------------
// The pose channel — how a mind CARRIES itself while the state machine runs.
//
// Position + yaw alone made the telegraph unreadable: a windup looked like a
// enemy merely stopping. These curves give every beat a body read, the classic
// anticipation → strike → recovery arc:
//
//   Alert   — one-frame squash flinch, easing back out (the "it noticed" pop)
//   Windup  — coils AWAY from the target: leans back, squashes down and wide,
//             quadratic ease-in so the gathering ACCELERATES into the commit
//   Crouch  — drops into the coil fast and HOLDS (the held stillness is the
//             pouncer's whole tell)
//   Strike  — the whip: sweeps coil→full lean-in with a fast ease-out, so most
//             of the swing lands in the first third of the dash, then holds
//   Recover — damped settle that continues EXACTLY from the strike pose and
//             wobbles through neutral — committed mass reeling back
//
// Contract: a PURE function of (state, t-in-state, tuning table). No RNG, no
// reads of the world, no writes to translation — behaviour (what the CSV trace
// records) is bit-identical with or without this channel, so a proof A/B can
// diff the traces and see nothing but pose.
// ---------------------------------------------------------------------------

/// Per-archetype pose amplitudes, parallel to `TUNING` and indexed the same
/// way. Angles in radians; scales are multipliers on the root transform.
struct PoseTuning {
    /// windup/crouch: how far the coil leans BACK (rearing away from prey)
    lean_back: f32,
    /// strike: how far the whip leans IN (committed weight into the lunge)
    lean_in: f32,
    /// y-scale at full coil (squash) and full dash (stretch)
    coil_sy: f32,
    dash_sy: f32,
    /// xz-scale at full coil (widens as it compresses) and full dash
    coil_sxz: f32,
    dash_sxz: f32,
}

const POSE: &[PoseTuning] = &[
    // Swarm — quick and light: small coil, snappy hop of a hit
    PoseTuning {
        lean_back: 0.10,
        lean_in: 0.19,
        coil_sy: 0.92,
        dash_sy: 1.07,
        coil_sxz: 1.04,
        dash_sxz: 0.96,
    },
    // Bruiser — the heavy read is the point: deepest coil, longest lean,
    // loudest landing wobble
    PoseTuning {
        lean_back: 0.22,
        lean_in: 0.28,
        coil_sy: 0.86,
        dash_sy: 1.12,
        coil_sxz: 1.08,
        dash_sxz: 0.94,
    },
    // Pouncer — the arrow: modest coil, biggest lean-in and stretch
    PoseTuning {
        lean_back: 0.15,
        lean_in: 0.34,
        coil_sy: 0.78,
        dash_sy: 1.16,
        coil_sxz: 1.10,
        dash_sxz: 0.92,
    },
];

fn pose_tuning(a: Archetype) -> &'static PoseTuning {
    match a {
        Archetype::Swarm => &POSE[0],
        Archetype::Bruiser => &POSE[1],
        Archetype::Pouncer => &POSE[2],
    }
}

/// One frame's pose. `pitch` is body-local around +X after the yaw: positive
/// tips the forward (−Z) axis upward = leaning back; negative noses down into
/// the strike.
struct Pose {
    pitch: f32,
    sy: f32,
    sxz: f32,
}

impl Pose {
    const NEUTRAL: Pose = Pose {
        pitch: 0.0,
        sy: 1.0,
        sxz: 1.0,
    };
}

fn pose_for(mind: &EnemyMind, t: &Tuning) -> Pose {
    let p = pose_tuning(mind.archetype);
    match mind.state {
        // free movement carries itself neutral — the pose only speaks on the
        // beats, so an interruption reads as an interruption
        AiState::Patrol | AiState::Chase | AiState::Advance | AiState::Stalk => Pose::NEUTRAL,

        AiState::Alert => {
            // startle: head snaps up + squash in one frame, eases back out
            let k = (mind.t / t.alert_beat.max(1e-4)).min(1.0);
            let dip = (1.0 - k) * (1.0 - k);
            Pose {
                pitch: 0.06 * dip,
                sy: 1.0 - 0.05 * dip,
                sxz: 1.0 + 0.03 * dip,
            }
        }

        AiState::Windup => {
            // anticipation: quadratic ease-in, the coil tightens right up to
            // the commit (continuous with the neutral it comes from)
            let k = (mind.t / t.windup.max(1e-4)).min(1.0);
            let e = k * k;
            Pose {
                pitch: p.lean_back * e,
                sy: 1.0 + (p.coil_sy - 1.0) * e,
                sxz: 1.0 + (p.coil_sxz - 1.0) * e,
            }
        }

        AiState::Crouch => {
            // the freeze drops into the coil fast (ease-out) and holds it
            let k = (mind.t / t.crouch.max(1e-4)).min(1.0);
            let e = 1.0 - (1.0 - k) * (1.0 - k);
            Pose {
                pitch: p.lean_back * e,
                sy: 1.0 + (p.coil_sy - 1.0) * e,
                sxz: 1.0 + (p.coil_sxz - 1.0) * e,
            }
        }

        AiState::Strike | AiState::Pounce => {
            // the whip: coil extremes → lean-in extremes, ease-out so the
            // swing front-loads. Continuous with the coil at k=0 (values
            // match; VELOCITY is the snap, which is the point)
            let k = (mind.t / t.strike_time.max(1e-4)).min(1.0);
            let e = 1.0 - (1.0 - k) * (1.0 - k);
            Pose {
                pitch: p.lean_back + (-p.lean_in - p.lean_back) * e,
                sy: p.coil_sy + (p.dash_sy - p.coil_sy) * e,
                sxz: p.coil_sxz + (p.dash_sxz - p.coil_sxz) * e,
            }
        }

        AiState::Recover => {
            // damped settle: env·cos starts at EXACTLY the strike-end pose and
            // wobbles through neutral — the weight of the commit reeling back
            let k = (mind.t / t.recover.max(1e-4)).min(1.0);
            let env = (-4.5 * k).exp();
            let w = (9.0 * k).cos();
            Pose {
                pitch: -p.lean_in * env * w,
                sy: 1.0 + (p.dash_sy - 1.0) * env * w,
                sxz: 1.0 + (p.dash_sxz - 1.0) * env * w,
            }
        }
    }
}

/// The mind's states. `Windup`/`Strike` belong to Swarm & Bruiser; `Crouch`/
/// `Pounce` are the Pouncer's pair. All of them print on transition — the
/// proof harness greps those lines to verify the machine actually moved
/// through its beats instead of sliding one averaged blob at the player.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AiState {
    Patrol,
    Alert,
    Chase,
    Advance,
    Stalk,
    Windup,
    Crouch,
    Strike,
    Pounce,
    Recover,
}

impl AiState {
    pub fn id(self) -> &'static str {
        match self {
            AiState::Patrol => "patrol",
            AiState::Alert => "alert",
            AiState::Chase => "chase",
            AiState::Advance => "advance",
            AiState::Stalk => "stalk",
            AiState::Windup => "windup",
            AiState::Crouch => "crouch",
            AiState::Strike => "strike",
            AiState::Pounce => "pounce",
            AiState::Recover => "recover",
        }
    }
}

/// Attach to any entity to give it a mind. In the proof bin this lands on the
/// roots `enemies::spawn_enemy` returns; nothing in this module knows that.
#[derive(Component)]
pub struct EnemyMind {
    pub id: u32,
    pub archetype: Archetype,
    pub state: AiState,
    /// time in the current state
    pub t: f32,
    /// current yaw (facing −Z at 0 — same convention as the bodies)
    pub facing: f32,
    /// patrol wander heading + how long until it re-rolls
    wander_yaw: f32,
    wander_left: f32,
    /// Pouncer orbit direction (+1/−1) and how long until it flips
    pub orbit_dir: f32,
    orbit_left: f32,
    /// stalk timer before the Pouncer commits even at range
    stalk_left: f32,
    /// strike direction, locked at the end of windup/crouch — never re-aims
    locked_dir: Vec3,
    /// home point for patrol: it wanders around where it was attached
    home: Vec3,
}

impl EnemyMind {
    pub fn new(id: u32, archetype: Archetype, at: Vec3) -> Self {
        EnemyMind {
            id,
            archetype,
            state: AiState::Patrol,
            t: 0.0,
            facing: 0.0,
            wander_yaw: 0.0,
            wander_left: 0.0,
            orbit_dir: if id % 2 == 0 { 1.0 } else { -1.0 },
            orbit_left: 0.0,
            stalk_left: 0.0,
            locked_dir: Vec3::Z,
            home: at,
        }
    }

    fn go(&mut self, s: AiState, dist: f32) {
        if self.state == s {
            return;
        }
        println!(
            "AI id={} {} {} -> {} dist={:.2}",
            self.id,
            self.archetype.id(),
            self.state.id(),
            s.id(),
            dist
        );
        self.state = s;
        self.t = 0.0;
    }
}

fn yaw_to(dir: Vec3) -> f32 {
    // facing −Z at yaw 0 (body convention): forward = (−sin y, 0, −cos y)
    (-dir.x).atan2(-dir.z)
}

fn turn_toward(cur: f32, target: f32, max_step: f32) -> f32 {
    let mut d = (target - cur) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    cur + d.clamp(-max_step, max_step)
}

/// The whole behaviour, one system, one pass over `iter_mut()`. Peer positions
/// for separation are snapshotted before any writes this frame (an O(n²)
/// separation over single-digit enemy counts; a spatial grid waits until a
/// map needs one).
pub fn enemy_ai(
    mut minds: Query<(Entity, &mut Transform, &mut EnemyMind)>,
    player: Query<&Transform, (With<AiPlayer>, Without<EnemyMind>)>,
    mut rng: ResMut<AiRng>,
    arena: Res<Arena>,
    time: Res<Time>,
) {
    let Ok(ptf) = player.single() else { return };
    let ptf = ptf.translation;
    let dt = time.delta_secs();
    let half = arena.half;
    let positions: Vec<Vec3> = minds.iter().map(|(_, tf, _)| tf.translation).collect();

    for (idx, (_entity, mut tf, mut mind)) in minds.iter_mut().enumerate() {
        let t = tuning(mind.archetype);
        let pos = tf.translation;
        let mut to_p = ptf - pos;
        to_p.y = 0.0;
        let dist = to_p.length();
        let dir_p = if dist > 1e-4 { to_p / dist } else { Vec3::NEG_Z };

        // ---- decide: desired direction + speed + facing for this frame ----
        let mut desired = Vec3::ZERO;
        let mut speed = 0.0;
        let mut want_yaw = mind.facing;

        match mind.state {
            AiState::Patrol => {
                if dist < t.aggro {
                    mind.go(AiState::Alert, dist);
                    want_yaw = yaw_to(dir_p);
                } else {
                    // wander around home, gently pulled back if it drifts
                    if mind.wander_left <= 0.0 {
                        mind.wander_yaw = rng.0.f32() * std::f32::consts::TAU;
                        mind.wander_left = 1.5 + rng.0.f32() * 2.5;
                    }
                    let dir = Vec3::new(-mind.wander_yaw.sin(), 0.0, -mind.wander_yaw.cos());
                    let home_pull = (mind.home - pos) * 0.08;
                    let v = dir * t.patrol_speed + home_pull.with_y(0.0);
                    speed = v.length().min(t.patrol_speed);
                    desired = v;
                    want_yaw = yaw_to(dir);
                }
            }
            AiState::Alert => {
                // the detect beat: freeze and FACE the player — motionless on
                // purpose, the stillness is the tell
                want_yaw = yaw_to(dir_p);
                if mind.t >= t.alert_beat {
                    let next = match mind.archetype {
                        Archetype::Swarm => AiState::Chase,
                        Archetype::Bruiser => AiState::Advance,
                        Archetype::Pouncer => {
                            mind.stalk_left = 4.0 + rng.0.f32() * 3.0;
                            AiState::Stalk
                        }
                    };
                    mind.go(next, dist);
                }
            }
            AiState::Chase => {
                if dist > t.deaggro {
                    mind.go(AiState::Patrol, dist);
                } else if dist < t.strike_range {
                    mind.go(AiState::Windup, dist);
                } else {
                    // zigzag: lateral weave so a swarm never reads as one rail
                    let lat = Vec3::new(-dir_p.z, 0.0, dir_p.x);
                    let weave = (time.elapsed_secs() * 3.1 + mind.id as f32 * 2.4).sin() * 1.15;
                    desired = (dir_p + lat * weave).normalize();
                    speed = t.chase_speed;
                    want_yaw = yaw_to(dir_p);
                }
            }
            AiState::Advance => {
                // Bruiser: deaggro is INFINITY — no give-up branch exists
                if dist > t.strike_range {
                    desired = dir_p;
                    speed = if dist < t.keep_dist { 0.0 } else { t.chase_speed };
                    want_yaw = yaw_to(dir_p);
                } else {
                    mind.go(AiState::Windup, dist);
                }
            }
            AiState::Stalk => {
                // orbit at orbit_radius; radial correction keeps the ring,
                // tangential motion keeps the circle
                let radial = if dist > 1e-4 { (dist - t.orbit_radius) / t.orbit_radius } else { 0.0 };
                let tang = Vec3::new(-dir_p.z, 0.0, dir_p.x) * mind.orbit_dir;
                desired = (dir_p * radial.clamp(-1.0, 1.0) + tang).normalize();
                speed = t.orbit_speed;
                want_yaw = yaw_to(dir_p);
                if mind.orbit_left <= 0.0 {
                    mind.orbit_dir *= -1.0;
                    mind.orbit_left = 2.0 + rng.0.f32() * 2.0;
                }
                if dist < t.pounce_from || mind.stalk_left <= 0.0 {
                    mind.go(AiState::Crouch, dist);
                }
            }
            AiState::Windup | AiState::Crouch => {
                // telegraph: stand still, track the player… until the windup
                // ends, then LOCK the strike direction — after the lock the
                // lunge cannot turn, so a dodge is real
                want_yaw = yaw_to(dir_p);
                let wind = if mind.state == AiState::Windup { t.windup } else { t.crouch };
                if mind.t >= wind {
                    mind.locked_dir = dir_p; // aims LATE — dodging early is safe
                    let next = if mind.state == AiState::Windup { AiState::Strike } else { AiState::Pounce };
                    println!(
                        "AI id={} {} TELEGRAPH->COMMIT locked=({:.2},{:.2}) dist={:.2}",
                        mind.id,
                        mind.archetype.id(),
                        mind.locked_dir.x,
                        mind.locked_dir.z,
                        dist
                    );
                    mind.go(next, dist);
                }
            }
            AiState::Strike | AiState::Pounce => {
                // committed: locked direction, no re-aim, overshoots on purpose
                desired = mind.locked_dir;
                speed = t.strike_speed;
                want_yaw = yaw_to(mind.locked_dir);
                if mind.t >= t.strike_time {
                    mind.go(AiState::Recover, dist);
                }
            }
            AiState::Recover => {
                // the opening: slow, dazed, face where the player went
                want_yaw = yaw_to(dir_p);
                if mind.t >= t.recover {
                    let next = match mind.archetype {
                        Archetype::Swarm => AiState::Chase,
                        Archetype::Bruiser => AiState::Advance,
                        Archetype::Pouncer => {
                            mind.stalk_left = 4.0 + rng.0.f32() * 3.0;
                            AiState::Stalk
                        }
                    };
                    mind.go(next, dist);
                }
            }
        }

        // ---- integrate: separation, clamp, facing, timers ----
        for (j, other) in positions.iter().enumerate() {
            if j == idx {
                continue;
            }
            let mut away = pos - *other;
            away.y = 0.0;
            let d = away.length();
            if d < 1.3 && d > 1e-4 {
                desired += (away / d) * (1.3 - d) * 2.0;
            }
        }

        if speed > 0.0 && desired.length_squared() > 1e-6 {
            let step = desired.normalize() * speed * dt;
            tf.translation += step;
        }
        tf.translation.x = tf.translation.x.clamp(-half, half);
        tf.translation.z = tf.translation.z.clamp(-half, half);
        tf.translation.y = 0.0; // flat-arena contract; combat wiring owns real ground

        mind.facing = turn_toward(mind.facing, want_yaw, 10.0 * dt);
        let pose = pose_for(&mind, t);
        tf.rotation = Quat::from_axis_angle(Vec3::Y, mind.facing)
            * Quat::from_axis_angle(Vec3::X, pose.pitch);
        tf.scale = Vec3::new(pose.sxz, pose.sy, pose.sxz);

        mind.t += dt;
        if mind.wander_left > 0.0 {
            mind.wander_left -= dt;
        }
        if mind.orbit_left > 0.0 {
            mind.orbit_left -= dt;
        }
        if mind.stalk_left > 0.0 {
            mind.stalk_left -= dt;
        }
    }
}

/// Ordering label for the AI pass. Callers that need to run before/after the
/// minds move order against THIS set, never against `enemy_ai` the fn: a
/// `SystemTypeSet` goes ambiguous the moment a schedule holds more than one
/// instance of the system (the proof bin panicked on exactly that,
/// bevy_ecs schedule.rs:566), while a named set never does.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnemyAiSet;

/// Wiring-ready plugin. The combat lane's eventual integration is:
/// `app.add_plugins(EnemyAiPlugin)`, insert `AiPlayer` on the player entity,
/// and `attach_mind` on each spawn it wants driven; neighbours order against
/// `EnemyAiSet`. No edits to this file.
pub struct EnemyAiPlugin;

impl Plugin for EnemyAiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Arena::default())
            .init_resource::<AiRng>()
            .add_systems(Update, enemy_ai.in_set(EnemyAiSet));
    }
}

/// Convenience for wiring code: attach a mind to an existing body root.
pub fn attach_mind(commands: &mut Commands, entity: Entity, id: u32, archetype: Archetype, at: Vec3) {
    commands.entity(entity).insert(EnemyMind::new(id, archetype, at));
}
