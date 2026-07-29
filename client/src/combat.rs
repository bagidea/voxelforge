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
    /// Remaining i-frame time (dodge). While > 0 the player ignores damage.
    pub iframes: f32,
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
        self.iframes > 0.0
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
    Swing1,
    Gap,
    Swing2,
    Recover,
    Staggered,
    Dead,
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
#[derive(Resource, Default)]
pub struct Shake {
    pub amp: f32,
    pub time: f32,
    pub dur: f32,
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
}

/// Marker components for the HUD bars.
#[derive(Component)]
pub struct HealthBar;
#[derive(Component)]
pub struct StaminaBar;
#[derive(Component)]
pub struct LockReticle;

/// Scripted headless combat proof (like `walk_demo`): drives `CombatIntent` on a
/// timeline, reads back component state, prints PASS lines, then exits.
#[derive(Resource, Default)]
pub struct CombatDemo {
    pub done: bool,
    pub start_stam: f32,
    pub after_attack_stam: f32,
    pub husk_hp0: f32,
    pub husk_hp1: f32,
    pub saw_iframe: bool,
    pub logged: bool,
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
) {
    let surface = terrain_height(wx, wz) as f32 + 1.0; // top face of the ground voxel
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

/// Spawn the two HUD bars (health, stamina) + the lock-on reticle. Called from
/// `setup`. Kept as absolute-positioned Nodes whose fill width is driven by
/// `hud_bars`.
pub fn spawn_combat_hud(commands: &mut Commands) {
    // Health bar (top-left, under the debug text).
    bar(commands, 34.0, Color::srgb(0.82, 0.20, 0.18), HealthBarTag::Health);
    // Stamina bar just below it.
    bar(commands, 50.0, Color::srgb(0.30, 0.78, 0.36), HealthBarTag::Stamina);

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
    *intent = CombatIntent {
        light: mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::KeyX),
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
    intent: Res<CombatIntent>,
    mut lock: ResMut<LockOn>,
    mut shake: ResMut<Shake>,
    mut player_q: Query<
        (&mut Transform, &mut PlayerCombat, &mut Stamina, &Health, &mut Poise),
        (With<FlyCam>, Without<Enemy>),
    >,
    mut enemy_q: Query<(Entity, &Transform, &mut Health, &mut Poise), (With<Enemy>, Without<FlyCam>)>,
) {
    let dt = time.delta_secs();
    let Ok((mut ptf, mut pc, mut stam, hp, mut poise)) = player_q.single_mut() else {
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
    if hp.dead() {
        pc.state = CombatState::Dead;
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
        pc.start_dodge(&mut stam);
    } else if intent.parry && pc.can_act(&stam) {
        if stam.try_spend(COST_PARRY, DELAY_PARRY) {
            pc.state = CombatState::Parry;
            pc.timer = 0.0;
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
            pc.start_charged(&mut stam);
        } else {
            pc.start_heavy(&mut stam);
        }
        pc.charge = 0.0;
    } else if intent.light {
        pc.start_light(&mut stam);
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
        for (_, etf, mut ehp, mut ep) in enemy_q.iter_mut() {
            if ehp.dead() {
                continue;
            }
            let to = etf.translation - origin;
            let dist = to.length();
            if dist > MELEE_RANGE + PLAYER_HALF_W + 0.6 {
                continue;
            }
            if !in_cone(facing, to, MELEE_CONE) {
                continue;
            }
            // Stagger amplifies damage (§3.2); a fresh parry punish adds +25% (§2.5).
            let mut mult = if ep.staggered() { STAGGER_DMG_MULT } else { 1.0 };
            if pc.punish > 0.0 {
                mult *= PARRY_PUNISH_MULT;
            }
            ehp.damage(dmg * mult);
            let broke = ep.take(poise_dmg, false);
            landed = true;
            // Hit-stop on both (§5.2) + screen-shake (§5.3).
            pc.hitstop = if broke { HITSTOP_STAGGER } else { HITSTOP_LIGHT };
            shake.hit(if matches!(pc.state, CombatState::Heavy | CombatState::Charged) {
                SHAKE_HEAVY
            } else {
                SHAKE_LIGHT
            });
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
    mut shake: ResMut<Shake>,
    mut enemy_q: Query<(&mut Transform, &mut Enemy, &Health, &mut Poise), (With<Enemy>, Without<FlyCam>)>,
    mut player_q: Query<
        (&Transform, &mut Health, &mut PlayerCombat, &mut Stamina, &mut Poise),
        (With<FlyCam>, Without<Enemy>),
    >,
) {
    let dt = time.delta_secs();
    let Ok((ptf, mut php, mut pc, mut pstam, mut ppoise)) = player_q.single_mut() else {
        return;
    };

    for (mut etf, mut e, ehp, mut ep) in enemy_q.iter_mut() {
        ep.tick(dt);
        if ehp.dead() {
            e.state = HuskState::Dead;
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
        if e.hitstop > 0.0 {
            e.hitstop = (e.hitstop - dt).max(0.0);
            continue;
        }

        let to_player = ptf.translation - etf.translation;
        let dist = Vec3::new(to_player.x, 0.0, to_player.z).length();
        e.timer += dt;

        match e.state {
            HuskState::Patrol => {
                // Walk a 12-block loop along X from the origin (§4.1).
                let step = HUSK_WALK * dt * e.patrol_dir;
                etf.translation.x += step;
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
                    e.state = HuskState::Telegraph; // begin honest wind-up
                    e.timer = 0.0;
                    e.hit_applied = false;
                } else {
                    let dir = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    etf.translation += dir * HUSK_WALK * dt;
                }
            }
            HuskState::Telegraph => {
                if e.timer >= HUSK_TELEGRAPH {
                    e.state = HuskState::Swing1;
                    e.timer = 0.0;
                    e.hit_applied = false;
                }
            }
            HuskState::Swing1 => {
                if !e.hit_applied && e.timer <= HUSK_ACTIVE {
                    if try_hit_player(
                        HUSK_SWING1_DMG, HUSK_SWING1_POISE, dist, &mut php, &mut pc,
                        &mut pstam, &mut ppoise, &mut shake,
                    ) {
                        e.hitstop = HITSTOP_ENEMY;
                        // Parried? The player's punish window just opened → the
                        // Husk eats posture damage and a longer hit-stop (§2.5).
                        if pc.punish > 0.0 {
                            ep.take(PARRY_POSTURE, false);
                            e.hitstop = HITSTOP_PARRY;
                        }
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
                    if try_hit_player(
                        HUSK_SWING2_DMG, HUSK_SWING2_POISE, dist, &mut php, &mut pc,
                        &mut pstam, &mut ppoise, &mut shake,
                    ) {
                        e.hitstop = HITSTOP_ENEMY;
                        if pc.punish > 0.0 {
                            ep.take(PARRY_POSTURE, false);
                            e.hitstop = HITSTOP_PARRY;
                        }
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

/// One Husk swing against the player. Honors i-frames (negate), block (50% +
/// stamina), parry window (negate + posture) per §2.5. Returns true if the swing
/// connected at all (for hit-stop bookkeeping).
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
) -> bool {
    if dist > MELEE_RANGE + PLAYER_HALF_W + 0.4 {
        return false; // player stepped out of reach
    }
    // Dodge i-frames negate everything (§2.2).
    if pc.invulnerable() {
        return false;
    }
    // Parry window (§2.5): tap parry as the hit lands → negate + posture + punish.
    if pc.state == CombatState::Parry && pc.timer <= PARRY_WINDOW {
        ppoise.take(0.0, false); // no self-damage; parry succeeded
        pc.punish = PARRY_PUNISH; // +25% window opens on the enemy
        shake.hit(SHAKE_LIGHT);
        pc.hitstop = HITSTOP_PARRY;
        return true;
    }
    // Failed parry (§2.5): threw the parry but the 0.20 s window had closed —
    // punished with +25% damage and a 0.5 s recovery lock.
    if pc.state == CombatState::Parry {
        php.damage(dmg * PARRY_FAIL_MULT);
        pc.recovery = PARRY_FAIL_RECOVER;
        ppoise.take(poise_dmg, false);
        shake.hit(SHAKE_ENEMY_HIT);
        return true;
    }
    // Block (§2.5): 50% off if stamina can pay, else guard-break stagger.
    if pc.state == CombatState::Block {
        if pstam.try_spend(COST_BLOCK, DELAY_BLOCK) {
            php.damage(dmg * (1.0 - BLOCK_REDUCTION));
            ppoise.take(poise_dmg * 0.5, false);
            shake.hit(SHAKE_LIGHT);
            return true;
        } else {
            php.damage(dmg);
            ppoise.stagger = GUARD_BREAK; // guard broken → stagger
            shake.hit(SHAKE_ENEMY_HIT);
            return true;
        }
    }
    // Plain hit.
    php.damage(dmg);
    ppoise.take(poise_dmg, pc.hyper_armor());
    shake.hit(SHAKE_ENEMY_HIT);
    true
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
pub fn apply_shake(
    time: Res<Time>,
    mut shake: ResMut<Shake>,
    mut cam_q: Query<&mut Transform, With<crate::OrbitCam>>,
) {
    if shake.amp <= 0.0 || shake.dur <= 0.0 {
        return;
    }
    shake.time += time.delta_secs();
    if shake.time >= shake.dur {
        shake.amp = 0.0;
        return;
    }
    let Ok(mut ctf) = cam_q.single_mut() else { return };
    let decay = 1.0 - (shake.time / shake.dur);
    // Deterministic pseudo-jitter from the phase (no Math.random needed).
    let ph = shake.time * 90.0;
    let off = Vec3::new(ph.sin(), (ph * 1.3).cos(), 0.0) * shake.amp * decay;
    ctf.translation += off;
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

// ===========================================================================
// Headless scripted proof (VOXELFORGE_COMBAT_DEMO=1) — mirrors walk_demo.
// ===========================================================================

/// Drives the real combat systems on a fixed timeline and prints PASS lines
/// proving the loop: attack → hit lands → stamina drops → dodge grants i-frames.
/// Then writes `AppExit`. Requires a player + one Husk in the scene.
#[allow(clippy::type_complexity)]
pub fn combat_demo(
    time: Res<Time>,
    mut demo: ResMut<CombatDemo>,
    mut intent: ResMut<CombatIntent>,
    player_q: Query<(&PlayerCombat, &Stamina), With<FlyCam>>,
    enemy_q: Query<&Health, With<Enemy>>,
    mut exit: MessageWriter<AppExit>,
) {
    if demo.done {
        return;
    }
    let t = time.elapsed_secs();
    let Ok((pc, stam)) = player_q.single() else { return };
    let husk_hp = enemy_q.iter().next().map(|h| h.cur).unwrap_or(-1.0);

    // Timeline (seconds): let the scene settle, snapshot, attack, dodge, report.
    // Override the keyboard intent for exactly the frames we act.
    *intent = CombatIntent::default();

    if t < 1.0 {
        demo.start_stam = stam.cur;
        demo.husk_hp0 = husk_hp;
        return;
    }
    // 1.0s: fire a light attack (single frame).
    if t >= 1.0 && t < 1.05 && demo.after_attack_stam == 0.0 {
        intent.light = true;
        return;
    }
    // 1.3s: attack resolved — snapshot stamina + husk HP.
    if (1.3..1.35).contains(&t) && demo.after_attack_stam == 0.0 {
        demo.after_attack_stam = stam.cur;
        demo.husk_hp1 = husk_hp;
        return;
    }
    // 1.6s: dodge.
    if (1.6..1.65).contains(&t) {
        intent.dodge = true;
        return;
    }
    // 1.6–1.9s: watch for active i-frames.
    if pc.invulnerable() {
        demo.saw_iframe = true;
    }
    // 2.4s: report and exit.
    if t >= 2.4 && !demo.logged {
        demo.logged = true;
        demo.done = true;
        let stam_dropped = demo.after_attack_stam < demo.start_stam - 0.01;
        let husk_dropped = demo.husk_hp1 < demo.husk_hp0 - 0.01;
        println!(
            "COMBAT_DEMO attack: husk_hp {:.0}->{:.0} ({}) | stamina {:.0}->{:.0} ({}) => {}",
            demo.husk_hp0, demo.husk_hp1,
            if husk_dropped { "PASS" } else { "FAIL" },
            demo.start_stam, demo.after_attack_stam,
            if stam_dropped { "PASS" } else { "FAIL" },
            if husk_dropped && stam_dropped { "PASS" } else { "FAIL" }
        );
        println!(
            "COMBAT_DEMO dodge i-frames: saw_iframe={} => {}",
            demo.saw_iframe,
            if demo.saw_iframe { "PASS" } else { "FAIL" }
        );
        let ok = husk_dropped && stam_dropped && demo.saw_iframe;
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

        // i-frames expire after ~167 ms.
        pc.tick(DODGE_IFRAMES + 0.001);
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
}
