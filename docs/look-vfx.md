# Look — combat & world VFX layer (`client/src/vfx.rs`)

**Lane:** yamamoto (per `docs/LANES.md`'s "Animation / VFX / asset import" row —
reassigned from pixel/Flamingo, who authored the original round documented
below) · **Files:** `client/src/vfx.rs`, `client/src/vfx_bridge.rs`,
`client/src/shot_main.rs`, `scripts/render_vfx.sh`, this doc · **Not touched:**
`scene.rs`, `combat.rs`, `main.rs`.

## 2026-08-04 — hit-feedback pass (yamamoto)

Two bridge-only fixes, no edit to `vfx.rs` itself or to any other lane's file:

* **Contact height.** `SfxEvent::HitLight/HitHeavy/HitParry/HitBlock` all carry
  `combat.rs`'s enemy *root* transform — the husk's feet, not the point of
  contact. Every spark/debris/flash burst was therefore centred on the ground,
  roughly half of it clipped underground. `vfx_bridge.rs` now lifts husk hits to
  chest height (`HUSK_CONTACT_Y`) and blends parry/block hits most of the way
  toward the player's guard (`contact_point`) — the deflection happens at the
  player, not at the husk's silhouette.
* **Weapon trail rides the real blade.** The husk's swing trail used to track
  `combat::HuskArm`, a placeholder box `anim.rs`'s `attach_rigs` hides the
  instant a procedural rig lands on that actor — the ribbon was tracing an
  *invisible* entity's own arc, not the blade actually on screen, and the
  player had no trail at all. `anim.rs` now tags each rig's weapon mesh with
  `RigWeapon { actor }`; `vfx_bridge.rs` attaches `SwingTrail` there for both
  actors and drives `hot` off `anim::AnimSwing`'s `SwingPhase::Contact` edge
  (docs/anim-events.md) instead of reading `combat::Enemy::state` directly.

**Known gap, not in this lane:** the camera kick (`vfx::VfxCamera`) is only
ever attached in this file's own showcase stage — the real play camera (`cam`
in `main.rs`, the `Camera3d` + `OrbitCam` entity) never gets the marker, so
`apply_cam_kick` has nothing to push in the shipped game yet. One line
(`vfx::VfxCamera` added to that spawn's component tuple) in `main.rs`, which is
outside this lane — flagged to the Director rather than edited here.

---

## What landed

| # | Effect | Where it lives | Driven by |
|---|---|---|---|
| ① | **Impact burst** — hot sparks + tumbling voxel debris + flavour motes (pale ash for a husk, dark vital red for the player, white ring for a parry) + a one-frame light burst that lights the surroundings | `on_impact` | `Impact` message |
| ② | **Weapon trail** — a fading ribbon laid along wherever the blade entity actually is, emitted at a fixed 90/s so density is frame-rate independent | `emit_trail` | `SwingTrail` component, `hot` flag |
| ③ | **Hit-stop + camera kick** — 80/100/120/150 ms per §5.2, plus a directional recoil along the hit vector with a snap-out `(1−t)²` curve | `tick_hitstop`, `apply_cam_kick` | `Impact` message, `VfxCamera` marker |
| ④ | **Campfire** — a breathing coal bed, embers rising on negative gravity + heavy drag, and a three-detuned-sine flicker light | `campfire_build`, `campfire_pulse`, `tick_coals` | `CampfireVfx` component |
| ⑤ | **Death dissolve** — the body is refilled with a 4×7×4 voxel grid that comes apart **top-down** (head first, feet last), drifts upward and shrinks to nothing, leaving rising ash | `on_unravel` | `Unravel` message |
| ⑥ | **Hit flash** — a shell wrapping the struck body for 110 ms; white when you land a hit, **red when you take one**, bright on a parry | `on_impact` | `Impact.body_half` |

Every number traces to a doc rather than to taste:

* **Hit-stop 80 / 100 / 120 / 150 ms** — `combat-design.md` §5.2, matching `combat.rs`'s
  own constants so the visual freeze and the gameplay freeze are the same length.
  §5.2 also says *"do not use global time dilation"*, so `Hitstop` freezes only this
  module's particle integration; `Time`, the camera and gameplay keep running.
* **White / red / bright-ring flash** — `combat-design.md` §5.1.
* **Ash instead of blood on a husk** — `story-bible.md`: a husk is a villager the
  Hollow is still draining, and the Unravelling is "blocks losing cohesion and
  returning to raw chaos". Same visual language as the death dissolve, so a player
  learns to read "this thing is being unmade" from the very first hit.
* **Key light at 18°** on the showcase stage — `look-bible.md` §2 (15–25°).

---

## Wiring — one line, and it is safe to add before anything uses it

```rust
mod vfx;                                   // next to the other `mod` lines
// ...
.add_plugins(vfx::VfxPlugin)               // ← the whole hook
```

With no messages written and no marker components in the world, the plugin costs a
handful of empty queries per frame and draws nothing. It can land in `main.rs`
ahead of any lane wiring it up.

**Schedule note:** `apply_cam_kick` adds a *delta* to the camera transform, so the
plugin's `Update` set should run after whatever sets that transform
(`fly_camera`, then `combat::apply_shake`).

### The four opt-ins, when each lane is ready

| Want | Add | Lane |
|---|---|---|
| bursts + flash + hit-stop + kick | `MessageWriter<vfx::Impact>` where damage is applied (`combat.rs` ~L879) | combat |
| dissolve instead of vanish | `MessageWriter<vfx::Unravel>` right before the husk despawn (`combat.rs` ~L922) | combat |
| trail on heavy swings | `vfx::SwingTrail` on the blade/arm entity; `hot = matches!(state, Heavy \| Charged)` | combat |
| campfire coals + embers | `vfx::CampfireVfx::default()` on the campfire root (`scene.rs::spawn_campfire`) | scene |
| camera takes the kick | `vfx::VfxCamera` on the play camera | app |

`vfx.rs` deliberately does **not** `use crate::combat` — that keeps it compiling in
any binary, keeps a VFX tweak from forcing a cross-lane rebuild, and is why this
round needed no edit to another owner's file.

---

## Why the shots are not rendered in the kitchen

`hero.rs` is the **locked golden beauty shot** (interior voxel kitchen) and
re-framing it is forbidden. Combat VFX in a kitchen would also be nonsense. So the
showcase gets its own stage in `vfx.rs::setup_showcase` — village road, ruined wall,
campfire, a husk and the player mid-swing — carrying `hero.rs`'s post stack:
ACES tonemap, Bloom, Ultra SSAO, Bokeh DOF, colour grading, distance fog.

**One deliberate difference from `hero.rs`: no TAA.** TAA accumulates several frames
to resolve PCSS/SSAO noise, which is right for a still kitchen. Particles move metres
per frame and newly spawned ones have no motion-vector history, so TAA smears them
into ghosts — the shot would be grading the anti-aliaser, not the VFX. Shadows use
the Gaussian filter instead, which needs no history.

## Rendering the set

```bash
bash scripts/build_safe.sh build --bin voxelforge_shot --target-dir target-vfx
bash scripts/render_vfx.sh
```

Four PNGs into `docs/assets/`, same stage and same camera each time — only the VFX
beat differs, which is the only way the "before" plate is worth anything:

| File | `VOXELFORGE_VFX` | Shows |
|---|---|---|
| `vfx-00-before.png` | `off` | the plate — layer loaded, nothing fired |
| `vfx-01-impact.png` | `impact` | ①②③⑥ together: three hits at three ages, trail, flash |
| `vfx-02-dissolve.png` | `dissolve` | ⑤ mid-Unravelling |
| `vfx-03-campfire.png` | `fire` | ④ coals, embers, flicker |

The showcase timeline is deterministic — fixed xorshift seed, fixed fire times — so a
re-render reproduces the same frame and the pair can be diffed rather than eyeballed.

---

## Honest limits of this round

* **Wired into the game binary** (`vfx::VfxPlugin` + `vfx_bridge::VfxBridgePlugin`
  in `main.rs`'s plugin list, as of the commit that added `vfx_bridge.rs`), but
  **not yet provably running in a real play session** — `--play` currently panics
  entering `ENTER_PLAY` on an unrelated Bevy `B0002` in `streaming.rs` (Poppy's P0,
  in progress). The 2026-08-04 fixes above have not yet been screenshotted
  through the real camera; that proof is blocked on the P0, not on this lane.
* **Not graded against `look-acceptance-rubric.md`.** That rubric's gates are written
  for the interior hero shot (G2 looks for a window light-bar, G6 eyedrops sunlit
  wood); an outdoor VFX stage has none of those to measure. Grading these frames
  needs outdoor gate criteria written first — flagged, not faked.
* **No audio.** The impact beat is half-built without it; there is still no
  `bevy_audio` anywhere in the repo.
* **No ambient falling ash** (`story-bible.md` Act 0) yet — the emitter exists in all
  but name (`Particle` with negative gravity), but it belongs to a world-ambience
  pass, not the combat-feedback one this round covers.
