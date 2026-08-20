# VERDICT — Sun combo system (input buffer + cancel window + finisher combo-4 + cooldown)

- Date: 2026-08-20
- Lane: combat
- Branch: `sun/combo-system`
- Commit: `1a58302e3eabf190b7f2fb49ae09401c1e9628d6`
- Files: `client/src/combat.rs` (+333/−31), `_sun_combo.progress`

## What the new combo does that the old one could not

### 1. Input buffer (`combo_input_buffer = 0.15 s`)
A press that lands while the player cannot act yet is no longer dropped. It is stored
as a `BufferedAttack` (None / Light / Heavy) and fired the frame the cancel window
opens, or the moment the player is idle again. Validity is owned by `buffer_time`;
when it hits 0 the buffered press expires and is dropped.

- Old behaviour: pressing even one frame early mid-chain ate the input.

### 2. Cancel window (`combo_cancel_from = 0.5`)
A light can be cancelled into the next chain step once 50% of the way through,
instead of being locked to the full action length. `next_light` advances and wraps
the L1→L2→L3→finisher chain at that point.

### 3. Finisher combo-4 (`CombatState::Finisher`)
A heavy finisher reachable only while a light chain is live (`combo != 0`).
Own tuning: `finisher_damage 60.0`, `finisher_poise 55.0`, `finisher_time 0.95`,
`finisher_hyper 0.40`, `finisher_active (0.55, 0.75)`. Heavier than a standalone
heavy, and it *ends* the chain. `anim.rs` gained the `combo = 4` Finisher arm plus
the `FINISHER_COCK` / `FINISHER_FOLLOW` pose keys.

### 4. Cooldown (`combo_cd = 0.35 s`)
A combo now has a readable end instead of turning into infinite mash:

- a dropped chain (cancel window missed) opens `combo_cd`,
- a finisher opens `combo_cd` the moment it completes,
- while `combo_cd > 0`, fresh light/heavy presses from Idle are refused.

## Unit tests (`client/src/combat.rs`, `mod tests`)

Fixed:
- `combo_ramps_damage_ten_percent_each` — now drives the chain via `start_next_light`

Added:
- `next_light_advances_and_wraps_the_chain`
- `cancel_window_opens_halfway_through_a_light`
- `finisher_requires_a_live_chain`
- `finisher_is_heavier_than_a_standalone_heavy`
- `chain_resets_and_opens_cooldown_when_window_missed`
- `finisher_opens_cooldown_on_completion`
- `fresh_light_refused_during_combo_cooldown`
- `buffered_attack_expires_after_input_buffer`

## Build / test status

- Sentinel (check 1, before writing this doc): `running` — Yamamoto's anim-before build
  (`cargo build --release -p voxelforge --bin voxelforge -j 2 --target-dir target-yamamoto-before`).
  Did not wait; did not fire a second build.
- Sentinel (check 2, after writing this doc): still `running` (same run id, ~6.7 min).
  Queue still busy → per lane protocol, did not loop-wait; report and hand off.
- `cargo build --release -p voxelforge --target-dir target-sun`: NOT RUN (queue busy both checks)
- `cargo test -p voxelforge --target-dir target-sun`: NOT RUN (queue busy both checks)

The `_sun_build2.status` BUILD_EXIT=0 (stamp 05:20) is **stale** and unusable as
evidence: `combat.rs` was last modified 20:56, after that green. Not cited here.
