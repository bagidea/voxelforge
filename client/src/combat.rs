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
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use crate::audio::SfxEvent;
use crate::{FlyCam, PLAYER_HALF_W};
use voxelforge_sim::worldgen::terrain_height;

// ===========================================================================
// CombatConfig — single source of truth for every tunable combat number.
// Change a value here, rebuild, done. The old `pub const` names below are thin
// backward-compatible aliases; new code should read `COMBAT.field` directly.
//
// Traceable to combat-design.md §2 / §3 / §4 / §5 / §6.
// ===========================================================================

/// Every numeric knob the soulslike combat loop reads, in one struct.
///
/// v1 has one player build and one enemy archetype (Guard Husk), so a `const`
/// instance is the right shape — no runtime loader, no data file, but every
/// number lives in exactly one named place and is greppable / diffable against
/// `combat-design.md`'s §-tags.
#[derive(Clone, Debug)]
pub struct CombatConfig {
    // -- Stamina (§2.1 / §6) ------------------------------------------------
    pub stamina_max: f32,
    pub stamina_regen: f32,
    pub cost_light: f32,
    pub cost_heavy: f32,
    pub cost_charged: f32,
    pub cost_dodge: f32,
    pub cost_block: f32,
    pub cost_parry: f32,
    pub delay_light: f32,
    pub delay_heavy: f32,
    pub delay_charged: f32,
    pub delay_dodge: f32,
    pub delay_block: f32,
    pub delay_parry: f32,
    pub exhaust_lock: f32,
    pub exhaust_clear: f32,

    // -- Dodge / roll (§2.2 / §6) ------------------------------------------
    pub dodge_iframes: f32,
    pub dodge_recovery: f32,
    pub dodge_distance: f32,

    // -- Lock-on (§2.3 / §6) ------------------------------------------------
    pub lock_range: f32,
    pub lock_cone_h: f32,
    pub lock_snap: f32,
    pub lock_drop_range: f32,

    // -- Attacks (§2.4 / §3) ------------------------------------------------
    pub light_damage: f32,
    pub light_poise: f32,
    pub light_time: f32,
    pub heavy_damage: f32,
    pub heavy_poise: f32,
    pub heavy_time: f32,
    pub heavy_hyper: f32,
    pub charged_damage: f32,
    pub charged_poise: f32,
    pub charged_time: f32,
    pub charge_hold: f32,
    pub combo_reset: f32,
    pub combo_step: f32,
    pub melee_range: f32,
    pub melee_cone: f32,
    /// Active-frame windows inside an attack (windup → active → recover).
    pub light_active: (f32, f32),
    pub heavy_active: (f32, f32),
    pub charged_active: (f32, f32),

    // -- Block & parry (§2.5) -----------------------------------------------
    pub block_reduction: f32,
    pub guard_break: f32,
    pub parry_window: f32,
    pub parry_posture: f32,
    pub parry_punish: f32,
    pub parry_punish_mult: f32,
    pub parry_fail_mult: f32,
    pub parry_fail_recover: f32,

    // -- Poise / posture (§3.2) ---------------------------------------------
    pub poise_player: f32,
    pub poise_husk: f32,
    pub poise_regen: f32,
    pub poise_regen_delay: f32,
    pub stagger_time: f32,
    pub stagger_dmg_mult: f32,
    pub hyper_armor_reduce: f32,

    // -- Health (§3.1) ------------------------------------------------------
    pub hp_player: f32,
    pub hp_husk: f32,

    // -- Guard Husk (§4.1) --------------------------------------------------
    pub husk_walk: f32,
    pub husk_turn: f32,
    pub husk_telegraph: f32,
    pub husk_swing1_dmg: f32,
    pub husk_swing1_poise: f32,
    pub husk_swing2_dmg: f32,
    pub husk_swing2_poise: f32,
    pub husk_active: f32,
    pub husk_gap: f32,
    pub husk_combo_pause: f32,
    pub husk_aggro_range: f32,
    pub husk_leash: f32,
    /// How long a husk sweeps the last-seen spot before giving up (§4.1 trail).
    pub husk_search_time: f32,

    // -- Guard Husk rhythm (§4.1 extended) ----------------------------------
    pub husk_telegraph_delayed: f32,
    pub husk_feint_hold: f32,
    pub husk_feint_recover: f32,
    pub husk_step_in: f32,

    // -- Guard Husk perception & tactics (§4.1 extended) -------------------
    // The "feels like a real enemy" pass: a detect→alert beat, a second
    // (gap-closing) attack, circling re-engagement, and squad separation so a
    // pack never stacks into one swinging blob.
    pub husk_alert_time: f32,        // "noticed you" beat before the chase commits
    pub husk_sight_half: f32,        // vision cone half-angle (deg)
    pub husk_hear_range: f32,        // aggro radius when the player is noisy
    pub husk_close_sense: f32,       // always-sense radius (behind the cone)
    pub husk_sprint_noise: f32,      // player speed (b/s) above which it is "heard"
    pub husk_lunge_range: f32,       // closes from mid-range where melee can't reach
    pub husk_lunge_dmg: f32,
    pub husk_lunge_poise: f32,
    pub husk_lunge_wind: f32,        // lunge telegraph hold (longer, readable)
    pub husk_lunge_dash: f32,        // dash travel time
    pub husk_lunge_speed: f32,       // dash speed (b/s)
    pub husk_lunge_cooldown: f32,
    pub husk_reposition_time: f32,   // how long it circles before re-committing
    pub husk_strafe_speed: f32,      // circling speed (× walk)
    pub husk_backoff_range: f32,     // backs off when closer than this while circling
    pub husk_separation: f32,        // desired min spacing between squadmates
    pub husk_separation_strength: f32,
    pub husk_attack_slots: u32,      // max husks whose swings may be live at once

    // -- Evade (the "it dodged me" beat) -----------------------------------
    pub husk_evade_range: f32,       // only bothers when the swing could reach it
    pub husk_evade_time: f32,        // burst duration
    pub husk_evade_speed: f32,       // burst speed (b/s)
    pub husk_evade_cooldown: f32,    // per-husk, so it cannot chain-dodge
    pub husk_evade_back: f32,        // how much of the hop is backwards vs lateral

    // -- Feedback (§5.2 / §5.3) ---------------------------------------------
    pub hitstop_light: f32,
    pub hitstop_parry: f32,
    pub hitstop_enemy: f32,
    pub hitstop_stagger: f32,
    pub hitstop_heavy: f32,
    pub hitstop_critical: f32,
    pub shake_light: (f32, f32),
    pub shake_heavy: (f32, f32),
    pub shake_enemy_hit: (f32, f32),
    pub shake_parry: (f32, f32),
    pub shake_stagger: (f32, f32),

    // -- Weight layer: knockback --------------------------------------------
    pub knockback_light: f32,
    pub knockback_heavy: f32,
    pub knockback_critical: f32,
    pub knockback_time: f32,

    // -- Weight layer: camera kick ------------------------------------------
    pub kick_light: f32,
    pub kick_heavy: f32,
    pub kick_critical: f32,
    pub kick_taken: f32,
    pub kick_time: f32,
}

/// The live tuning instance. Every system below reads from this; no magic
/// number is hardcoded inline.
pub const COMBAT: CombatConfig = CombatConfig {
    // -- Stamina (§2.1 / §6) ------------------------------------------------
    stamina_max: 100.0,
    stamina_regen: 40.0,
    cost_light: 15.0,
    cost_heavy: 35.0,
    cost_charged: 50.0,
    cost_dodge: 20.0,
    cost_block: 15.0,
    cost_parry: 10.0,
    delay_light: 0.25,
    delay_heavy: 0.40,
    delay_charged: 0.50,
    delay_dodge: 0.30,
    delay_block: 0.20,
    delay_parry: 0.35,
    exhaust_lock: 0.8,
    exhaust_clear: 20.0,

    // -- Dodge / roll (§2.2 / §6) ------------------------------------------
    dodge_iframes: 10.0 / 60.0,
    dodge_recovery: 12.0 / 60.0,
    dodge_distance: 2.5,

    // -- Lock-on (§2.3 / §6) ------------------------------------------------
    lock_range: 16.0,
    lock_cone_h: 45.0,
    lock_snap: 8.0,
    lock_drop_range: 20.0,

    // -- Attacks (§2.4 / §3) ------------------------------------------------
    light_damage: 20.0,
    light_poise: 15.0,
    light_time: 0.35,
    heavy_damage: 45.0,
    heavy_poise: 40.0,
    heavy_time: 0.85,
    heavy_hyper: 0.40,
    charged_damage: 70.0,
    charged_poise: 60.0,
    charged_time: 1.10,
    charge_hold: 0.60,
    combo_reset: 0.60,
    combo_step: 0.10,
    melee_range: 2.0,
    melee_cone: 60.0,
    light_active: (0.12, 0.22),
    heavy_active: (0.55, 0.72),
    charged_active: (0.75, 0.95),

    // -- Block & parry (§2.5) -----------------------------------------------
    block_reduction: 0.50,
    guard_break: 0.60,
    parry_window: 0.20,
    parry_posture: 25.0,
    parry_punish: 1.20,
    parry_punish_mult: 1.25,
    parry_fail_mult: 1.25,
    parry_fail_recover: 0.50,

    // -- Poise / posture (§3.2) ---------------------------------------------
    poise_player: 40.0,
    poise_husk: 30.0,
    poise_regen: 10.0,
    poise_regen_delay: 2.0,
    stagger_time: 1.5,
    stagger_dmg_mult: 1.30,
    hyper_armor_reduce: 0.75,

    // -- Health (§3.1) ------------------------------------------------------
    hp_player: 100.0,
    hp_husk: 80.0,

    // -- Guard Husk (§4.1) --------------------------------------------------
    husk_walk: 1.5,
    husk_turn: 2.0,
    husk_telegraph: 0.8,
    husk_swing1_dmg: 15.0,
    husk_swing1_poise: 15.0,
    husk_swing2_dmg: 20.0,
    husk_swing2_poise: 20.0,
    husk_active: 0.2,
    husk_gap: 0.25,
    husk_combo_pause: 1.5,
    husk_aggro_range: 12.0,
    husk_leash: 6.0,
    husk_search_time: 3.0,

    // -- Guard Husk rhythm (§4.1 extended) ----------------------------------
    husk_telegraph_delayed: 1.55,
    husk_feint_hold: 0.42,
    husk_feint_recover: 0.55,
    husk_step_in: 0.5,

    // -- Guard Husk perception & tactics (§4.1 extended) -------------------
    husk_alert_time: 0.45,
    husk_sight_half: 55.0,
    husk_hear_range: 18.0,
    husk_close_sense: 4.5,
    husk_sprint_noise: 4.2,
    husk_lunge_range: 8.0,
    husk_lunge_dmg: 22.0,
    husk_lunge_poise: 30.0,
    husk_lunge_wind: 0.55,
    husk_lunge_dash: 0.30,
    husk_lunge_speed: 9.0,
    husk_lunge_cooldown: 4.5,
    husk_reposition_time: 1.1,
    husk_strafe_speed: 0.9,
    husk_backoff_range: 2.3,
    husk_separation: 2.4,
    husk_separation_strength: 1.0,
    husk_attack_slots: 2,

    // -- Evade (the "it dodged me" beat) -----------------------------------
    husk_evade_range: 3.6,
    husk_evade_time: 0.30,
    husk_evade_speed: 7.5,
    husk_evade_cooldown: 3.0,
    husk_evade_back: 0.55,

    // -- Feedback (§5.2 / §5.3) ---------------------------------------------
    hitstop_light: 0.080,
    hitstop_parry: 0.120,
    hitstop_enemy: 0.100,
    hitstop_stagger: 0.150,
    hitstop_heavy: 0.140,
    hitstop_critical: 0.200,
    shake_light: (0.04, 0.10),
    shake_heavy: (0.12, 0.22),
    shake_enemy_hit: (0.16, 0.28),
    // §5.3: a parry lands harder than a light tap — its own row, not the
    // light's, otherwise a successful deflect reads as a whiff.
    shake_parry: (0.12, 0.18),
    // §5.3: the stagger-break row — the loudest thing in the table, louder than
    // any swing or enemy hit, so a poise break shakes the camera no matter what
    // swing caused it.
    shake_stagger: (0.20, 0.30),

    // -- Weight layer: knockback --------------------------------------------
    knockback_light: 0.18,
    knockback_heavy: 0.32,
    knockback_critical: 0.55,
    knockback_time: 0.15,

    // -- Weight layer: camera kick ------------------------------------------
    kick_light: 0.045,
    kick_heavy: 0.120,
    kick_critical: 0.155,
    kick_taken: 0.130,
    kick_time: 0.16,
};

// ===========================================================================
// Backward-compatible aliases — prefer `COMBAT.field` in new code.
// These exist so anim.rs, dodge_parry.rs, scene.rs, quest.rs, and main.rs
// continue to compile without changes.
// ===========================================================================

// -- Stamina (§2.1 / §6) ----------------------------------------------------
pub const STAMINA_MAX: f32 = COMBAT.stamina_max;
pub const STAMINA_REGEN: f32 = COMBAT.stamina_regen;
pub const COST_LIGHT: f32 = COMBAT.cost_light;
pub const COST_HEAVY: f32 = COMBAT.cost_heavy;
pub const COST_CHARGED: f32 = COMBAT.cost_charged;
pub const COST_DODGE: f32 = COMBAT.cost_dodge;
pub const COST_BLOCK: f32 = COMBAT.cost_block;
pub const COST_PARRY: f32 = COMBAT.cost_parry;
pub const DELAY_LIGHT: f32 = COMBAT.delay_light;
pub const DELAY_HEAVY: f32 = COMBAT.delay_heavy;
pub const DELAY_CHARGED: f32 = COMBAT.delay_charged;
pub const DELAY_DODGE: f32 = COMBAT.delay_dodge;
pub const DELAY_BLOCK: f32 = COMBAT.delay_block;
pub const DELAY_PARRY: f32 = COMBAT.delay_parry;
pub const EXHAUST_LOCK: f32 = COMBAT.exhaust_lock;
pub const EXHAUST_CLEAR: f32 = COMBAT.exhaust_clear;

// -- Dodge / roll (§2.2 / §6) ----------------------------------------------
pub const DODGE_IFRAMES: f32 = COMBAT.dodge_iframes;
pub const DODGE_RECOVERY: f32 = COMBAT.dodge_recovery;
pub const DODGE_DISTANCE: f32 = COMBAT.dodge_distance;

// -- Lock-on (§2.3 / §6) ----------------------------------------------------
pub const LOCK_RANGE: f32 = COMBAT.lock_range;
pub const LOCK_CONE_H: f32 = COMBAT.lock_cone_h;
pub const LOCK_SNAP: f32 = COMBAT.lock_snap;
pub const LOCK_DROP_RANGE: f32 = COMBAT.lock_drop_range;

// -- Attacks (§2.4 / §3) ----------------------------------------------------
pub const LIGHT_DAMAGE: f32 = COMBAT.light_damage;
pub const LIGHT_POISE: f32 = COMBAT.light_poise;
pub const LIGHT_TIME: f32 = COMBAT.light_time;
pub const HEAVY_DAMAGE: f32 = COMBAT.heavy_damage;
pub const HEAVY_POISE: f32 = COMBAT.heavy_poise;
pub const HEAVY_TIME: f32 = COMBAT.heavy_time;
pub const HEAVY_HYPER: f32 = COMBAT.heavy_hyper;
pub const CHARGED_DAMAGE: f32 = COMBAT.charged_damage;
pub const CHARGED_POISE: f32 = COMBAT.charged_poise;
pub const CHARGED_TIME: f32 = COMBAT.charged_time;
pub const CHARGE_HOLD: f32 = COMBAT.charge_hold;
pub const COMBO_RESET: f32 = COMBAT.combo_reset;
pub const COMBO_STEP: f32 = COMBAT.combo_step;
pub const MELEE_RANGE: f32 = COMBAT.melee_range;
pub const MELEE_CONE: f32 = COMBAT.melee_cone;
pub const LIGHT_ACTIVE: (f32, f32) = COMBAT.light_active;
pub const HEAVY_ACTIVE: (f32, f32) = COMBAT.heavy_active;
pub const CHARGED_ACTIVE: (f32, f32) = COMBAT.charged_active;

// -- Block & parry (§2.5) ---------------------------------------------------
pub const BLOCK_REDUCTION: f32 = COMBAT.block_reduction;
pub const GUARD_BREAK: f32 = COMBAT.guard_break;
pub const PARRY_WINDOW: f32 = COMBAT.parry_window;
pub const PARRY_POSTURE: f32 = COMBAT.parry_posture;
pub const PARRY_PUNISH: f32 = COMBAT.parry_punish;
pub const PARRY_PUNISH_MULT: f32 = COMBAT.parry_punish_mult;
pub const PARRY_FAIL_MULT: f32 = COMBAT.parry_fail_mult;
pub const PARRY_FAIL_RECOVER: f32 = COMBAT.parry_fail_recover;

// -- Poise / posture (§3.2) -------------------------------------------------
pub const POISE_PLAYER: f32 = COMBAT.poise_player;
pub const POISE_HUSK: f32 = COMBAT.poise_husk;
pub const POISE_REGEN: f32 = COMBAT.poise_regen;
pub const POISE_REGEN_DELAY: f32 = COMBAT.poise_regen_delay;
pub const STAGGER_TIME: f32 = COMBAT.stagger_time;
pub const STAGGER_DMG_MULT: f32 = COMBAT.stagger_dmg_mult;
pub const HYPER_ARMOR_REDUCE: f32 = COMBAT.hyper_armor_reduce;

// -- Health (§3.1) ----------------------------------------------------------
pub const HP_PLAYER: f32 = COMBAT.hp_player;
pub const HP_HUSK: f32 = COMBAT.hp_husk;

// -- Guard Husk (§4.1) ------------------------------------------------------
pub const HUSK_WALK: f32 = COMBAT.husk_walk;
pub const HUSK_TURN: f32 = COMBAT.husk_turn;
pub const HUSK_TELEGRAPH: f32 = COMBAT.husk_telegraph;
pub const HUSK_SWING1_DMG: f32 = COMBAT.husk_swing1_dmg;
pub const HUSK_SWING1_POISE: f32 = COMBAT.husk_swing1_poise;
pub const HUSK_SWING2_DMG: f32 = COMBAT.husk_swing2_dmg;
pub const HUSK_SWING2_POISE: f32 = COMBAT.husk_swing2_poise;
pub const HUSK_ACTIVE: f32 = COMBAT.husk_active;
pub const HUSK_GAP: f32 = COMBAT.husk_gap;
pub const HUSK_COMBO_PAUSE: f32 = COMBAT.husk_combo_pause;
pub const HUSK_AGGRO_RANGE: f32 = COMBAT.husk_aggro_range;
pub const HUSK_LEASH: f32 = COMBAT.husk_leash;
pub const HUSK_SEARCH_TIME: f32 = COMBAT.husk_search_time;

// -- Feedback (§5.2 / §5.3) -------------------------------------------------
pub const HITSTOP_LIGHT: f32 = COMBAT.hitstop_light;
pub const HITSTOP_PARRY: f32 = COMBAT.hitstop_parry;
pub const HITSTOP_ENEMY: f32 = COMBAT.hitstop_enemy;
pub const HITSTOP_STAGGER: f32 = COMBAT.hitstop_stagger;
pub const SHAKE_LIGHT: (f32, f32) = COMBAT.shake_light;
pub const SHAKE_HEAVY: (f32, f32) = COMBAT.shake_heavy;
pub const SHAKE_ENEMY_HIT: (f32, f32) = COMBAT.shake_enemy_hit;
pub const SHAKE_PARRY: (f32, f32) = COMBAT.shake_parry;
pub const SHAKE_STAGGER: (f32, f32) = COMBAT.shake_stagger;

// -- Weight layer -----------------------------------------------------------
pub const HITSTOP_HEAVY: f32 = COMBAT.hitstop_heavy;
pub const HITSTOP_CRITICAL: f32 = COMBAT.hitstop_critical;
/// How far a connected blow shoves the target along the blade's direction. Small
/// on purpose — a souls-like nudges, it does not punt (a punt would push the
/// enemy out of the follow-up's reach and break every combo).
pub const KNOCKBACK_LIGHT: f32 = COMBAT.knockback_light;
pub const KNOCKBACK_HEAVY: f32 = COMBAT.knockback_heavy;
pub const KNOCKBACK_CRITICAL: f32 = COMBAT.knockback_critical;
/// The shove is spread over this window so the body slides, never teleports.
pub const KNOCKBACK_TIME: f32 = COMBAT.knockback_time;
/// Directional camera kick — rides on top of [`Shake`]'s omni-directional
/// rattle: the rattle says "something happened", the kick says "*that* way".
pub const KICK_LIGHT: f32 = COMBAT.kick_light;
pub const KICK_HEAVY: f32 = COMBAT.kick_heavy;
pub const KICK_CRITICAL: f32 = COMBAT.kick_critical;
pub const KICK_TAKEN: f32 = COMBAT.kick_taken;
pub const KICK_TIME: f32 = COMBAT.kick_time;

// -- Guard Husk rhythm (§4.1 extended) --------------------------------------
// One fixed 0.8 s wind-up is a metronome: after two swings the player has the
// timing and the fight is over as a threat. The signature of a soulslike boss is
// that the *same* wind-up resolves at different times, so the dodge has to be
// read, not memorised.
pub const HUSK_TELEGRAPH_DELAYED: f32 = COMBAT.husk_telegraph_delayed;
pub const HUSK_FEINT_HOLD: f32 = COMBAT.husk_feint_hold;
pub const HUSK_FEINT_RECOVER: f32 = COMBAT.husk_feint_recover;
/// A husk that has not closed the gap keeps stepping in *while* winding up (at
/// half walk speed). Standing still through a 1.5 s wind-up would let the player
/// simply back off, and it would let hit-knockback slide the fight apart.
pub const HUSK_STEP_IN: f32 = COMBAT.husk_step_in;

// -- Guard Husk perception & tactics (§4.1 extended) ------------------------
pub const HUSK_ALERT_TIME: f32 = COMBAT.husk_alert_time;
pub const HUSK_SIGHT_HALF: f32 = COMBAT.husk_sight_half;
pub const HUSK_HEAR_RANGE: f32 = COMBAT.husk_hear_range;
pub const HUSK_CLOSE_SENSE: f32 = COMBAT.husk_close_sense;
pub const HUSK_SPRINT_NOISE: f32 = COMBAT.husk_sprint_noise;
pub const HUSK_LUNGE_RANGE: f32 = COMBAT.husk_lunge_range;
pub const HUSK_LUNGE_DMG: f32 = COMBAT.husk_lunge_dmg;
pub const HUSK_LUNGE_POISE: f32 = COMBAT.husk_lunge_poise;
pub const HUSK_LUNGE_WIND: f32 = COMBAT.husk_lunge_wind;
pub const HUSK_LUNGE_DASH: f32 = COMBAT.husk_lunge_dash;
pub const HUSK_LUNGE_SPEED: f32 = COMBAT.husk_lunge_speed;
pub const HUSK_LUNGE_COOLDOWN: f32 = COMBAT.husk_lunge_cooldown;
pub const HUSK_REPOSITION_TIME: f32 = COMBAT.husk_reposition_time;
pub const HUSK_STRAFE_SPEED: f32 = COMBAT.husk_strafe_speed;
pub const HUSK_BACKOFF_RANGE: f32 = COMBAT.husk_backoff_range;
pub const HUSK_SEPARATION: f32 = COMBAT.husk_separation;
pub const HUSK_SEPARATION_STRENGTH: f32 = COMBAT.husk_separation_strength;
pub const HUSK_ATTACK_SLOTS: u32 = COMBAT.husk_attack_slots;

// -- Evade ------------------------------------------------------------------
pub const HUSK_EVADE_RANGE: f32 = COMBAT.husk_evade_range;
pub const HUSK_EVADE_TIME: f32 = COMBAT.husk_evade_time;
pub const HUSK_EVADE_SPEED: f32 = COMBAT.husk_evade_speed;
pub const HUSK_EVADE_COOLDOWN: f32 = COMBAT.husk_evade_cooldown;
pub const HUSK_EVADE_BACK: f32 = COMBAT.husk_evade_back;


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
    /// Spotted the player this frame — the observable "noticed you" beat. The
    /// husk plants its feet and snaps to face the player for `HUSK_ALERT_TIME`
    /// before committing to a chase. Anything that closes from aggro straight
    /// into a sprint reads as a homing missile, not a thing that *saw* you.
    Alert,
    Chase,
    Telegraph, // wind-up (honest — no active hitbox yet)
    Feint,     // wind-up aborted on purpose — no hitbox ever appears
    Swing1,
    Gap,
    Swing2,
    Recover,
    /// Circling / backing off between commitments. After a combo (or when the
    /// melee attack-slot is taken by a squadmate) the husk strafes around the
    /// player instead of beelining back in — it re-engages from a flank, so a
    /// pack never collapses into one clump swinging in unison.
    Reposition,
    /// Lost the player (leashed, or detection dropped while still committed):
    /// walks to the last place the player was *detected* (`Enemy::last_seen`),
    /// sweeps its sight cone around that spot for [`HUSK_SEARCH_TIME`], then
    /// gives up and walks home. Without this state a lost husk either kept
    /// homing on the true player position (wallhack chase) or snapped straight
    /// back to patrol (amnesia) — neither reads as a creature that lost a trail.
    Search,
    /// Wind-up for the gap-closing lunge — a distinct, longer telegraph (crouch
    /// back) so a kiting player reads the leap coming and can sidestep.
    LungeWind,
    /// The dash itself — travels forward and carries an active hitbox across its
    /// travel, punishing a player who keeps the husk at whip-range.
    LungeDash,
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
    /// Lunge cooldown remaining (counts down each frame). A husk that can leap
    /// on demand every step reads as a tickle-gun; the cooldown keeps the lunge
    /// a committed, readable threat.
    pub lunge_cd: f32,
    /// +1 / -1 — which way this husk circles the player while repositioning.
    /// Picked at spawn and occasionally flipped after a circuit so a pack orbits
    /// in mixed directions instead of marching in lockstep.
    pub strafe_dir: f32,
    /// Last state the AI logger reported for this husk. Tracks transitions: the
    /// log system prints exactly once per `state != logged_state`, so every
    /// patrol→alert→chase→telegraph→… move is surfaced without spamming.
    pub logged_state: HuskState,
    /// Which silhouette this body wears (`enemies.rs`, Monanisa's lane). The AI
    /// is kind-agnostic except for the telegraph arm pick (see `husk_telegraph`)
    /// — behaviour differences between kinds are a later decision, not baked in.
    pub kind: crate::enemies::EnemyKind,
    /// Where the player was last *actually detected*. A husk that loses the
    /// player (out of the sight cone, beyond close-sense, not heard) walks to
    /// this spot and sweeps for [`HUSK_SEARCH_TIME`] before giving up — the
    /// "followed my trail" beat, instead of omnisciently beelining or amnesiac
    /// snapping back to patrol.
    pub last_seen: Vec3,
}

impl Enemy {
    /// A fresh husk standing at `feet` on `surface`, wearing `kind`.
    fn at(feet: Vec3, surface: f32, kind: crate::enemies::EnemyKind) -> Self {
        Self {
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
            lunge_cd: 0.0,
            strafe_dir: 1.0,
            logged_state: HuskState::Patrol,
            kind,
            last_seen: feet,
        }
    }
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
    /// Switch to the next-nearest valid target while already locked on (§2.3).
    /// No-op when unlocked or when no other target is in range.
    pub lock_switch: bool,
}

/// Lock-on state (§2.3).
#[derive(Resource, Default)]
pub struct LockOn {
    pub target: Option<Entity>,
}

// ===========================================================================
// R-key routing — one key, two consumers
// ===========================================================================

/// What the R press resolved to this frame.
///
/// R is read by two independent systems: `gather_input` (lock-on toggle, §2.3)
/// and `quest::check_block_place_triggers` (the `place_block` objective). Bevy's
/// `ButtonInput::just_pressed` is a *query*, not a consume — the flag stays set
/// for every later reader until the next `PreUpdate` — so without an arbiter a
/// single press fired both in the same frame: the build objective completed AND
/// the camera snapped onto an enemy.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RKeyUse {
    /// Nobody has taken the press yet this frame.
    #[default]
    Unclaimed,
    /// A `place_block` objective was in reach → the press builds.
    QuestPlace,
    /// Nothing contextual wanted it → the press toggles lock-on.
    LockOn,
}

/// Per-frame owner of the R key. Cleared in `First`, claimed by exactly one
/// consumer per frame, read by everyone who wants to act on R.
///
/// The priority is encoded as *system order*, not as an if-chain spread across
/// two files: the contextual consumer runs first (`check_block_place_triggers`
/// is ordered `.before(combat::gather_input)`) and only claims when it can
/// actually act; lock-on is the fallback and takes whatever is left. First
/// claimer wins, every later claimer is denied and must leave the key alone.
#[derive(Resource, Default)]
pub struct RKeyRoute {
    route: RKeyUse,
    by: &'static str,
    frame: u64,
    /// `VOXELFORGE_RKEY_LOG=1` (or the probe) prints one line per claim/denial —
    /// the ordering proof reads those lines instead of assuming an order.
    pub log: bool,
}

impl RKeyRoute {
    /// Try to take this frame's R press. Returns `false` when another system
    /// already took it — the caller must then do nothing with the key.
    pub fn claim(&mut self, want: RKeyUse, who: &'static str) -> bool {
        if self.route != RKeyUse::Unclaimed {
            if self.log {
                println!(
                    "R_ROUTE frame={} DENY  want={:?} who={} already={:?} claimed_by={}",
                    self.frame, want, who, self.route, self.by
                );
            }
            return false;
        }
        self.route = want;
        self.by = who;
        if self.log {
            println!("R_ROUTE frame={} CLAIM {:?} who={}", self.frame, want, who);
        }
        true
    }

    /// Who owns the key this frame (`Unclaimed` until someone claims it).
    pub fn route(&self) -> RKeyUse {
        self.route
    }

    /// Name of the claiming system.
    pub fn claimed_by(&self) -> &'static str {
        self.by
    }

    /// Monotonic frame counter, stamped by [`reset_r_route`].
    pub fn frame(&self) -> u64 {
        self.frame
    }
}

/// Clears the route at the top of every frame.
///
/// Runs in `First` — before Bevy's own input clearing in `PreUpdate` and before
/// any `Update` reader — and unconditionally: state-gating it would leave a
/// stale claim behind on the frame Play is entered or left, and a stale claim
/// silently eats the next real press.
pub fn reset_r_route(mut route: ResMut<RKeyRoute>) {
    route.frame = route.frame.wrapping_add(1);
    route.route = RKeyUse::Unclaimed;
    route.by = "";
}

/// When the routing probe taps R, in seconds of elapsed run time.
///
/// `VOXELFORGE_RKEY_PROBE=1` takes the default pair; `VOXELFORGE_RKEY_PROBE=12,40`
/// retimes the taps without a rebuild — a 25-minute link is too expensive to spend
/// on "the enemy hadn't spawned yet".
#[derive(Resource)]
pub struct RKeyProbe {
    pub taps: Vec<f32>,
}

impl Default for RKeyProbe {
    fn default() -> Self {
        // Lock on, then lock off, both early enough that no `place_block`
        // objective can be in reach yet.
        Self { taps: vec![2.0, 2.6] }
    }
}

impl RKeyProbe {
    /// Parse `VOXELFORGE_RKEY_PROBE`. Anything that is not a comma-separated list
    /// of seconds (including the plain `1` that just switches the probe on) keeps
    /// the default pair.
    fn from_env(raw: &str) -> Self {
        let taps: Vec<f32> = raw
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .filter(|t| *t > 1.0)
            .collect();
        if taps.is_empty() { Self::default() } else { Self { taps } }
    }
}

/// Scripted R presses for the routing proof (`VOXELFORGE_RKEY_PROBE=1`).
///
/// The two R states have to be *shown*, not argued. The quest demo already
/// presses R standing on the `place_block` objective; this taps R early in the
/// same run, while no build objective is anywhere near the player, so one log
/// carries a `CLAIM LockOn` next to the later `CLAIM QuestPlace`.
///
/// Ordered `.before(gather_input)` and ahead of the quest trigger, so both
/// consumers see the synthetic press on the frame it is made — `just_pressed` is
/// cleared next `PreUpdate`, and a press made after them is a press nobody sees.
pub fn r_route_probe(
    time: Res<Time>,
    probe: Res<RKeyProbe>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut tap: Local<usize>,
    mut held: Local<bool>,
) {
    // Release the previous tap first: `just_pressed` only fires on a rising edge.
    // Once the taps are spent this also hands the key back for good — the quest
    // demo needs R for its own press in phase 4.
    if *held {
        keys.reset(KeyCode::KeyR);
        *held = false;
        return;
    }
    let Some(at) = probe.taps.get(*tap) else { return };
    let t = time.elapsed_secs();
    if t < *at {
        return;
    }
    keys.press(KeyCode::KeyR);
    *held = true;
    *tap += 1;
    println!("R_PROBE tap={} t={:.2}", *tap, t);
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

/// Marker for the player's visible body mesh children (capsule + face block).
/// The dodge ghost system reads this to flash/pulse the body during i-frames.
#[derive(Component)]
pub struct PlayerBody;

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

/// Fired exactly once when a husk (enemy) dies. Sun wires this to quest
/// progression so q2 can complete — without it the game has no ending
/// (`docs/art-order-2026-08-09-composition.md` § combat loop unlock).
#[derive(Debug, Clone, Copy)]
pub struct EnemyDied {
    pub entity: Entity,
    pub position: Vec3,
}

impl Message for EnemyDied {}

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

/// Player movement feed for the husks' hearing channel. `husk_ai` refreshes it
/// every frame; a player moving faster than [`HUSK_SPRINT_NOISE`] (or mid-attack
/// / dodge) counts as "noisy" and can be detected out to [`HUSK_HEAR_RANGE`],
/// beyond the sight cone's reach.
#[derive(Resource, Default)]
pub struct PlayerKinematics {
    pub last_pos: Vec3,
    pub speed: f32,
    /// False until the first frame has seeded `last_pos` (so the first speed
    /// sample isn't measured against the world origin).
    pub seeded: bool,
}

/// Enables the AI state-transition log (`VOXELFORGE_AI_LOG`). Prints exactly one
/// `AI_HUSK` line per state change per husk — the observable evidence layer for
/// the perception/tactics pass: patrol→alert→chase→telegraph→attack→reposition.
#[derive(Resource, Default)]
pub struct AiLog {
    pub enabled: bool,
}

/// The before/after lever for the perception & tactics pass (`VOXELFORGE_AI_LEGACY`).
///
/// With `legacy` set, [`husk_ai`] falls back to the pre-pass behaviour it
/// replaced: aggro on raw distance in any direction (no sight cone, no
/// hearing), straight from patrol into a chase with no "noticed you" beat,
/// beeline approach with no separation steering, no squad attack-slot ceiling,
/// no gap-closing lunge, and back to a chase the instant a combo recovers
/// instead of circling. That is one binary that can shoot **both** sides of the
/// A/B — a separate "before" build would differ in link stamps and scene drift
/// too, and could not be trusted to isolate the AI change.
///
/// It is a proof lever, not a gameplay option: unset (the default) is the
/// shipping behaviour.
#[derive(Resource, Default)]
pub struct AiTactics {
    pub legacy: bool,
}

/// Per-frame husk position/state trace (`VOXELFORGE_AI_TRACE=<path.csv>`).
///
/// The AI state log proves the *state machine* moves; it says nothing about
/// where the bodies went. This dumps `t,entity,x,z,state,dist` every frame so
/// the same run can be plotted top-down — which is the only way "they fan out
/// and take turns" vs "they stack into one clump" is visible as evidence
/// rather than as a claim. Rows are buffered and flushed on app exit.
#[derive(Resource, Default)]
pub struct AiTrace {
    pub path: Option<String>,
    pub rows: Vec<String>,
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
/// `(wx,wz)`. The body is the **Bone Sentinel** silhouette (`enemies.rs`,
/// Monanisa's design — the exact swap `docs/enemy-design.md` leaves to this
/// lane): the old three-flat-grey-cuboid husk is gone from the live game.
/// Same signature as ever, so every call site (scene, quest, demos) swaps in
/// one move. Returns nothing — pure world spawn.
pub fn spawn_guard_husk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wx: f32,
    wz: f32,
    surface_y: Option<f32>,
) {
    spawn_husk_of_kind(
        commands,
        meshes,
        materials,
        crate::enemies::EnemyKind::Sentinel,
        wx,
        wz,
        surface_y,
    );
}

/// Spawn one enemy of any authored [`enemies::EnemyKind`] with the full combat
/// kit (`Enemy` AI + `Health` + `Poise`) on the same root `enemies::spawn_enemy`
/// built. The AI is kind-agnostic; only `husk_telegraph`'s arm pick reads
/// `Enemy::kind`. The vanilla game spawns `Sentinel`; the AI squad/demo may mix
/// kinds in later without touching this function again.
pub fn spawn_husk_of_kind(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: crate::enemies::EnemyKind,
    wx: f32,
    wz: f32,
    surface_y: Option<f32>,
) {
    let surface = surface_y.unwrap_or_else(|| terrain_height(wx, wz) as f32 + 1.0);
    let feet = Vec3::new(wx, surface, wz);

    // The silhouette + root entity come from the design lane; the combat kit
    // rides on the same root so hit detection, poise, knockback and the AI all
    // keep working on the transform they always worked on.
    let root =
        crate::enemies::spawn_enemy(commands, meshes, materials, kind, feet, 0.0);
    commands.entity(root).insert((
        Enemy::at(feet, surface, kind),
        Health::new(kind_hp(kind)),
        Poise::new(POISE_HUSK),
    ));
    println!("HUSK2_SPAWN kind={} at ({wx:.1},{surface:.1},{wz:.1})", kind.id());
}

/// Max HP per enemy kind. The Bone Sentinel is the tank of the first playable
/// (§4.1); the Ghoul Reaver is the light, fast one, and the Thornclaw Stalker
/// sits in between. Same combat kit, different endurance.
fn kind_hp(kind: crate::enemies::EnemyKind) -> f32 {
    use crate::enemies::EnemyKind;
    match kind {
        EnemyKind::Reaver => 50.0,
        EnemyKind::Sentinel => HP_HUSK,
        EnemyKind::Stalker => 65.0,
    }
}

/// What an enemy drops into the player's bag on death. The only meaningful
/// consumable is the health potion, so the drop table is all potions with a
/// per-kind count: the heavier the fight, the bigger the reward. The drop is
/// applied in `husk_ai`'s death block via `Inventory::add`.
fn loot(kind: crate::enemies::EnemyKind) -> (crate::inventory::ItemKind, u32) {
    use crate::enemies::EnemyKind;
    use crate::inventory::ItemKind;
    match kind {
        EnemyKind::Reaver => (ItemKind::HealthPotion, 1),
        EnemyKind::Sentinel => (ItemKind::HealthPotion, 2),
        EnemyKind::Stalker => (ItemKind::HealthPotion, 1),
    }
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
    mut route: ResMut<RKeyRoute>,
) {
    let light_pressed = mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::KeyX);
    // R is shared with the quest build prompt. The contextual consumer runs first
    // (`quest::check_block_place_triggers`, ordered `.before(gather_input)`) and
    // claims the press when a `place_block` objective is in reach; lock-on is the
    // fallback, so it only toggles while the press is still unclaimed. The `&&`
    // short-circuits on purpose — no press, no claim, or lock-on would eat every
    // frame's route and starve the quest side.
    let lock_toggle =
        keys.just_pressed(KeyCode::KeyR) && route.claim(RKeyUse::LockOn, "combat::gather_input");
    *intent = CombatIntent {
        light: light_pressed,
        heavy_down: keys.pressed(KeyCode::KeyC),
        dodge: keys.just_pressed(KeyCode::Space),
        block: mouse.pressed(MouseButton::Right),
        parry: keys.just_pressed(KeyCode::KeyV),
        lock_toggle,
        lock_switch: keys.just_pressed(KeyCode::KeyQ),
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
    // Only reached when `RKeyRoute` handed R to lock-on this frame — the log line
    // is what makes "which of the two R consumers acted" readable in a real run.
    if intent.lock_toggle {
        if lock.target.is_some() {
            lock.target = None;
            info!("LOCK_ON off");
        } else {
            lock.target = nearest_target(&ptf.translation, pc_facing(&ptf), &enemy_q);
            info!("LOCK_ON on target={:?}", lock.target);
        }
    }
    // Switch target (§2.3): only meaningful while already locked; picks the
    // nearest OTHER live enemy in range. No candidate => stay on current target.
    if intent.lock_switch {
        if let Some(cur) = lock.target {
            let candidates: Vec<(Entity, Vec3, bool)> =
                enemy_q.iter().map(|(e, tf, hp, _)| (e, tf.translation, hp.dead())).collect();
            if let Some(next) = cycle_target(cur, &ptf.translation, &candidates) {
                lock.target = Some(next);
            }
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

            // ---- the weight layer --------------------------------------------
            // One classification drives every channel below — sound, shake,
            // freeze, shove, kick — so a riposte or a poise break can never end
            // up sounding or shaking like the light that caused it.
            let weight = match pc.state {
                _ if riposte => ImpactWeight::Critical, // a riposte is the loudest hit there is
                _ if broke => ImpactWeight::Critical, // a poise break outranks the swing
                CombatState::Charged => ImpactWeight::Critical,
                CombatState::Heavy => ImpactWeight::Heavy,
                _ => ImpactWeight::Light,
            };
            // Audio: the impact sound tracks the blow's *weight*, not the button
            // that threw it — a riposte and a poise break both read Critical and
            // must not masquerade as the light tap their swing started as.
            sfx.write(impact_sfx(weight, etf.translation));
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
            // 3. Camera: the omni rattle scales with the same weight, and a poise
            //    break borrows the loudest row (§5.3 stagger break).
            shake.hit(if broke {
                SHAKE_STAGGER
            } else if weight == ImpactWeight::Light {
                SHAKE_LIGHT
            } else {
                SHAKE_HEAVY
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

/// Smallest absolute angle between two yaw angles (radians), the short way
/// round — the same normalisation [`turn`] applies, but as a distance, for the
/// husk's sight-cone check.
#[inline]
pub fn angle_delta(a: f32, b: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let d = (b - a).rem_euclid(TAU);
    if d > PI {
        TAU - d
    } else {
        d
    }
}

/// Is this AI state one that is *committed to an attack* — i.e. it occupies one
/// of the squad's [`HUSK_ATTACK_SLOTS`]? Recover / Gap / Reposition deliberately
/// do not count: those between-commitment beats are exactly when a squadmate is
/// allowed to step in, which is what makes a pack flow instead of pulsing.
#[inline]
pub fn husk_attacking(s: HuskState) -> bool {
    matches!(
        s,
        HuskState::Telegraph
            | HuskState::Feint
            | HuskState::Swing1
            | HuskState::Swing2
            | HuskState::LungeWind
            | HuskState::LungeDash
    )
}

/// Separation steering: a push away from squadmates closer than
/// [`HUSK_SEPARATION`], strengthening as a neighbour closes. Pure — the unit
/// tests grade the anti-clumping direction and reach off this, not off a copy.
pub fn husk_separation(squad: &[(Entity, Vec3, HuskState)], me: Entity, pos: Vec3) -> Vec3 {
    let mut push = Vec3::ZERO;
    for &(other, opos, _) in squad {
        if other == me {
            continue;
        }
        let d = Vec3::new(pos.x - opos.x, 0.0, pos.z - opos.z);
        let len = d.length();
        if len > 1e-4 && len < HUSK_SEPARATION {
            push += d / len * ((HUSK_SEPARATION - len) / HUSK_SEPARATION);
        }
    }
    push * HUSK_SEPARATION_STRENGTH
}

/// Per-husk evade bookkeeping — the "it dodged me" beat.
///
/// **Why a resource and not two fields on [`Enemy`].** `anim.rs` (Flamingo's
/// lane) builds an `Enemy` with an exhaustive struct literal in its own test,
/// so *every* added field is a compile break in a file this lane does not own.
/// Same for a new [`HuskState`] variant — `anim.rs::husk_beat` matches the enum
/// exhaustively. Keeping the evade timer in a side table owned by `combat.rs`
/// buys the behaviour without reaching across the lane fence at all.
///
/// Entries are dropped when the husk despawns (see [`husk_evade_gc`]), so this
/// cannot grow without bound across respawns.
#[derive(Resource, Default)]
pub struct HuskEvade {
    pub per: std::collections::HashMap<Entity, EvadeSlot>,
}

/// One husk's evade state: burst time left, cooldown left, and the direction it
/// committed to when the hop started (locked at the start so the hop is a
/// readable commitment, not a homing slide).
#[derive(Clone, Copy, Debug, Default)]
pub struct EvadeSlot {
    pub left: f32,
    pub cd: f32,
    pub dir: Vec3,
}

/// Is the player mid-commitment to a swing the husk could plausibly react to?
/// Blocking / idle / already-recovering do not count — a husk that hops away
/// from a player just standing there reads as jitter, not as a read.
#[inline]
pub fn player_is_swinging(s: CombatState) -> bool {
    matches!(s, CombatState::Light | CombatState::Heavy | CombatState::Charged)
}

/// Should this husk start an evade hop *this frame*?
///
/// Deliberately conservative: only while the player is actually swinging, only
/// inside the range that swing could reach, only if this husk is not itself
/// mid-commitment (the honest-telegraph contract cuts both ways — a husk that
/// could cancel its own wind-up by dodging would make every tell a lie), and
/// only once per [`HUSK_EVADE_COOLDOWN`].
#[inline]
pub fn husk_should_evade(
    player_state: CombatState,
    husk_state: HuskState,
    dist: f32,
    slot: EvadeSlot,
) -> bool {
    player_is_swinging(player_state)
        && dist <= HUSK_EVADE_RANGE
        && slot.left <= 0.0
        && slot.cd <= 0.0
        && matches!(husk_state, HuskState::Chase | HuskState::Reposition)
}

/// The hop vector: mostly sideways (so it clears the arc rather than backing
/// straight down the swing's line), part backwards. `radial` points from the
/// husk toward the player; `side` is ±1 and normally the husk's own orbit
/// direction, so a dodging pack keeps fanning out instead of converging.
#[inline]
pub fn husk_evade_vector(radial: Vec3, side: f32) -> Vec3 {
    let tangent = Vec3::new(-radial.z, 0.0, radial.x) * side.signum();
    (tangent * (1.0 - HUSK_EVADE_BACK) - radial * HUSK_EVADE_BACK).normalize_or_zero()
}

/// Drops evade slots whose husk no longer exists, so the side table cannot grow
/// across deaths and respawns.
pub fn husk_evade_gc(mut evade: ResMut<HuskEvade>, q: Query<Entity, With<Enemy>>) {
    if evade.per.is_empty() {
        return;
    }
    let live: std::collections::HashSet<Entity> = q.iter().collect();
    evade.per.retain(|e, _| live.contains(e));
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
    mut died: MessageWriter<EnemyDied>,
    mut impacts: MessageWriter<ImpactEvent>,
    mut staggers: MessageWriter<StaggerEvent>,
    feel: Res<FeelLog>,
    ailog: Res<AiLog>,
    mut dp: ResMut<crate::dodge_parry::DodgeParryState>,
    mut kin: ResMut<PlayerKinematics>,
    tac: Res<AiTactics>,
    mut evade: ResMut<HuskEvade>,
    mut inv: ResMut<crate::inventory::Inventory>,
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

    // Hearing channel input: a sprinting or attacking player is "loud" and can
    // be detected well past the vision cone. Seeded on the first frame so the
    // very first speed sample isn't measured against the world origin.
    if !kin.seeded {
        kin.last_pos = ptf.translation;
        kin.seeded = true;
    }
    kin.speed = (ptf.translation.distance(kin.last_pos) / dt.max(1e-4)).min(64.0);
    kin.last_pos = ptf.translation;
    let player_noisy = kin.speed >= HUSK_SPRINT_NOISE
        || matches!(
            pc.state,
            CombatState::Light | CombatState::Heavy | CombatState::Charged | CombatState::Dodge
        );

    // Squad snapshot (immutable pre-pass): who is where, and who currently
    // occupies an attack slot. Powers separation steering and attack-slot
    // arbitration in the mutable pass below. Taken once per frame so the two
    // query borrows stay sequential, not nested.
    let squad: Vec<(Entity, Vec3, HuskState)> = enemy_q
        .iter()
        .map(|(e, t, en, _, _, _)| (e, t.translation, en.state))
        .collect();
    // Legacy (`VOXELFORGE_AI_LEGACY`) had no squad arbitration at all, so every
    // husk could commit at once. Expressing that as an unreachable ceiling —
    // rather than as a second copy of each branch — keeps the A/B honest: both
    // sides run the *same* code path, differing only in this number and the
    // handful of `tac.legacy` gates below.
    let slots = if tac.legacy { u32::MAX } else { HUSK_ATTACK_SLOTS };
    let attackers_now = squad
        .iter()
        .filter(|(_, _, s)| husk_attacking(*s))
        .count() as u32;

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
                died.write(EnemyDied {
                    entity,
                    position: etf.translation,
                });
                // Loot loop: the kill pays into the item bag, so fighting has a
                // reason. `add` stacks the drop on an existing slot (or a fresh
                // one) and returns any part the full bag couldn't hold.
                let (item, n) = loot(e.kind);
                let leftover = inv.add(item, n);
                println!(
                    "LOOT_DROP kind={} +{} (leftover {})",
                    e.kind.id(),
                    n,
                    leftover
                );
                info!("COMBAT enemy died — EnemyDied fired at {:?}", etf.translation);
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
        e.lunge_cd = (e.lunge_cd - dt).max(0.0);
        let facing_target = to_player.z.atan2(to_player.x);

        // Perception: sight is a forward cone (where the model is looking), a
        // player inside CLOSE_SENSE is felt regardless of facing, and a noisy
        // (sprinting / attacking) player is heard out to HEAR_RANGE. The old
        // code aggroed on raw distance in any direction — a husk with its back
        // turned still snapped into a chase, which reads as a proximity mine,
        // not a creature that saw you.
        let seen = dist <= HUSK_AGGRO_RANGE
            && angle_delta(e.facing, facing_target) <= HUSK_SIGHT_HALF.to_radians();
        let sensed = dist <= HUSK_CLOSE_SENSE;
        let heard = dist <= HUSK_HEAR_RANGE && player_noisy;
        // Legacy aggroed on raw distance in any direction — that is the whole
        // "walks straight at you like a proximity mine" read this pass removes.
        let detected = if tac.legacy {
            dist <= HUSK_AGGRO_RANGE
        } else {
            seen || sensed || heard
        };
        // Every *actual* detection refreshes the trail. A husk that loses the
        // player walks to this spot, not to the player's true position — the
        // chase must be honest about what the creature can know.
        if detected {
            e.last_seen = ptf.translation;
        }

        // -- Evade: the "it dodged me" beat ---------------------------------
        // Runs *before* the state machine and, while a hop is live, replaces
        // that state's own movement. It never changes `e.state`: a husk mid
        // wind-up keeps its wind-up (the tell must not lie), and no new
        // `HuskState` variant means `anim.rs` stays untouched.
        let slot = evade.per.get(&entity).copied().unwrap_or_default();
        let radial = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
        let mut slot = EvadeSlot {
            left: (slot.left - dt).max(0.0),
            cd: (slot.cd - dt).max(0.0),
            dir: slot.dir,
        };
        if !tac.legacy && husk_should_evade(pc.state, e.state, dist, slot) {
            slot.left = HUSK_EVADE_TIME;
            slot.cd = HUSK_EVADE_COOLDOWN;
            slot.dir = husk_evade_vector(radial, e.strafe_dir);
            if ailog.enabled {
                println!(
                    "AI_HUSK_EVADE entity={} dist={dist:.2} player={:?} from={}",
                    entity.to_bits(),
                    pc.state,
                    husk_state_name(e.state),
                );
            }
        }
        let evading = slot.left > 0.0;
        if evading && !shoved {
            etf.translation += slot.dir * HUSK_EVADE_SPEED * dt;
            // Keep the eyes on the player through the hop — a husk that dodges
            // facing away looks like it slipped, not like it read the swing.
            e.facing = turn(e.facing, facing_target, HUSK_TURN * dt);
        }
        evade.per.insert(entity, slot);

        // Mid-range gap-closer: only worth committing when melee can't reach,
        // the cooldown has spun, and a squad attack slot is open.
        let can_lunge = !tac.legacy
            && dist > MELEE_RANGE + 0.5
            && dist <= HUSK_LUNGE_RANGE
            && e.lunge_cd <= 0.0;

        match e.state {
            HuskState::Patrol => {
                // Walk a 12-block loop along X from the origin (§4.1).
                let step = HUSK_WALK * dt * e.patrol_dir;
                if !shoved {
                    etf.translation.x += step;
                }
                // Face the direction of the walk. atan2(z,x) convention: 0 = +X,
                // π = −X — the old ±π/2 here made the husk strut sideways along
                // its patrol line for the grey boxes' whole life.
                e.facing = if e.patrol_dir > 0.0 { 0.0 } else { std::f32::consts::PI };
                if (etf.translation.x - e.patrol_origin.x).abs() > 6.0 {
                    e.patrol_dir = -e.patrol_dir;
                }
                if detected {
                    // Detected ≠ chasing. The husk first buys the observable
                    // "noticed you" beat — a player watching from range sees the
                    // patrol stop and the head snap over. Legacy skipped this
                    // entirely and snapped straight into the chase.
                    e.state = if tac.legacy {
                        HuskState::Chase
                    } else {
                        HuskState::Alert
                    };
                    e.timer = 0.0;
                }
            }
            HuskState::Alert => {
                // The "noticed you" beat: plant, snap to face the player, hold
                // for HUSK_ALERT_TIME. The turn runs 1.5× the normal rate — an
                // attentive snap, not a lazy drift.
                e.facing = turn(e.facing, facing_target, HUSK_TURN * 1.5 * dt);
                if e.timer >= HUSK_ALERT_TIME {
                    e.state = HuskState::Chase;
                    e.timer = 0.0;
                } else if dist > HUSK_HEAR_RANGE {
                    // The player vanished mid-beat — stand down.
                    e.state = HuskState::Patrol;
                    e.timer = 0.0;
                }
            }
            HuskState::Chase => {
                e.facing = turn(e.facing, facing_target, HUSK_TURN * dt);
                // The leash measures distance from the patrol origin (home), not
                // from the player: aggro reaches 12 blocks out, so a player-based
                // leash of 6 made a husk at 7–12 blocks flicker Patrol↔Chase every
                // frame. A husk chases as far as it is willing to leave home.
                let leash_ref = if tac.legacy {
                    dist // legacy leashed off the player, which flickered
                } else {
                    etf.translation.distance(e.patrol_origin)
                };
                if husk_should_leash(leash_ref) {
                    // Leashed: too far from home to keep chasing. Legacy amnesia
                    // walked straight home; the tactics pass first walks the
                    // trail (last-seen spot) — the player who broke away at
                    // range is followed to where they were last *seen*, not
                    // forgotten on the spot.
                    e.state = if tac.legacy {
                        HuskState::Patrol
                    } else {
                        HuskState::Search
                    };
                    e.timer = 0.0;
                } else if !tac.legacy
                    && !detected
                    && etf.translation.distance(e.last_seen) > 1.0
                {
                    // Lost mid-chase: player left the sight cone, is beyond
                    // close-sense and not heard, and the husk hasn't even
                    // reached the last place it saw them. Keep following the
                    // trail instead of homing on a position it cannot know.
                    e.state = HuskState::Search;
                    e.timer = 0.0;
                } else if can_lunge && attackers_now < slots {
                    // Mid-range gap-closer: a kiting player gets the lunge — with
                    // its own longer telegraph, so the leap stays dodgeable.
                    e.state = HuskState::LungeWind;
                    e.timer = 0.0;
                    e.hit_applied = false;
                    e.lunge_cd = HUSK_LUNGE_COOLDOWN;
                } else if dist <= MELEE_RANGE && attackers_now < slots {
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
                } else if !shoved && !evading {
                    // Approach with separation steering so a pack fans out around
                    // the player instead of stacking into one clump. Legacy is the
                    // bare radial beeline — three husks converging on one point.
                    // Suppressed mid-hop so the evade burst isn't cancelled out by
                    // the approach pulling in the opposite direction.
                    let radial = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    let mut move_dir = if tac.legacy {
                        radial
                    } else {
                        radial + husk_separation(&squad, entity, etf.translation)
                    };
                    let mut speed = HUSK_WALK;
                    if !tac.legacy && attackers_now >= slots && dist <= MELEE_RANGE + 0.6 {
                        // Melee is fully booked: orbit at the rim instead of
                        // pressing into a body-block clump. This is what makes a
                        // squad take turns instead of swinging in unison.
                        let tangent = Vec3::new(-radial.z, 0.0, radial.x) * e.strafe_dir;
                        move_dir = tangent + radial * 0.2
                            + husk_separation(&squad, entity, etf.translation);
                        speed = HUSK_WALK * HUSK_STRAFE_SPEED;
                    }
                    if move_dir.length_squared() > 1e-6 {
                        etf.translation += move_dir.normalize() * speed * dt;
                    }
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
                        // The blade comes down — the whoosh belongs to the swing
                        // itself, not to the hit (a whiffed dodge still hears
                        // the cleaver cut air). Layered per Yamamoto's table:
                        // `swing_light.wav` + `swing_whoosh.wav`.
                        sfx.write(SfxEvent::SwingLight { position: etf.translation });
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
                    // The heavier half of the combo gets the heavier whoosh —
                    // `swing_heavy.wav` + a louder `swing_whoosh.wav` layer.
                    sfx.write(SfxEvent::SwingHeavy { position: etf.translation });
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
            HuskState::Search => {
                // Follow the trail: walk to the last place the player was
                // actually detected, then sweep the sight cone around that spot
                // — a slow rotation is what "looking for them" reads as. Give up
                // after HUSK_SEARCH_TIME and walk home. Any re-detection snaps
                // back through the Alert beat, same as spotting from patrol.
                let to_last = e.last_seen - etf.translation;
                let flat = Vec3::new(to_last.x, 0.0, to_last.z);
                let arrived = flat.length() <= 0.9;
                if !arrived && !shoved && e.timer < HUSK_SEARCH_TIME * 0.6 {
                    // Hustle to the spot first; the sweep only starts on arrival
                    // (or once most of the budget is spent still walking).
                    e.facing = turn(e.facing, flat.z.atan2(flat.x), HUSK_TURN * dt);
                    etf.translation += flat.normalize_or_zero() * HUSK_WALK * dt;
                } else {
                    // Sweep: rotate the sight cone at a quarter turn per second,
                    // outward-in — the search pattern, not an idle spin.
                    e.facing += HUSK_TURN * 0.8 * dt;
                }
                if e.timer >= HUSK_SEARCH_TIME || detected {
                    e.state = if detected {
                        HuskState::Alert // found them again — the noticed-you beat
                    } else {
                        HuskState::Patrol // trail gone cold — walk home
                    };
                    e.timer = 0.0;
                    if ailog.enabled && detected {
                        println!(
                            "AI_HUSK_TRAIL entity={} re_acquired_at dist={dist:.2}",
                            entity.to_bits(),
                        );
                    }
                }
            }
            HuskState::Recover => {
                if e.timer >= HUSK_COMBO_PAUSE {
                    // Circle before re-committing (perception/tactics pass): a
                    // straight line in → swing → straight line in again reads as
                    // a pendulum, not a fighter sizing you up. Legacy went
                    // straight back to the chase, which is that pendulum.
                    e.state = if tac.legacy {
                        HuskState::Chase
                    } else {
                        HuskState::Reposition
                    };
                    e.timer = 0.0;
                }
            }
            HuskState::Reposition => {
                // Orbit the player, peel off when crowded, lean in when drifted
                // out — then re-commit from whatever flank it lands on. Also the
                // waiting room for a squadmate whose attack slot is taken.
                e.facing = turn(e.facing, facing_target, HUSK_TURN * dt);
                if !shoved && !evading {
                    let radial = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    let tangent = Vec3::new(-radial.z, 0.0, radial.x) * e.strafe_dir;
                    // Too close → peel outward; past melee reach → lean back in.
                    let radial_bias = if dist < HUSK_BACKOFF_RANGE {
                        0.7
                    } else if dist > MELEE_RANGE * 1.7 {
                        -0.6
                    } else {
                        0.0
                    };
                    let mut move_dir = tangent * HUSK_STRAFE_SPEED + radial * radial_bias
                        + husk_separation(&squad, entity, etf.translation);
                    if move_dir.length_squared() > 1e-6 {
                        etf.translation += move_dir.normalize() * HUSK_WALK * dt;
                    }
                }
                if e.timer >= HUSK_REPOSITION_TIME {
                    e.state = HuskState::Chase;
                    e.timer = 0.0;
                    // Flip the orbit direction about half the time (seeded, so a
                    // headless proof stays deterministic) — mixed orbits, not a
                    // carousel riding one way forever.
                    if pick_rhythm((entity.to_bits() as u32) ^ e.combo_no) == HuskRhythm::Delayed {
                        e.strafe_dir = -e.strafe_dir;
                    }
                }
            }
            HuskState::LungeWind => {
                // The leap's tell: face the player through the whole (longer)
                // wind-up. The arm pulls back low — a distinct crouch-and-coil
                // pose next to the overhead melee telegraph.
                e.facing = turn(e.facing, facing_target, HUSK_TURN * dt);
                if e.timer >= HUSK_LUNGE_WIND {
                    e.state = HuskState::LungeDash;
                    e.timer = 0.0;
                    e.hit_applied = false;
                    // The leap itself: a heavy whoosh at the moment the coil
                    // releases, so a sidestepping player hears the pass-by even
                    // when the hitbox never touches them.
                    sfx.write(SfxEvent::SwingHeavy { position: etf.translation });
                }
            }
            HuskState::LungeDash => {
                // Charge forward. The hitbox is live across the travel, so the
                // lunge connects on whichever frame it actually reaches the
                // player — and only once per leap.
                if !shoved {
                    let dir = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    etf.translation += dir * HUSK_LUNGE_SPEED * dt;
                }
                if !e.hit_applied && e.timer <= HUSK_LUNGE_DASH {
                    let (connected, outcome, broke) = try_hit_player(
                        HUSK_LUNGE_DMG, HUSK_LUNGE_POISE, dist, &mut php, &mut pc,
                        &mut pstam, &mut ppoise, &mut shake,
                    );
                    crate::dodge_parry::resolve_defence(
                        outcome, HUSK_LUNGE_DMG, &feel, &mut dp, &mut commands, entity,
                        &mut e, &mut ep, &mut pc, &php, etf.translation, to_player,
                        &mut shake, &mut staggers,
                    );
                    if connected {
                        e.hitstop = e.hitstop.max(HITSTOP_ENEMY);
                        emit_enemy_hit_sfx(&outcome, etf.translation, &mut sfx);
                        taken_feel(
                            ImpactWeight::Heavy, broke, outcome, &to_player, ptf.translation,
                            entity, player, &mut pc, &mut shake, &mut impacts, &mut staggers,
                        );
                        e.hit_applied = true;
                    }
                }
                if e.timer >= HUSK_LUNGE_DASH {
                    e.state = HuskState::Recover;
                    e.timer = 0.0;
                }
            }
            HuskState::Dead | HuskState::Staggered => {}
        }

        // Keep the Husk planted on the terrain surface it spawned on (no drift).
        etf.translation.y = e.surface_y;
        // `e.facing` uses the atan2(z,x) convention (facing 0 = looking +X —
        // see the sight-cone unit test), but a yaw of 0 points the model's −Z
        // front at −Z. The −π/2 offset reconciles them, so the *asymmetric*
        // bodies (Sentinel's eye-slits face −Z, weapon rides +X) actually look
        // where the sight cone says they look. On the old symmetric grey boxes
        // this 90° skew was invisible; on the new silhouettes it is the
        // difference between being stared at and being side-eyed.
        etf.rotation = Quat::from_axis_angle(Vec3::Y, e.facing - std::f32::consts::FRAC_PI_2);
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
        // §5.3: a successful parry lands harder than a light tap — its own row.
        shake.hit(SHAKE_PARRY);
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
        // §5.3: the player's stance broke — the stagger-break shake, louder than
        // the plain-hit rattle already applied in `try_hit_player`.
        shake.hit(SHAKE_STAGGER);
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

/// The impact sound for a blow the *player* landed, chosen by [`ImpactWeight`]
/// rather than by which button was pressed. A riposte and a poise break both
/// classify as Critical, and neither may sound like the light tap their swing
/// started as — there is no `hit_critical.wav` yet, so Critical borrows the
/// heavy impact until the audio lane ships one.
fn impact_sfx(weight: ImpactWeight, pos: Vec3) -> SfxEvent {
    match weight {
        ImpactWeight::Light => SfxEvent::HitLight { position: pos },
        ImpactWeight::Heavy | ImpactWeight::Critical => SfxEvent::HitHeavy { position: pos },
    }
}

/// Local-space translation of each kind's weapon-arm box — the telegraph's
/// "arm" that rises during wind-up. Mirrors the authored arrays in `enemies.rs`
/// (centre of the Bx range × `VX`): the design doc explicitly names the
/// Sentinel's weapon forearm/gauntlet as the natural `HuskArm` substitute and
/// leaves the pick to this lane. `husk_telegraph` finds the child mesh sitting
/// at this offset; if Monanisa re-authors the geometry, this table is the only
/// thing to re-measure.
fn weapon_arm_offset(kind: crate::enemies::EnemyKind) -> Vec3 {
    use crate::enemies::EnemyKind;
    match kind {
        // b(4.6, 4.2, -2.4, 6.4, 8.8, 2.4) → centre (5.5, 6.5, 0.0) × 0.125
        EnemyKind::Sentinel => Vec3::new(0.6875, 0.8125, 0.0),
        // the overlong right forearm b(2.6, 2.2, -2.0, 3.8, 10.2, 1.0)
        EnemyKind::Reaver => Vec3::new(0.4, 0.775, -0.0625),
        // the sickle-arm upper b(2.2, 6.8, -3.0, 3.6, 9.2, 0.6)
        EnemyKind::Stalker => Vec3::new(0.3625, 1.0, -0.15),
    }
}

/// Move the enemy's telegraph arm up during wind-up so the incoming swing reads
/// at a distance (§5.1 honest visual telegraph — a big pose change).
///
/// The legacy body marked its arm child with [`HuskArm`]; the new silhouettes
/// are spawned wholesale by `enemies::spawn_enemy`, which has no combat-lane
/// marker, so the arm is found by geometry instead: the child whose local
/// translation sits at [`weapon_arm_offset`] for this body's kind. A marker
/// match (if one ever exists) still wins over the geometric pick.
pub fn husk_telegraph(
    enemy_q: Query<(&Enemy, &Children)>,
    mut arm_q: Query<&mut Transform>,
    marker_q: Query<Entity, With<HuskArm>>,
) {
    for (e, children) in enemy_q.iter() {
        let raised = matches!(e.state, HuskState::Telegraph);
        let mid_swing = matches!(e.state, HuskState::Swing1 | HuskState::Swing2);
        // The lunge reads as a different move from the melee combo on purpose:
        // the coil pulls the arm back low (vs. the overhead raise), and the dash
        // thrusts it past the melee slam angle.
        let coiling = matches!(e.state, HuskState::LungeWind);
        let thrusting = matches!(e.state, HuskState::LungeDash);
        // Pick the arm child: a `HuskArm`-marked one if present, else the child
        // nearest this kind's weapon-arm offset (an exact authored position, so
        // the nearest match is the intended box by a wide margin).
        let want = weapon_arm_offset(e.kind);
        let mut best: Option<(f32, Entity)> = None;
        for child in children.iter() {
            if marker_q.get(child).is_ok() {
                best = Some((f32::NEG_INFINITY, child)); // marker always wins
                break;
            }
            let Ok(t) = arm_q.get(child) else { continue };
            let d = t.translation.distance_squared(want);
            if best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, child));
            }
        }
        let Some((_, arm)) = best else { continue };
        let Ok(mut t) = arm_q.get_mut(arm) else { continue };
        // Raise the arm overhead while winding up; slam down on the swing.
        let target = if raised {
            -1.1 // rotate back/up (radians about X)
        } else if mid_swing {
            0.6
        } else if coiling {
            -0.7
        } else if thrusting {
            0.9
        } else {
            0.0
        };
        let cur = t.rotation.to_scaled_axis().x;
        let next = cur + (target - cur) * 0.35;
        t.rotation = Quat::from_axis_angle(Vec3::X, next);
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

/// Ease-out cubic: starts fast (the blow lands), decelerates to a stop.
/// `t` ∈ [0,1] → output ∈ [0,1]. The curve is `1 - (1-t)³`.
#[inline]
fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Slide a struck body along the blade direction (§ weight layer).
///
/// Runs between `player_combat` (which books the impulse) and `husk_ai` (which
/// re-plants the body on its surface and may walk it back in), so the shove is
/// always resolved against the same frame's hit.
///
/// The slide uses an ease-out cubic curve — the body jolts back on the first
/// frame (the impact lands with weight) and then friction tapers it to a stop.
/// A linear slide at 0.12 s reads as "the enemy glitched three pixels left";
/// the same distance with ease-out reads as "the blow connected and shoved it."
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
        kb.time -= dt;
        // Progress through the knockback window: 0 = contact frame, 1 = settled.
        // Clamped so a late frame that overshoots the window still delivers the
        // full distance rather than freezing the body mid-slide.
        let progress = (1.0 - (kb.time / KNOCKBACK_TIME).clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let eased = ease_out_cubic(progress);
        let target = kb.total * eased;
        // Step = how much further the body should have travelled by now, minus
        // what it has already travelled. The eased curve front-loads the shove:
        // ~60% of the distance lands in the first 33% of the window.
        let step = (target - kb.slid).max(0.0);
        tf.translation += kb.dir * step;
        kb.slid += step;
        kb.left = kb.total - target;
        // Belt-and-braces, not the fix for anything observed. Float drift from
        // the easing arithmetic can leave a sub-millimetre remainder when the
        // window closes; this pays it out and records it in `topup`.
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
// Enemy probe (VOXELFORGE_ENEMY_PROBE=before|after) — the deterministic
// before/after still of the LIVE enemy, through the real spawn path.
// ---------------------------------------------------------------------------

/// Which side of the silhouette swap the probe shoots.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyProbeMode {
    /// The retired body: the three grey cuboids the old `spawn_guard_husk`
    /// built, spawned without an `enemies::EnemyBody` so the anim rig dresses
    /// them exactly as the shipped game did — this is what players actually
    /// saw before the swap, not the raw boxes.
    Before,
    /// The live body: `spawn_guard_husk` as it stands now (Bone Sentinel).
    After,
}

/// Bookkeeping for [`enemy_probe`].
#[derive(Resource, Default)]
pub struct EnemyProbe {
    pub mode: Option<EnemyProbeMode>,
    pub spawned: bool,
    pub shot: bool,
}

/// Spawn one enemy straight ahead of the player through the real spawn path,
/// let the scene settle, snap one still, exit. Deterministic framing, no demo
/// scripting, no dialogue in the way — the A/B pair the AI-demo shots cannot
/// guarantee (their camera follows a scripted player that rarely faces the
/// fight).
#[allow(clippy::type_complexity)]
pub fn enemy_probe(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut probe: ResMut<EnemyProbe>,
    mut lock: ResMut<LockOn>,
    player_q: Query<&Transform, (With<FlyCam>, Without<Enemy>)>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mode) = probe.mode else { return };
    let t = time.elapsed_secs();
    let Ok(ptf) = player_q.single() else { return };

    if !probe.spawned && t >= 1.5 {
        // Dead ahead, 5 blocks out, on the terrain — centre of the view.
        let fwd = ptf.rotation * Vec3::NEG_Z;
        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let (wx, wz) = (ptf.translation.x + flat.x * 5.0, ptf.translation.z + flat.z * 5.0);
        // Same ground as the PLAYER, not `terrain_height` of the target column:
        // the demo world has a 13-block stone structure two columns ahead, and a
        // probe body spawned on its roof is invisible from a level camera for a
        // completely non-rendering reason. The eye sits EYE_HEIGHT above the
        // feet, so the player's own ground is eye.y − EYE_HEIGHT.
        let ground = ptf.translation.y - crate::EYE_HEIGHT;
        match mode {
            EnemyProbeMode::Before => spawn_legacy_husk_body(&mut commands, &mut meshes, &mut materials, wx, wz, Some(ground)),
            EnemyProbeMode::After => spawn_guard_husk(&mut commands, &mut meshes, &mut materials, wx, wz, Some(ground)),
        }
        probe.spawned = true;
        println!(
            "ENEMY_PROBE mode={:?} spawned at ({wx:.1},{wz:.1}) player_facing=({:.2},{:.2})",
            mode, flat.x, flat.z,
        );
        return;
    }
    // Hold the real lock-on on the probe's enemy: the orbit camera aims at the
    // lock target (`lock_on_camera`), which is what actually guarantees the
    // body is framed — the camera's own yaw is not the player's facing.
    if probe.spawned && !probe.shot {
        if let Some((entity, _)) = enemies
            .iter()
            .min_by(|a, b| {
                a.1.translation
                    .distance_squared(ptf.translation)
                    .partial_cmp(&b.1.translation.distance_squared(ptf.translation))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            lock.target = Some(entity);
        }
    }
    if probe.spawned && !probe.shot && t >= 3.3 && !enemies.is_empty() {
        // Framing diagnostic: where the camera actually sits versus the body.
        // The A/B is only honest if the numbers say the enemy is inside the
        // frustum — "no creature visible" in a still can mean a rendering bug
        // OR a camera that never turned, and these two lines tell them apart.
        if let Some((entity, etf)) = enemies
            .iter()
            .min_by(|a, b| {
                a.1.translation
                    .distance_squared(ptf.translation)
                    .partial_cmp(&b.1.translation.distance_squared(ptf.translation))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            let to = etf.translation - ptf.translation;
            println!(
                "ENEMY_PROBE eye=({:.1},{:.1},{:.1}) enemy#{:?}=({:.1},{:.1},{:.1}) dist={:.1} lock={:?}",
                ptf.translation.x, ptf.translation.y, ptf.translation.z,
                entity.to_bits(),
                etf.translation.x, etf.translation.y, etf.translation.z,
                to.length(),
                lock.target,
            );
        }
        let dir = std::env::var("VOXELFORGE_ENEMY_PROBE_DIR")
            .unwrap_or_else(|_| "_rose_probe".to_string());
        let path = std::path::Path::new(&dir)
            .join(match mode { EnemyProbeMode::Before => "enemy_before.png", EnemyProbeMode::After => "enemy_after.png" });
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        println!("ENEMY_PROBE shot -> {}", path.display());
        probe.shot = true;
        return;
    }
    if probe.shot && t >= 3.8 {
        println!("ENEMY_PROBE done mode={:?}", mode);
        exit.write(AppExit::Success);
    }
}

/// The retired three-cuboid Guard Husk body, verbatim — kept ONLY as the
/// probe's "before" so one binary can shoot both sides of the swap. The rig
/// hides these boxes and renders the shipped guard over them, which is the
/// honest before: that is what the game looked like.
fn spawn_legacy_husk_body(
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
            Enemy::at(feet, surface, crate::enemies::EnemyKind::Sentinel),
            Health::new(HP_HUSK),
            Poise::new(POISE_HUSK),
        ))
        .with_children(|p| {
            p.spawn((Mesh3d(torso), MeshMaterial3d(armor.clone()), Transform::from_xyz(0.0, 1.0, 0.0)));
            p.spawn((Mesh3d(head), MeshMaterial3d(head_mat), Transform::from_xyz(0.0, 2.0, 0.0)));
            p.spawn((Mesh3d(arm), MeshMaterial3d(armor), Transform::from_xyz(0.55, 1.1, -0.2), HuskArm));
        });
    println!("ENEMY_SPAWN kind=legacy_husk parts=3 height=2.28b feet=({wx:.1},{surface:.1},{wz:.1})");
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
// ===========================================================================
// AI evidence layer — state-transition log + scripted AI exercise
// (VOXELFORGE_AI_LOG / VOXELFORGE_AI_DEMO)
// ===========================================================================

/// Lower-case name for one [`HuskState`], as the AI log prints it.
pub fn husk_state_name(s: HuskState) -> &'static str {
    match s {
        HuskState::Patrol => "patrol",
        HuskState::Alert => "alert",
        HuskState::Chase => "chase",
        HuskState::Telegraph => "telegraph",
        HuskState::Feint => "feint",
        HuskState::Swing1 => "attack(swing1)",
        HuskState::Gap => "gap",
        HuskState::Swing2 => "attack(swing2)",
        HuskState::Recover => "recover",
        HuskState::Reposition => "reposition",
        HuskState::Search => "search",
        HuskState::LungeWind => "telegraph(lunge)",
        HuskState::LungeDash => "attack(lunge-dash)",
        HuskState::Staggered => "staggered",
        HuskState::Dead => "dead",
    }
}

/// Prints exactly one `AI_HUSK` line per state change per husk, plus a periodic
/// `AI_SQUAD` census once there is more than one husk alive — the attack-slot
/// ceiling is only observable as a population, never from one husk's log. This
/// is the perception/tactics pass's evidence layer; run with `VOXELFORGE_AI_LOG`
/// (or the AI demo, which turns it on).
/// Buffers one CSV row per husk per frame for `VOXELFORGE_AI_TRACE`, and flushes
/// the file once the run ends. Position is what the top-down A/B plot is drawn
/// from — the state log alone cannot show a pack clumping or fanning out.
pub fn husk_ai_trace(
    time: Res<Time>,
    mut trace: ResMut<AiTrace>,
    evade: Res<HuskEvade>,
    player_q: Query<&Transform, (With<FlyCam>, Without<Enemy>)>,
    enemy_q: Query<(Entity, &Transform, &Enemy), With<Enemy>>,
) {
    if trace.path.is_none() {
        return;
    }
    let Ok(ptf) = player_q.single() else {
        return;
    };
    let t = time.elapsed_secs();
    // The player is written as its own row (entity `player`) so the plot can
    // draw what the husks were actually converging on, not an assumed origin.
    trace.rows.push(format!(
        "{t:.3},player,{:.3},{:.3},player,0.000,0",
        ptf.translation.x, ptf.translation.z
    ));
    for (entity, etf, e) in enemy_q.iter() {
        let d = Vec3::new(
            etf.translation.x - ptf.translation.x,
            0.0,
            etf.translation.z - ptf.translation.z,
        )
        .length();
        // Evade is not a `HuskState` (see `HuskEvade`), so it needs its own
        // column — otherwise the one behaviour that has no state name would be
        // the one behaviour the plot could not show.
        let evading = evade.per.get(&entity).is_some_and(|s| s.left > 0.0);
        trace.rows.push(format!(
            "{t:.3},{},{:.3},{:.3},{},{d:.3},{}",
            entity.to_bits(),
            etf.translation.x,
            etf.translation.z,
            husk_state_name(e.state),
            u8::from(evading),
        ));
    }
}

/// Writes the buffered trace out when the app is shutting down. Buffering and
/// flushing once keeps a per-frame `File::write` out of the sampled run — an
/// AI trace that changes the frame timing would be measuring itself.
///
/// Gated on an actual `AppExit` rather than "every frame in `Last`": the latter
/// would rewrite a growing CSV once per frame, which is exactly the per-frame
/// disk cost the buffering exists to avoid.
pub fn husk_ai_trace_flush(
    mut exits: bevy::ecs::message::MessageReader<AppExit>,
    trace: Res<AiTrace>,
) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(path) = trace.path.as_ref() else {
        return;
    };
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(dir);
        }
    }
    let mut out = String::from("t,entity,x,z,state,dist,evading\n");
    out.push_str(&trace.rows.join("\n"));
    out.push('\n');
    match std::fs::write(path, out) {
        Ok(()) => println!("AI_TRACE wrote {} rows -> {}", trace.rows.len(), path),
        Err(err) => println!("AI_TRACE FAILED {path}: {err}"),
    }
}

pub fn husk_ai_log(
    time: Res<Time>,
    log: Res<AiLog>,
    mut census: Local<f32>,
    mut q: Query<(Entity, &mut Enemy, &Health)>,
) {
    if !log.enabled {
        return;
    }
    for (entity, mut e, hp) in q.iter_mut() {
        if e.state != e.logged_state {
            println!(
                "AI_HUSK entity={:?} t={:6.2} hp={:3.0} state: {} -> {}",
                entity.to_bits(),
                time.elapsed_secs(),
                hp.cur,
                husk_state_name(e.logged_state),
                husk_state_name(e.state),
            );
            e.logged_state = e.state;
        }
    }
    if q.iter().count() > 1 {
        *census += time.delta_secs();
        if *census >= 0.5 {
            *census = 0.0;
            let (mut total, mut attacking, mut circling) = (0, 0, 0);
            for (_, e, hp) in q.iter() {
                if hp.dead() {
                    continue;
                }
                total += 1;
                if husk_attacking(e.state) {
                    attacking += 1;
                } else if e.state == HuskState::Reposition {
                    circling += 1;
                }
            }
            println!(
                "AI_SQUAD total={} attacking={} (slots={}) circling={}",
                total, attacking, HUSK_ATTACK_SLOTS, circling
            );
        }
    }
}

/// Scripted headless AI exercise (`VOXELFORGE_AI_DEMO`, alongside
/// `VOXELFORGE_PLAY`): drives the *player* through the ranges that light up
/// each AI state — far (patrol), sprinting in (heard → alert → chase), standing
/// ground under melee (telegraph → swings → reposition), kiting out to mid-range
/// (lunge telegraph → dash) — then reports a per-state PASS/FAIL checklist and
/// exits. The husk AI itself is untouched: this only moves the player and holds
/// block, exactly like a human tester would. Each state seen for the first time
/// also saves an in-engine still (`VOXELFORGE_AI_SHOTS`, default
/// `_ai_demo_shots/`) so the run carries its own picture evidence.
#[derive(Resource, Default)]
pub struct HuskAiDemo {
    pub phase: u8,
    pub done: bool,
    pub logged: bool,
    /// Phase-start timestamp (elapsed seconds).
    pub t0: f32,
    /// Observed-state checklist, marked off the live enemy states every frame.
    /// Index order = [`HUSK_AI_STATES`].
    pub saw: [bool; 8],
}

/// Checklist labels, in `HuskAiDemo::saw` index order.
pub const HUSK_AI_STATES: [&str; 8] = [
    "patrol",
    "alert",
    "chase",
    "telegraph",
    "attack(swing)",
    "reposition",
    "lunge telegraph",
    "lunge dash",
];

/// Place the player's eye flush above the terrain surface at `(x, z)` — the
/// same convention `main.rs`'s spawn uses (`surface + 1 + EYE_HEIGHT`). Gravity
/// re-settles anything the terrain height field disagrees with by a fraction.
fn set_ground_eye(ptf: &mut Transform, x: f32, z: f32) {
    ptf.translation = Vec3::new(
        x,
        terrain_height(x, z) as f32 + 1.0 + crate::EYE_HEIGHT,
        z,
    );
}

#[allow(clippy::type_complexity)]
pub fn husk_ai_demo(
    time: Res<Time>,
    mut commands: Commands,
    mut demo: ResMut<HuskAiDemo>,
    mut intent: ResMut<CombatIntent>,
    mut player_q: Query<
        (&mut Transform, &mut Health),
        (With<FlyCam>, Without<Enemy>),
    >,
    enemy_q: Query<(&Transform, &Enemy), (With<Enemy>, Without<FlyCam>)>,
    mut exit: MessageWriter<AppExit>,
) {
    if demo.done {
        return;
    }
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    let Ok((mut ptf, mut php)) = player_q.single_mut() else {
        return;
    };
    if enemy_q.is_empty() {
        return;
    }

    // Mark the observed-state checklist off the live states, every frame. The
    // first frame a state appears, snap an in-engine still of it.
    let shot_dir = std::env::var("VOXELFORGE_AI_SHOTS")
        .unwrap_or_else(|_| "_ai_demo_shots".to_string());
    for (_, e) in enemy_q.iter() {
        let idx = match e.state {
            HuskState::Patrol => Some(0),
            HuskState::Alert => Some(1),
            HuskState::Chase => Some(2),
            HuskState::Telegraph | HuskState::Feint => Some(3),
            HuskState::Swing1 | HuskState::Swing2 => Some(4),
            HuskState::Reposition => Some(5),
            HuskState::LungeWind => Some(6),
            HuskState::LungeDash => Some(7),
            _ => None,
        };
        if let Some(i) = idx {
            if !demo.saw[i] {
                demo.saw[i] = true;
                let path = std::path::Path::new(&shot_dir).join(format!(
                    "ai_{}.png",
                    HUSK_AI_STATES[i].replace('(', "_").replace(')', "").replace(' ', "_")
                ));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path.clone()));
                println!("AI_DEMO shot -> {}", path.display());
            }
        }
    }

    // Nearest living husk — the anchor all the range scripting measures against.
    let anchor = enemy_q
        .iter()
        .filter(|(_, e)| e.state != HuskState::Dead)
        .min_by(|a, b| {
            a.0.translation
                .distance_squared(ptf.translation)
                .partial_cmp(&b.0.translation.distance_squared(ptf.translation))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(tf, _)| tf.translation)
        .unwrap_or(ptf.translation);
    let to_anchor = anchor - ptf.translation;
    let flat = Vec3::new(to_anchor.x, 0.0, to_anchor.z);
    let dist = flat.length();
    let dir_to = if dist > 1e-3 { flat / dist } else { Vec3::Z };

    // No keys headless — the demo owns the intent every frame.
    *intent = CombatIntent::default();
    let phase_age = t - demo.t0;

    match demo.phase {
        0 => {
            // Seed: jump 21 blocks out (beyond every sense), and give the tester
            // a fat HP pool — this proof grades AI states, not damage numbers
            // (combat_demo already owns those). Fat enough that surviving does
            // not depend on whether the demo's block input wins the frame
            // ordering race against `gather_input`.
            let away = ptf.translation - anchor;
            let away = if away.length_squared() < 1e-4 {
                Vec3::Z
            } else {
                Vec3::new(away.x, 0.0, away.z).normalize()
            };
            set_ground_eye(&mut ptf, anchor.x + away.x * 21.0, anchor.z + away.z * 21.0);
            php.max = 1000.0;
            php.cur = 1000.0;
            demo.phase = 1;
            demo.t0 = t;
        }
        1 => {
            // Hold at range: the squad leashes home and patrols.
            if phase_age >= 5.0 {
                demo.phase = 2;
                demo.t0 = t;
            }
        }
        2 => {
            // Sprint in at 7 b/s — above HUSK_SPRINT_NOISE, so the squad hears
            // the player before it ever sees them.
            if dist > 8.0 {
                let step = (7.0 * dt).min(dist - 8.0);
                let (x, z) = (ptf.translation.x + dir_to.x * step, ptf.translation.z + dir_to.z * step);
                set_ground_eye(&mut ptf, x, z);
            } else {
                demo.phase = 3;
                demo.t0 = t;
            }
        }
        3 => {
            // Stand ground, guard up: watch alert→chase→telegraph→swing→reposition.
            intent.block = true;
            if (demo.saw[3] && demo.saw[4] && demo.saw[5]) || phase_age >= 14.0 {
                demo.phase = 4;
                demo.t0 = t;
            }
        }
        4 => {
            // Kite out to mid-range and hold: bait the lunge.
            intent.block = true;
            if dist < 7.2 {
                let step = (7.0 * dt).min(7.2 - dist);
                let (x, z) = (ptf.translation.x - dir_to.x * step, ptf.translation.z - dir_to.z * step);
                set_ground_eye(&mut ptf, x, z);
            } else if (demo.saw[6] && demo.saw[7]) || phase_age >= 10.0 {
                demo.phase = 5;
                demo.t0 = t;
            }
        }
        5 => {
            // Let the lunge land, walk back in, and let a second exchange (and
            // the lunge cooldown firing again) play out on the log.
            intent.block = true;
            if phase_age >= 2.0 && dist > 2.8 {
                let step = (7.0 * dt).min(dist - 2.8);
                let (x, z) = (ptf.translation.x + dir_to.x * step, ptf.translation.z + dir_to.z * step);
                set_ground_eye(&mut ptf, x, z);
            }
            if phase_age >= 9.0 {
                demo.phase = 6;
                demo.t0 = t;
            }
        }
        _ => {
            if !demo.logged {
                demo.logged = true;
                demo.done = true;
                for (i, name) in HUSK_AI_STATES.iter().enumerate() {
                    println!(
                        "AI_DEMO state {name}: {}",
                        if demo.saw[i] { "PASS" } else { "FAIL" }
                    );
                }
                let all = demo.saw.iter().all(|&ok| ok);
                println!("AI_DEMO overall => {}", if all { "PASS" } else { "FAIL" });
                exit.write(AppExit::Success);
            }
        }
    }
}

/// Demo-only (`VOXELFORGE_AI_DEMO`): tops the encounter up to a 3-husk squad so
/// separation steering and attack-slot arbitration are actually exercised.
/// Normal play is untouched — `spawn_encounter`'s single husk stands.
fn spawn_ai_squad(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player: Query<&Transform, With<FlyCam>>,
) {
    let Ok(tf) = player.single() else {
        return;
    };
    // Two extra husks on flanking angles: inside aggro, outside melee, and
    // spaced from each other so the separation force has room to be seen working.
    let offs = [(6.0, 2.0), (-5.0, 4.0)];
    for &(dx, dz) in &offs {
        spawn_guard_husk(
            &mut commands,
            &mut meshes,
            &mut materials,
            tf.translation.x + dx,
            tf.translation.z + dz,
            None,
        );
    }
    println!("AI_DEMO squad spawned: 2 flanking husks added (3 total)");
}

pub struct CombatFeelPlugin;

impl Plugin for CombatFeelPlugin {
    fn build(&self, app: &mut App) {
        let probe = std::env::var("VOXELFORGE_FEEL_PROBE").is_ok();
        let log = probe || std::env::var("VOXELFORGE_FEEL_LOG").is_ok();
        // The R-key arbiter rides along here so no other file has to change: the
        // resource, its once-per-frame reset, and the scripted routing probe.
        let r_probe_env = std::env::var("VOXELFORGE_RKEY_PROBE").ok();
        let r_probe = r_probe_env.is_some();
        let r_log = r_probe || std::env::var("VOXELFORGE_RKEY_LOG").is_ok();
        // AI evidence layer: transition log + the scripted exercise. The demo
        // turns the log on too — a run should never have to remember two flags.
        let ai_demo = std::env::var("VOXELFORGE_AI_DEMO").is_ok();
        let ai_log = ai_demo || std::env::var("VOXELFORGE_AI_LOG").is_ok();
        // The before/after lever + the top-down trace the A/B plot is drawn from.
        // Both default off, so shipping play is untouched by either.
        let ai_legacy = std::env::var("VOXELFORGE_AI_LEGACY").is_ok();
        let ai_trace = std::env::var("VOXELFORGE_AI_TRACE").ok();
        // Deterministic enemy-silhouette still (VOXELFORGE_ENEMY_PROBE=before|after).
        let probe_mode = match std::env::var("VOXELFORGE_ENEMY_PROBE").as_deref() {
            Ok("before") => Some(EnemyProbeMode::Before),
            Ok("after") => Some(EnemyProbeMode::After),
            _ => None,
        };
        let ai_tracing = ai_trace.is_some();
        if ai_legacy {
            println!("AI_MODE legacy (pre-perception/tactics behaviour)");
        } else {
            println!("AI_MODE tactics (perception + squad pass)");
        }
        app.add_message::<ImpactEvent>()
            .add_message::<StaggerEvent>()
            .add_message::<DodgeEvent>()
            .add_message::<EnemyDied>()
            .insert_resource(FeelLog { enabled: log })
            .insert_resource(FeelProbe { enabled: probe, ..default() })
            .insert_resource(RKeyRoute { log: r_log, ..default() })
            .insert_resource(
                r_probe_env
                    .as_deref()
                    .map(RKeyProbe::from_env)
                    .unwrap_or_default(),
            )
            .insert_resource(PlayerKinematics::default())
            .insert_resource(AiLog { enabled: ai_log })
            .insert_resource(AiTactics { legacy: ai_legacy })
            .insert_resource(HuskEvade::default())
            .insert_resource(AiTrace { path: ai_trace, rows: Vec::new() })
            .insert_resource(HuskAiDemo::default())
            .insert_resource(EnemyProbe { mode: probe_mode, spawned: false, shot: false })
            // Not state-gated: a claim left over from the last Play frame would
            // silently eat the first press of the next one.
            .add_systems(First, reset_r_route)
            .add_systems(
                OnEnter(crate::editor::AppState::Play),
                spawn_ai_squad.run_if(move || ai_demo),
            )
            .add_systems(
                Update,
                (
                    enemy_probe
                        .before(crate::fly_camera)
                        .run_if(move || probe_mode.is_some()),
                    feel_probe
                        .before(gather_input)
                        .before(crate::fly_camera)
                        .run_if(|p: Res<FeelProbe>| p.enabled),
                    r_route_probe
                        .before(gather_input)
                        .run_if(move || r_probe),
                    apply_knockback
                        .after(player_combat)
                        .before(husk_ai),
                    // The demo moves the player *before* the husks perceive, so a
                    // scripted step registers the same frame it happens — and
                    // *after* gather_input, so its held block intent is what
                    // player_combat reads rather than being overwritten by the
                    // (keyless) keyboard gather in the same frame.
                    husk_ai_demo
                        .after(gather_input)
                        .before(husk_ai)
                        .run_if(move || ai_demo),
                    husk_ai_log
                        .after(husk_ai)
                        .run_if(|l: Res<AiLog>| l.enabled),
                    // Sampled after the AI has moved the bodies, so a row is
                    // where the husk ended the frame, not where it started.
                    husk_ai_trace
                        .after(husk_ai)
                        .run_if(move || ai_tracing),
                    combat_feel_log
                        .after(husk_ai)
                        .run_if(|f: Res<FeelLog>| f.enabled),
                )
                    .run_if(in_state(crate::editor::AppState::Play)),
            )
            // Not Play-gated and not in Update: the demo writes `AppExit` from
            // inside Update, so a Play-gated flush in the same schedule can lose
            // the final frames. `Last` runs after the exit is booked but before
            // the app actually tears down.
            .add_systems(Last, husk_ai_trace_flush.run_if(move || ai_tracing));
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

/// Pure target-switch logic (§2.3): nearest live candidate other than `current`
/// within `LOCK_RANGE`. Takes plain tuples (not a `Query`) so it is unit-testable
/// without spinning up a `World` — mirrors `nearest_target`'s range gate, but
/// intentionally skips the acquisition cone: once locked, the player is already
/// oriented at the fight, and gating a *switch* by facing would make the enemy
/// you just turned away from unreachable.
fn cycle_target(current: Entity, origin: &Vec3, candidates: &[(Entity, Vec3, bool)]) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for &(e, pos, dead) in candidates {
        if e == current || dead {
            continue;
        }
        let dist = pos.distance(*origin);
        if dist > LOCK_RANGE {
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

    #[test]
    fn impact_sound_scales_with_weight_not_the_button() {
        // A riposte and a poise break both classify as Critical — the heaviest
        // hits in the kit — and neither may sound like the light tap their swing
        // started as. No distinct critical asset exists yet, so Critical borrows
        // the heavy impact.
        assert!(matches!(impact_sfx(ImpactWeight::Light, Vec3::ZERO), SfxEvent::HitLight { .. }));
        assert!(matches!(impact_sfx(ImpactWeight::Heavy, Vec3::ZERO), SfxEvent::HitHeavy { .. }));
        assert!(matches!(impact_sfx(ImpactWeight::Critical, Vec3::ZERO), SfxEvent::HitHeavy { .. }));
    }

    #[test]
    fn parry_and_stagger_shake_louder_than_a_light_hit() {
        // §5.3: parry (0.12) and stagger break (0.20) both out-shake a light
        // (0.04); the stagger break is the loudest row in the table.
        assert!(SHAKE_PARRY.0 > SHAKE_LIGHT.0);
        assert!(SHAKE_STAGGER.0 > SHAKE_PARRY.0);
        assert!(SHAKE_STAGGER.0 > SHAKE_ENEMY_HIT.0);
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

    // -- lock-on target switching (§2.3) ------------------------------------

    #[test]
    fn cycle_target_switches_to_the_next_nearest_live_enemy() {
        let cur = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let near = Entity::from_raw_u32(3).unwrap();
        let origin = Vec3::ZERO;
        let candidates = [
            (cur, Vec3::new(2.0, 0.0, 0.0), false),
            (far, Vec3::new(10.0, 0.0, 0.0), false),
            (near, Vec3::new(4.0, 0.0, 0.0), false),
        ];
        // `cur` itself must never be returned, and among the remaining live
        // candidates the nearer one wins over the farther one.
        assert_eq!(cycle_target(cur, &origin, &candidates), Some(near));
    }

    #[test]
    fn cycle_target_skips_dead_and_out_of_range_candidates() {
        let cur = Entity::from_raw_u32(1).unwrap();
        let dead = Entity::from_raw_u32(2).unwrap();
        let out_of_range = Entity::from_raw_u32(3).unwrap();
        let origin = Vec3::ZERO;
        let candidates = [
            (cur, Vec3::new(2.0, 0.0, 0.0), false),
            (dead, Vec3::new(3.0, 0.0, 0.0), true),
            (out_of_range, Vec3::new(LOCK_RANGE + 1.0, 0.0, 0.0), false),
        ];
        assert_eq!(cycle_target(cur, &origin, &candidates), None);
    }

    #[test]
    fn cycle_target_is_a_noop_when_no_other_enemy_exists() {
        let cur = Entity::from_raw_u32(1).unwrap();
        let origin = Vec3::ZERO;
        let candidates = [(cur, Vec3::new(2.0, 0.0, 0.0), false)];
        assert_eq!(cycle_target(cur, &origin, &candidates), None);
    }

    // -----------------------------------------------------------------------
    // R-key routing — one key, two consumers (`RKeyRoute`)
    // -----------------------------------------------------------------------

    /// Stands in for `quest::check_block_place_triggers`: claims R when the build
    /// prompt is in reach. Runs first, exactly like the real one does via
    /// `.before(combat::gather_input)`.
    fn stub_quest_claim(keys: Res<ButtonInput<KeyCode>>, mut route: ResMut<RKeyRoute>) {
        if keys.just_pressed(KeyCode::KeyR) {
            route.claim(RKeyUse::QuestPlace, "test::stub_quest_claim");
        }
    }

    /// A frame with the same wiring the app has: reset in `First`, the contextual
    /// consumer ahead of `gather_input` in `Update`. `quest_in_reach` decides
    /// whether the build prompt is there to claim the press.
    fn r_route_app(quest_in_reach: bool) -> App {
        let mut app = App::new();
        app.init_resource::<RKeyRoute>()
            .init_resource::<CombatIntent>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(First, reset_r_route);
        if quest_in_reach {
            app.add_systems(Update, (stub_quest_claim, gather_input).chain());
        } else {
            app.add_systems(Update, gather_input);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app
    }

    #[test]
    fn quest_claim_starves_lock_on_in_the_same_frame() {
        let mut app = r_route_app(true);
        app.update();
        // One press, one consumer: the build prompt took it, so the intent the
        // combat state machine reads carries no lock toggle at all.
        assert_eq!(app.world().resource::<RKeyRoute>().route(), RKeyUse::QuestPlace);
        assert_eq!(
            app.world().resource::<RKeyRoute>().claimed_by(),
            "test::stub_quest_claim"
        );
        assert!(!app.world().resource::<CombatIntent>().lock_toggle);
    }

    #[test]
    fn lock_on_takes_r_when_nothing_contextual_claims_it() {
        let mut app = r_route_app(false);
        app.update();
        assert_eq!(app.world().resource::<RKeyRoute>().route(), RKeyUse::LockOn);
        assert!(app.world().resource::<CombatIntent>().lock_toggle);
    }

    #[test]
    fn an_unpressed_frame_claims_nothing() {
        let mut app = r_route_app(false);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(KeyCode::KeyR);
        app.update();
        // Lock-on must not squat on the route while R is idle — the quest side
        // would then be denied on the frame the player actually presses it.
        assert_eq!(app.world().resource::<RKeyRoute>().route(), RKeyUse::Unclaimed);
        assert!(!app.world().resource::<CombatIntent>().lock_toggle);
    }

    #[test]
    fn the_route_is_cleared_every_frame() {
        let mut app = r_route_app(true);
        app.update();
        let f1 = app.world().resource::<RKeyRoute>().frame();
        // Second frame, same held key: the reset must have wiped the claim, so
        // the claim below is a fresh one rather than a leftover.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        let route = app.world().resource::<RKeyRoute>();
        assert_eq!(route.frame(), f1 + 1);
        assert_eq!(route.route(), RKeyUse::Unclaimed);
    }

    #[test]
    fn probe_taps_come_from_the_env_but_fall_back_to_the_default_pair() {
        // `VOXELFORGE_RKEY_PROBE=1` is the "just switch it on" form.
        assert_eq!(RKeyProbe::from_env("1").taps, RKeyProbe::default().taps);
        assert_eq!(RKeyProbe::from_env("").taps, RKeyProbe::default().taps);
        // A retimed pair survives whitespace and is taken verbatim.
        assert_eq!(RKeyProbe::from_env("12, 40.5").taps, vec![12.0, 40.5]);
    }

    #[test]
    fn a_second_claim_in_one_frame_is_denied() {
        let mut route = RKeyRoute::default();
        assert!(route.claim(RKeyUse::QuestPlace, "first"));
        assert!(!route.claim(RKeyUse::LockOn, "second"));
        assert_eq!(route.route(), RKeyUse::QuestPlace);
        assert_eq!(route.claimed_by(), "first");
    }

    // -- Perception & tactics pass -------------------------------------------

    #[test]
    fn sight_cone_only_covers_the_front_half_the_husk_looks_at() {
        // Facing yaw 0 (atan2(z,x) convention → +X): a player at +X is in the
        // 55° cone; a player behind (-X, 180° away) is not — a husk with its
        // back turned must not "see", it has to hear or be crowded first.
        let facing = 0.0f32;
        let player_ahead = 0.0f32; // atan2(0, 1)
        let player_behind = std::f32::consts::PI; // atan2(0, -1)
        assert!(angle_delta(facing, player_ahead) <= HUSK_SIGHT_HALF.to_radians());
        assert!(angle_delta(facing, player_behind) > HUSK_SIGHT_HALF.to_radians());
        // The delta itself is the short way round and never negative-signed big.
        assert!((angle_delta(0.0, std::f32::consts::TAU - 0.1) - 0.1).abs() < 1e-4);
    }

    #[test]
    fn separation_pushes_away_from_a_close_squadmate() {
        // Two squadmates a block apart: the push points away from the neighbour
        // and is non-trivial — this is the anti-clump force.
        let a = Entity::from_bits(1);
        let b = Entity::from_bits(2);
        let squad = vec![(a, Vec3::new(0.0, 0.0, 0.0), HuskState::Chase)];
        let push = husk_separation(&squad, b, Vec3::new(1.0, 0.0, 0.0));
        assert!(push.x < 0.0, "push must point away from the squadmate");
        assert!((push.z - 0.0).abs() < 1e-6);
        assert!(push.length() > 0.1);
    }

    #[test]
    fn separation_fades_to_zero_past_the_radius_and_ignores_self() {
        let a = Entity::from_bits(1);
        let far = vec![(a, Vec3::new(50.0, 0.0, 0.0), HuskState::Chase)];
        assert_eq!(husk_separation(&far, a, Vec3::ZERO), Vec3::ZERO);
        // Beyond HUSK_SEPARATION apart: no force either.
        let me = Entity::from_bits(2);
        let edge = vec![(a, Vec3::new(HUSK_SEPARATION + 0.5, 0.0, 0.0), HuskState::Chase)];
        assert_eq!(husk_separation(&edge, me, Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn only_committed_states_hold_an_attack_slot() {
        // Wind-ups and swings (melee and lunge alike) occupy a slot; the
        // between-commitment beats do not — that gap is what lets a squadmate
        // step in, so a pack flows instead of pulsing in unison.
        for held in [
            HuskState::Telegraph,
            HuskState::Feint,
            HuskState::Swing1,
            HuskState::Swing2,
            HuskState::LungeWind,
            HuskState::LungeDash,
        ] {
            assert!(husk_attacking(held), "{held:?} must hold a slot");
        }
        for free in [
            HuskState::Patrol,
            HuskState::Alert,
            HuskState::Chase,
            HuskState::Gap,
            HuskState::Recover,
            HuskState::Reposition,
            HuskState::Staggered,
            HuskState::Dead,
        ] {
            assert!(!husk_attacking(free), "{free:?} must not hold a slot");
        }
    }

    #[test]
    fn alert_beat_outruns_the_human_reaction_it_mimics() {
        // The "noticed you" pause must be long enough to read (>0.3 s) but
        // short enough to feel like surprise, not idle (<1 s).
        assert!(HUSK_ALERT_TIME >= 0.3 && HUSK_ALERT_TIME <= 1.0);
        // And the lunge telegraph must be *longer* than the melee one — it
        // covers more ground, so the player needs more warning to sidestep.
        assert!(HUSK_LUNGE_WIND > HUSK_TELEGRAPH);
        // The lunge only exists as a gap-closer: its band sits beyond melee.
        assert!(HUSK_LUNGE_RANGE > MELEE_RANGE);
        // Attack slots must actually limit something (>= 2 lets a pair press).
        assert!(HUSK_ATTACK_SLOTS >= 1);
    }
}
