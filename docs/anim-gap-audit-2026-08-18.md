# Anim gap audit — 2026-08-18

**The note this answers:** CEO — *"animation ตอนนี้ห่วยมาก ไม่เหมือนเกม AAA เลย"*. This is not
an opinion pass — every item below is backed by a source-line citation, a real captured frame,
or both. Owner of `client/src/anim.rs`: Rose. This audit does not edit anim.rs, `scene.rs`,
`characters.rs` or `equipment.rs` — findings + evidence only, per this lane's assignment
(character/animation code, `client/src/anim*.rs`).

Evidence captured with the existing `target/release/voxelforge.exe` (built 2026-08-18 08:01,
newer than the last commits touching `anim.rs`/`combat.rs` — confirmed current, not stale) —
**no rebuild was done for this audit**, so nothing here changed the code it's measuring.
Screenshots + raw runlogs: `docs/assets/anim-audit-2026-08-18/`.

---

## 0. What the system actually is (so the ranking below isn't guessing)

`anim.rs` (3,315 lines) is a **fully procedural** rig — no skeletal clips, nothing sampled from
`assets/models` (there is no rigged source; see the file's own header). Per actor it builds a
13-joint box skeleton at runtime and drives every joint by hand-authored math:

- **Locomotion**: idle → walk → run blended continuously via `smoothstep(IDLE_SPEED, WALK_SPEED,
  speed)` / `smoothstep(RUN_LO, RUN_HI, speed)` (`anim.rs:1663-1666`), stride phase advanced by
  **distance travelled**, not time, so there's no foot-skate.
- **Attacks**: real anticipation → strike → follow-through with an authored *overshoot-past-rest*
  on the recovery leg (`anim.rs:2333-2348`, `ease_out_back`), plus a separate root-motion layer
  that gathers/sinks/leans the whole body through the swing (`swing_root_motion`, `anim.rs:2375-
  2408) — three chained light-attack arcs that each read differently (`LIGHT1/2/3_COCK/FOLLOW`,
  `anim.rs:2413-2536`), not the same swing repeated.
- **Secondary motion**: a damped spring per cloth panel (cloak/shawl) that laughably out-classes
  "boxes glued together" — it genuinely lags the body (`anim.rs:1891-1903`).
- **Also present**: turn-in-place with cross-stepping feet, landing squash-and-stretch, hit-react
  recoil, stagger wobble, death fall, and a whole conversation layer (contrapposto weight-shift,
  gesture, nod variety) that is honestly the best-animated thing in the game.

This matters for the ranking below: **the authoring quality inside `anim.rs` is not the
problem.** Every finding that follows is either (a) a wiring/integration bug that throws this
work away, (b) a gap in what `anim.rs` covers, or (c) a decision elsewhere in the stack that
makes the good parts of `anim.rs` invisible.

---

## 1. Ranked findings (worst-first)

### #1 — Story NPCs render as TWO overlapping bodies, every single session (code bug, anim.rs fix)

**Verified live**, not theoretical — every runlog captured for this audit shows both dressing
systems firing on the same NPCs in the same session:

```
ANIM_RIG_NPC dressed entity=629v0 look=Villager     <- anim.rs's own procedural box rig
ANIM_RIG_NPC dressed entity=630v0 look=Villager
CAST_DRESSED id=maren placeholder_mesh=removed body=maren feet=(32.0,13.0,4.0)   <- scene.rs's sculpted body
CAST_DRESSED id=toma  placeholder_mesh=removed body=toma  feet=(16.0,3.0,24.0)
CAST_SPAWN_NPC id=toma feet=(16.5,3.0,24.5) region=west_house
ANIM_RIG_NPC dressed entity=820v0 look=Villager     <- Toma's NEW entity also rigged
```
(full logs: `idle.log`, `combat-approach.log`, `quest-twoshot.log`)

**Root cause**, read from both sides:

- `scene.rs::dress_act1_cast` (`scene.rs:788-827`) finds every `quest::Npc` entity, strips its
  placeholder `Mesh3d`, and spawns a **brand-new, standalone** sculpted body via
  `characters::spawn_character` (`characters.rs:640-651`) — tapered limbs, rotated boxes, the
  24-box face from `docs/character-sculpt-pass.md`. It marks the original NPC entity `Dressed`
  (`scene.rs:809`) so it never re-dresses.
- `anim.rs::attach_rigs`'s NPC arm (`anim.rs:1368`, `1408-1415`) queries `With<quest::Npc>,
  Without<Enemy>, Without<Rigged>` — **it has no `Without<Dressed>` filter**, so it doesn't know
  scene.rs already gave this NPC a body. It builds its own plain-`Cuboid` Villager rig as a
  child of the *same* NPC entity, on the *same* transform.

Result: Maren, Toma and Garren each have a static sculpted body **and** a separate animated
box-rig standing inside/around it, simultaneously, in every normal play session. The Husk path
already guards against exactly this (`Without<crate::enemies::EnemyBody>` at `anim.rs:1365`) —
the villager path is the one spot nobody added the equivalent guard.

**This is very likely a real chunk of the "boxes glued together" read**: two conflicting meshes
sharing one silhouette reads as broken geometry, not as animation quality.

**Fix is pure code, one lane (`anim.rs`)**: add `Without<crate::scene::Dressed>` (or whatever
marker scene.rs exports) to `attach_rigs`'s `npc_q`, mirroring the existing `EnemyBody` pattern.
Rose owns this file; flagging rather than editing per this lane's scope.

---

### #2 — The sculpted geometry that was supposed to fix "boxes glued together" never reaches the Player or the Husk (needs new numbers + code, cross-lane)

`docs/character-sculpt-pass.md` (2026-08-14) is explicitly the fix for *"if it still looks like
boxes glued together, it isn't there yet"* — diagonals on shoulders/limbs, taper on every limb
segment, a 24-box face. It lives entirely in `client/src/characters.rs` + `equipment.rs`, and is
only ever spawned via `characters::spawn_character` — which, per §0/§1 above, is wired to:

1. Story NPCs (`scene.rs::dress_act1_cast`) — and even there, double-rendered (#1).
2. `VOXELFORGE_CAST_LINEUP` — a capture-only debug lineup, gated off in real play
   (`scene.rs:892-917`, "Unset ⇒ not registered, and `--play` is byte-for-byte the session it
   was").
3. The isolated `voxelforge_charshot` bin (`char_shot_main.rs`) — a portrait-shot tool, not part
   of the playable game at all.

Meanwhile the body the player actually **plays as** (Auren) and the body they actually **fight**
(the generic Husk, when it has no `EnemyBody`) are built by `anim.rs::init_rig_assets`
(`anim.rs:900-1034`) entirely from **flat, axis-aligned `Cuboid::new(...)` primitives** — zero
rotation, zero taper, one skin cuboid for the head. Every `Bx`/`rot`/taper mechanism
`character-sculpt-pass.md` built (`Bx::top()`, `br(...)`, the diagonal table) is a different,
parallel system that `anim.rs` never calls.

So the fix the CEO is presumably remembering from the sculpt pass exists in the codebase, was
shipped, and simply never reached the two characters the player spends the most time looking at.
`character-bible.md:266-270` already flags this exact gap from the art side ("`client/src/hero.rs`,
`client/src/look.rs`, `client/src/anim.rs` were not opened or edited [by the sculpt pass]").

**Not a quick fix**: porting `Bx`-style taper/rotation into `anim.rs`'s live rig means either (a)
re-authoring `anim.rs`'s Player/Husk `Dims`/`Parts` with the same box-count and rotation table
`characters.rs` uses (new numbers, still code), or (b) a deeper refactor to have `anim.rs` drive
`characters.rs`'s own box tree instead of its own `Parts` struct (cross-lane: touches Rose's file
and whatever owns `characters.rs`/`equipment.rs`).

---

### #3 — The player never sees their own combat animation (design/camera gap, verified by pixel-diff)

Gameplay is **pure first-person** — `main.rs` only ever spawns a `FlyCam` for play (`OrbitCam` is
editor/menu/cutscene-only; grepped, no third-person toggle exists in `main.rs`). There is no
view-model (no player arms/weapon rendered in view), and the dodge-roll's rotation lives on the
rig root — a *child* of the actor entity — not on the camera itself (`anim.rs:1908`,
`root_tf.rotation = ... * root_extra`), so the camera does not inherit it either way.

**Verified, not assumed** — captured the same idle scene with `VOXELFORGE_ANIM_POSE=attack`,
`=dodge`, `=parry` against the exact same starting frame (`docs/assets/anim-audit-2026-08-18/
{idle,attack,dodge,parry}.png`) and pixel-diffed them (16px sample grid, ΔRGB>15 threshold):

| pose vs idle | changed pixels |
|---|---|
| attack | 4.16% |
| dodge | 8.29% |
| parry | 3.98% |

That residual is fully explained by the scene's own animated ash/ember particles (visible as
scattered orange dots in every frame, including plain idle) — there is no visible arm, blade, or
camera roll change between any of these frames. All of §0's carefully authored anticipation /
strike / overshoot / weight-shift work for the **player's own** swing — the single most detailed
part of `anim.rs` — is invisible during solo play. The only way a player ever sees that system
render is by watching the **enemy's** copy of it, or during a scripted two-shot
(`quest-twoshot.png`).

This is not a bug to patch in `anim.rs` — it's a standing design/camera decision (no viewmodel,
no third-person option) that undercuts the most expensive animation work in the file. Flagging
for a design call, not proposing a fix within this lane's scope.

---

### #4 — Idle is one loop, forever (code-only fix, anim.rs)

`locomotion()`'s idle layer (`anim.rs:1993-1997`) is a single continuous sine:

```rust
let breath = (elapsed * 1.35).sin();
let idle_torso = pitch(0.020 * breath) * yaw(0.020 * (elapsed * 0.7).sin());
let idle_arm = 0.09 + 0.035 * breath;
```

That's the entire idle: one breathing cycle, constant amplitude, no weight shift, no glance, no
occasional re-settle — for as long as the player stands still. Contrast this with the
conversation layer three functions later (`conversation()`, `anim.rs:2066-2116`), which already
has a real contrapposto weight-shift on an irregular multi-sine cycle ("periods do not divide one
another, so the stance never lands back on the same frame" — the file's own comment at
`anim.rs:2062-2064`). That richer idle-like machinery exists in the file **today** — it's just
scoped to dialogue only. A standing, non-talking player or NPC never gets it.

Task brief asked specifically "idle มีกี่แบบ" — answer: **one**, and it's the plainest layer in
the whole system next to everything else `anim.rs` does.

**Fix is pure code, in-lane**: reuse the `load`/weight-shift pattern from `conversation()` for the
ordinary standing idle, gated the same way (`1.0 - loco`) it already is.

---

### #5 — No foot/ground IK anywhere in the rig (code-only fix, anim.rs, moderate effort)

Confirmed by grep — `anim.rs` has no `ik`, `foot_plant`, or ground-raycast of any kind. Leg pose
is pure angle: `pose.hip_l = pitch(leg_a * s)` etc., keyed only to stride phase
(`anim.rs:2017-2024`). Nothing clamps a foot's height to the terrain directly under it — the only
ground awareness in the whole file is combat's own `surface_y` clamp on the *root* (keeps the
Husk's whole body from sinking into the floor), which says nothing about an individual foot.

In a voxel game with visible stairs and uneven terrain, a walk cycle with no per-foot ground
solve will show a foot floating above a step or clipping into a rise on anything but flat ground
— exactly the kind of thing that reads as "not AAA" even when the limb angles themselves (§0) are
well-authored. Not captured visually in this pass (would need a scripted walk across stairs,
which the current demo hooks don't drive), but the absence in source is unambiguous and the
mechanism (no per-foot ground query) is unconditional, not situational.

**Fix is pure code, in-lane**, but more work than #1/#4: needs a per-foot downward query against
`World`'s voxel heightfield (already available — `scene.rs::ground_feet`/`highest_solid` do
exactly this kind of lookup for NPC placement) and a small IK blend on the shin/knee angle to
close the gap. Medium effort, not a one-line fix.

---

## 2. Split: code-fixable in this lane vs needs something else

| # | Finding | Fixable in `anim.rs` alone | Needs |
|---|---|---|---|
| 1 | NPC double-body | ✅ yes — one filter clause | — |
| 2 | Sculpted geometry not on Player/Husk | ⚠️ partial | new authored numbers and/or a cross-lane refactor touching `characters.rs`/`equipment.rs` |
| 3 | Combat anim invisible (no viewmodel/3rd-person) | ❌ no | a camera/design decision above this lane |
| 4 | One idle loop | ✅ yes | — |
| 5 | No foot IK | ✅ yes | moderate implementation effort (ground query + IK blend) |

## 3. Evidence index

All captured live from `target/release/voxelforge.exe` (2026-08-18 08:01 build), no source edits:

| file | how | shows |
|---|---|---|
| `idle.png` / `.log` | `--play` | baseline first-person idle |
| `attack.png` `dodge.png` `parry.png` / `.log` | `VOXELFORGE_ANIM_POSE=attack\|dodge\|parry --play` | pose-override captures used for the §3 pixel-diff |
| `clash.png` / `.log` | `VOXELFORGE_ANIM_POSE=clash VOXELFORGE_COMBAT_DEMO=1` | player + Husk both held at contact frame |
| `combat-approach.png` / `.log` | `VOXELFORGE_COMBAT_DEMO=1` | in-scene combat framing (Husk close-up) |
| `quest-twoshot.png` / `.log` | `VOXELFORGE_QUEST_DEMO=1` | source of the #1 double-dress log evidence |

All paths under `docs/assets/anim-audit-2026-08-18/`.

---

*Scope note: this audit only reads/captures. No edits were made to `client/src/anim.rs`,
`scene.rs`, `characters.rs`, or `equipment.rs` — those are Rose's / other lanes' files. Findings
#1, #4, #5 are one-file fixes for whoever owns `anim.rs`; #2 and #3 need a decision above a
single lane before any code moves.*
