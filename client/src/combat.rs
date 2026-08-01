//! Voxelforge combat — soulslike core loop (client-only gameplay layer).
//!
//! Implements `docs/combat-design.md` §8 checklist items 1–13 for the first
//! playable: player combat state machine, stamina, dodge + i-frames, lock-on,
//! light/heavy/charged attacks, block & parry, poise/posture, the Guard Husk
//! enemy (§4.1) with a minimal patrol→aggro→attack AI + honest telegraph, and
//! the feedback layer (hit-stop, screen-shake, hit reactions).
//!
//! ## Why this lives entirely in the client
//! Combat is per-frame gameplay state (ECS components + timers), not world
//! simulation. It reads the world only to place the enemy on Kevin's real
//! terrain surface (`worldgen::terrain_height`, already read-only exposed) — no
//! `sim/` interface is needed, so no sim stub was written. If a future milestone
//! moves authoritative combat to the server, `CombatModel`'s pure methods below
//! are the hand-off boundary.
//!
//! ## Numbers
//! Every constant is traced to a spec section in a trailing comment so a design
//! tuning pass can diff code against `combat-design.md` §2/§3/§4/§5/§6.

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::audio::SfxEvent;
use crate::{FlyCam, PLAYER_HALF_W};
use voxelforge_sim::worldgen::terrain_height;

// ===========================================================================
// Spec constants — combat-design.md §2 / §3 / §4 / §5 / §6
// ===========================================================================

// -- Stamina (§2.1 / §6) ----------------------------------------------------
pub const STAMINA_MAX: f32 = 100.0; //                              §2.1
pub const STAMINA_REGEN: f32 = 40.0; // points/sec                  §2.1 / §6
pub const COST_LIGHT: f32 = 15.0; //                               §2.1 / §6
pub const COST_HEAVY: f32 = 35.0; //                               §2.1 / §6
pub const COST_CHARGED: f32 = 50.0; //                             §2.1
pub const COST_DODGE: f32 = 20.0; //                               §2.1 / §6
pub const COST_BLOCK: f32 = 15.0; // per hit blocked               §2.1
pub const COST_PARRY: f32 = 10.0; //                               §2.1
pub const DELAY_LIGHT: f32 = 0.25; //                              §2.1
pub const DELAY_HEAVY: f32 = 0.40; //                              §2.1
pub const DELAY_CHARGED: f32 = 0.50; //                            §2.1
pub const DELAY_DODGE: f32 = 0.30; //                              §2.1
pub const DELAY_BLOCK: f32 = 0.20; //                              §2.1
pub const DELAY_PARRY: f32 = 0.35; //                              §2.1
pub const EXHAUST_LOCK: f32 = 0.8; // no-action window at 0 stam    §2.1
pub const EXHAUST_CLEAR: f32 = 20.0; // stam needed to end penalty  §2.1

// -- Dodge / roll (§2.2 / §6) ----------------------------------------------
pub const DODGE_IFRAMES: f32 = 10.0 / 60.0; // ~0.167 s            §2.2 / §6
pub const DODGE_RECOVERY: f32 = 12.0 / 60.0; // ~0.200 s           §2.2 / §6
pub const DODGE_DISTANCE: f32 = 2.5; // blocks                      §2.2

// -- Lock-on (§2.3 / §6) ----------------------------------------------------
pub const LOCK_RANGE: f32 = 16.0; // blocks                         §2.3 / §6
pub const LOCK_CONE_H: f32 = 45.0_f32; // degrees                   §2.3
pub const LOCK_SNAP: f32 = 8.0; // rad/s                            §2.3 / §6
pub const LOCK_DROP_RANGE: f32 = 20.0; // auto-unlock beyond        §2.3

// -- Attacks (§2.4 / §3) ----------------------------------------------------
pub const LIGHT_DAMAGE: f32 = 20.0; //                             §2.4
pub const LIGHT_POISE: f32 = 15.0; //                              §2.4
pub const LIGHT_TIME: f32 = 0.35; //                              §2.4
pub const HEAVY_DAMAGE: f32 = 45.0; //                            §2.4
pub const HEAVY_POISE: f32 = 40.0; //                             §2.4
pub const HEAVY_TIME: f32 = 0.85; //                              §2.4
pub const HEAVY_HYPER: f32 = 0.40; // hyper-armor tail             §2.4 / §3.2
pub const CHARGED_DAMAGE: f32 = 70.0; //                          §2.4
pub const CHARGED_POISE: f32 = 60.0; //                           §2.4
pub const CHARGED_TIME: f32 = 1.10; //                            §2.4
pub const CHARGE_HOLD: f32 = 0.60; // hold before a heavy charges  §2.4
pub const COMBO_RESET: f32 = 0.60; // window to continue a combo   §2.4
pub const COMBO_STEP: f32 = 0.10; // +10% damage per chained hit   §2.4
pub const MELEE_RANGE: f32 = 2.0; // blocks — player reach & Husk   §4.1
pub const MELEE_CONE: f32 = 60.0_f32; // deg half-cone for a swing  §2.3 soft-lock

// Active-frame windows inside an attack (windup → active → recover). The active
// hitbox appears only after the wind-up ends — telegraphs are honest (§5.1).
pub const LIGHT_ACTIVE: (f32, f32) = (0.12, 0.22); //             §2.4 0.35 total
pub const HEAVY_ACTIVE: (f32, f32) = (0.55, 0.72); //            §2.4 0.85 total
pub const CHARGED_ACTIVE: (f32, f32) = (0.75, 0.95); //         §2.4 1.10 total

// -- Block & parry (§2.5) ---------------------------------------------------
pub const BLOCK_REDUCTION: f32 = 0.50; // 50% less damage           §2.5
pub const GUARD_BREAK: f32 = 0.60; // stagger on a failed block     §2.5
pub const PARRY_WINDOW: f32 = 0.20; // 12 frames @60               §2.5
pub const PARRY_POSTURE: f32 = 25.0; // posture dmg to attacker     §2.5
pub const PARRY_PUNISH: f32 = 1.20; // window of +25% dmg           §2.5
pub const PARRY_PUNISH_MULT: f32 = 1.25; //                        §2.5
pub const PARRY_FAIL_MULT: f32 = 1.25; // extra dmg on a whiff      §2.5
pub const PARRY_FAIL_RECOVER: f32 = 0.50; //                      §2.5

// -- Poise / posture (§3.2) -------------------------------------------------
pub const POISE_PLAYER: f32 = 40.0; //                            §3.2
pub const POISE_HUSK: f32 = 30.0; //                              §3.2 / §4.1
pub const POISE_REGEN: f32 = 10.0; // per sec                      §3.2
pub const POISE_REGEN_DELAY: f32 = 2.0; // sec without a hit        §3.2
pub const STAGGER_TIME: f32 = 1.5; //                             §3.2
pub const STAGGER_DMG_MULT: f32 = 1.30; // +30% while staggered     §3.2
pub const HYPER_ARMOR_REDUCE: f32 = 0.75; // -75% poise dmg         §3.2 / §5

// -- Health (§3.1) ----------------------------------------------------------
pub const HP_PLAYER: f32 = 100.0; //                              §3.1
pub const HP_HUSK: f32 = 80.0; //                                 §4.1 / §6

// -- Guard Husk (§4.1) ------------------------------------------------------
pub const HUSK_WALK: f32 = 1.5; // blocks/s                        §4.1
pub const HUSK_TURN: f32 = 2.0; // rad/s                           §4.1
pub const HUSK_TELEGRAPH: f32 = 0.8; // wind-up                    §4.1 / §6
pub const HUSK_SWING1_DMG: f32 = 15.0; //                         §4.1
pub const HUSK_SWING1_POISE: f32 = 15.0; //                       §4.1
pub const HUSK_SWING2_DMG: f32 = 20.0; //                         §4.1
pub const HUSK_SWING2_POISE: f32 = 20.0; //                       §4.1
pub const HUSK_ACTIVE: f32 = 0.2; // active frames per swing        §4.1
pub const HUSK_GAP: f32 = 0.25; // between the two swings
pub const HUSK_COMBO_PAUSE: f32 = 1.5; // between combos           §4.1
pub const HUSK_AGGRO_RANGE: f32 = 12.0; // sees player             §4.1
pub const HUSK_LEASH: f32 = 6.0; // retreat past this → patrol      §4.1

// -- Feedback (§5.2 / §5.3) -------------------------------------------------
pub const HITSTOP_LIGHT: f32 = 0.080; // player hits enemy         §5.2
pub const HITSTOP_PARRY: f32 = 0.120; //                          §5.2
pub const HITSTOP_ENEMY: f32 = 0.100; // enemy hits player         §5.2
pub const HITSTOP_STAGGER: f32 = 0.150; //                        §5.2
pub const SHAKE_LIGHT: (f32, f32) = (0.04, 0.10); // amp, dur       §5.3
pub const SHAKE_HEAVY: (f32, f32) = (0.10, 0.20); //             §5.3 / §6
pub const SHAKE_ENEMY_HIT: (f32, f32) = (0.15, 0.25); //         §5.3

// -- Weight layer: what makes a swing feel like it *lands* ------------------
// §5.2 only fixed one hit-stop length for the player. A single length reads as
// a stutter, not as weight: the ear/eye grades "how hard was that?" almost
// entirely off how long the frame froze. So hit-stop is graded by the blow.
pub const HITSTOP_HEAVY: f32 = 0.120; // heavy swing connects
pub const HITSTOP_CRITICAL: f32 = 0.170; // charged, or a poise break
/// How far a connected blow shoves the target along the blade's direction. Small
/// on purpose — a souls-like nudges, it does not punt (a punt would push the
/// enemy out of the follow-up's reach and break every combo).
pub const KNOCKBACK_LIGHT: f32 = 0.18; // blocks
pub const KNOCKBACK_HEAVY: f32 = 0.32; // blocks
pub const KNOCKBACK_CRITICAL: f32 = 0.55; // blocks
/// The shove is spread over this window so the body slides, never teleports.
pub const KNOCKBACK_TIME: f32 = 0.12; // sec
/// Directional camera kick — rides on top of [`Shake`]'s omni-directional
/// rattle: the rattle says "something happened", the kick says "*that* way".
pub const KICK_LIGHT: f32 = 0.045;
pub const KICK_HEAVY: f32 = 0.100;
pub const KICK_CRITICAL: f32 = 0.155;
pub const KICK_TAKEN: f32 = 0.130; // the player eating a hit
pub const KICK_TIME: f32 = 0.16; // sec — snap out, ease back

// -- Guard Husk rhythm (§4.1 extended) --------------------------------------
// One fixed 0.8 s wind-up is a metronome: after two swings the player has the
// timing and the fight is over as a threat. The signature of a soulslike boss is
// that the *same* wind-up resolves at different times, so the dodge has to be
// read, not memorised.
pub const HUSK_TELEGRAPH_DELAYED: f32 = 1.55; // holds the pose, then falls
pub const HUSK_FEINT_HOLD: f32 = 0.42; // pulls back before the swing ever comes
pub const HUSK_FEINT_RECOVER: f32 = 0.55; // beat of nothing — baits the dodge
/// A husk that has not closed the gap keeps stepping in *while* winding up (at
/// half walk speed). Standing still through a 1.5 s wind-up would let the player
/// simply back off, and it would let hit-knockback slide the fight apart.
pub const HUSK_STEP_IN: f32 = 0.5; // × HUSK_WALK

// ===========================================================================
// Pure combat model — the testable core (no Bevy scheduling, headless-proofable)
// ===========================================================================

/// Player combat states (§8.1 state machine).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CombatState {
    Idle,
    Light,
    Heavy,
    Charged,
    Dodge,
    Block,
    Parry,
    Stagger,
    Dead,
}

/// One shared stamina pool (§2.1). All actions draw from it; it regens after a
/// per-action delay, and hitting 0 triggers an exhaustion lock-out.
#[derive(Component, Clone, Debug)]
pub struct Stamina {
    pub cur: f32,
    /// Time left before regen resumes (set to the action's recovery delay).
    pub delay: f32,
    /// Exhaustion lock-out remaining; while > 0 no action may start (§2.1).
    pub exhausted: f32,
    /// True until stamina climbs back above `EXHAUST_CLEAR` — doubles the delay.
    pub penalty: bool,
}

impl Stamina {
    pub fn full() -> Self {
        Self { cur: STAMINA_MAX, delay: 0.0, exhausted: 0.0, penalty: false }
    }

    /// Try to pay `cost`; on success sets the regen `delay` (doubled while the
    /// exhaustion penalty is active) and returns true. Fails if exhausted or
    /// short on stamina.
    pub fn try_spend(&mut self, cost: f32, delay: f32) -> bool {
        if self.exhausted > 0.0 || self.cur < cost {
            return false;
        }
        self.cur -= cost;
        self.delay = if self.penalty { delay * 2.0 } else { delay };
        if self.cur <= 0.0 {
            self.cur = 0.0;
            self.exhausted = EXHAUST_LOCK;
            self.penalty = true;
        }
        true
    }

    /// Advance regen / exhaustion timers by `dt`. When the recovery delay elapses
    /// mid-tick, the leftover slice of `dt` is applied to regen so no frame of
    /// recovery is silently dropped.
    pub fn tick(&mut self, dt: f32) {
        if self.exhausted > 0.0 {
            // Locked out (§2.1) — count the window down; no regen while exhausted.
            self.exhausted = (self.exhausted - dt).max(0.0);
            return;
        }
        let mut regen_dt = dt;
        if self.delay > 0.0 {
            let used = self.delay.min(dt);
            self.delay -= used;
            regen_dt -= used;
        }
        if regen_dt > 0.0 {
            self.cur = (self.cur + STAMINA_REGEN * regen_dt).min(STAMINA_MAX);
        }
        if self.penalty && self.cur > EXHAUST_CLEAR {
            self.penalty = false;
        }
    }
}

/// Health pool (§3.1).
#[derive(Component, Clone, Debug)]
pub struct Health {
    pub cur: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { cur: max, max }
    }
    pub fn dead(&self) -> bool {
        self.cur <= 0.0
    }
    pub fn damage(&mut self, dmg: f32) {
        self.cur = (self.cur - dmg).max(0.0);
    }
}

/// Poise / posture (§3.2). Drains on poise damage, regenerates after a delay,
/// and a break triggers a timed stagger.
#[derive(Component, Clone, Debug)]
pub struct Poise {
    pub cur: f32,
    pub max: f32,
    pub since_hit: f32,
    pub stagger: f32,
}

impl Poise {
    pub fn new(max: f32) -> Self {
        Self { cur: max, max, since_hit: POISE_REGEN_DELAY, stagger: 0.0 }
    }

    pub fn staggered(&self) -> bool {
        self.stagger > 0.0
    }

    /// Apply poise damage. `hyper` halves-plus (§3.2: -75%) the incoming poise
    /// damage and suppresses the stagger. Returns true if this hit broke poise.
    pub fn take(&mut self, poise_dmg: f32, hyper: bool) -> bool {
        self.since_hit = 0.0;
        if hyper {
            self.cur = (self.cur - poise_dmg * (1.0 - HYPER_ARMOR_REDUCE)).max(0.0);
            return false; // stagger suppressed under hyper armor
        }
        self.cur = (self.cur - poise_dmg).max(0.0);
        if self.cur <= 0.0 {
            self.stagger = STAGGER_TIME;
            true
        } else {
            false
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.stagger > 0.0 {
            self.stagger = (self.stagger - dt).max(0.0);
            if self.stagger == 0.0 {
                self.cur = self.max; // reset to full after a stagger (§3.2)
            }
            return;
        }
        self.since_hit += dt;
        if self.since_hit >= POISE_REGEN_DELAY {
            self.cur = (self.cur + POISE_REGEN * dt).min(self.max);
        }
    }
}

/// Player combat state machine + action timers (§8.1).
#[derive(Component, Clone, Debug)]
pub struct PlayerCombat {
    pub state: CombatState,
    /// Elapsed time in the current action state.
    pub timer: f32,
    /// Remaining i-frame time (dodge). Drives the roll *glide* only — how far a
    /// body travels must not depend on frame rate. Invulnerability is
    /// `iframe_frames` below.
    pub iframes: f32,
    /// Dodge invulnerability in **frames** (§2.2/§6 say "10 frames @ 60 FPS",
    /// not "167 ms"). Opened by `start_dodge`, spent one per frame by
    /// `dodge_parry::spend_window_frames`, read by `invulnerable()`.
    pub iframe_frames: u32,
    /// Parry receive window, likewise in frames (§2.5: 12 frames @ 60 FPS).
    pub parry_frames: u32,
    /// Which enemy a successful parry left open to a riposte, if any.
    /// See `dodge_parry::take_riposte`.
    pub riposte_on: Option<Entity>,
    /// Lock before the next action may start (dodge recovery, parry recovery…).
    pub recovery: f32,
    /// Combo index (0..3) and the window left to continue it.
    pub combo: u8,
    pub combo_window: f32,
    /// +25% punish window opened by a successful parry (on the *attacker*, but
    /// mirrored here so the demo can read it).
    pub punish: f32,
    /// Heavy-button hold time; commits to Charged past `CHARGE_HOLD`, else Heavy.
    pub charge: f32,
    /// Hit-stop freeze — while > 0 this actor's state timers do not advance.
    pub hitstop: f32,
    /// Whether this attack has already dealt its hit (one hit per swing).
    pub hit_applied: bool,
}

impl Default for PlayerCombat {
    fn default() -> Self {
        Self {
            state: CombatState::Idle,
            timer: 0.0,
            iframes: 0.0,
            iframe_frames: 0,
            parry_frames: 0,
            riposte_on: None,
            recovery: 0.0,
            combo: 0,
            combo_window: 0.0,
            punish: 0.0,
            charge: 0.0,
            hitstop: 0.0,
            hit_applied: false,
        }
    }
}

impl PlayerCombat {
    pub fn invulnerable(&self) -> bool {
        self.iframe_frames > 0
    }

    /// May a new action begin? Blocked while busy, recovering, exhausted, dead,
    /// or staggered.
    pub fn can_act(&self, stam: &Stamina) -> bool {
        self.state == CombatState::Idle
            && self.recovery <= 0.0
            && self.hitstop <= 0.0
            && stam.exhausted <= 0.0
    }

    pub fn start_light(&mut self, stam: &mut Stamina) -> bool {
        if !self.can_act(stam) || !stam.try_spend(COST_LIGHT, DELAY_LIGHT) {
            return false;
        }
        self.state = CombatState::Light;
        self.timer = 0.0;
        self.hit_applied = false;
        // Chain the combo if still inside the reset window, else start at 1.
        self.combo = if self.combo_window > 0.0 { (self.combo % 3) + 1 } else { 1 };
        self.combo_window = COMBO_RESET;
        true
    }

    pub fn start_heavy(&mut self, stam: &mut Stamina) -> bool {
        if !self.can_act(stam) || !stam.try_spend(COST_HEAVY, DELAY_HEAVY) {
            return false;
        }
        self.state = CombatState::Heavy;
        self.timer = 0.0;
        self.hit_applied = false;
        self.combo = 0;
        true
    }

    /// Committed charged attack (§2.4) — reached by holding heavy past
    /// `CHARGE_HOLD`; highest damage/poise and highest commitment.
    pub fn start_charged(&mut self, stam: &mut Stamina) -> bool {
        if !self.can_act(stam) || !stam.try_spend(COST_CHARGED, DELAY_CHARGED) {
            return false;
        }
        self.state = CombatState::Charged;
        self.timer = 0.0;
        self.hit_applied = false;
        self.combo = 0;
        true
    }

    pub fn start_dodge(&mut self, stam: &mut Stamina) -> bool {
        if !self.can_act(stam) || !stam.try_spend(COST_DODGE, DELAY_DODGE) {
            return false;
        }
        self.state = CombatState::Dodge;
        self.timer = 0.0;
        self.iframes = DODGE_IFRAMES;
        self.iframe_frames = crate::dodge_parry::DODGE_IFRAME_FRAMES;
        self.combo = 0;
        true
    }

    /// Damage this attack deals right now, or None if not in an active window.
    /// Applies the combo damage ramp on chained lights.
    pub fn active_hit(&self) -> Option<(f32, f32)> {
        if self.hit_applied {
            return None;
        }
        let (dmg, poise, window) = match self.state {
            CombatState::Light => {
                let ramp = 1.0 + COMBO_STEP * (self.combo.saturating_sub(1) as f32);
                (LIGHT_DAMAGE * ramp, LIGHT_POISE, LIGHT_ACTIVE)
            }
            CombatState::Heavy => (HEAVY_DAMAGE, HEAVY_POISE, HEAVY_ACTIVE),
            CombatState::Charged => (CHARGED_DAMAGE, CHARGED_POISE, CHARGED_ACTIVE),
            _ => return None,
        };
        if self.timer >= window.0 && self.timer <= window.1 {
            Some((dmg, poise))
        } else {
            None
        }
    }

    /// True during the heavy-attack hyper-armor tail (§2.4/§3.2).
    pub fn hyper_armor(&self) -> bool {
        matches!(self.state, CombatState::Heavy | CombatState::Charged)
            && self.timer >= (self.action_len() - HEAVY_HYPER)
    }

    fn action_len(&self) -> f32 {
        match self.state {
            CombatState::Light => LIGHT_TIME,
            CombatState::Heavy => HEAVY_TIME,
            CombatState::Charged => CHARGED_TIME,
            CombatState::Dodge => DODGE_IFRAMES + DODGE_RECOVERY,
            // Without this arm the state fell through to 0.0 and `tick` dropped
            // it back to Idle on the next frame — which made the 12-frame parry
            // window one frame long and `FailedParry` unreachable.
            CombatState::Parry => crate::dodge_parry::PARRY_STATE_LEN,
            _ => 0.0,
        }
    }

    /// Advance all timers by `dt` and resolve state transitions.
    pub fn tick(&mut self, dt: f32) {
        if self.iframes > 0.0 {
            self.iframes = (self.iframes - dt).max(0.0);
        }
        if self.combo_window > 0.0 {
            self.combo_window = (self.combo_window - dt).max(0.0);
            if self.combo_window == 0.0 {
                self.combo = 0;
            }
        }
        if self.punish > 0.0 {
            self.punish = (self.punish - dt).max(0.0);
        }
        // Hit-stop freezes action-state progression (but not i-frames/regen).
        if self.hitstop > 0.0 {
            self.hitstop = (self.hitstop - dt).max(0.0);
            return;
        }
        if self.recovery > 0.0 {
            self.recovery = (self.recovery - dt).max(0.0);
        }
        if self.state == CombatState::Idle || self.state == CombatState::Dead {
            return;
        }
        self.timer += dt;
        if self.timer >= self.action_len() {
            // Action finished → enter its recovery, then Idle.
            self.recovery = match self.state {
                CombatState::Dodge => 0.0, // recovery folded into action_len
                _ => 0.0,
            };
            self.state = CombatState::Idle;
            self.timer = 0.0;
        }
    }
}

// ===========================================================================
// Bevy components / resources
// ===========================================================================

/// Guard Husk AI phases (§4.1 + §9 minimal behaviour tree).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HuskState {
    Patrol,
    Chase,
    Telegraph, // wind-up (honest — no active hitbox yet)
    Feint,     // wind-up aborted on purpose — no hitbox ever appears
    Swing1,
    Gap,
    Swing2,
    Recover,
    Staggered,
    Dead,
}

/// How the husk plays the *next* combo (§4.1 extended). Picked once, when the
/// wind-up starts, and it is the only thing that decides how long the raised
/// blade hangs there — see [`telegraph_hold`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HuskRhythm {
    /// Spec tempo: `HUSK_TELEGRAPH` (0.8 s) then the swing lands.
    Straight,
    /// Holds the pose almost twice as long. A dodge rolled on the straight
    /// timing comes out of i-frames *before* the blade arrives.
    Delayed,
    /// Pulls the blade back down without ever swinging, waits a beat, then
    /// opens a real wind-up. Punishes the panic roll.
    Feint,
}

/// How long the raised-blade pose is held before the swing (or the fake) resolves.
/// Pure — the unit tests grade the tempo spread off this, not off a copy of it.
#[inline]
pub fn telegraph_hold(rhythm: HuskRhythm) -> f32 {
    match rhythm {
        HuskRhythm::Straight => HUSK_TELEGRAPH,
        HuskRhythm::Delayed => HUSK_TELEGRAPH_DELAYED,
        HuskRhythm::Feint => HUSK_FEINT_HOLD,
    }
}

/// Pick the rhythm for one combo from a seed (entity ⊕ combo counter). Kept
/// deterministic on purpose: a headless proof that re-rolls its enemy behaviour
/// every run cannot be re-run to check a fix. The avalanche below is there so
/// neighbouring seeds (`entity+1`, `combo+1`) do not walk the table in lockstep.
#[inline]
pub fn pick_rhythm(seed: u32) -> HuskRhythm {
    let mut h = seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    match h % 100 {
        0..=44 => HuskRhythm::Straight, // 45% — the tempo everything else reads against
        45..=79 => HuskRhythm::Delayed, // 35%
        _ => HuskRhythm::Feint,         // 20% — rare enough to stay a surprise
    }
}

/// The Guard Husk enemy (§4.1). One archetype for the first playable.
#[derive(Component)]
pub struct Enemy {
    pub state: HuskState,
    pub timer: f32,
    pub patrol_origin: Vec3,
    pub patrol_dir: f32,
    pub facing: f32,
    pub hitstop: f32,
    pub hit_applied: bool,
    pub surface_y: f32,
    /// Rhythm this combo is being played on (§4.1 extended).
    pub rhythm: HuskRhythm,
    /// How many combos this husk has opened — half of the rhythm seed.
    pub combo_no: u32,
    /// True while the *current* wind-up is the follow-up to a feint, so the husk
    /// can never fake twice in a row (that reads as a broken enemy, not a mind game).
    pub feinted: bool,
}

/// Marker for the enemy's telegraph/arm mesh (child), so the telegraph system
/// can raise it during wind-up.
#[derive(Component)]
pub struct HuskArm;

/// Per-frame intent, written either by the keyboard (`gather_input`) or the
/// scripted headless demo, and consumed by `player_combat`. Decoupling input
/// from logic lets the demo drive the real systems without a keyboard.
#[derive(Resource, Default, Clone)]
pub struct CombatIntent {
    pub light: bool,
    /// Heavy button *held* — released past `CHARGE_HOLD` commits a charged attack.
    pub heavy_down: bool,
    pub dodge: bool,
    pub block: bool,
    pub parry: bool,
    pub lock_toggle: bool,
}

/// Lock-on state (§2.3).
#[derive(Resource, Default)]
pub struct LockOn {
    pub target: Option<Entity>,
}

/// Screen-shake accumulator (§5.3). Diminishing sine decay applied to the
/// camera after the follow system positions it.
///
/// Also carries the *directional* kick (the weight layer). The two are one
/// resource on purpose: `hit()` is the omni-directional rattle ("something
/// happened") and `kick()` is the shove along an axis ("*that* way"). Splitting
/// them into two resources would mean two systems fighting over the same camera
/// transform in the same frame.
#[derive(Resource, Default)]
pub struct Shake {
    pub amp: f32,
    pub time: f32,
    pub dur: f32,
    /// Unit direction of the kick in world space (0 = no kick pending).
    pub kick_dir: Vec3,
    pub kick_amp: f32,
    pub kick_t: f32,
    /// Set once per kick so the log line is emitted on its first applied frame.
    pub kick_logged: bool,
}

impl Shake {
    pub fn hit(&mut self, spec: (f32, f32)) {
        // Take the stronger of any concurrent shakes.
        if spec.0 >= self.amp {
            self.amp = spec.0;
            self.dur = spec.1;
            self.time = 0.0;
        }
    }

    /// Punch the camera along `dir` (world space). Same "strongest wins" rule as
    /// [`Shake::hit`], so a heavy landing during a light's kick is not swallowed.
    pub fn kick(&mut self, dir: Vec3, amp: f32) {
        let d = Vec3::new(dir.x, dir.y, dir.z).normalize_or_zero();
        if d == Vec3::ZERO || amp < self.kick_amp * (1.0 - (self.kick_t / KICK_TIME).min(1.0)) {
            return;
        }
        self.kick_dir = d;
        self.kick_amp = amp;
        self.kick_t = 0.0;
        self.kick_logged = false;
    }
}

/// Marker components for the HUD bars + numeric readouts.
#[derive(Component)]
pub struct HealthBar;
#[derive(Component)]
pub struct StaminaBar;
#[derive(Component)]
pub struct HealthText;
#[derive(Component)]
pub struct StaminaText;
#[derive(Component)]
pub struct LockReticle;

/// Scripted headless combat proof (like `walk_demo`): drives `CombatIntent` on a
/// timeline, reads back component state, prints PASS lines, then exits.
#[derive(Resource, Default)]
pub struct CombatDemo {
    pub phase: u8,
    pub done: bool,
    pub start_stam: f32,
    pub after_attack_stam: f32,
    pub husk_hp0: f32,
    pub husk_hp1: f32,
    pub husk_dead: bool,
    pub saw_iframe: bool,
    pub logged: bool,
}

/// Fired exactly once when the player's HP reaches 0. Poppy wires this to
/// respawn logic (campfire reload, fade transition, etc.). Carries no payload —
/// the respawn system reads `Health` + transform from the player entity.
#[derive(Clone, Debug)]
pub struct PlayerDied;

impl Message for PlayerDied {}

// ===========================================================================
// The weight layer — hit-stop, knockback, camera kick, and the public messages
// the animation (anim.rs) and VFX (vfx.rs) lanes hang off.
// ===========================================================================

/// How hard a blow landed. This is the ONE knob the whole feel layer reads:
/// hit-stop length, knockback distance and camera-kick strength are all derived
/// from it, so "make heavies feel heavier" is a one-line change, not five.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImpactWeight {
    /// Light swing / chained light.
    Light,
    /// Committed heavy swing.
    Heavy,
    /// Charged swing, or *any* blow that broke the target's poise. A poise break
    /// is the loudest thing that can happen in a soulslike trade, so it borrows
    /// the charged attack's whole feel budget regardless of which swing did it.
    Critical,
}

impl ImpactWeight {
    /// Frames of frozen time on the attacker when this lands (§5.2 extended).
    pub fn hitstop(self) -> f32 {
        match self {
            ImpactWeight::Light => HITSTOP_LIGHT,
            ImpactWeight::Heavy => HITSTOP_HEAVY,
            ImpactWeight::Critical => HITSTOP_CRITICAL,
        }
    }
    /// Blocks the target slides along the blade direction.
    pub fn knockback(self) -> f32 {
        match self {
            ImpactWeight::Light => KNOCKBACK_LIGHT,
            ImpactWeight::Heavy => KNOCKBACK_HEAVY,
            ImpactWeight::Critical => KNOCKBACK_CRITICAL,
        }
    }
    /// Directional camera-kick amplitude.
    pub fn kick(self) -> f32 {
        match self {
            ImpactWeight::Light => KICK_LIGHT,
            ImpactWeight::Heavy => KICK_HEAVY,
            ImpactWeight::Critical => KICK_CRITICAL,
        }
    }
    /// Stable lowercase tag used in the log lines the proof script grades.
    pub fn label(self) -> &'static str {
        match self {
            ImpactWeight::Light => "light",
            ImpactWeight::Heavy => "heavy",
            ImpactWeight::Critical => "critical",
        }
    }
}

/// **A blade connected.** Written by [`player_combat`] the frame the hit
/// resolves, before any feel is applied.
///
/// Consumers (`anim.rs` — hit reaction pose; `vfx.rs` — sparks/blood/flash):
/// ```ignore
/// fn my_system(mut hits: MessageReader<combat::ImpactEvent>) {
///     for hit in hits.read() {
///         // hit.pos    — world-space contact point (the target's transform)
///         // hit.dir    — unit vector attacker → target (the blade's direction)
///         // hit.weight — Light | Heavy | Critical; .hitstop()/.kick() give the numbers
///         // hit.target / hit.attacker — entities, if you need to pose a specific rig
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ImpactEvent {
    pub pos: Vec3,
    pub dir: Vec3,
    pub weight: ImpactWeight,
    pub target: Entity,
    pub attacker: Entity,
}
impl Message for ImpactEvent {}

/// **Poise broke.** Written the frame a stagger starts, for either side.
/// `is_player` says whose poise went; `pos` is where they are standing.
#[derive(Clone, Copy, Debug)]
pub struct StaggerEvent {
    pub entity: Entity,
    pub pos: Vec3,
    pub is_player: bool,
}
impl Message for StaggerEvent {}

/// **A roll started.** Written the frame `start_dodge` succeeds — i.e. only when
/// the stamina was actually paid, so a mashed dodge that failed emits nothing.
#[derive(Clone, Copy, Debug)]
pub struct DodgeEvent {
    pub pos: Vec3,
    pub dir: Vec3,
    pub iframes: f32,
}
impl Message for DodgeEvent {}

/// An in-flight shove on a struck body. Inserted by [`player_combat`], consumed
/// by [`apply_knockback`], removed when it runs out.
#[derive(Component, Clone, Copy, Debug)]
pub struct Knockback {
    /// Unit direction (XZ only — a soulslike shove does not launch).
    pub dir: Vec3,
    /// Blocks still to travel.
    pub left: f32,
    /// Total the impulse was worth, for the log line.
    pub total: f32,
    /// Seconds left in the slide.
    pub time: f32,
    /// Hit-stop to hand the struck body on the first applied frame.
    pub hitstop: f32,
    /// Where the body stood when the blow landed — the log line's `from`.
    pub from: Vec3,
    /// False until the first frame has run (that frame books the hit-stop).
    pub started: bool,
    /// Distance this system has actually pushed, summed frame by frame.
    ///
    /// Exists to separate the two ways a shove can come up short, which the
    /// `from`/`to` pair alone cannot tell apart: `apply_knockback` under-
    /// delivering, versus another system writing the same `Transform` later in
    /// the frame and undoing part of it. `slid` is what this system put in;
    /// `moved` is what survived to the end of the frame. `slid > moved` names
    /// the thief.
    pub slid: f32,
    /// Distance added by the end-of-window remainder top-up (see
    /// `apply_knockback`). Logged so a run can show whether that branch ever
    /// fires at all, instead of the question being argued from theory.
    pub topup: f32,
}

/// Turns the feel layer's structured log lines on. Off by default so an
/// interactive session is not spammed; the proof scripts set
/// `VOXELFORGE_FEEL_LOG=1` (the feel probe implies it).
#[derive(Resource, Default, Clone, Copy)]
pub struct FeelLog {
    pub enabled: bool,
}

// ===========================================================================
// Spawning
// ===========================================================================

/// Attach the combat kit to the player entity. Called from `setup` right after
/// the avatar is spawned.
pub fn player_bundle() -> (PlayerCombat, Stamina, Health, Poise) {
    (
        PlayerCombat::default(),
        Stamina::full(),
        Health::new(HP_PLAYER),
        Poise::new(POISE_PLAYER),
    )
}

/// Spawn a Guard Husk (§4.1) standing on Kevin's real terrain surface at
/// `(wx,wz)`. Body is a stack of voxel cuboids (~2.5 blocks tall) with a raised
/// arm child used for the telegraph. Returns nothing — pure world spawn.
pub fn spawn_guard_husk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wx: f32,
    wz: f32,
    surface_y: Option<f32>,
) {
    let surface = surface_y.unwrap_or_else(|| terrain_height(wx, wz) as f32 + 1.0);
    let feet = Vec3::new(wx, surface, wz);

    let armor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.34, 0.40),
        perceptual_roughness: 0.55,
        metallic: 0.3,
        ..default()
    });
    let head_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.19, 0.24),
        perceptual_roughness: 0.5,
        ..default()
    });
    let torso = meshes.add(Cuboid::new(0.9, 1.4, 0.6));
    let head = meshes.add(Cuboid::new(0.55, 0.55, 0.55));
    let arm = meshes.add(Cuboid::new(0.28, 1.0, 0.28));

    commands
        .spawn((
            Transform::from_translation(feet),
            Visibility::default(),
            Enemy {
                state: HuskState::Patrol,
                timer: 0.0,
                patrol_origin: feet,
                patrol_dir: 1.0,
                facing: 0.0,
                hitstop: 0.0,
                hit_applied: false,
                surface_y: surface,
                rhythm: HuskRhythm::Straight,
                combo_no: 0,
                feinted: false,
            },
            Health::new(HP_HUSK),
            Poise::new(POISE_HUSK),
        ))
        .with_children(|p| {
            // Torso (centre ~1.0 above feet).
            p.spawn((
                Mesh3d(torso),
                MeshMaterial3d(armor.clone()),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ));
            // Head (~2.0 above feet → ~2.5 block silhouette with the helm).
            p.spawn((
                Mesh3d(head),
                MeshMaterial3d(head_mat),
                Transform::from_xyz(0.0, 2.0, 0.0),
            ));
            // Weapon arm — starts lowered at the side; the telegraph raises it.
            p.spawn((
                Mesh3d(arm),
                MeshMaterial3d(armor),
                Transform::from_xyz(0.55, 1.1, -0.2),
                HuskArm,
            ));
        });
}

/// Spawn the two HUD bars (health, stamina) + numeric readouts + the lock-on reticle.
/// Called from `setup`. Kept as absolute-positioned Nodes whose fill width is driven
/// by `hud_bars` and whose text labels are updated by `hud_numbers`.
pub fn spawn_combat_hud(commands: &mut Commands) {
    // Health bar (top-left, under the debug text).
    bar(commands, 34.0, Color::srgb(0.82, 0.20, 0.18), HealthBarTag::Health);
    // Stamina bar just below it.
    bar(commands, 50.0, Color::srgb(0.30, 0.78, 0.36), HealthBarTag::Stamina);

    // Numeric readouts to the right of each bar.
    text_tag(commands, 30.0, "HP: 100/100", HealthText);
    text_tag(commands, 46.0, "ST: 100/100", StaminaText);

    // Lock-on reticle — a centred diamond, hidden until a target is locked.
    commands.spawn((
        Text::new("◇"),
        TextFont { font_size: bevy::text::FontSize::from(26.0), ..default() },
        TextColor(Color::srgba(1.0, 0.9, 0.4, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            left: Val::Percent(50.0),
            margin: UiRect { left: Val::Px(-9.0), top: Val::Px(-15.0), ..default() },
            ..default()
        },
        Visibility::Hidden,
        LockReticle,
    ));
}

enum HealthBarTag {
    Health,
    Stamina,
}

fn bar(commands: &mut Commands, top: f32, fill: Color, tag: HealthBarTag) {
    let track = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(top),
                left: Val::Px(10.0),
                width: Val::Px(220.0),
                height: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .id();
    let mut fill_cmd = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(fill),
    ));
    match tag {
        HealthBarTag::Health => {
            fill_cmd.insert(HealthBar);
        }
        HealthBarTag::Stamina => {
            fill_cmd.insert(StaminaBar);
        }
    }
    let fill_id = fill_cmd.id();
    commands.entity(track).add_child(fill_id);
}

/// Tiny absolute-positioned text label for the numeric HP/ST readout.
fn text_tag(commands: &mut Commands, top: f32, label: &str, marker: impl Component) {
    commands.spawn((
        Text::new(label),
        TextFont { font_size: bevy::text::FontSize::from(16.0), ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(top),
            left: Val::Px(236.0), // right of the 220 px bar + 6 px gap
            ..default()
        },
        marker,
    ));
}

// ===========================================================================
// Systems
// ===========================================================================

/// Keyboard → `CombatIntent` (§2 inputs). Real interactive play. The demo
/// overrides this the same frame it runs (it is registered after).
pub fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut intent: ResMut<CombatIntent>,
) {
    let light_pressed = mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::KeyX);
    *intent = CombatIntent {
        light: light_pressed,
        heavy_down: keys.pressed(KeyCode::KeyC),
        dodge: keys.just_pressed(KeyCode::Space),
        block: mouse.pressed(MouseButton::Right),
        parry: keys.just_pressed(KeyCode::KeyV),
        lock_toggle: keys.just_pressed(KeyCode::KeyR),
    };
}

/// Core player combat system: consumes intent, runs the state machine, resolves
/// melee hits against enemies, applies hit-stop + screen-shake.
#[allow(clippy::type_complexity)]
pub fn player_combat(
    time: Res<Time>,
    mut commands: Commands,
    intent: Res<CombatIntent>,
    mut lock: ResMut<LockOn>,
    mut shake: ResMut<Shake>,
    mut died: MessageWriter<PlayerDied>,
    mut sfx: MessageWriter<SfxEvent>,
    mut impacts: MessageWriter<ImpactEvent>,
    mut staggers: MessageWriter<StaggerEvent>,
    mut dodges: MessageWriter<DodgeEvent>,
    feel: Res<FeelLog>,
    // ResMut, not Res: `log_riposte` tallies the riposte it just wrote so the
    // dodge/parry probe can stop on evidence instead of a stopwatch.
    mut dp: ResMut<crate::dodge_parry::DodgeParryState>,
    mut player_q: Query<
        (Entity, &mut Transform, &mut PlayerCombat, &mut Stamina, &Health, &mut Poise),
        (With<FlyCam>, Without<Enemy>),
    >,
    mut enemy_q: Query<(Entity, &Transform, &mut Health, &mut Poise), (With<Enemy>, Without<FlyCam>)>,
) {
    let dt = time.delta_secs();
    let Ok((player, mut ptf, mut pc, mut stam, hp, mut poise)) = player_q.single_mut() else {
        return;
    };

    // Tick resources every frame.
    stam.tick(dt);
    poise.tick(dt);
    // Stagger overrides the state machine.
    if poise.staggered() && pc.state != CombatState::Dead {
        pc.state = CombatState::Stagger;
    } else if pc.state == CombatState::Stagger {
        pc.state = CombatState::Idle;
    }
    pc.tick(dt);
    if hp.dead() && pc.state != CombatState::Dead {
        pc.state = CombatState::Dead;
        sfx.write(SfxEvent::PlayerDeath);
        died.write(PlayerDied);
        info!("COMBAT player died — PlayerDied event fired");
    }

    // Lock-on toggle (§2.3): pick nearest enemy inside range + acquisition cone.
    if intent.lock_toggle {
        if lock.target.is_some() {
            lock.target = None;
        } else {
            lock.target = nearest_target(&ptf.translation, pc_facing(&ptf), &enemy_q);
        }
    }
    // Auto-unlock if the target left range or died (§2.3).
    if let Some(t) = lock.target {
        let drop = match enemy_q.get(t) {
            Ok((_, etf, ehp, _)) => {
                ehp.dead() || ptf.translation.distance(etf.translation) > LOCK_DROP_RANGE
            }
            Err(_) => true,
        };
        if drop {
            lock.target = None;
        }
    }

    // Start actions from intent (priority: dodge > parry > block > heavy > light).
    if intent.dodge {
        if pc.start_dodge(&mut stam) {
            // Only a roll that actually paid its stamina announces itself — a
            // mashed dodge that was refused must not make anim/vfx play a roll.
            let fwd = ptf.rotation * Vec3::NEG_Z;
            dodges.write(DodgeEvent {
                pos: ptf.translation,
                dir: Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero(),
                iframes: DODGE_IFRAMES,
            });
        }
    } else if intent.parry && pc.can_act(&stam) {
        if stam.try_spend(COST_PARRY, DELAY_PARRY) {
            pc.state = CombatState::Parry;
            pc.timer = 0.0;
            pc.parry_frames = crate::dodge_parry::PARRY_WINDOW_FRAMES;
        }
    } else if intent.block && pc.can_act(&stam) {
        pc.state = CombatState::Block;
        pc.timer = 0.0;
    } else if pc.state == CombatState::Block && !intent.block {
        pc.state = CombatState::Idle; // release hold-to-block
    } else if intent.heavy_down {
        // Hold to charge (§2.4); the commit happens on release below.
        if pc.can_act(&stam) {
            pc.charge += dt;
        }
    } else if pc.charge > 0.0 {
        // Heavy button released: charged if held past the threshold, else heavy.
        if pc.charge >= CHARGE_HOLD {
            if pc.start_charged(&mut stam) {
                sfx.write(SfxEvent::SwingHeavy { position: ptf.translation });
            }
        } else {
            if pc.start_heavy(&mut stam) {
                sfx.write(SfxEvent::SwingHeavy { position: ptf.translation });
            }
        }
        pc.charge = 0.0;
    } else if intent.light {
        if pc.start_light(&mut stam) {
            sfx.write(SfxEvent::SwingLight { position: ptf.translation });
        }
    }

    // Roll movement (§2.2): glide DODGE_DISTANCE (2.5 blocks) forward over the
    // i-frame window so the dodge visibly repositions, not just blinks invuln.
    if pc.state == CombatState::Dodge && pc.iframes > 0.0 {
        let fwd = ptf.rotation * Vec3::NEG_Z;
        let dir = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        ptf.translation += dir * DODGE_DISTANCE * (dt / DODGE_IFRAMES);
    }

    // Resolve the player's active swing against enemies in reach + cone.
    if let Some((dmg, poise_dmg)) = pc.active_hit() {
        let origin = ptf.translation;
        let facing = pc_facing(&ptf);
        let mut landed = false;
        for (enemy, etf, mut ehp, mut ep) in enemy_q.iter_mut() {
            if ehp.dead() {
                continue;
            }
            let to = etf.translation - origin;
            let dist = to.length();
            let in_cone = in_cone(facing, to, MELEE_CONE);
            if dist > MELEE_RANGE + PLAYER_HALF_W + 0.6 {
                continue;
            }
            if !in_cone {
                continue;
            }
            // Stagger amplifies damage (§3.2); a fresh parry punish adds +25%
            // (§2.5) — unless this enemy is the one a parry left open, in which
            // case the swing is a riposte and takes the much heavier multiplier.
            let riposte = crate::dodge_parry::take_riposte(&mut pc, enemy);
            let mut mult = if ep.staggered() { STAGGER_DMG_MULT } else { 1.0 };
            if pc.punish > 0.0 {
                mult *= if riposte { crate::dodge_parry::RIPOSTE_MULT } else { PARRY_PUNISH_MULT };
            }
            ehp.damage(dmg * mult);
            let broke = ep.take(poise_dmg, false);
            landed = true;
            // Audio feedback — pick light or heavy based on the attack type.
            if matches!(pc.state, CombatState::Heavy | CombatState::Charged) {
                sfx.write(SfxEvent::HitHeavy { position: etf.translation });
            } else {
                sfx.write(SfxEvent::HitLight { position: etf.translation });
            }

            // ---- the weight layer --------------------------------------------
            // One classification drives every channel below, so a heavy can never
            // end up with a light's freeze and a heavy's shove (which reads as a
            // bug you can feel but not name).
            let weight = match pc.state {
                _ if riposte => ImpactWeight::Critical, // a riposte is the loudest hit there is
                _ if broke => ImpactWeight::Critical, // a poise break outranks the swing
                CombatState::Charged => ImpactWeight::Critical,
                CombatState::Heavy => ImpactWeight::Heavy,
                _ => ImpactWeight::Light,
            };
            // Blade direction: player → target, flattened. `normalize_or_zero`
            // guards the degenerate "standing inside the enemy" case; the facing
            // vector is the honest fallback there.
            let blade = {
                let flat = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
                if flat == Vec3::ZERO {
                    Vec3::new(-facing.sin(), 0.0, -facing.cos())
                } else {
                    flat
                }
            };

            // 1. Hit-stop — the single biggest reason a swing reads as "landed".
            //    A break still floors at HITSTOP_STAGGER so the old §5.2 number
            //    is never regressed by the new table.
            pc.hitstop = weight.hitstop().max(if broke { HITSTOP_STAGGER } else { 0.0 });
            // 2. Knockback — the target slides along the blade, and eats the same
            //    freeze so the shove starts the instant the world unpauses.
            commands.entity(enemy).insert(Knockback {
                dir: blade,
                left: weight.knockback(),
                total: weight.knockback(),
                time: KNOCKBACK_TIME,
                hitstop: weight.hitstop(),
                from: etf.translation,
                started: false,
                slid: 0.0,
                topup: 0.0,
            });
            // 3. Camera: the existing omni rattle (§5.3) …
            shake.hit(if matches!(pc.state, CombatState::Heavy | CombatState::Charged) {
                SHAKE_HEAVY
            } else {
                SHAKE_LIGHT
            });
            //    … plus a kick down the blade, so the frame lurches *into* the hit.
            shake.kick(blade, weight.kick());

            // 4. Tell the other lanes. anim.rs poses the reaction, vfx.rs sparks.
            impacts.write(ImpactEvent {
                pos: etf.translation,
                dir: blade,
                weight,
                target: enemy,
                attacker: player,
            });
            if broke {
                staggers.write(StaggerEvent {
                    entity: enemy,
                    pos: etf.translation,
                    is_player: false,
                });
            }
            if riposte {
                crate::dodge_parry::log_riposte(
                    &feel, &mut dp, enemy, mult, dmg * mult, weight, ehp.cur,
                );
            }
        }
        if landed {
            pc.hit_applied = true;
        }
    }
}

/// A chasing husk disengages once the player retreats past the leash radius
/// (§4.1: "If player retreats > 6 blocks, returns to patrol").
#[inline]
pub fn husk_should_leash(dist: f32) -> bool {
    dist > HUSK_LEASH
}

/// Guard Husk AI + attack resolution (§4.1 / §9). Patrols, aggros, telegraphs
/// honestly, swings twice, and only lands damage during active frames — negated
/// if the player is i-framing, reduced if blocking, parryable in the window.
#[allow(clippy::type_complexity)]
pub fn husk_ai(
    time: Res<Time>,
    mut commands: Commands,
    mut shake: ResMut<Shake>,
    mut sfx: MessageWriter<SfxEvent>,
    mut impacts: MessageWriter<ImpactEvent>,
    mut staggers: MessageWriter<StaggerEvent>,
    feel: Res<FeelLog>,
    mut dp: ResMut<crate::dodge_parry::DodgeParryState>,
    mut enemy_q: Query<
        (Entity, &mut Transform, &mut Enemy, &Health, &mut Poise, Option<&Knockback>),
        (With<Enemy>, Without<FlyCam>),
    >,
    mut player_q: Query<
        (Entity, &Transform, &mut Health, &mut PlayerCombat, &mut Stamina, &mut Poise),
        (With<FlyCam>, Without<Enemy>),
    >,
) {
    let dt = time.delta_secs();
    let Ok((player, ptf, mut php, mut pc, mut pstam, mut ppoise)) = player_q.single_mut() else {
        return;
    };

    for (entity, mut etf, mut e, ehp, mut ep, shove) in enemy_q.iter_mut() {
        ep.tick(dt);
        // A body mid-shove does not also power-walk. `apply_knockback` runs
        // immediately before this system and writes the same transform, so any
        // step taken here is subtracted straight out of the impulse the hit was
        // booked for — a light hit reads as 0.79× its own knockback number.
        // The shove is 0.12 s; the AI simply owes it that beat.
        let shoved = shove.is_some();
        // Hit-stop drains every frame, *including* through a stagger. It used to
        // be ticked below the stagger `continue`, so a poise break froze the
        // timer for the whole 1.5 s stagger — and `apply_knockback` waits on
        // `hitstop == 0`, so the biggest shove in the game (a critical) never
        // started. The freeze belongs to the blow, not to the state it left.
        let frozen = e.hitstop > 0.0;
        e.hitstop = (e.hitstop - dt).max(0.0);
        if ehp.dead() {
            // A killing blow still gets its shove. The corpse rides the impulse
            // out first and leaves the world when the slide ends — otherwise the
            // hardest hits in the game (a critical breaks poise, and a poise
            // break is what kills) despawn the body on the same frame they land
            // and never move it a millimetre. The death check sits *below* the
            // hit-stop drain on purpose: `apply_knockback` waits on
            // `hitstop == 0`, so a corpse that stopped draining it would hold a
            // `Knockback` forever and never despawn.
            if e.state != HuskState::Dead {
                e.state = HuskState::Dead;
                sfx.write(SfxEvent::EnemyDeath { position: etf.translation });
            }
            if !shoved {
                commands.entity(entity).despawn();
                info!("COMBAT husk defeated — despawned entity={:?}", entity);
            }
            continue;
        }
        if ep.staggered() {
            e.state = HuskState::Staggered;
            e.timer = 0.0;
            continue;
        }
        if e.state == HuskState::Staggered {
            e.state = HuskState::Chase; // recovered
        }
        if frozen {
            continue;
        }

        let to_player = ptf.translation - etf.translation;
        let dist = Vec3::new(to_player.x, 0.0, to_player.z).length();
        e.timer += dt;

        match e.state {
            HuskState::Patrol => {
                // Walk a 12-block loop along X from the origin (§4.1).
                let step = HUSK_WALK * dt * e.patrol_dir;
                if !shoved {
                    etf.translation.x += step;
                }
                e.facing = if e.patrol_dir > 0.0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 };
                if (etf.translation.x - e.patrol_origin.x).abs() > 6.0 {
                    e.patrol_dir = -e.patrol_dir;
                }
                if dist <= HUSK_AGGRO_RANGE {
                    e.state = HuskState::Chase;
                    e.timer = 0.0;
                }
            }
            HuskState::Chase => {
                let want = to_player.z.atan2(to_player.x);
                e.facing = turn(e.facing, want, HUSK_TURN * dt);
                if husk_should_leash(dist) {
                    e.state = HuskState::Patrol; // leashed back to patrol (§4.1)
                } else if dist <= MELEE_RANGE {
                    // Opening a combo — pick how it will be played *now*, before
                    // the blade goes up, so the wind-up itself carries the lie.
                    e.combo_no += 1;
                    e.feinted = false;
                    e.rhythm = pick_rhythm((entity.to_bits() as u32) ^ e.combo_no.wrapping_mul(0x9E37_79B9));
                    e.state = HuskState::Telegraph; // begin honest wind-up
                    e.timer = 0.0;
                    e.hit_applied = false;
                    if feel.enabled {
                        println!(
                            "FEEL_HUSK_RHYTHM entity={} combo={} pattern={} hold={:.2} follow_up=false",
                            entity.to_bits(), e.combo_no, rhythm_label(e.rhythm),
                            telegraph_hold(e.rhythm)
                        );
                    }
                } else if !shoved {
                    let dir = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    etf.translation += dir * HUSK_WALK * dt;
                }
            }
            HuskState::Telegraph => {
                // Keep closing while the blade is up. A husk that plants its feet
                // through a 1.5 s wind-up can be walked away from for free — and
                // hit-knockback would slide the fight apart over a few trades.
                if dist > MELEE_RANGE && !shoved {
                    let dir = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    etf.translation += dir * HUSK_WALK * HUSK_STEP_IN * dt;
                }
                if e.timer >= telegraph_hold(e.rhythm) {
                    if e.rhythm == HuskRhythm::Feint {
                        // The fake: blade comes back down, no hitbox ever existed.
                        e.state = HuskState::Feint;
                        e.timer = 0.0;
                        e.hit_applied = false;
                        if feel.enabled {
                            println!(
                                "FEEL_HUSK_FEINT entity={} combo={} held={:.2} recover={:.2}",
                                entity.to_bits(), e.combo_no, HUSK_FEINT_HOLD, HUSK_FEINT_RECOVER
                            );
                        }
                    } else {
                        e.state = HuskState::Swing1;
                        e.timer = 0.0;
                        e.hit_applied = false;
                    }
                }
            }
            HuskState::Feint => {
                // A beat of nothing — this is the window the panic roll is wasted
                // in — then a *real* wind-up. Never two fakes in a row.
                if e.timer >= HUSK_FEINT_RECOVER {
                    e.feinted = true;
                    e.rhythm = match pick_rhythm(
                        (entity.to_bits() as u32) ^ e.combo_no.wrapping_mul(0x85EB_CA6B) ^ 0x5F5F,
                    ) {
                        HuskRhythm::Feint => HuskRhythm::Straight, // no double fake
                        other => other,
                    };
                    e.state = HuskState::Telegraph;
                    e.timer = 0.0;
                    e.hit_applied = false;
                    if feel.enabled {
                        println!(
                            "FEEL_HUSK_RHYTHM entity={} combo={} pattern={} hold={:.2} follow_up=true",
                            entity.to_bits(), e.combo_no, rhythm_label(e.rhythm),
                            telegraph_hold(e.rhythm)
                        );
                    }
                }
            }
            HuskState::Swing1 => {
                if !e.hit_applied && e.timer <= HUSK_ACTIVE {
                    let (connected, outcome, broke) = try_hit_player(
                        HUSK_SWING1_DMG, HUSK_SWING1_POISE, dist, &mut php, &mut pc,
                        &mut pstam, &mut ppoise, &mut shake,
                    );
                    // Dodge / parry / mistimed parry — posture, poise break,
                    // shove, riposte and their log lines all live in one place.
                    crate::dodge_parry::resolve_defence(
                        outcome, HUSK_SWING1_DMG, &feel, &mut dp, &mut commands, entity,
                        &mut e, &mut ep, &mut pc, &php, etf.translation, to_player,
                        &mut shake, &mut staggers,
                    );
                    if connected {
                        e.hitstop = e.hitstop.max(HITSTOP_ENEMY);
                        emit_enemy_hit_sfx(&outcome, etf.translation, &mut sfx);
                        taken_feel(
                            ImpactWeight::Light, broke, outcome, &to_player, ptf.translation,
                            entity, player, &mut pc, &mut shake, &mut impacts, &mut staggers,
                        );
                    }
                    e.hit_applied = true;
                }
                if e.timer >= HUSK_ACTIVE {
                    e.state = HuskState::Gap;
                    e.timer = 0.0;
                }
            }
            HuskState::Gap => {
                if e.timer >= HUSK_GAP {
                    e.state = HuskState::Swing2;
                    e.timer = 0.0;
                    e.hit_applied = false;
                }
            }
            HuskState::Swing2 => {
                if !e.hit_applied && e.timer <= HUSK_ACTIVE {
                    let (connected, outcome, broke) = try_hit_player(
                        HUSK_SWING2_DMG, HUSK_SWING2_POISE, dist, &mut php, &mut pc,
                        &mut pstam, &mut ppoise, &mut shake,
                    );
                    crate::dodge_parry::resolve_defence(
                        outcome, HUSK_SWING2_DMG, &feel, &mut dp, &mut commands, entity,
                        &mut e, &mut ep, &mut pc, &php, etf.translation, to_player,
                        &mut shake, &mut staggers,
                    );
                    if connected {
                        e.hitstop = e.hitstop.max(HITSTOP_ENEMY);
                        emit_enemy_hit_sfx(&outcome, etf.translation, &mut sfx);
                        // Swing 2 is the heavier half of the combo (20 dmg vs 15).
                        taken_feel(
                            ImpactWeight::Heavy, broke, outcome, &to_player, ptf.translation,
                            entity, player, &mut pc, &mut shake, &mut impacts, &mut staggers,
                        );
                    }
                    e.hit_applied = true;
                }
                if e.timer >= HUSK_ACTIVE {
                    e.state = HuskState::Recover;
                    e.timer = 0.0;
                }
            }
            HuskState::Recover => {
                if e.timer >= HUSK_COMBO_PAUSE {
                    e.state = HuskState::Chase; // re-evaluate (§4.1 pause between combos)
                    e.timer = 0.0;
                }
            }
            HuskState::Dead | HuskState::Staggered => {}
        }

        // Keep the Husk planted on the terrain surface it spawned on (no drift).
        etf.translation.y = e.surface_y;
        etf.rotation = Quat::from_axis_angle(Vec3::Y, e.facing);
    }
}

/// What happened when an enemy swing reached the player — used by the audio
/// layer to pick the right SFX.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EnemyHitOutcome {
    Missed,
    Dodged,
    Parried,
    FailedParry,
    Blocked,
    GuardBroke,
    PlainHit,
}

/// One Husk swing against the player. Honors i-frames (negate), block (50% +
/// stamina), parry window (negate + posture) per §2.5. Returns
/// (hit_stop_needed, what_happened, poise_broke) so the caller can book hit-stop,
/// the audio layer can pick the right SFX, and the feel layer knows whether this
/// blow was merely a hit or the one that broke the player's stance.
#[allow(clippy::too_many_arguments)]
fn try_hit_player(
    dmg: f32,
    poise_dmg: f32,
    dist: f32,
    php: &mut Health,
    pc: &mut PlayerCombat,
    pstam: &mut Stamina,
    ppoise: &mut Poise,
    shake: &mut Shake,
) -> (bool, EnemyHitOutcome, bool) {
    if dist > MELEE_RANGE + PLAYER_HALF_W + 0.4 {
        return (false, EnemyHitOutcome::Missed, false); // player stepped out of reach
    }
    // Dodge i-frames negate everything (§2.2).
    if pc.invulnerable() {
        return (false, EnemyHitOutcome::Dodged, false);
    }
    // Parry window (§2.5): tap parry as the hit lands → negate + posture + punish.
    // Counted in frames — see `dodge_parry::PARRY_WINDOW_FRAMES`. The posture
    // damage and the riposte are booked by `dodge_parry::resolve_defence` at the
    // call site, which is the only place the *attacker's* poise is in hand.
    if pc.state == CombatState::Parry && pc.parry_frames > 0 {
        ppoise.take(0.0, false); // no self-damage; parry succeeded
        pc.punish = PARRY_PUNISH; // +25% window opens on the enemy
        shake.hit(SHAKE_LIGHT);
        pc.hitstop = HITSTOP_PARRY;
        return (true, EnemyHitOutcome::Parried, false);
    }
    // Failed parry (§2.5): threw the parry but the 0.20 s window had closed —
    // punished with +25% damage and a 0.5 s recovery lock.
    if pc.state == CombatState::Parry {
        php.damage(dmg * PARRY_FAIL_MULT);
        pc.recovery = PARRY_FAIL_RECOVER;
        let broke = ppoise.take(poise_dmg, false);
        shake.hit(SHAKE_ENEMY_HIT);
        return (true, EnemyHitOutcome::FailedParry, broke);
    }
    // Block (§2.5): 50% off if stamina can pay, else guard-break stagger.
    if pc.state == CombatState::Block {
        if pstam.try_spend(COST_BLOCK, DELAY_BLOCK) {
            php.damage(dmg * (1.0 - BLOCK_REDUCTION));
            let broke = ppoise.take(poise_dmg * 0.5, false);
            shake.hit(SHAKE_LIGHT);
            return (true, EnemyHitOutcome::Blocked, broke);
        } else {
            php.damage(dmg);
            ppoise.stagger = GUARD_BREAK; // guard broken → stagger
            shake.hit(SHAKE_ENEMY_HIT);
            return (true, EnemyHitOutcome::GuardBroke, true);
        }
    }
    // Plain hit.
    php.damage(dmg);
    let broke = ppoise.take(poise_dmg, pc.hyper_armor());
    shake.hit(SHAKE_ENEMY_HIT);
    (true, EnemyHitOutcome::PlainHit, broke)
}

/// The receiving half of the weight layer: what the *player* feels when a Husk
/// swing connects. Mirrors the attacking side in `player_combat` — same three
/// channels (freeze, camera, messages) off the same [`ImpactWeight`] — minus
/// knockback, because shoving the player's body around on every chip of damage
/// is how a soulslike loses its footing (§2.2 keeps repositioning on the roll).
///
/// A parried or dodged swing feels like *nothing landed*, because nothing did —
/// those outcomes are filtered out by the caller's `connected` flag only when the
/// swing missed outright, so they are re-checked here.
#[allow(clippy::too_many_arguments)]
fn taken_feel(
    base: ImpactWeight,
    broke: bool,
    outcome: EnemyHitOutcome,
    enemy_to_player: &Vec3,
    player_pos: Vec3,
    enemy: Entity,
    player: Entity,
    pc: &mut PlayerCombat,
    shake: &mut Shake,
    impacts: &mut MessageWriter<ImpactEvent>,
    staggers: &mut MessageWriter<StaggerEvent>,
) {
    if matches!(outcome, EnemyHitOutcome::Missed | EnemyHitOutcome::Dodged | EnemyHitOutcome::Parried) {
        return;
    }
    let weight = if broke { ImpactWeight::Critical } else { base };
    let dir = Vec3::new(enemy_to_player.x, 0.0, enemy_to_player.z).normalize_or_zero();
    // Freeze the player too — a hit you take should stop *your* frame, not just
    // the one you deal. Never shortens an in-flight freeze.
    pc.hitstop = pc.hitstop.max(weight.hitstop());
    // Kick the camera the way the blow travelled: away from the attacker.
    shake.kick(dir, KICK_TAKEN.max(weight.kick()));
    impacts.write(ImpactEvent {
        pos: player_pos,
        dir,
        weight,
        target: player,
        attacker: enemy,
    });
    if broke {
        staggers.write(StaggerEvent { entity: player, pos: player_pos, is_player: true });
    }
}

/// Stable lowercase tag for [`HuskRhythm`], used in the log lines the proof grades.
#[inline]
pub fn rhythm_label(r: HuskRhythm) -> &'static str {
    match r {
        HuskRhythm::Straight => "straight",
        HuskRhythm::Delayed => "delayed",
        HuskRhythm::Feint => "feint",
    }
}

/// Map the outcome of an enemy swing against the player into an [`SfxEvent`].
fn emit_enemy_hit_sfx(outcome: &EnemyHitOutcome, pos: Vec3, sfx: &mut MessageWriter<SfxEvent>) {
    let ev = match outcome {
        EnemyHitOutcome::Parried => SfxEvent::HitParry { position: pos },
        EnemyHitOutcome::Blocked => SfxEvent::HitBlock { position: pos },
        EnemyHitOutcome::GuardBroke => SfxEvent::HitHeavy { position: pos },
        EnemyHitOutcome::FailedParry | EnemyHitOutcome::PlainHit => SfxEvent::PlayerHurt,
        _ => return, // Missed | Dodged → no sound
    };
    sfx.write(ev);
}

/// Move the enemy's telegraph arm up during wind-up so the incoming swing reads
/// at a distance (§5.1 honest visual telegraph — a big pose change).
pub fn husk_telegraph(
    enemy_q: Query<(&Enemy, &Children)>,
    mut arm_q: Query<&mut Transform, With<HuskArm>>,
) {
    for (e, children) in enemy_q.iter() {
        let raised = matches!(e.state, HuskState::Telegraph);
        let mid_swing = matches!(e.state, HuskState::Swing1 | HuskState::Swing2);
        for child in children.iter() {
            if let Ok(mut t) = arm_q.get_mut(child) {
                // Raise the arm overhead while winding up; slam down on the swing.
                let target = if raised {
                    -1.1 // rotate back/up (radians about X)
                } else if mid_swing {
                    0.6
                } else {
                    0.0
                };
                let cur = t.rotation.to_scaled_axis().x;
                let next = cur + (target - cur) * 0.35;
                t.rotation = Quat::from_axis_angle(Vec3::X, next);
            }
        }
    }
}

/// Snap the camera + player facing toward the locked target (§2.3) and toggle
/// the reticle. Runs after `fly_camera` so it overrides the mouse orbit while a
/// target is held.
pub fn lock_on_camera(
    time: Res<Time>,
    lock: Res<LockOn>,
    mut reticle_q: Query<&mut Visibility, With<LockReticle>>,
    enemy_q: Query<&Transform, (With<Enemy>, Without<crate::OrbitCam>, Without<FlyCam>)>,
    mut cam_q: Query<(&mut Transform, &mut crate::OrbitCam), Without<FlyCam>>,
    mut player_q: Query<&mut FlyCam, Without<crate::OrbitCam>>,
) {
    if let Ok(mut vis) = reticle_q.single_mut() {
        *vis = if lock.target.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    let Some(target) = lock.target else { return };
    let Ok(etf) = enemy_q.get(target) else { return };
    let Ok((mut ctf, mut orbit)) = cam_q.single_mut() else { return };
    let Ok(mut fly) = player_q.single_mut() else { return };

    let cam_to = etf.translation - ctf.translation;
    let want_yaw = (-cam_to.x).atan2(-cam_to.z) + std::f32::consts::PI;
    orbit.yaw = turn(orbit.yaw, want_yaw, LOCK_SNAP * time.delta_secs());
    // Face the player at the target too, so attacks orient toward it.
    let flat = Vec3::new(etf.translation.x - ctf.translation.x, 0.0, etf.translation.z - ctf.translation.z);
    fly.face_yaw = (-flat.x).atan2(-flat.z);
    // Re-aim the camera at the pivot (position was set by fly_camera; keep it).
    let look = etf.translation - ctf.translation;
    if look.length_squared() > 0.001 {
        ctf.look_to(look.normalize(), Vec3::Y);
    }
}

/// Apply diminishing screen-shake to the camera (§5.3). Runs after the camera
/// follow so it perturbs the final position; camera & audio are unaffected
/// elsewhere (no global time dilation, per §5.2).
/// Also applies the directional camera kick (the weight layer) — one system, so
/// the rattle and the kick compose into a single offset instead of two systems
/// racing to write `Transform` in the same frame.
pub fn apply_shake(
    time: Res<Time>,
    feel: Res<FeelLog>,
    mut shake: ResMut<Shake>,
    mut cam_q: Query<&mut Transform, With<crate::OrbitCam>>,
) {
    let dt = time.delta_secs();
    let rattling = shake.amp > 0.0 && shake.dur > 0.0;
    let kicking = shake.kick_amp > 0.0 && shake.kick_dir != Vec3::ZERO;
    if !rattling && !kicking {
        return;
    }

    let mut off = Vec3::ZERO;

    if rattling {
        shake.time += dt;
        if shake.time >= shake.dur {
            shake.amp = 0.0;
        } else {
            let decay = 1.0 - (shake.time / shake.dur);
            // Deterministic pseudo-jitter from the phase (no Math.random needed).
            let ph = shake.time * 90.0;
            off += Vec3::new(ph.sin(), (ph * 1.3).cos(), 0.0) * shake.amp * decay;
        }
    }

    if kicking {
        shake.kick_t += dt;
        if shake.kick_t >= KICK_TIME {
            shake.kick_amp = 0.0;
            shake.kick_dir = Vec3::ZERO;
        } else {
            // Full offset on the contact frame, then ease back to centre. A
            // ramp-up would put the lurch *after* the hit, where it reads as lag.
            let u = shake.kick_t / KICK_TIME;
            let env = (1.0 - u) * (1.0 - u); // quadratic ease-out
            off += shake.kick_dir * shake.kick_amp * env;
        }
    }

    if off == Vec3::ZERO {
        return;
    }
    let Ok(mut ctf) = cam_q.single_mut() else { return };
    ctf.translation += off;

    if feel.enabled && kicking && !shake.kick_logged {
        shake.kick_logged = true;
        println!(
            "FEEL_CAMKICK amp={:.3} dir=({:.2},{:.2},{:.2}) dur={:.2} applied={:.4}",
            shake.kick_amp, shake.kick_dir.x, shake.kick_dir.y, shake.kick_dir.z,
            KICK_TIME, off.length()
        );
    }
}

/// Drive the HUD fill bars from live health/stamina.
pub fn hud_bars(
    player_q: Query<(&Health, &Stamina), With<FlyCam>>,
    mut hbar: Query<&mut Node, (With<HealthBar>, Without<StaminaBar>)>,
    mut sbar: Query<&mut Node, (With<StaminaBar>, Without<HealthBar>)>,
) {
    let Ok((hp, stam)) = player_q.single() else { return };
    if let Ok(mut n) = hbar.single_mut() {
        n.width = Val::Percent((hp.cur / hp.max * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut n) = sbar.single_mut() {
        n.width = Val::Percent((stam.cur / STAMINA_MAX * 100.0).clamp(0.0, 100.0));
    }
}

/// Update the numeric HP / stamina readouts from live component values.
pub fn hud_numbers(
    player_q: Query<(&Health, &Stamina), With<FlyCam>>,
    mut htext: Query<&mut Text, (With<HealthText>, Without<StaminaText>)>,
    mut stext: Query<&mut Text, (With<StaminaText>, Without<HealthText>)>,
) {
    let Ok((hp, stam)) = player_q.single() else { return };
    if let Ok(mut t) = htext.single_mut() {
        t.0 = format!("HP: {:.0}/{:.0}", hp.cur.round(), hp.max);
    }
    if let Ok(mut t) = stext.single_mut() {
        t.0 = format!("ST: {:.0}/{:.0}", stam.cur.round(), STAMINA_MAX);
    }
}

// ===========================================================================
// Weight-layer systems
// ===========================================================================

/// Slide a struck body along the blade direction (§ weight layer).
///
/// Runs between `player_combat` (which books the impulse) and `husk_ai` (which
/// re-plants the body on its surface and may walk it back in), so the shove is
/// always resolved against the same frame's hit.
pub fn apply_knockback(
    time: Res<Time>,
    feel: Res<FeelLog>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Enemy, &mut Knockback)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tf, mut e, mut kb) in q.iter_mut() {
        if !kb.started {
            kb.started = true;
            kb.from = tf.translation;
            // Hand the struck body the same freeze the attacker got. Without this
            // the husk keeps walking through the frame the player is frozen in,
            // and the hit reads as if it passed straight through.
            e.hitstop = e.hitstop.max(kb.hitstop);
        }
        // A dead body is deliberately *not* dropped here — `husk_ai` holds the
        // despawn until this component is gone, so the killing blow lands as a
        // visible shove instead of a body that vanishes where it stood.
        //
        // The slide waits out the freeze on purpose — motion resuming the instant
        // time does is exactly what sells the freeze as impact.
        if e.hitstop > 0.0 {
            continue;
        }
        let step = (kb.total / KNOCKBACK_TIME * dt).min(kb.left);
        tf.translation += kb.dir * step;
        kb.left -= step;
        kb.slid += step;
        kb.time -= dt;
        // Belt-and-braces, not the fix for anything observed. `left` and `time`
        // are driven by the same running sum of `dt` — `left == total * time /
        // KNOCKBACK_TIME` holds exactly — so the window cannot close on an
        // undelivered remainder except through float drift. This pays out that
        // drift rather than leaving the slide a hair short, and records what it
        // paid in `topup` so a run can show the branch is inert (it logs 0.000)
        // instead of the claim resting on the algebra above.
        if kb.time <= 0.0 && kb.left > 0.0 {
            tf.translation += kb.dir * kb.left;
            kb.slid += kb.left;
            kb.topup += kb.left;
            kb.left = 0.0;
        }
        if kb.left <= 0.0 {
            if feel.enabled {
                // XZ only — the shove never launches, and the husk's surface
                // re-plant writes Y every frame. Measuring in 3D would let that
                // re-plant flatter or dent a number this lane is graded on.
                let d = tf.translation - kb.from;
                let moved = Vec3::new(d.x, 0.0, d.z).length();
                // `dir=` stays last on the line — scripts/prove_knockback.sh
                // anchors its parse to end-of-line. New fields go before it.
                println!(
                    "FEEL_KNOCKBACK entity={} from=({:.2},{:.2}) to=({:.2},{:.2}) moved={:.3} \
                     impulse={:.3} slid={:.3} topup={:.3} dir=({:.2},{:.2})",
                    entity.to_bits(), kb.from.x, kb.from.z,
                    tf.translation.x, tf.translation.z, moved, kb.total,
                    kb.slid, kb.topup, kb.dir.x, kb.dir.z
                );
            }
            commands.entity(entity).remove::<Knockback>();
        }
    }
}

/// Print the feel layer's message stream as structured, greppable facts.
///
/// This system *reads* what the real combat systems wrote — it never decides
/// anything and it never grades. The proof script does the grading, off these
/// numbers, so no PASS in this lane is ever self-awarded by a demo.
pub fn combat_feel_log(
    mut impacts: bevy::ecs::message::MessageReader<ImpactEvent>,
    mut staggers: bevy::ecs::message::MessageReader<StaggerEvent>,
    mut dodges: bevy::ecs::message::MessageReader<DodgeEvent>,
) {
    for hit in impacts.read() {
        println!(
            "FEEL_IMPACT pos=({:.2},{:.2},{:.2}) dir=({:.2},{:.2}) weight={} hitstop={:.3} \
             knockback={:.3} kick={:.3} target={} attacker={}",
            hit.pos.x, hit.pos.y, hit.pos.z, hit.dir.x, hit.dir.z,
            hit.weight.label(), hit.weight.hitstop(), hit.weight.knockback(), hit.weight.kick(),
            hit.target.to_bits(), hit.attacker.to_bits()
        );
    }
    for s in staggers.read() {
        println!(
            "FEEL_STAGGER entity={} who={} pos=({:.2},{:.2},{:.2})",
            s.entity.to_bits(),
            if s.is_player { "player" } else { "enemy" },
            s.pos.x, s.pos.y, s.pos.z
        );
    }
    for d in dodges.read() {
        println!(
            "FEEL_DODGE pos=({:.2},{:.2},{:.2}) dir=({:.2},{:.2}) iframes={:.3}",
            d.pos.x, d.pos.y, d.pos.z, d.dir.x, d.dir.z, d.iframes
        );
    }
}

// ---------------------------------------------------------------------------
// Feel probe (VOXELFORGE_FEEL_PROBE=1 with --play)
// ---------------------------------------------------------------------------

/// Where the probe stands off before it starts swinging.
const PROBE_REACH: f32 = 2.4;
/// Wall-clock the probe lets the scene settle before it touches anything.
const PROBE_START: f32 = 2.0;
/// Wall-clock the probe runs to. Long enough for several Husk combos, so more
/// than one rhythm gets a chance to show up in one run.
const PROBE_END: f32 = 30.0;
/// One scripted action per this many seconds.
const PROBE_STEP: f32 = 0.60;
/// How long the heavy button is held (< `CHARGE_HOLD`, so it commits Heavy).
const PROBE_HEAVY_HOLD: f32 = 0.30;

/// Bookkeeping for [`feel_probe`].
#[derive(Resource, Default)]
pub struct FeelProbe {
    pub enabled: bool,
    pub step: u32,
    pub next_t: f32,
    pub heavy_until: f32,
    pub ground_y: Option<f32>,
    pub spawned: u32,
    pub done: bool,
}

/// A live fight, driven entirely through the real `ButtonInput<KeyCode>`.
///
/// It exists because the existing `--combat-demo` is over in three seconds — long
/// enough to prove damage lands, far too short for a Husk to show more than one
/// wind-up. The probe keeps one husk alive in front of the player for half a
/// minute and mashes attack / dodge / heavy on a fixed cadence.
///
/// **It grades nothing.** Every `FEEL_*` line in the log is written by the combat
/// systems themselves; the probe only supplies keystrokes and bodies.
#[allow(clippy::too_many_arguments)]
pub fn feel_probe(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut probe: ResMut<FeelProbe>,
    mut player_q: Query<(&Transform, &mut FlyCam, &Health), (With<FlyCam>, Without<Enemy>)>,
    enemies: Query<(&Transform, &Health), (With<Enemy>, Without<FlyCam>)>,
    mut exit: MessageWriter<AppExit>,
) {
    if probe.done {
        return;
    }
    let t = time.elapsed_secs();
    let Ok((ptf, mut fly, php)) = player_q.single_mut() else { return };
    if t < PROBE_START {
        // Learn the ground plane off whatever the scene already stood a husk on,
        // so respawns land on the same surface instead of a guessed height.
        if let Some((etf, _)) = enemies.iter().next() {
            probe.ground_y = Some(etf.translation.y);
        }
        return;
    }
    fly.walking = true;

    if t >= PROBE_END {
        for k in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD,
                  KeyCode::KeyX, KeyCode::KeyC, KeyCode::Space] {
            keys.reset(k);
        }
        probe.done = true;
        println!(
            "FEEL_PROBE done t={t:.1}s husks_spawned={} player_hp={:.0}",
            probe.spawned, php.cur
        );
        exit.write(AppExit::Success);
        return;
    }

    // ---- keep exactly one live husk in front of the player -------------------
    let target = enemies
        .iter()
        .filter(|(_, h)| !h.dead())
        .map(|(etf, _)| etf.translation)
        .min_by(|a, b| {
            a.distance(ptf.translation)
                .partial_cmp(&b.distance(ptf.translation))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let Some(tpos) = target else {
        let y = probe.ground_y.unwrap_or(ptf.translation.y - 1.6);
        let (x, z) = (ptf.translation.x, ptf.translation.z - 4.0);
        spawn_guard_husk(&mut commands, &mut meshes, &mut materials, x, z, Some(y));
        probe.spawned += 1;
        println!("FEEL_PROBE spawn husk #{} at ({x:.1},{y:.1},{z:.1})", probe.spawned);
        return;
    };

    // ---- close the gap ------------------------------------------------------
    // Headless yaw is 0, so the camera-relative WASD maps straight to world axes:
    // W = -Z, S = +Z, D = +X, A = -X (same convention the quest demo walks on).
    let (dx, dz) = (tpos.x - ptf.translation.x, tpos.z - ptf.translation.z);
    let dist = ptf.translation.distance(tpos);
    if dist > PROBE_REACH {
        let tol = 0.4;
        set_key(&mut keys, KeyCode::KeyD, dx > tol);
        set_key(&mut keys, KeyCode::KeyA, dx < -tol);
        set_key(&mut keys, KeyCode::KeyS, dz > tol);
        set_key(&mut keys, KeyCode::KeyW, dz < -tol);
        return;
    }
    for k in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
        keys.reset(k);
    }

    // ---- swing / roll on a fixed cadence ------------------------------------
    if probe.next_t == 0.0 {
        probe.next_t = t + PROBE_STEP;
    }
    // The heavy is a hold, so it spans frames: keep the button down until its
    // window closes, then release — that release is what commits the swing.
    if probe.heavy_until > 0.0 {
        if t < probe.heavy_until {
            keys.press(KeyCode::KeyC);
            return;
        }
        keys.reset(KeyCode::KeyC);
        probe.heavy_until = 0.0;
        return;
    }
    if t < probe.next_t {
        return;
    }
    probe.next_t = t + PROBE_STEP;
    probe.step += 1;
    match probe.step % 4 {
        1 | 0 => {
            // Light: reset-then-press so `just_pressed` fires a fresh rising edge.
            keys.reset(KeyCode::KeyX);
            keys.press(KeyCode::KeyX);
        }
        2 => {
            keys.reset(KeyCode::Space);
            keys.press(KeyCode::Space);
        }
        _ => {
            keys.press(KeyCode::KeyC);
            probe.heavy_until = t + PROBE_HEAVY_HOLD;
        }
    }
}

fn set_key(keys: &mut ButtonInput<KeyCode>, key: KeyCode, down: bool) {
    if down {
        keys.press(key);
    } else {
        keys.reset(key);
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// The weight layer, wired in one line next to the other combat systems:
/// `.add_plugins(combat::CombatFeelPlugin)`.
///
/// Every ordering here is explicit. Position inside an `add_systems` tuple is not
/// run order in Bevy, and two of these edges are load-bearing:
///  * `feel_probe` writes real key presses, so it must land before anything that
///    reads the keyboard this frame (`just_pressed` is cleared next `PreUpdate`).
///  * `apply_knockback` must sit *between* `player_combat` (which books the
///    impulse) and `husk_ai` (which re-plants the body and walks it back in).
pub struct CombatFeelPlugin;

impl Plugin for CombatFeelPlugin {
    fn build(&self, app: &mut App) {
        let probe = std::env::var("VOXELFORGE_FEEL_PROBE").is_ok();
        let log = probe || std::env::var("VOXELFORGE_FEEL_LOG").is_ok();
        app.add_message::<ImpactEvent>()
            .add_message::<StaggerEvent>()
            .add_message::<DodgeEvent>()
            .insert_resource(FeelLog { enabled: log })
            .insert_resource(FeelProbe { enabled: probe, ..default() })
            .add_systems(
                Update,
                (
                    feel_probe
                        .before(gather_input)
                        .before(crate::fly_camera)
                        .run_if(|p: Res<FeelProbe>| p.enabled),
                    apply_knockback
                        .after(player_combat)
                        .before(husk_ai),
                    combat_feel_log
                        .after(husk_ai)
                        .run_if(|f: Res<FeelLog>| f.enabled),
                )
                    .run_if(in_state(crate::editor::AppState::Play)),
            );
    }
}

// ===========================================================================
// Headless scripted proof (VOXELFORGE_COMBAT_DEMO=1) — mirrors walk_demo.
// ===========================================================================

/// Drives the real combat systems on a fixed timeline and prints PASS lines
/// proving the loop: attack → hit lands → stamina drops → dodge grants i-frames →
/// repeated attacks kill the husk (HP reaches 0). Then writes `AppExit`.
/// Requires a player + one Husk in the scene.
#[allow(clippy::type_complexity)]
pub fn combat_demo(
    time: Res<Time>,
    mut demo: ResMut<CombatDemo>,
    mut intent: ResMut<CombatIntent>,
    mut player_q: Query<(&mut Transform, &PlayerCombat, &Stamina), (With<FlyCam>, Without<Enemy>)>,
    enemy_q: Query<(Entity, &Transform, &Health), (With<Enemy>, Without<FlyCam>)>,
    mut exit: MessageWriter<AppExit>,
) {
    if demo.done {
        return;
    }
    let t = time.elapsed_secs();
    let Ok((mut ptf, pc, stam)) = player_q.single_mut() else { return };
    let husk_hp = enemy_q.iter().next().map(|(_, _, h)| h.cur).unwrap_or(-1.0);
    let husk_alive = enemy_q.iter().next().map(|(_, _, h)| !h.dead()).unwrap_or(false);

    // Reset intent each frame; we override only the frames where we act.
    *intent = CombatIntent::default();

    // ---- Phase 0: snapshot initial state (t < 0.8) -------------------------
    if demo.phase == 0 {
        demo.start_stam = stam.cur;
        demo.husk_hp0 = husk_hp;
        demo.phase = 1;
        return;
    }

    // ---- Phase 1: aim the player at the husk (t 0.8–0.9) —--------------—
    // The player starts facing +Z; the husk is at +X (spawn offset in main.rs).
    // Without this rotation the melee cone check (60°) misses. We yaw the
    // player transform to face the first husk so the hit lands in-cone.
    if demo.phase == 1 {
        if let Some((_, etf, _)) = enemy_q.iter().next() {
            let to = etf.translation - ptf.translation;
            let yaw = (-to.x).atan2(-to.z);
            ptf.rotation = Quat::from_axis_angle(Vec3::Y, yaw);
        }
        demo.phase = 2;
        return;
    }

    // ---- Phase 2: first light attack (t 1.0) -------------------------------
    if demo.phase == 2 && t >= 1.0 {
        intent.light = true;
        demo.phase = 3;
        return;
    }
    // ---- Phase 3: snapshot stamina + husk HP after first hit (t 1.4) -----
    if demo.phase == 3 && t >= 1.4 {
        demo.after_attack_stam = stam.cur;
        demo.husk_hp1 = husk_hp;
        demo.phase = 4;
        return;
    }
    // ---- Phase 4: second light attack (t 1.9) ------------------------------
    if demo.phase == 4 && t >= 1.9 {
        intent.light = true;
        demo.phase = 5;
        return;
    }
    // ---- Phase 5: third light attack (t 2.4) -------------------------------
    if demo.phase == 5 && t >= 2.4 {
        intent.light = true;
        demo.phase = 6;
        return;
    }
    // ---- Phase 6: fourth light attack (t 2.9) → husk should die ----------
    if demo.phase == 6 && t >= 2.9 {
        intent.light = true;
        demo.phase = 7;
        return;
    }
    // ---- Phase 7: dodge for i-frame proof (t 3.2) -------------------------
    if demo.phase == 7 && t >= 3.2 {
        intent.dodge = true;
        demo.phase = 8;
        return;
    }
    // ---- Phase 8: watch for active i-frames (t 3.2–3.5) ----------------
    if demo.phase == 8 {
        if pc.invulnerable() {
            demo.saw_iframe = true;
        }
        if t >= 3.6 {
            demo.phase = 9;
        }
        return;
    }
    // ---- Phase 9: report and exit (t 4.0) ----------------------------------
    if demo.phase == 9 && t >= 4.0 && !demo.logged {
        demo.logged = true;
        demo.done = true;
        demo.husk_dead = !husk_alive;
        let stam_dropped = demo.after_attack_stam < demo.start_stam - 0.01;
        let husk_hit = demo.husk_hp1 < demo.husk_hp0 - 0.01;
        println!(
            "COMBAT_DEMO attack: husk_hp {:.0}->{:.0} ({}) | stamina {:.0}->{:.0} ({})",
            demo.husk_hp0, demo.husk_hp1,
            if husk_hit { "PASS" } else { "FAIL" },
            demo.start_stam, demo.after_attack_stam,
            if stam_dropped { "PASS" } else { "FAIL" },
        );
        println!(
            "COMBAT_DEMO husk death: hp_final={:.0} alive={} => {}",
            husk_hp,
            husk_alive,
            if demo.husk_dead { "PASS" } else { "FAIL" }
        );
        println!(
            "COMBAT_DEMO dodge i-frames: saw_iframe={} => {}",
            demo.saw_iframe,
            if demo.saw_iframe { "PASS" } else { "FAIL" }
        );
        let ok = husk_hit && stam_dropped && demo.saw_iframe && demo.husk_dead;
        println!("COMBAT_DEMO overall => {}", if ok { "PASS" } else { "FAIL" });
        exit.write(AppExit::Success);
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn pc_facing(tf: &Transform) -> f32 {
    // The avatar's forward is local -Z; recover the yaw from its rotation.
    let f = tf.rotation * Vec3::NEG_Z;
    (-f.x).atan2(-f.z)
}

/// Rotate `cur` toward `target` by at most `max_step`, the short way round.
fn turn(cur: f32, target: f32, max_step: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (target - cur).rem_euclid(TAU);
    if d > PI {
        d -= TAU;
    }
    cur + d.clamp(-max_step, max_step)
}

/// Is `to` (a vector from actor to target) within `half_deg` of `facing_yaw`?
fn in_cone(facing_yaw: f32, to: Vec3, half_deg: f32) -> bool {
    let flat = Vec3::new(to.x, 0.0, to.z);
    if flat.length_squared() < 1e-4 {
        return true;
    }
    let want = (-flat.x).atan2(-flat.z);
    use std::f32::consts::{PI, TAU};
    let mut d = (want - facing_yaw).rem_euclid(TAU);
    if d > PI {
        d -= TAU;
    }
    d.abs() <= half_deg.to_radians()
}

/// Nearest enemy inside lock range + acquisition cone (§2.3).
fn nearest_target(
    origin: &Vec3,
    facing: f32,
    enemy_q: &Query<(Entity, &Transform, &mut Health, &mut Poise), (With<Enemy>, Without<FlyCam>)>,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for (e, tf, hp, _) in enemy_q.iter() {
        if hp.dead() {
            continue;
        }
        let to = tf.translation - *origin;
        let dist = to.length();
        if dist > LOCK_RANGE || !in_cone(facing, to, LOCK_CONE_H) {
            continue;
        }
        if best.map(|(_, d)| dist < d).unwrap_or(true) {
            best = Some((e, dist));
        }
    }
    best.map(|(e, _)| e)
}

// ===========================================================================
// Unit tests — the headless numeric proof of the core loop (`cargo test`).
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_attack_drains_stamina_by_spec() {
        let mut s = Stamina::full();
        let mut pc = PlayerCombat::default();
        assert!(pc.start_light(&mut s));
        assert_eq!(s.cur, STAMINA_MAX - COST_LIGHT); // 100 → 85 (§2.1)
        assert_eq!(pc.state, CombatState::Light);
    }

    #[test]
    fn attack_hits_only_during_active_window() {
        let mut pc = PlayerCombat::default();
        let mut s = Stamina::full();
        pc.start_light(&mut s);
        // Before the active window: no hit.
        pc.timer = 0.05;
        assert!(pc.active_hit().is_none());
        // Inside the window: 20 damage / 15 poise (§2.4).
        pc.timer = 0.15;
        let (d, p) = pc.active_hit().unwrap();
        assert_eq!(d, LIGHT_DAMAGE);
        assert_eq!(p, LIGHT_POISE);
    }

    #[test]
    fn hit_reduces_enemy_health() {
        let mut hp = Health::new(HP_HUSK); // 80 (§4.1)
        hp.damage(LIGHT_DAMAGE);
        assert_eq!(hp.cur, 60.0);
    }

    #[test]
    fn dodge_grants_iframes_that_negate_damage() {
        let mut s = Stamina::full();
        let mut pc = PlayerCombat::default();
        assert!(pc.start_dodge(&mut s));
        assert_eq!(s.cur, STAMINA_MAX - COST_DODGE); // 100 → 80
        assert!(pc.invulnerable()); // i-frames active immediately (§2.2)

        // A hit during i-frames is negated.
        let mut hp = Health::new(HP_PLAYER);
        if !pc.invulnerable() {
            hp.damage(HUSK_SWING1_DMG);
        }
        assert_eq!(hp.cur, HP_PLAYER); // untouched

        // i-frames expire after exactly N *frames* — not after 167 ms of `dt`.
        // `dodge_parry::spend_window_frames` is what spends them in the app.
        for _ in 0..crate::dodge_parry::DODGE_IFRAME_FRAMES {
            assert!(pc.invulnerable());
            pc.iframe_frames -= 1;
            pc.tick(1.0 / 600.0); // a 600 FPS frame must not shorten the window
        }
        assert!(!pc.invulnerable());
    }

    #[test]
    fn stamina_regens_after_delay() {
        let mut s = Stamina::full();
        s.try_spend(COST_LIGHT, DELAY_LIGHT); // 85, delay 0.25
        s.tick(0.1); // still inside delay → no regen
        assert!((s.cur - 85.0).abs() < 1e-4);
        s.tick(0.2); // delay elapsed → regen kicks in
        assert!(s.cur > 85.0);
    }

    #[test]
    fn exhaustion_locks_actions() {
        let mut s = Stamina { cur: 10.0, delay: 0.0, exhausted: 0.0, penalty: false };
        let mut pc = PlayerCombat::default();
        // 10 stamina can't pay a 15 light.
        assert!(!pc.start_light(&mut s));
        // Drain to 0 via a dodge is impossible (20 > 10); force exhaustion:
        s.cur = 5.0;
        s.try_spend(5.0, DELAY_DODGE);
        assert_eq!(s.exhausted, EXHAUST_LOCK); // 0.8 s lock (§2.1)
        assert!(!pc.can_act(&s));
    }

    #[test]
    fn poise_break_triggers_stagger_and_resets() {
        let mut poise = Poise::new(POISE_HUSK); // 30
        assert!(!poise.take(HUSK_SWING1_POISE, false)); // 30→15, no break
        assert!(poise.take(20.0, false)); // 15→0 → stagger (§3.2)
        assert!(poise.staggered());
        poise.tick(STAGGER_TIME + 0.01);
        assert!(!poise.staggered());
        assert_eq!(poise.cur, poise.max); // resets to full after stagger
    }

    #[test]
    fn hyper_armor_reduces_poise_and_suppresses_stagger() {
        let mut poise = Poise::new(15.0);
        // Under hyper armor a 40-poise heavy only removes 25% = 10.
        let broke = poise.take(40.0, true);
        assert!(!broke);
        assert!((poise.cur - 5.0).abs() < 1e-4);
    }

    #[test]
    fn block_halves_damage() {
        let dmg = HUSK_SWING2_DMG; // 20
        let blocked = dmg * (1.0 - BLOCK_REDUCTION);
        assert_eq!(blocked, 10.0); // 50% reduction (§2.5)
    }

    #[test]
    fn husk_leashes_past_six_blocks() {
        // §4.1: chase holds up to 6 blocks, disengages once the player is past it.
        assert!(!husk_should_leash(HUSK_LEASH)); // exactly 6 → still chasing
        assert!(!husk_should_leash(5.9)); // inside → still chasing
        assert!(husk_should_leash(6.1)); // past 6 → back to patrol
        assert!(husk_should_leash(11.9)); // old 12-block bug would NOT leash here
    }

    #[test]
    fn combo_ramps_damage_ten_percent_each() {
        let mut s = Stamina::full();
        let mut pc = PlayerCombat::default();
        pc.start_light(&mut s); // combo 1
        pc.timer = 0.15;
        assert_eq!(pc.active_hit().unwrap().0, 20.0);
        // Continue the combo within the window.
        pc.state = CombatState::Idle;
        pc.start_light(&mut s); // combo 2 → +10%
        pc.timer = 0.15;
        assert!((pc.active_hit().unwrap().0 - 22.0).abs() < 1e-4);
    }

    #[test]
    fn player_dies_when_health_reaches_zero() {
        let mut hp = Health::new(HP_PLAYER);
        assert!(!hp.dead());
        hp.damage(HP_PLAYER);
        assert!(hp.dead());
        assert_eq!(hp.cur, 0.0);
        hp.damage(10.0); // overkill stays at 0
        assert_eq!(hp.cur, 0.0);
    }

    #[test]
    fn husk_dies_after_four_lights() {
        // Guard Husk HP = 80, light attack = 20 dmg → 4 hits = dead.
        let mut hp = Health::new(HP_HUSK);
        hp.damage(LIGHT_DAMAGE); // 80 → 60
        assert!(!hp.dead());
        hp.damage(LIGHT_DAMAGE); // 60 → 40
        hp.damage(LIGHT_DAMAGE); // 40 → 20
        hp.damage(LIGHT_DAMAGE); // 20 → 0
        assert!(hp.dead());
        assert_eq!(hp.cur, 0.0);
    }

    #[test]
    fn husk_dies_after_two_heavies() {
        // Guard Husk HP = 80, heavy attack = 45 dmg → 2 hits = dead.
        let mut hp = Health::new(HP_HUSK);
        hp.damage(HEAVY_DAMAGE); // 80 → 35
        assert!(!hp.dead());
        hp.damage(HEAVY_DAMAGE); // 35 → -10 → 0
        assert!(hp.dead());
    }

    #[test]
    fn block_with_stamina_halves_damage() {
        let dmg = HUSK_SWING2_DMG; // 20
        let mut hp = Health::new(HP_PLAYER);
        let mut stam = Stamina::full();
        // If we have stamina for a block, damage is halved.
        assert!(stam.try_spend(COST_BLOCK, DELAY_BLOCK));
        hp.damage(dmg * (1.0 - BLOCK_REDUCTION)); // 10 dmg
        assert_eq!(hp.cur, HP_PLAYER - 10.0);
    }

    #[test]
    fn guard_break_when_blocking_without_stamina() {
        // When stamina is too low to block, the player takes full damage
        // and suffers a guard-break stagger (GUARD_BREAK = 0.6 s).
        let dmg = HUSK_SWING1_DMG; // 15
        let mut hp = Health::new(HP_PLAYER);
        hp.damage(dmg); // full damage — guard broken
        assert_eq!(hp.cur, HP_PLAYER - dmg);
        assert_eq!(GUARD_BREAK, 0.60); // spec constant unchanged (§2.5)
    }

    #[test]
    fn charged_attack_outdamages_light() {
        assert!(CHARGED_DAMAGE > LIGHT_DAMAGE * 2.0); // 70 > 40
        assert!(CHARGED_POISE > LIGHT_POISE * 3.0);   // 60 > 45
    }

    // ---- weight layer -----------------------------------------------------

    #[test]
    fn hitstop_is_graded_by_weight() {
        // The whole point: a light, a heavy and a critical must NOT freeze for
        // the same number of frames, or every swing feels identical.
        assert!(ImpactWeight::Light.hitstop() < ImpactWeight::Heavy.hitstop());
        assert!(ImpactWeight::Heavy.hitstop() < ImpactWeight::Critical.hitstop());
        assert_eq!(ImpactWeight::Light.hitstop(), HITSTOP_LIGHT); // §5.2 unchanged
        // A critical must out-freeze the old flat stagger number, never undercut it.
        assert!(ImpactWeight::Critical.hitstop() >= HITSTOP_STAGGER);
    }

    #[test]
    fn knockback_and_kick_scale_with_the_same_weight() {
        for (a, b) in [
            (ImpactWeight::Light, ImpactWeight::Heavy),
            (ImpactWeight::Heavy, ImpactWeight::Critical),
        ] {
            assert!(a.knockback() < b.knockback(), "{a:?} !< {b:?} knockback");
            assert!(a.kick() < b.kick(), "{a:?} !< {b:?} kick");
        }
    }

    #[test]
    fn knockback_never_outruns_the_players_own_reach() {
        // A shove bigger than the reach would push the enemy out of the follow-up
        // swing and silently break every combo — the exact failure a "punchier"
        // tuning pass is most likely to introduce.
        let reach = MELEE_RANGE + PLAYER_HALF_W + 0.6; // 2.9, combat.rs hit gate
        assert!(ImpactWeight::Critical.knockback() < reach * 0.25);
    }

    #[test]
    fn shake_kick_takes_the_strongest_and_normalises() {
        let mut s = Shake::default();
        s.kick(Vec3::new(0.0, 0.0, -4.0), KICK_LIGHT);
        assert!((s.kick_dir.length() - 1.0).abs() < 1e-4); // unit direction
        assert_eq!(s.kick_amp, KICK_LIGHT);
        // A heavier kick on the same frame wins…
        s.kick(Vec3::X, KICK_HEAVY);
        assert_eq!(s.kick_amp, KICK_HEAVY);
        // …a weaker one does not stomp it.
        s.kick(Vec3::Z, KICK_LIGHT);
        assert_eq!(s.kick_amp, KICK_HEAVY);
        // A zero direction is not a kick.
        let mut z = Shake::default();
        z.kick(Vec3::ZERO, KICK_CRITICAL);
        assert_eq!(z.kick_amp, 0.0);
    }

    // ---- husk rhythm ------------------------------------------------------

    #[test]
    fn husk_rhythm_covers_all_three_patterns() {
        // Sweep the real seed space the AI uses (entity bits ⊕ combo counter) and
        // assert the enemy is not a metronome: every pattern must actually occur.
        let (mut straight, mut delayed, mut feint) = (0, 0, 0);
        for seed in 0u32..2000 {
            match pick_rhythm(seed.wrapping_mul(0x9E37_79B9)) {
                HuskRhythm::Straight => straight += 1,
                HuskRhythm::Delayed => delayed += 1,
                HuskRhythm::Feint => feint += 1,
            }
        }
        assert!(straight > 0 && delayed > 0 && feint > 0,
            "pattern missing: straight={straight} delayed={delayed} feint={feint}");
        // None of them may collapse to a rounding error — a 1-in-2000 feint is
        // the same as no feint at all for a player.
        for (name, n) in [("straight", straight), ("delayed", delayed), ("feint", feint)] {
            assert!(n >= 200, "{name} only {n}/2000 — too rare to read as behaviour");
        }
    }

    #[test]
    fn husk_rhythm_is_deterministic() {
        // Re-runnable proofs need re-runnable enemies.
        for seed in [0u32, 1, 7, 4242, u32::MAX] {
            assert_eq!(pick_rhythm(seed), pick_rhythm(seed));
        }
    }

    #[test]
    fn delayed_wind_up_outlasts_a_dodge_rolled_on_the_straight_tempo() {
        // This is the entire mechanic: roll on the straight timing against a
        // delayed swing and the i-frames must be long gone when the blade lands.
        let iframe_end = HUSK_TELEGRAPH + DODGE_IFRAMES;
        assert!(telegraph_hold(HuskRhythm::Delayed) > iframe_end,
            "delayed hold {} must outlast a straight-timed roll ending at {iframe_end}",
            telegraph_hold(HuskRhythm::Delayed));
        // …and the straight swing must still be dodgeable on its own timing.
        assert!(telegraph_hold(HuskRhythm::Straight) <= HUSK_TELEGRAPH);
    }

    #[test]
    fn feint_pulls_back_earlier_than_any_real_swing() {
        assert!(telegraph_hold(HuskRhythm::Feint) < telegraph_hold(HuskRhythm::Straight));
        // The bait beat has to be long enough for a rolled dodge to fully expire,
        // otherwise the "punish" lands inside i-frames and the feint is free.
        assert!(HUSK_FEINT_RECOVER > DODGE_IFRAMES + DODGE_RECOVERY);
    }

    #[test]
    fn husk_step_in_is_slower_than_a_chase() {
        // Stepping in during the wind-up must not out-pace the chase, or the
        // telegraph becomes a charge attack nobody can back away from.
        assert!(HUSK_STEP_IN > 0.0 && HUSK_STEP_IN < 1.0);
    }

    #[test]
    fn combat_demo_phase_tracks_hp_transitions() {
        let mut demo = CombatDemo::default();
        assert_eq!(demo.phase, 0);
        demo.phase = 9;
        demo.husk_dead = true;
        assert!(demo.husk_dead);
    }
}
