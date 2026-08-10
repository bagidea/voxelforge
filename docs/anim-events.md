# Animation timing events — the anim ↔ combat/VFX/audio contract

> Owner: **Rose** (`client/src/anim.rs`). This doc is the contract every other lane
> subscribes to. It mirrors `client/src/anim.rs` commit `33c8177` exactly — if the
> two disagree, **the code is right and this doc is stale**, not the other way
> around.

`anim.rs` is the procedural-rig layer (idle/walk/run blend, attacks, dodge, parry,
hit-reaction, death). It now **fires a Bevy message stream** so combat / VFX /
audio can sync the *picture* — the exact frame the rig reaches a phase — without
reading animation internals and without editing `anim.rs`.

## 0. The one rule

**anim ↔ gameplay events are split on purpose.** `combat.rs` already publishes the
*gameplay truth*; anim publishes the *visual timing*:

| question | answered by | message |
|---|---|---|
| Did the blade actually connect / count? | `combat.rs` | `combat::ImpactEvent` (only on a real hit) |
| Did a roll actually get paid for & start? | `combat.rs` | `combat::DodgeEvent` |
| Did poise break? | `combat.rs` | `combat::StaggerEvent` |
| **Where is the body in the move right now?** | **`anim.rs`** | **the events below (hit *or* whiff)** |

Use combat's events for gameplay decisions (damage, i-frames, state). Use anim's
events to time things that must follow the *picture* — the spark, the whoosh, the
camera pop. The anim event fires on every swing/roll/parry whether or not it
landed, because the body still made that motion.

## 1. The events

All are Bevy 0.19 **messages** (`MessageWriter`/`MessageReader`, **not** the old
`EventWriter` — that type does not exist in this engine). `AnimPlugin` registers
every one (`app.add_message::<..>()`), so consumers only declare a reader. anim.rs
is the sole writer.

### `AnimSwing` — attack phases (anticipation → contact → follow-through)

```rust
pub enum SwingPhase { Windup, Contact, Recover }
pub struct AnimSwing { pub actor: Actor, pub phase: SwingPhase, pub combo: u8 }
```

Fired **once per phase entry** (edge-detected — never spams per frame). `actor` is
`Actor::Player` or `Actor::Husk`. `combo` is `1..=3` for chained player lights,
`0` for a heavy / charged / husk swing.

| phase | meaning | when (normalised `beat.t`) |
|---|---|---|
| `Windup` | blade cocked back — the telegraph | `t < active.0` |
| `Contact` | hitbox open, blade at fastest — **spawn the spark/SFX here** | `active.0 ≤ t < active.1` |
| `Recover` | energy spent, swinging back to guard | `t ≥ active.1` |

The `Contact` band is **exactly** combat's active window (`LIGHT_ACTIVE` etc.),
normalised the same way `player_beat`/`husk_beat` normalise it — so the anim
contact event and the gameplay hitbox open on the same frames.

### `AnimDodge` — dodge i-frame edges

```rust
pub enum DodgePhase { IframeStart, IframeEnd }
pub struct AnimDodge { pub actor: Actor, pub phase: DodgePhase }
```

The i-frame window is the first `DODGE_IFRAMES / (DODGE_IFRAMES + DODGE_RECOVERY)`
of the roll.

| phase | when (`beat.t`) | real time |
|---|---|---|
| `IframeStart` | roll begins, `t` enters `[0, iframe_end]` | 0.000 s |
| `IframeEnd` | `t` crosses `10/22 ≈ 0.4545` | ≈ 0.167 s (10 frames @60) |

### `AnimParry` — parry receive-window edges

```rust
pub enum ParryPhase { Open, Close }
pub struct AnimParry { pub actor: Actor, pub phase: ParryPhase }
```

| phase | when (`beat.t`) | real time |
|---|---|---|
| `Open` | guard flashes into parry pose | 0.000 s |
| `Close` | `t` crosses `PARRY_WINDOW_FRAC = 0.20/0.70 ≈ 0.286` | ≈ 0.200 s (12 frames @60) |

### `AnimHit` — flinch / hit-reaction

```rust
pub struct AnimHit { pub actor: Actor }
```

Fires the frame the rig's HP drops (additive recoil arms). **Distinct from
`combat::StaggerEvent`**: every stagger is a hit, not every hit is a stagger. Use
this for the small flinch VFX/SFX; use combat's `StaggerEvent` for the poise-break
reaction.

### `AnimFootstep` — footfall sync (audio)

```rust
pub enum Foot { Left, Right }
pub struct AnimFootstep { pub actor: Actor, pub foot: Foot }
```

Twice per stride cycle, alternating L/R. Only while actually locomoting and free
of a committed action — the stride phase is advanced by **distance travelled**, so
a standing body crosses no phase boundary and emits nothing (no idle footsteps).

## 2. Subscribe (no edit to `anim.rs`)

```rust
use bevy::ecs::message::MessageReader;
use crate::anim::{AnimSwing, SwingPhase};

/// VFX: spawn the blade spark exactly on the contact frame, hit or whiff.
fn spawn_contact_spark(mut swings: MessageReader<AnimSwing>) {
    for s in swings.read() {
        if s.phase == SwingPhase::Contact {
            // s.actor  — whose blade (Player / Husk)
            // s.combo  — 1..=3 for chained lights, 0 for heavy/charged/husk
        }
    }
}
```

`MessageReader` cursors are per-reader, so consuming the stream here does **not**
steal it from another consumer (same rule `vfx_bridge.rs` already relies on for
`SfxEvent`). Register the system in your own plugin's `build`; the message type is
already wired by `AnimPlugin`.

## 3. Capture hook — `VOXELFORGE_ANIM_POSE`

For an isolated PNG of a single move's canonical frame, anim.rs holds the **player
rig** at that pose when the env var is set (native only; `None` in every normal
play session, so gameplay is untouched):

```
VOXELFORGE_ANIM_POSE=attack   # Contact frame of a light-1 (blade mid-strike)
VOXELFORGE_ANIM_POSE=dodge    # mid-roll tuck (beat.t = 0.5)
VOXELFORGE_ANIM_POSE=parry    # centre of the receive window (guard up to deflect)
VOXELFORGE_ANIM_POSE=clash    # player AND the nearest Husk, each at their own
                               # contact frame — for two-actor shots where the
                               # brief wants blades meeting, not two independently
                               # posed actors (e.g. --combat-demo clash shots)
```

Grab a frame with the **main** `voxelforge` bin (that is where `AnimPlugin` runs —
the isolated `voxelforge_shot` bin renders hero/VFX only and cannot host the rig,
which depends on the full game graph):

```
VOXELFORGE_ANIM_POSE=attack VOXELFORGE_SHOT=docs/assets/anim-attack.png \
  cargo run --bin voxelforge -- --play
```

Each held pose sits at the move's most legible still, and the same edge logic that
fires events in play also fires the representative event during a grab (e.g. the
attack pose logs `AnimSwing(Contact)`, the parry pose logs `AnimParry(Open)`).

## 4. Why the timings cannot drift

Every threshold above is computed from **combat's own `pub const`s** inside
`player_beat` / `husk_beat` and the edge logic — `LIGHT_ACTIVE`, `LIGHT_TIME`,
`DODGE_IFRAMES`, `DODGE_RECOVERY`, `PARRY_WINDOW`, `PARRY_STATE_LEN`. Tune the feel
in `combat.rs` and both the gameplay window and the anim event move together; there
is no second set of magic numbers to keep in sync. The unit tests in `anim.rs`
(`*_matches_combat`, `*_lands_inside_the_gameplay_hitbox`) fail loudly if that
invariant ever breaks.

## 5. Proof checklist — `RigWeapon` really is the blade on screen

> Status: **written, not yet run** — no green `voxelforge` exe existed at write time
> (a teammate's build owned `target/debug`). Every marker below is printed by the
> game itself (`anim.rs` / `vfx_bridge.rs`), not by a wrapper script — a script that
> prints its own PASS/FAIL is not a valid substitute for these lines appearing in
> the real log. Run each item against a freshly built `voxelforge` exe and read the
> raw stdout; do not trust a script's summary of it.

`da858cc` (`feat(anim): expose RigWeapon on the rigged blade entity`) made
`anim::build_rig` spawn the weapon as its own `RigWeapon{actor}` entity instead of
a `skin()`-attached child, so `vfx_bridge.rs` can put the swing trail on the exact
blade mesh instead of `combat::HuskArm` (a placeholder `attach_rigs` hides the
moment a rig lands — see `vfx_bridge.rs`'s `attach_rig_weapon_trail` doc comment).
Three markers make that chain provable end to end:

| marker | printed by | fires |
|---|---|---|
| `ANIM_RIG_WEAPON spawn actor=<Player\|Husk> entity=<N>v<G>` | `anim::build_rig` | once per rig built (live rig **and** corpse rig — both call `build_rig`) |
| `VFX_RIG_WEAPON_TRAIL attach actor=<..> entity=<N>v<G>` | `vfx_bridge::attach_rig_weapon_trail` | once per `RigWeapon` entity, the frame after it spawns |
| `VFX_SWING_TRAIL hot=<bool> actor=<..> phase=<Windup\|Contact\|Recover>` | `vfx_bridge::drive_rig_weapon_trail` | once per `AnimSwing` phase edge (never per-frame — `AnimSwing` is itself edge-detected) |

All three use `{:?}` on `Entity`, so `entity=` prints as bevy's `<index>v<generation>`
(e.g. `3v1`) — compare that whole token, not just the index.

### 5.1 — `RigWeapon` is spawned for both actors

```
VOXELFORGE_COMBAT_DEMO=1 cargo run --bin voxelforge > docs/proof/combat_demo.log 2>&1
```

(`--combat-demo` boots straight to Play at `spawn_encounter`'s tighter 2.2-block
distance and scripts a full attack/dodge sequence — see `main.rs`'s
`combat_demo_env_only` / the `COMBAT_DEMO` markers already in `combat.rs`.)

- **Pass:** at least one `ANIM_RIG_WEAPON spawn actor=Player ...` line and at least
  one `ANIM_RIG_WEAPON spawn actor=Husk ...` line, both appearing **before** the
  first `COMBAT_DEMO attack:` line.
- **Fail:** either actor missing, or zero `ANIM_RIG_WEAPON` lines at all (rig never
  built — the query in `attach_rig_weapon_trail` would then have nothing to find).

### 5.2 — the trail attaches to the *same* entity `anim.rs` built, not a proxy

From the same log:

- **Pass:** every `VFX_RIG_WEAPON_TRAIL attach actor=X entity=E` line has a
  matching `ANIM_RIG_WEAPON spawn actor=X entity=E` line earlier in the log with
  the **identical** `entity=` token. Exactly one `attach` line per `spawn` line
  (1:1 — the `Without<SwingTrail>>` filter in `attach_rig_weapon_trail` means a
  weapon entity is picked up exactly once).
- **Fail:** an `attach` line whose `entity=` does not appear in any prior `spawn`
  line (proof the trail landed on some other entity — the exact bug `da858cc`
  fixed for the husk), or an entity attached more than once.

### 5.3 — the trail goes hot on `Contact` and cold on `Recover`, in lockstep with the real hitbox

From the same log, for the player's first scripted attack:

- **Pass:** a `VFX_SWING_TRAIL hot=true actor=Player phase=Contact` line appears
  at or before the next `COMBAT_DEMO attack: husk_hp ...` line (contact frame
  gates the hit), followed later by `VFX_SWING_TRAIL hot=false actor=Player
  phase=Recover` before the next attack's `Contact`. No `hot=true` line with
  `phase=Windup` or `phase=Recover` (those must stay cold per §1's table).
- **Fail:** trail never goes hot, goes hot on the wrong phase, or stays hot past
  the matching `Recover` line.

### 5.4 — the husk corpse gets its own rig + trail too (not a dangling reference into the despawned live husk)

From the same log, after the `COMBAT_DEMO husk death: ... alive=false` line:

- **Pass:** one more `ANIM_RIG_WEAPON spawn actor=Husk ...` line (the corpse's rig,
  from `spawn_husk_corpse` calling `build_rig` again) and a matching
  `VFX_RIG_WEAPON_TRAIL attach actor=Husk ...` with the same new entity token. Final
  tally across the whole log: `actor=Husk` appears **twice** in `ANIM_RIG_WEAPON`
  (live + corpse) and **twice** in `VFX_RIG_WEAPON_TRAIL attach`; `actor=Player`
  appears **once** in each (no player corpse path exists).
- **Fail:** corpse has no `ANIM_RIG_WEAPON`/`attach` pair (trail would be riding the
  despawned live husk's now-invalid entity, i.e. silently doing nothing).

### 5.5 — manual fallback (no scripted demo)

If `--combat-demo` is ever changed to skip the rig path, the same four checks work
against a normal session — just trigger the swing by hand instead of trusting the
script's timing:

```
cargo run --bin voxelforge -- --play
```

Left-click (or `X`) once locked in as the player facing the Guard Husk, then read
stdout for the same five marker families above. Numeric criteria are identical;
only the trigger changed from scripted to manual.
