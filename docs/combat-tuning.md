# Voxelforge Combat Tuning — Config Reference

> **Engineer-facing.** Every numeric knob the soulslike combat loop reads, where it
> lives in code, and what design section it answers to.
> **Owner:** Yamamoto (Engine) — `client/src/combat.rs`, `client/src/dodge_parry.rs`.
> **Design source of truth:** `docs/combat-design.md`. This file is the *code* mirror —
> if a number here disagrees with combat-design.md, the design doc's §-comment trail
> on each constant tells you which one is stale.

All values below are `pub const` — no magic numbers live inline in the systems.
Change a value, `cargo build`, done; nothing needs a recompile of a data file or a
runtime config loader (v1 doesn't need one — see "Why plain consts" at the bottom).

---

## 1. Stamina (`combat.rs`) — §2.1

| Constant | Value | Meaning |
|---|---|---|
| `STAMINA_MAX` | 100.0 | Max pool, flat for v1 (no Endurance stat) |
| `STAMINA_REGEN` | 40.0 /s | Regen rate once out of an action-state |
| `COST_LIGHT` | 15.0 | Stamina per light attack |
| `COST_HEAVY` | 35.0 | Stamina per heavy attack |
| `COST_CHARGED` | 50.0 | Stamina per charged attack |
| `COST_DODGE` | 20.0 | Stamina per dodge roll |
| `COST_BLOCK` | 15.0 | Stamina per hit blocked (holding is free) |
| `COST_PARRY` | 10.0 | Stamina per parry attempt |
| `DELAY_LIGHT` / `_HEAVY` / `_CHARGED` / `_DODGE` / `_BLOCK` / `_PARRY` | 0.25 / 0.40 / 0.50 / 0.30 / 0.20 / 0.35 s | Regen-delay after each action — this is what makes spamming a losing move |
| `EXHAUST_LOCK` | 0.8 s | No attack/dodge/block while stamina is at 0 |
| `EXHAUST_CLEAR` | 20.0 | Stamina needed to clear the exhaustion penalty |

## 2. Dodge / i-frames (`dodge_parry.rs`) — §2.2

| Constant | Value | Meaning |
|---|---|---|
| `DODGE_IFRAME_FRAMES` | **10 frames** | Invulnerability window, counted per-frame (not `dt`-scaled — see file header for why a duration was the bug) |
| `combat::DODGE_IFRAMES` | 10/60 s | Same window, kept in seconds only to drive the roll *glide* animation timing |
| `combat::DODGE_RECOVERY` | 12/60 s (~200 ms) | Recovery frames before the next action — the anti-spam punish |
| `combat::DODGE_DISTANCE` | 2.5 blocks | Roll travel distance |

## 3. Parry / riposte (`dodge_parry.rs`) — §2.5

| Constant | Value | Meaning |
|---|---|---|
| `PARRY_WINDOW_FRAMES` | **12 frames** | Parry receive window, counted per-frame |
| `PARRY_STATE_LEN` | `PARRY_WINDOW + PARRY_FAIL_RECOVER` (0.70 s) | How long `CombatState::Parry` stays up — deliberately longer than the window itself so a miss is reachable |
| `RIPOSTE_WINDOW` | = `combat::PARRY_PUNISH` (1.20 s) | Punish window after a landed parry |
| `RIPOSTE_MULT` | **2.5×** | Riposte damage multiplier (diverges from combat-design.md's 1.25× punish window on purpose — see file header: a 12-frame read needs a bigger bargain than a lucky trade) |
| `PARRY_SHOVE` | `ImpactWeight::Heavy` | Knockback row applied to the parried attacker |
| `combat::PARRY_POSTURE` | 25.0 | Posture damage dealt on a successful parry — always a guaranteed poise break in the shipped build |
| `combat::PARRY_FAIL_MULT` | 1.25× | Extra damage taken on a whiffed parry |
| `combat::PARRY_FAIL_RECOVER` | 0.50 s | Recovery after a failed parry |

## 4. Block (`combat.rs`) — §2.5

| Constant | Value | Meaning |
|---|---|---|
| `BLOCK_REDUCTION` | 0.50 | Damage taken while blocking (50% reduction) |
| `GUARD_BREAK` | 0.60 s | Stagger duration when blocking without enough stamina |

## 5. Lock-on (`combat.rs`) — §2.3

| Constant | Value | Meaning |
|---|---|---|
| `LOCK_RANGE` | 16.0 blocks | Max acquisition + switch range |
| `LOCK_CONE_H` | 45° | Horizontal acquisition cone (toggle-on only, see below) |
| `LOCK_SNAP` | 8.0 rad/s | Camera snap-to-target speed |
| `LOCK_DROP_RANGE` | 20.0 blocks | Auto-unlock distance |

**Inputs** (`gather_input`, raw `KeyCode` — no `input_map.rs` indirection in combat):

| Action | Key |
|---|---|
| Toggle lock on/off | `R` |
| **Switch target** | `Q` |

Switching (`cycle_target`) picks the nearest *other* live enemy within `LOCK_RANGE`
of the player. It intentionally does **not** re-apply `LOCK_CONE_H` — once locked,
the player is already oriented at the fight, and gating a switch by facing would
make the enemy you just turned away from unreachable. This is the gap the CEO's
brief called out explicitly (§2.3 "lock-on พร้อมสลับเป้า"); the Guard Husk encounter
is single-target so it isn't exercisable yet, but Ash Hound (§4.2, "designed to
teach lock-on switching") needs it day one. Covered by
`cycle_target_switches_to_the_next_nearest_live_enemy`,
`cycle_target_skips_dead_and_out_of_range_candidates`,
`cycle_target_is_a_noop_when_no_other_enemy_exists` in `combat.rs`'s test module.

## 6. Attacks (`combat.rs`) — §2.4

| Constant | Value | Meaning |
|---|---|---|
| `LIGHT_DAMAGE` / `LIGHT_POISE` / `LIGHT_TIME` | 20.0 / 15.0 / 0.35 s | Light attack |
| `HEAVY_DAMAGE` / `HEAVY_POISE` / `HEAVY_TIME` | 45.0 / 40.0 / 0.85 s | Heavy attack |
| `HEAVY_HYPER` | 0.40 s | Hyper-armor tail on the heavy wind-up |
| `CHARGED_DAMAGE` / `CHARGED_POISE` / `CHARGED_TIME` | 70.0 / 60.0 / 1.10 s | Charged attack |
| `CHARGE_HOLD` | 0.60 s | Hold time before a heavy becomes charged |
| `COMBO_RESET` | 0.60 s | Window to continue a light combo |
| `COMBO_STEP` | +10% | Damage ramp per chained hit |
| `LIGHT_ACTIVE` / `HEAVY_ACTIVE` / `CHARGED_ACTIVE` | (0.12,0.22) / (0.55,0.72) / (0.75,0.95) s | Active-hitbox windows inside each swing's total time |
| `MELEE_RANGE` | 2.0 blocks | Player reach + Husk melee range |
| `MELEE_CONE` | 60° | Half-cone for whether a swing connects |

## 7. Poise / Health (`combat.rs`) — §3

| Constant | Value | Meaning |
|---|---|---|
| `POISE_PLAYER` | 40.0 | Player max poise |
| `POISE_HUSK` | 30.0 | Guard Husk max poise |
| `POISE_REGEN` | 10.0 /s | Poise regen rate |
| `POISE_REGEN_DELAY` | 2.0 s | No-hit delay before poise starts regenerating |
| `STAGGER_TIME` | 1.5 s | Stagger duration on a poise break |
| `STAGGER_DMG_MULT` | 1.30× | Extra damage taken while staggered |
| `HYPER_ARMOR_REDUCE` | 0.75 (−75%) | Poise-damage reduction during hyper armor |
| `HP_PLAYER` | 100.0 | Player max HP |
| `HP_HUSK` | 80.0 | Guard Husk HP |

## 8. Guard Husk AI + telegraph (`combat.rs`) — §4.1

| Constant | Value | Meaning |
|---|---|---|
| `HUSK_WALK` / `HUSK_TURN` | 1.5 blocks/s / 2.0 rad/s | Patrol/chase movement |
| `HUSK_TELEGRAPH` | **0.8 s** | Baseline wind-up before the first swing — the honest read window |
| `HUSK_TELEGRAPH_DELAYED` | 1.55 s | Delayed-rhythm variant — holds the pose, then swings late, punishing a dodge rolled on the straight tempo |
| `HUSK_FEINT_HOLD` | 0.42 s | Feint pulls back before any real swing exists |
| `HUSK_FEINT_RECOVER` | 0.55 s | Beat of nothing after a feint — must outlast `DODGE_IFRAMES + DODGE_RECOVERY` or the feint is free |
| `HUSK_SWING1_DMG` / `_POISE` | 15.0 / 15.0 | First swing of the 2-hit combo |
| `HUSK_SWING2_DMG` / `_POISE` | 20.0 / 20.0 | Second swing |
| `HUSK_ACTIVE` | 0.2 s | Active-hitbox duration per swing |
| `HUSK_GAP` | 0.25 s | Gap between the two swings |
| `HUSK_COMBO_PAUSE` | 1.5 s | Pause between combos |
| `HUSK_AGGRO_RANGE` | 12.0 blocks | Aggro sight range |
| `HUSK_LEASH` | 6.0 blocks | Retreat distance that returns the Husk to patrol |
| `HUSK_STEP_IN` | 0.5× `HUSK_WALK` | Closing speed during wind-up — always slower than a chase, so the telegraph is never a free hit |

Telegraph rhythm is picked per-encounter by `pick_rhythm(seed)` (deterministic —
`husk_rhythm_is_deterministic` test) across `HuskRhythm::{Straight, Delayed, Feint}`
so the same read doesn't work twice in a row without the player adapting.

## 9. Feedback — hit-stop, screen-shake, knockback (`combat.rs`) — §5

> Canonical: `31f1d71` — the values in this section follow that commit (combat.rs is the source of truth).

| Constant | Value | Meaning |
|---|---|---|
| `HITSTOP_LIGHT` | 0.080 s | Player light lands (both sides pause) |
| `HITSTOP_HEAVY` | 0.140 s | Player heavy/charged lands |
| `HITSTOP_PARRY` | 0.120 s | Parry succeeds |
| `HITSTOP_ENEMY` | 0.100 s | Enemy hits player |
| `HITSTOP_STAGGER` | 0.150 s | Stagger / critical punish |
| `HITSTOP_CRITICAL` | 0.200 s | Charged hit or a poise break (riposte rides this row) |
| `SHAKE_LIGHT` | amp 0.04 / 0.10 s | Screen-shake, light hit |
| `SHAKE_HEAVY` | amp 0.12 / 0.22 s | Screen-shake, heavy/charged hit |
| `SHAKE_ENEMY_HIT` | amp 0.16 / 0.28 s | Screen-shake, enemy hits player |
| `SHAKE_PARRY` | amp 0.12 / 0.18 s | Screen-shake, parry lands |
| `SHAKE_STAGGER` | amp 0.20 / 0.30 s | Screen-shake, stagger / poise break |
| `KNOCKBACK_LIGHT` / `_HEAVY` / `_CRITICAL` | 0.18 / 0.32 / 0.55 blocks | Push-back distance by impact weight |
| `KNOCKBACK_TIME` | 0.15 s | Time the knockback plays over |
| `KICK_LIGHT` / `_HEAVY` / `_CRITICAL` / `_TAKEN` | 0.045 / 0.120 / 0.155 / 0.130 | Directional camera-kick magnitude by weight (`_TAKEN` = player eating a hit) |
| `KICK_TIME` | 0.16 s | Kick snap-out/ease-back time |

`ImpactWeight::{Light, Heavy, Critical}` bundles a hit-stop/knockback/kick row so
every impact — player attack, parry, riposte, enemy hit — books one of exactly
three weight classes instead of inventing bespoke numbers per call site
(`prove_knockback.sh` grades this).

## 10. Events other lanes hook (`combat.rs` / `dodge_parry.rs`)

Combat never touches anim/vfx state directly — it emits Bevy `Message`s and the
owning lanes subscribe:

| Message | Fired when | Consumer |
|---|---|---|
| `ImpactEvent` | Any hit lands (player or enemy), carries `ImpactWeight` | `anim.rs` (hit-reaction pose), `vfx.rs` (sparks/blood/flash) |
| `StaggerEvent` | Poise breaks (player or enemy) | `anim.rs` (stun pose), `vfx.rs` (stagger sparks) |
| `DodgeEvent` | Dodge roll starts | `anim.rs` (roll animation) |
| `PlayerDied` | Player HP hits 0 | game-over flow |

No new hook is being requested from Rose (`anim.rs`) this round — the three
gameplay-facing messages above already carry everything a reaction animation
needs (weight class, entity, whether it broke poise); they were there before this
pass and are unchanged.

## 11. Why plain `pub const`, not a data file

v1 has one player build, one enemy archetype live (Guard Husk), and no live-ops
need to retune without a rebuild. A `const` is the cheapest thing that is still
"config-driven": every number lives in exactly one named place, is greppable,
diffable against `combat-design.md`'s §-tags, and is exercised by the unit tests
below instead of a runtime loader that could silently fail to parse. If a second
enemy archetype or a difficulty-select ships, the natural next step is a
`RonAsset`/`toml` table keyed by archetype — deliberately **out of scope** for the
first playable per `combat-design.md` §3.3/§6's own scope boundary.

## 12. Test coverage map

| Behaviour | Test(s) | File |
|---|---|---|
| i-frame window is exactly N frames, not `dt`-scaled | `iframe_window_is_exactly_n_frames_regardless_of_dt` | `dodge_parry.rs` |
| Dodge negates damage while i-frames are open | `dodge_grants_iframes_that_negate_damage` | `combat.rs` |
| Parry window is exactly N frames | `parry_window_is_exactly_n_frames` | `dodge_parry.rs` |
| Parry state outlives its window (miss is reachable) | `parry_state_outlives_its_window` | `dodge_parry.rs` |
| Riposte rewards more than the plain punish window | `riposte_out_rewards_the_plain_punish_window` | `dodge_parry.rs` |
| Riposte is single-use and target-bound | `riposte_is_single_use_and_target_bound` | `dodge_parry.rs` |
| Stamina drains/regens/exhausts by spec | `light_attack_drains_stamina_by_spec`, `stamina_regens_after_delay`, `exhaustion_locks_actions` | `combat.rs` |
| Poise break → stagger → reset; hyper armor suppresses it | `poise_break_triggers_stagger_and_resets`, `hyper_armor_reduces_poise_and_suppresses_stagger` | `combat.rs` |
| Block/guard-break | `block_halves_damage`, `block_with_stamina_halves_damage`, `guard_break_when_blocking_without_stamina` | `combat.rs` |
| Hit-stop / shake / knockback scale with weight | `hitstop_is_graded_by_weight`, `knockback_and_kick_scale_with_the_same_weight`, `shake_kick_takes_the_strongest_and_normalises` | `combat.rs` |
| Husk telegraph rhythms are honest and deterministic | `husk_rhythm_covers_all_three_patterns`, `husk_rhythm_is_deterministic`, `delayed_wind_up_outlasts_a_dodge_rolled_on_the_straight_tempo`, `feint_pulls_back_earlier_than_any_real_swing` | `combat.rs` |
| Lock-on target switching | `cycle_target_switches_to_the_next_nearest_live_enemy`, `cycle_target_skips_dead_and_out_of_range_candidates`, `cycle_target_is_a_noop_when_no_other_enemy_exists` | `combat.rs` |

Integration-level proof (real binary, not test-driver arithmetic):
`scripts/prove_dodge_parry.sh` (i-frame/parry/riposte, graded against the shipping
build's own log lines), `scripts/prove_combat.sh`, `scripts/prove_knockback.sh`.

---

*Engineering: Yamamoto. Design source: Sahara (`docs/combat-design.md`).*
