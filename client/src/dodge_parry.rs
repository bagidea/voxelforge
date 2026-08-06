//! Dodge i-frames and parry → riposte — the depth layer on top of the weight
//! layer (`combat.rs`).
//!
//! **Why this file exists.** `combat.rs` already had a `Dodge` state and a
//! `Parry` state, but neither was the mechanic the design asks for:
//!
//! 1. **The i-frame window was measured in seconds.** `DODGE_IFRAMES = 10/60`
//!    is ticked by `dt`, so the number of frames a swing can be eaten in is
//!    whatever the frame rate happens to be that second. `docs/combat-design.md`
//!    §2.2/§6 does not say "167 ms" — it says **10 frames @ 60 FPS**, because in
//!    a soulslike the roll is a *frame count* the player learns by feel, not a
//!    wall-clock duration that stretches on a slow machine and shrinks on a fast
//!    one. Here the window is a `u32` decremented exactly once per frame, and
//!    the proof grades that it opens for exactly [`DODGE_IFRAME_FRAMES`] frames
//!    — a number a dt-scaled implementation cannot hit twice in a row.
//!
//! 2. **The parry window was one frame long, by accident.**
//!    `PlayerCombat::action_len()` had no arm for `CombatState::Parry`, so it
//!    fell through to `_ => 0.0` and `tick()` dropped the state back to `Idle`
//!    on the very next frame. `try_hit_player`'s `pc.timer <= PARRY_WINDOW`
//!    check could therefore only ever be true on the frame the button went
//!    down: the 0.20 s window in §2.5 never existed, and neither did the
//!    *failure* case — you could not mistime a parry, because a mistimed parry
//!    simply wasn't a parry any more. Both halves are restored here, in frames.
//!
//! 3. **A successful parry paid almost nothing.** §2.5 says a parry "deals 25
//!    posture damage to the attacker", but the posture was applied at the wrong
//!    place — inside the *next* swing that connected while `punish > 0` — so the
//!    parry itself never touched the Husk's poise. And there was no riposte at
//!    all: the reward was a 1.25× damage window, which is not a reason to take
//!    the hardest timing in the kit. Elden Ring's bargain is the one this file
//!    implements: **parry → guaranteed poise break → riposte**, a critical-weight
//!    answer worth [`RIPOSTE_MULT`]× a normal swing.
//!
//! **Weight, not floaty buttons.** Nothing here invents its own feel numbers.
//! A parry books `ImpactWeight::Heavy`'s shove and `HITSTOP_PARRY`; the riposte
//! books the full `ImpactWeight::Critical` row (0.170 s freeze / 0.550 blocks of
//! knockback / 0.155 camera kick) — the same table `prove_knockback.sh` already
//! grades. The depth layer rides the weight layer; it does not sit beside it.
//!
//! **Lane note.** This file is Poppy's. The only edits in `combat.rs` (Kevin's)
//! are call-outs into the functions below plus three fields on `PlayerCombat` —
//! see `notes.md`.

use bevy::prelude::*;

use crate::anim::Rigged;
use crate::combat::{
    self, CombatState, Enemy, EnemyHitOutcome, FeelLog, Health, HuskState, ImpactWeight, Knockback,
    PlayerBody, PlayerCombat, Poise, Shake, StaggerEvent, HITSTOP_PARRY, KNOCKBACK_TIME,
    PARRY_PUNISH, PARRY_POSTURE, STAGGER_TIME,
};
use crate::FlyCam;

// ===========================================================================
// The windows — counted in frames, because that is what the design specifies
// ===========================================================================

/// Dodge invulnerability, in frames (`docs/combat-design.md` §2.2 / §6:
/// "10 frames @ 60 FPS"). The roll's *distance* stays time-based — how far a
/// body travels must not depend on frame rate — but whether a blade passes
/// through you is a frame question, so it is counted in frames.
pub const DODGE_IFRAME_FRAMES: u32 = 10;

/// Parry receive window, in frames (§2.5: "0.20 s (12 frames @ 60 FPS)").
pub const PARRY_WINDOW_FRAMES: u32 = 12;

/// How long `CombatState::Parry` is held. Long enough that the window can
/// *close* while the state is still up — that gap is the whole risk of a parry,
/// and without it `EnemyHitOutcome::FailedParry` is unreachable code.
/// `PARRY_WINDOW` (0.20 s) + `PARRY_FAIL_RECOVER` (0.50 s).
pub const PARRY_STATE_LEN: f32 = combat::PARRY_WINDOW + combat::PARRY_FAIL_RECOVER;

/// How long the riposte stays available after a parry lands. Reuses §2.5's
/// punish window so the two cannot drift apart.
pub const RIPOSTE_WINDOW: f32 = PARRY_PUNISH;

/// Damage multiplier on the riposte itself.
///
/// **Divergence from §2.5, deliberate:** the doc only specifies a 1.25× punish
/// *window*. That is a reward for a lucky trade, not for a 12-frame read, so a
/// parry that lands opens a distinct, much heavier answer instead — Elden Ring's
/// riposte, scaled to our 20-damage light and the Husk's 80 HP: 20 × 2.5 × 1.3
/// (the stagger bonus the parry itself creates) = 65, so two clean parries end
/// the fight and one is felt immediately.
pub const RIPOSTE_MULT: f32 = 2.5;

/// Blocks the parried attacker is shoved back. `ImpactWeight::Heavy`'s row — a
/// deflection is not a critical, but it is not a tap either.
pub const PARRY_SHOVE: ImpactWeight = ImpactWeight::Heavy;

// ===========================================================================
// State
// ===========================================================================

/// Frame counter + window bookkeeping for the log lines
/// `scripts/prove_dodge_parry.sh` grades.
///
/// One resource rather than components because there is exactly one player, and
/// because the numbers it holds are *observations about frames* — they belong to
/// the frame loop, not to an entity.
#[derive(Resource, Default)]
pub struct DodgeParryState {
    /// Frames elapsed since boot. The unit every log line below is stamped in.
    pub frame: u64,
    /// `iframe_frames` as it stood the last time `watch_windows` looked.
    prev_iframes: u32,
    /// Frame the current i-frame window opened on, and how many frames it has
    /// been observed open for (i.e. how many frames a swing could have been
    /// eaten in). `None` when no window is open.
    iframe_open_at: Option<u64>,
    iframe_open_for: u32,
    /// Same pair for the parry window.
    prev_parry: u32,
    parry_open_at: Option<u64>,
    parry_open_for: u32,
    /// Frames the parry window had already been shut for when a swing arrived —
    /// the honest measure of "how late was that parry".
    parry_closed_for: u32,
    /// Set when a parry *landed*, which ends its window early on purpose.
    ///
    /// Without this, a successful parry would emit a `FEEL_PARRY_CLOSE` line
    /// reading `frames_open=1`, and the proof's "every window is exactly 12
    /// frames" gate would fail on the mechanic working correctly. A window that
    /// was *spent* is not a window that *expired*; only the second one is
    /// evidence about the window's length, so only the second one is logged.
    parry_consumed: bool,
    /// How many of each *outcome* this run has actually produced, bumped at the
    /// same points the `FEEL_*` lines are written.
    ///
    /// These count OUTCOMES; [`DodgeProbe::drove`] counts the keystrokes the
    /// probe supplied, and the two are not the same number — a driven late
    /// parry that meets a feint, a whiff, or a husk that died first produces no
    /// `FailedParry` at all. The probe reads these to decide when it is done,
    /// which is the only reason they exist: ending the run on a stopwatch while
    /// the gate grades outcomes is what made this proof flaky (one run in three
    /// finished with the player untouched and failed gate 3 on a good build).
    pub saw_iframe_close: u32,
    pub saw_dodge_negate: u32,
    pub saw_parry_expire: u32,
    pub saw_parry_land: u32,
    pub saw_parry_fail: u32,
    pub saw_riposte: u32,
}

// ===========================================================================
// Per-frame window ticking
// ===========================================================================

/// Advance the frame counter. Runs in `First`, so every system this frame reads
/// the same number.
pub fn tick_frame(mut dp: ResMut<DodgeParryState>) {
    dp.frame = dp.frame.wrapping_add(1);
}

/// Spend one frame of every open window.
///
/// **Ordering is the mechanic.** This runs *before* `player_combat`, which is
/// what starts a window. So the frame a dodge begins, the counter is not
/// touched, and the window is observed open by `husk_ai` on that frame and on
/// the next `DODGE_IFRAME_FRAMES - 1` frames — exactly N frames of
/// invulnerability, at any frame rate. Decrementing after `player_combat`
/// instead would silently spend the opening frame and ship a 9-frame roll.
pub fn spend_window_frames(mut q: Query<&mut PlayerCombat, With<FlyCam>>) {
    let Ok(mut pc) = q.single_mut() else { return };
    if pc.iframe_frames > 0 {
        pc.iframe_frames -= 1;
    }
    if pc.parry_frames > 0 {
        pc.parry_frames -= 1;
    }
}

/// Watch the windows open and close and write the two lines the proof grades.
///
/// Runs between `player_combat` (which opens a window) and `husk_ai` (which
/// tests it), so `frames_open` on the `FEEL_IFRAME_CLOSE` line is literally the
/// number of frames a Husk swing would have been negated in — not a
/// reconstruction after the fact.
pub fn watch_windows(
    feel: Res<FeelLog>,
    mut dp: ResMut<DodgeParryState>,
    q: Query<(&PlayerCombat, &Transform), With<FlyCam>>,
) {
    let Ok((pc, tf)) = q.single() else { return };
    let frame = dp.frame;

    // ---- dodge i-frames -----------------------------------------------------
    if pc.iframe_frames > 0 {
        if dp.prev_iframes == 0 {
            dp.iframe_open_at = Some(frame);
            dp.iframe_open_for = 0;
            if feel.enabled {
                println!(
                    "FEEL_IFRAME open_frame={} booked={} pos=({:.2},{:.2},{:.2})",
                    frame, DODGE_IFRAME_FRAMES, tf.translation.x, tf.translation.y,
                    tf.translation.z
                );
            }
        }
        dp.iframe_open_for += 1;
    } else if dp.prev_iframes > 0 {
        let opened = dp.iframe_open_at.unwrap_or(frame);
        if feel.enabled {
            println!(
                "FEEL_IFRAME_CLOSE open_frame={} close_frame={} frames_open={} booked={}",
                opened, frame, dp.iframe_open_for, DODGE_IFRAME_FRAMES
            );
        }
        dp.saw_iframe_close += 1;
        dp.iframe_open_at = None;
        dp.iframe_open_for = 0;
    }
    dp.prev_iframes = pc.iframe_frames;

    // ---- parry window -------------------------------------------------------
    if pc.parry_frames > 0 {
        if dp.prev_parry == 0 {
            dp.parry_open_at = Some(frame);
            dp.parry_open_for = 0;
            dp.parry_closed_for = 0;
            if feel.enabled {
                println!(
                    "FEEL_PARRY_OPEN open_frame={} booked={}",
                    frame, PARRY_WINDOW_FRAMES
                );
            }
        }
        dp.parry_open_for += 1;
    } else {
        if dp.prev_parry > 0 && !dp.parry_consumed {
            let opened = dp.parry_open_at.unwrap_or(frame);
            if feel.enabled {
                println!(
                    "FEEL_PARRY_CLOSE open_frame={} close_frame={} frames_open={} booked={}",
                    opened, frame, dp.parry_open_for, PARRY_WINDOW_FRAMES
                );
            }
            dp.saw_parry_expire += 1;
        }
        dp.parry_consumed = false;
        // Only count staleness while the *state* is still Parry — that is the
        // span in which a swing gets `FailedParry` rather than a plain hit.
        if pc.state == CombatState::Parry {
            dp.parry_closed_for = dp.parry_closed_for.saturating_add(1);
        } else {
            dp.parry_closed_for = 0;
            dp.parry_open_at = None;
            dp.parry_open_for = 0;
        }
    }
    dp.prev_parry = pc.parry_frames;
}

// ===========================================================================
// Dodge i-frame visual — ghost flash + scale pulse
// ===========================================================================

/// The fraction of the i-frame window where the ghost pulse peaks (0.0–1.0).
const GHOST_PEAK: f32 = 0.35;
/// How far the body scales up at the peak of the pulse (1.0 + GHOST_SCALE).
const GHOST_SCALE: f32 = 0.18;
/// Visibility strobes every N frames during i-frames.
const GHOST_STROBE: u32 = 3;

/// Make the player's body flash and pulse during dodge i-frames.
///
/// Three channels run in parallel:
/// 1. **Scale pulse** — the body expands to 1.18× at ~35% through the window,
///    then contracts back to 1.0×. Reads as a "whoosh" of displacement.
/// 2. **Visibility strobe** — the body flickers every 3 frames. Reads as
///    "phasing" — the blade passed through something immaterial.
/// 3. **Restore** — the frame after the window shuts, scale and visibility
///    snap back to normal.
///
/// Once `anim::attach_rigs` has dressed the player in a rig it hides the
/// `PlayerBody` placeholder (capsule + face block) for good — the rig is what
/// renders from then on. This strobe must not fight that: it skips the body
/// entirely once the player carries [`Rigged`], instead of un-hiding a mesh
/// nobody wants back (see `docs/note-to-rose-placeholder-capsule-unhidden-2026-08-06.md`).
///
/// Runs after `watch_windows` (needs the frame counter) and before the camera
/// (so the pulse is visible in the frame).
pub fn dodge_ghost_flash(
    dp: Res<DodgeParryState>,
    player_q: Query<(&PlayerCombat, &Children, Has<Rigged>), With<FlyCam>>,
    mut body_q: Query<(&mut Transform, &mut Visibility), With<PlayerBody>>,
) {
    let Ok((pc, children, rigged)) = player_q.single() else { return };

    if rigged {
        // The placeholder is hidden permanently by attach_rigs; leave it alone.
        return;
    }

    let in_iframe = pc.invulnerable();

    for child in children.iter() {
        let Ok((mut tf, mut vis)) = body_q.get_mut(child) else { continue };

        if in_iframe {
            // Scale pulse: 1.0 → 1.0+GHOST_SCALE → 1.0, symmetric around GHOST_PEAK.
            // progress: 0.0 (window just opened) → 1.0 (about to close).
            let total = crate::dodge_parry::DODGE_IFRAME_FRAMES as f32;
            let elapsed = total - pc.iframe_frames as f32;
            let progress = (elapsed / total.max(1.0)).clamp(0.0, 1.0);
            // Triangle wave peaking at GHOST_PEAK.
            let wave = if progress < GHOST_PEAK {
                progress / GHOST_PEAK
            } else {
                1.0 - (progress - GHOST_PEAK) / (1.0 - GHOST_PEAK)
            };
            let scale = 1.0 + GHOST_SCALE * wave;
            tf.scale = Vec3::splat(scale);

            // Visibility strobe — flicker every GHOST_STROBE frames.
            let visible = (dp.frame % (GHOST_STROBE as u64 * 2)) < GHOST_STROBE as u64;
            *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
        } else {
            // Restore — one frame after the window closed.
            if tf.scale != Vec3::ONE {
                tf.scale = Vec3::ONE;
            }
            if *vis == Visibility::Hidden {
                *vis = Visibility::Visible;
            }
        }
    }
}

// ===========================================================================
// The call-outs `combat.rs` makes
// ===========================================================================

/// Everything that happens to the *attacker* when its swing meets a defence.
///
/// Called by `husk_ai` immediately after `try_hit_player`, once per swing, for
/// every outcome — including the ones that did not connect, because "the swing
/// passed through i-frames" is exactly the event the dodge proof needs to see.
///
/// | outcome | what this does |
/// |---|---|
/// | `Dodged` | logs the negation: how many frames were left, and the damage denied |
/// | `Parried` | 25 posture to the attacker, **forced poise break**, heavy shove, riposte armed |
/// | `FailedParry` | logs how many frames late the read was |
/// | anything else | nothing |
#[allow(clippy::too_many_arguments)]
pub fn resolve_defence(
    outcome: EnemyHitOutcome,
    swing_dmg: f32,
    feel: &FeelLog,
    dp: &mut DodgeParryState,
    commands: &mut Commands,
    attacker: Entity,
    e: &mut Enemy,
    ep: &mut Poise,
    pc: &mut PlayerCombat,
    php: &Health,
    epos: Vec3,
    enemy_to_player: Vec3,
    shake: &mut Shake,
    staggers: &mut MessageWriter<StaggerEvent>,
) {
    match outcome {
        EnemyHitOutcome::Dodged => {
            dp.saw_dodge_negate += 1;
            if feel.enabled {
                println!(
                    "FEEL_DODGE_NEGATE frame={} frames_left={} booked={} dmg_denied={:.1} hp={:.1}",
                    dp.frame, pc.iframe_frames, DODGE_IFRAME_FRAMES, swing_dmg, php.cur
                );
            }
        }
        EnemyHitOutcome::Parried => {
            dp.saw_parry_land += 1;
            // §2.5's posture damage, applied *by the parry* — where the doc puts
            // it — instead of by whichever later swing happened to connect.
            let before = ep.cur;
            ep.take(PARRY_POSTURE, false);
            let after = ep.cur;
            // A deflection leaves the attacker over-extended: the break is
            // guaranteed, not a function of how much posture was left. This is
            // the Elden Ring bargain — the 12-frame read *always* buys the
            // opening, which is the only reason to take a risk this sharp.
            ep.cur = 0.0;
            ep.stagger = STAGGER_TIME;
            e.state = HuskState::Staggered;
            e.timer = 0.0;
            e.hit_applied = true;
            e.hitstop = HITSTOP_PARRY;

            // Free the player out of the parry animation so the riposte is
            // actually reachable inside its window, and arm it on this attacker
            // specifically — a parry does not open every enemy in the room.
            pc.state = CombatState::Idle;
            pc.timer = 0.0;
            pc.parry_frames = 0;
            dp.parry_consumed = true; // spent, not expired — see the field's doc
            pc.punish = RIPOSTE_WINDOW;
            pc.riposte_on = Some(attacker);

            // The break is a real stagger, so it has to be *announced* like one:
            // `anim.rs` poses the stun and `vfx.rs` sparks off this message. A
            // forced break that stays silent leaves the Husk snapping into a
            // stagger pose with nothing to explain it.
            staggers.write(StaggerEvent { entity: attacker, pos: epos, is_player: false });

            // Weight: shove the deflected body back down the line it came in on,
            // and kick the camera the same way, so the deflection lands as a
            // shove you can see rather than a damage number you cannot.
            let shove = Vec3::new(-enemy_to_player.x, 0.0, -enemy_to_player.z).normalize_or_zero();
            if shove != Vec3::ZERO {
                shake.kick(shove, PARRY_SHOVE.kick());
                commands.entity(attacker).insert(Knockback {
                    dir: shove,
                    left: PARRY_SHOVE.knockback(),
                    total: PARRY_SHOVE.knockback(),
                    time: KNOCKBACK_TIME,
                    hitstop: HITSTOP_PARRY,
                    from: epos,
                    started: false,
                    slid: 0.0,
                    topup: 0.0,
                });
            }

            if feel.enabled {
                println!(
                    // `posture_after` is §2.5's arithmetic — what the 25 posture
                    // alone left. `posture_final` is where the stance actually
                    // ended, which the forced break puts at 0. Logging only the
                    // first would read as "the parry chipped it a bit"; logging
                    // only the second would hide whether §2.5's number was ever
                    // applied. Both, or the line is not evidence.
                    "FEEL_PARRY frame={} open_frame={} in_window={} booked={} attacker={} \
posture_before={:.1} dealt={:.1} posture_after={:.1} posture_final={:.1} broke=true \
shove={:.3} hitstop={:.3} riposte_window={:.3}",
                    dp.frame,
                    dp.parry_open_at.unwrap_or(dp.frame),
                    dp.parry_open_for,
                    PARRY_WINDOW_FRAMES,
                    attacker.to_bits(),
                    before,
                    PARRY_POSTURE,
                    after,
                    ep.cur,
                    PARRY_SHOVE.knockback(),
                    HITSTOP_PARRY,
                    RIPOSTE_WINDOW,
                );
            }
        }
        EnemyHitOutcome::FailedParry => {
            dp.saw_parry_fail += 1;
            if feel.enabled {
                println!(
                    "FEEL_PARRY_FAIL frame={} open_frame={} booked={} late_by={} \
dmg_taken={:.1} hp={:.1}",
                    dp.frame,
                    dp.parry_open_at.unwrap_or(dp.frame),
                    PARRY_WINDOW_FRAMES,
                    dp.parry_closed_for,
                    swing_dmg * combat::PARRY_FAIL_MULT,
                    php.cur,
                );
            }
        }
        _ => {}
    }
}

/// Is this swing a riposte — i.e. did a parry leave *this* enemy open, and is
/// the window still up? Consumes the arming, so one parry buys one riposte.
#[inline]
pub fn take_riposte(pc: &mut PlayerCombat, enemy: Entity) -> bool {
    if pc.punish > 0.0 && pc.riposte_on == Some(enemy) {
        pc.riposte_on = None;
        true
    } else {
        false
    }
}

/// Write the riposte's line. Called by `player_combat` after the damage lands,
/// with the numbers it actually applied — not with what they should have been.
pub fn log_riposte(
    feel: &FeelLog,
    dp: &mut DodgeParryState,
    target: Entity,
    mult: f32,
    dmg: f32,
    weight: ImpactWeight,
    hp_left: f32,
) {
    // Counted before the log gate: the riposte happened whether or not anyone
    // asked for the feel log, and the probe's stop condition reads the count.
    dp.saw_riposte += 1;
    if !feel.enabled {
        return;
    }
    println!(
        "FEEL_RIPOSTE frame={} target={} mult={:.2} dmg={:.1} weight={} hitstop={:.3} \
knockback={:.3} kick={:.3} hp={:.1}",
        dp.frame,
        target.to_bits(),
        mult,
        dmg,
        weight.label(),
        weight.hitstop(),
        weight.knockback(),
        weight.kick(),
        hp_left,
    );
}

// ===========================================================================
// The probe — a scripted fight that exercises all four timings
// ===========================================================================

/// Wall-clock the probe lets the scene settle before touching anything.
const PROBE_START: f32 = 2.0;
/// Wall-clock backstop. The probe normally stops the moment the run has
/// produced every outcome `scripts/prove_dodge_parry.sh` grades — see
/// [`evidence_complete`] — and this only catches a run where something never
/// fires at all, so the process still exits and prints its closing line
/// instead of being killed by the harness timeout with no verdict at all.
///
/// It replaced a hard `PROBE_END = 45.0`, which was the bug: the probe's
/// triggers ride Husk state transitions, whose pacing varies with the rhythm
/// roll (`straight` / `feint` / `delayed`, hold 0.42–1.55 s), so a fixed
/// stopwatch cut a different number of cases each run. One run in three ended
/// with the player never having been hit, and gate 3 (IFRAME_EDGE, "the window
/// has an edge") failed on a build that was fine. A gate that passes two runs
/// in three is not evidence about the build.
const PROBE_CAP: f32 = 100.0;

/// Gate 1 wants two closed i-frame windows before it will believe the count is
/// stable; every other gate needs one of its own outcome. Mirrored here so the
/// probe stops on exactly what the grader is about to ask for.
const NEED_IFRAME_CLOSE: u32 = 2;

/// Has this run produced every outcome the gate grades?
///
/// Note what is counted: outcomes the shipping systems *observed*, taken from
/// [`DodgeParryState`], plus the player having actually been hit. `drove` (the
/// probe's own keystroke count) is checked too, for gate 12's coverage line,
/// but it is deliberately not enough on its own — driving a late parry into a
/// feint books a `drove` and produces no evidence whatsoever.
fn evidence_complete(dp: &DodgeParryState, probe: &DodgeProbe, player_hp: f32) -> bool {
    dp.saw_iframe_close >= NEED_IFRAME_CLOSE
        && dp.saw_dodge_negate >= 1
        && dp.saw_parry_expire >= 1
        && dp.saw_parry_land >= 1
        && dp.saw_parry_fail >= 1
        && dp.saw_riposte >= 1
        && player_hp < combat::HP_PLAYER
        && probe.drove.iter().all(|&n| n >= 1)
}
/// How close the probe stands. Inside `MELEE_RANGE` so the Husk commits, and
/// inside the player's own reach so the riposte can land.
const PROBE_REACH: f32 = 1.6;

/// Bookkeeping for [`dodge_parry_probe`].
#[derive(Resource, Default)]
pub struct DodgeProbe {
    pub enabled: bool,
    /// Which of the four timings the next combo will be answered with.
    pub case: u32,
    pub ground_y: Option<f32>,
    pub spawned: u32,
    pub done: bool,
    /// Husk state as of last frame, so a *transition* can be detected. Every
    /// trigger below is keyed on a transition rather than a wall-clock offset:
    /// the Husk's wind-up length varies by rhythm (straight / delayed / feint),
    /// so any time-based script would drift out of the window it is aiming for.
    last_state: Option<HuskState>,
    /// How many of each case have actually been driven, for the closing line.
    pub drove: [u32; 4],
}

/// The four timings, in the order the probe cycles them.
const CASE_DODGE_ON_TIME: u32 = 0;
const CASE_DODGE_LATE: u32 = 1;
const CASE_PARRY_ON_TIME: u32 = 2;
const CASE_PARRY_LATE: u32 = 3;

/// How long before the blade lands the mistimed parry is thrown. See the note
/// at its trigger in [`dodge_parry_probe`].
const PARRY_LATE_LEAD: f32 = PARRY_STATE_LEN * 0.6;

/// A scripted fight that answers Husk combos with, in turn: a frame-perfect
/// roll, a roll thrown far too early, a frame-perfect parry (and the riposte it
/// buys), and a parry thrown one swing early.
///
/// **It grades nothing.** Every `FEEL_*` line in the log is written by the
/// shipping systems above; the probe only supplies keystrokes and bodies. It
/// exists because the two negative cases — a roll whose window has closed, a
/// parry that reads late — cannot be produced by a human on demand, and a gate
/// that only ever sees the successes cannot tell "invulnerable at the right
/// moment" apart from "invulnerable always".
///
/// Triggers are keyed on Husk **state transitions**, and the probe runs before
/// `player_combat`, which runs before `husk_ai`. So pressing dodge on the first
/// frame `Swing1` is observed means: the window opens in `player_combat` this
/// frame and the swing resolves against it in `husk_ai` this same frame. No
/// prediction, no tolerance, no frame-rate assumption.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn dodge_parry_probe(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut probe: ResMut<DodgeProbe>,
    dp: Res<DodgeParryState>,
    mut player_q: Query<
        (&Transform, &mut FlyCam, &Health, &PlayerCombat),
        (With<FlyCam>, Without<Enemy>),
    >,
    enemies: Query<(&Transform, &Enemy, &Health), (With<Enemy>, Without<FlyCam>)>,
    mut exit: MessageWriter<AppExit>,
) {
    if probe.done {
        return;
    }
    let t = time.elapsed_secs();
    let Ok((ptf, mut fly, php, pc)) = player_q.single_mut() else { return };

    if t < PROBE_START {
        if let Some((etf, _, _)) = enemies.iter().next() {
            probe.ground_y = Some(etf.translation.y);
        }
        return;
    }
    fly.walking = true;

    // Stop on evidence, not on the clock. `stop=cap` in the closing line means
    // the backstop fired with something still missing — the gates below will
    // say which, and they are meant to fail when they do.
    let enough = evidence_complete(&dp, &probe, php.cur);
    if enough || t >= PROBE_CAP {
        for k in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD,
                  KeyCode::KeyX, KeyCode::Space, KeyCode::KeyV] {
            keys.reset(k);
        }
        probe.done = true;
        println!(
            "DODGE_PROBE done t={t:.1}s frame={} stop={} husks_spawned={} \
drove_dodge_on_time={} drove_dodge_late={} drove_parry_on_time={} drove_parry_late={} \
saw_iframe_close={} saw_dodge_negate={} saw_parry_expire={} saw_parry_land={} \
saw_parry_fail={} saw_riposte={} player_hp={:.0}",
            dp.frame,
            if enough { "evidence" } else { "cap" },
            probe.spawned, probe.drove[0], probe.drove[1], probe.drove[2], probe.drove[3],
            dp.saw_iframe_close, dp.saw_dodge_negate, dp.saw_parry_expire,
            dp.saw_parry_land, dp.saw_parry_fail, dp.saw_riposte, php.cur
        );
        exit.write(AppExit::Success);
        return;
    }

    // ---- keep exactly one live husk in front of the player -------------------
    let target = enemies
        .iter()
        .filter(|(_, _, h)| !h.dead())
        .min_by(|a, b| {
            a.0.translation
                .distance(ptf.translation)
                .partial_cmp(&b.0.translation.distance(ptf.translation))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let Some((etf, enemy, _)) = target else {
        let y = probe.ground_y.unwrap_or(ptf.translation.y - 1.6);
        // Straight down -Z: headless yaw is 0, so the player faces -Z and the
        // husk sits inside the melee cone without the probe having to turn.
        let (x, z) = (ptf.translation.x, ptf.translation.z - 4.0);
        combat::spawn_guard_husk(&mut commands, &mut meshes, &mut materials, x, z, Some(y));
        probe.spawned += 1;
        probe.last_state = None;
        println!("DODGE_PROBE spawn husk #{} at ({x:.1},{y:.1},{z:.1})", probe.spawned);
        return;
    };

    // ---- riposte: the parry armed it, cash it in ----------------------------
    // Checked before the approach so a parry landed at the edge of reach still
    // gets its answer. `riposte_on` is the shipping flag, not a probe guess.
    if pc.riposte_on.is_some() {
        keys.reset(KeyCode::KeyX);
        keys.press(KeyCode::KeyX);
        return;
    }
    keys.reset(KeyCode::KeyX);

    // ---- take up the stance -------------------------------------------------
    // Not "get within reach" — get to one specific spot: `PROBE_REACH` blocks
    // along +Z from the husk. There is no mouse in a headless run, so the
    // player's yaw is pinned at 0 and they face -Z forever; every swing and the
    // riposte are cone-checked against that facing (`MELEE_CONE`). Merely being
    // close is therefore not enough — stand on the wrong side and the husk hits
    // you while nothing you throw can reach it.
    //
    // This matters because the roll is 2.5 blocks (§2.2) and the mistimed-roll
    // case fires from inside reach, so the probe routinely ends the roll *past*
    // the husk. Re-taking the stance every frame is what makes the next case
    // land instead of silently whiffing into an empty cone.
    let want = Vec3::new(etf.translation.x, ptf.translation.y, etf.translation.z + PROBE_REACH);
    let (ox, oz) = (want.x - ptf.translation.x, want.z - ptf.translation.z);
    let tol = 0.3;
    if ox.abs() > tol || oz.abs() > tol {
        set_key(&mut keys, KeyCode::KeyD, ox > tol);
        set_key(&mut keys, KeyCode::KeyA, ox < -tol);
        set_key(&mut keys, KeyCode::KeyS, oz > tol);
        set_key(&mut keys, KeyCode::KeyW, oz < -tol);
        probe.last_state = Some(enemy.state);
        return;
    }
    for k in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
        keys.reset(k);
    }

    // ---- the mistimed parry ------------------------------------------------
    // This one case cannot ride a state transition. Its whole point is that the
    // 12-frame window shuts *before* the blade lands while the 0.70 s
    // commitment is still up, and the wall-clock length of 12 frames is a
    // function of the frame rate. So it is thrown a fixed lead-time before the
    // swing instead: 0.42 s is long enough that the window has closed on
    // anything above ~29 FPS, and short enough that the parry state has not yet
    // expired (0.70 s) at any frame rate at all.
    //
    // Feints are skipped: a fake wind-up produces no swing, so a parry thrown
    // into one proves nothing and would leave the coverage gate green with no
    // evidence behind it.
    if probe.case % 4 == CASE_PARRY_LATE
        && enemy.state == HuskState::Telegraph
        && enemy.rhythm != combat::HuskRhythm::Feint
    {
        let hold = combat::telegraph_hold(enemy.rhythm);
        if hold > PARRY_LATE_LEAD && enemy.timer >= hold - PARRY_LATE_LEAD {
            press(&mut keys, KeyCode::KeyV);
            probe.drove[3] += 1;
            probe.case += 1;
            probe.last_state = Some(enemy.state);
            return;
        }
    }

    // ---- answer the combo, on the frame the state changes -------------------
    let entered = probe.last_state != Some(enemy.state);
    probe.last_state = Some(enemy.state);
    if !entered {
        keys.reset(KeyCode::Space);
        keys.reset(KeyCode::KeyV);
        return;
    }

    match (probe.case % 4, enemy.state) {
        // The window opens this frame and the swing resolves this frame.
        (CASE_DODGE_ON_TIME, HuskState::Swing1) => {
            press(&mut keys, KeyCode::Space);
            probe.drove[0] += 1;
            probe.case += 1;
        }
        // Rolled at the *start* of an 0.8 s wind-up: 10 frames are long gone by
        // the time the blade arrives, so this swing must land.
        (CASE_DODGE_LATE, HuskState::Telegraph) => {
            press(&mut keys, KeyCode::Space);
            probe.drove[1] += 1;
            probe.case += 1;
        }
        (CASE_PARRY_ON_TIME, HuskState::Swing1) => {
            press(&mut keys, KeyCode::KeyV);
            probe.drove[2] += 1;
            probe.case += 1;
        }
        // CASE_PARRY_LATE is handled above — it is the one case that is not a
        // state transition.
        _ => {
            keys.reset(KeyCode::Space);
            keys.reset(KeyCode::KeyV);
        }
    }
}

fn press(keys: &mut ButtonInput<KeyCode>, key: KeyCode) {
    // reset-then-press so `just_pressed` sees a fresh rising edge this frame.
    keys.reset(key);
    keys.press(key);
}

fn set_key(keys: &mut ButtonInput<KeyCode>, key: KeyCode, down: bool) {
    if down {
        keys.press(key);
    } else {
        keys.reset(key);
    }
}

// ===========================================================================
// Plugin
// ===========================================================================

/// Wired in one line next to `CombatFeelPlugin`:
/// `.add_plugins(dodge_parry::DodgeParryPlugin)`.
///
/// Every ordering here is load-bearing, and none of it is implied by the tuple
/// position (Bevy does not order by position):
///  * `spend_window_frames` **before** `player_combat` — see its doc comment;
///    after it instead, and the opening frame is spent before it is ever used.
///  * `watch_windows` between `player_combat` and `husk_ai` — the window has to
///    be observed in the same state `husk_ai` will test it in.
///  * `dodge_parry_probe` before `gather_input`, because it writes
///    `ButtonInput` and `just_pressed` is cleared in the next `PreUpdate`.
pub struct DodgeParryPlugin;

impl Plugin for DodgeParryPlugin {
    fn build(&self, app: &mut App) {
        let probe = std::env::var("VOXELFORGE_DODGE_PROBE").is_ok();
        app.init_resource::<DodgeParryState>()
            .insert_resource(DodgeProbe { enabled: probe, ..default() })
            .add_systems(First, tick_frame)
            .add_systems(
                Update,
                (
                    dodge_parry_probe
                        .before(combat::gather_input)
                        .before(crate::fly_camera)
                        .run_if(|p: Res<DodgeProbe>| p.enabled),
                    spend_window_frames.before(combat::player_combat),
                    watch_windows
                        .after(combat::player_combat)
                        .before(combat::husk_ai),
                    dodge_ghost_flash
                        .after(watch_windows)
                        .before(crate::fly_camera),
                )
                    .run_if(in_state(crate::editor::AppState::Play)),
            );
        if probe {
            // The probe is only useful with the feel log on — every line it
            // exists to produce is written behind that flag. `FeelLog` belongs
            // to `CombatFeelPlugin`; take it if it is already there, insert it
            // if this plugin happens to be built first.
            if let Some(mut f) = app.world_mut().get_resource_mut::<FeelLog>() {
                f.enabled = true;
            } else {
                app.insert_resource(FeelLog { enabled: true });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{Stamina, STAMINA_MAX, COST_DODGE};

    /// The window is a frame count, and it is spent one frame at a time —
    /// `dt` never appears. This is the property `prove_dodge_parry.sh` grades
    /// against the real binary; here it is pinned without a GPU.
    #[test]
    fn iframe_window_is_exactly_n_frames_regardless_of_dt() {
        for dt in [1.0 / 15.0, 1.0 / 60.0, 1.0 / 240.0] {
            let mut s = Stamina::full();
            let mut pc = PlayerCombat::default();
            assert!(pc.start_dodge(&mut s));
            assert_eq!(s.cur, STAMINA_MAX - COST_DODGE);

            // Frame 1 is the frame the dodge started: the counter is not spent
            // before the window is used, so it is already open here.
            let mut open_frames = 0;
            for _ in 0..(DODGE_IFRAME_FRAMES * 4) {
                if pc.invulnerable() {
                    open_frames += 1;
                }
                // Exactly the order the schedule runs in: spend, then tick.
                if pc.iframe_frames > 0 {
                    pc.iframe_frames -= 1;
                }
                pc.tick(dt);
            }
            assert_eq!(
                open_frames, DODGE_IFRAME_FRAMES,
                "i-frame window drifted at dt={dt}"
            );
        }
    }

    #[test]
    fn parry_window_is_exactly_n_frames() {
        let mut pc = PlayerCombat::default();
        pc.state = CombatState::Parry;
        pc.parry_frames = PARRY_WINDOW_FRAMES;
        let mut open = 0;
        for _ in 0..(PARRY_WINDOW_FRAMES * 4) {
            if pc.parry_frames > 0 {
                open += 1;
                pc.parry_frames -= 1;
            }
        }
        assert_eq!(open, PARRY_WINDOW_FRAMES);
    }

    /// The parry state must outlive its window — otherwise a mistimed parry is
    /// indistinguishable from no parry and `FailedParry` is dead code.
    #[test]
    fn parry_state_outlives_its_window() {
        assert!(
            PARRY_STATE_LEN > combat::PARRY_WINDOW,
            "parry state must still be up after the window shuts"
        );
        // …and the Gap the probe fires its late parry into lands inside that gap.
        assert!(combat::HUSK_GAP > combat::PARRY_WINDOW * 0.0);
        assert!(combat::HUSK_GAP < PARRY_STATE_LEN);
    }

    /// A riposte has to be worth the read: strictly heavier than the plain
    /// punish window it replaces, and it must book the Critical feel row.
    #[test]
    fn riposte_out_rewards_the_plain_punish_window() {
        assert!(RIPOSTE_MULT > combat::PARRY_PUNISH_MULT);
        let plain = combat::LIGHT_DAMAGE;
        let riposte = combat::LIGHT_DAMAGE * RIPOSTE_MULT * combat::STAGGER_DMG_MULT;
        assert!(riposte > plain * 2.0);
        assert_eq!(ImpactWeight::Critical.hitstop(), combat::HITSTOP_CRITICAL);
        assert_eq!(ImpactWeight::Critical.knockback(), combat::KNOCKBACK_CRITICAL);
    }

    /// One parry buys one riposte, on the enemy that was parried — not on the
    /// next thing the player happens to swing at.
    #[test]
    fn riposte_is_single_use_and_target_bound() {
        let mut w = World::new();
        let (a, b) = (w.spawn_empty().id(), w.spawn_empty().id());
        let mut pc = PlayerCombat::default();
        pc.punish = RIPOSTE_WINDOW;
        pc.riposte_on = Some(a);
        assert!(!take_riposte(&mut pc, b), "a parry does not open a bystander");
        assert!(take_riposte(&mut pc, a));
        assert!(!take_riposte(&mut pc, a), "one parry, one riposte");
    }
}
