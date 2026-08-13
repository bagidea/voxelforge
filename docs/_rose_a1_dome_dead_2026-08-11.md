# A1 sky-dome: the dome never reaches the fragment (DECISIVE)

**Date:** 2026-08-11 · **Lane:** Rose (engineer) · **Branch:** `poppy/native-only` @ `f17fe90`
**Status:** DEBUG-A1 reverted (`git checkout -- client/src/look.rs`); look.rs diff clean again.

> Director's question, answered first: *is the green measuring dome — when rendered —
> "green" or "still gold"?*
>
> **Answer: still gold. Every frame. The dome never reaches the fragment shader.**

If the dome's fragment ever ran, the HDR green emissive (`rgb(0, 8.0, 0)`) would make the
whole frame nuclear green — there is no "subtle" at intensity 8.0 after tonemap + bloom.
It did not. Therefore the gold sky the team has been grading is **not the dome material**
we have been tuning. It is the background (ClearColor / fog plate). **Every A1 sky axis
was measured at the wrong point.**

---

## The debug that was live (now reverted)

`client/src/look.rs` `sky_dome()` material, DEBUG-A1 block (HEAD-clean after revert):

```rust
// DEBUG-A1: vertex colors DISABLED — does base_color reach the fragment?
// mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);   // <-- was commented out
...
let material = materials.add(StandardMaterial {
    // DEBUG-A1: vivid green HDR emissive — if the dome renders at all, the
    // whole frame goes green; if it stays gold, the dome never reaches the
    // fragment (cull/clip/alpha) and what we see is the ClearColor instead.
    base_color: Color::BLACK,
    emissive: LinearRgba::rgb(0.0, 8.0, 0.0),
    unlit: true,
    fog_enabled: false,
    cull_mode: None,
    ...
});
commands.spawn((
    SkyDome,
    Mesh3d(mesh),
    MeshMaterial3d(material),
    ...
    // DEBUG-A1: explicit visibility — without this the dome is extracted as
    // invisible (no ViewVisibility) and never reaches the render phase.
    Visibility::default(),
));
```

This is the designed falsification test. The result falsified "the dome renders."

## Pixel evidence (8-bit RGB, top-40% sky band)

Script: `scripts/_rose_a1_dome_pixels.py` (gitignored `_rose_` prefix).

| file | sky RGB (R,G,B) | G-dom (G/(R+B)) | %sky px where G is max |
|---|---|---|---|
| `_probe_base.png` (baseline, no debug) | 146, 94, 35 | 0.52 | 0.2% |
| **`_dbg_dome_emissive.png` (green 8.0 ON)** | **138, 85, 25** | **0.52** | **0.0%** |
| `_dbg_dome.png` | 138, 85, 30 | 0.51 | 0.0% |
| `_dbg_dome_vis.png` | 138, 85, 25 | 0.52 | 0.0% |
| `_probe_green.png` (named green — but isn't) | 146, 94, 34 | 0.52 | 0.2% |
| `sky-dome.png` | 146, 94, 35 | 0.52 | 0.2% |

**Reading:** a rendered green-dome frame would show `G-dom >> 1.5` and `%G-max ≈ 100%`.
Every frame — debug or not — sits at `G-dom ≈ 0.52` with **0%** green-max sky pixels.
G is *below* R everywhere: this is a warm/gold sky, identical whether or not the green
emissive is active.

Per-pixel diff `_dbg_dome_emissive` vs `_probe_base` (same 2560×1440): max 255, mean 7.9,
32% of pixels changed >5. That 32% is **global luminance drift between two captures**
(different sim time-of-day / exposure), **not** a green dome appearing — if it were, the
green channel would dominate and `G-dom` would spike. It does not. The three `_dbg_dome*`
variants are near-identical to each other regardless of which debug flag was set, which is
itself the signature of "the material change has no visible effect."

## Why this kills the A1 verdicts

1. **The dome material is not what is on screen.** We have been editing `base_color`,
   vertex gradient, `sky_gain`, fog on the dome's `StandardMaterial`. None of it can reach
   the frame if the dome geometry never enters the render phase. The gold is the
   **ClearColor / background**, untouched by every dome knob turned so far.
2. **`sky_min_frac` was read as a fraction (20%) but the code applies it as percent (0.2%).**
   `scripts/art_order_grade.py`:
   - L67: `"sky_min_frac": 0.20,  # % of frame that must be sky before A1 is gradeable`
   - L172: `if core.mean() * 100 < T["sky_min_frac"]:`

   `core.mean()` is a fraction (0..1); `* 100` turns it into percent (0..100); compared
   `< 0.20`. So the "is there enough sky to grade" floor is **0.2% of the frame**
   (≈ 3,686 px of 2560×1440), **not 20%**. Anyone reading the threshold `0.20` as "20% of
   the frame must be sky" was off by 100×. The gate is nearly trivial, so it never flagged
   that the "sky" being graded was the wrong surface.
3. **The blue-sky detector can't see a gold sky anyway.** `sky_region()` (L171) gates
   `core` on `hue ∈ [185, 255]` (cyan→blue). A gold sky (hue ~30–50) yields `core ≈ ∅`,
   so real frames report "no sky to grade." The synth control `synth_sky()` builds a *blue*
   ramp and passes A1 — so "A1 passes" was true only of the synthetic blue control, never
   of a real gold-sky frame. The dome that *would* supply a real gradient never rendered.

## What is actually rendering the gold (TBD — root cause next)

Not yet pinned. Candidates, in order of likelihood:
- **Camera clear color** — a warm `ClearColor` fills the frame wherever no geometry draws;
  the dome (the only thing meant to cover the sky) doesn't draw, so the clear shows through.
  `hero.rs:42` imports `ClearColorConfig`; the actual `ClearColor` resource value needs
  confirming (grep did not surface an explicit assignment — likely a default or plugin-set).
- **ViewVisibility** — the code comment itself flags that the dome is "extracted as
  invisible (no ViewVisibility)" without the explicit `Visibility`. `Visibility::default()`
  is `Inherited`; for a root-spawned entity that *should* compute visible, but Bevy's
  visibility propagation has bitten this codebase before. This is the prime suspect.
- **Near/far clip** — dome radius is `SKY_DOME_RADIUS` (~640 units). If the camera's far
  clip is shorter, the dome is clipped entirely. Needs checking against the projection.

The green-emissive probe cannot distinguish these (all of them yield "gold"). The next
debug should target visibility/clip directly (e.g. spawn a small bright cube at a known
near distance to confirm *something* reaches the camera, then walk it out to dome radius).

## State of the disk (evidence preserved)

- Probe PNGs kept (untracked, harmless): `docs/assets/gate3/_dbg_dome*.png`,
  `_probe_green.png`, `_probe_base.png`, etc.
- `scripts/_rose_a1_dome_pixels.py` — re-runnable pixel probe (this evidence).
- `client/src/look.rs` — reverted to HEAD `f17fe90`; DEBUG-A1 fully removed.
- `client/src/quest.rs` — NOT touched (other lane, +53).

## Bottom line

The dome is dead at the fragment level. Until it is made to actually render, no dome-knob
A1 number is meaningful, and `sky_min_frac=0.20` must be re-read as **0.2%** (or the code
fixed to match the intended 20% — a product decision for Director/Shiba).

---

## UPDATE 2026-08-14 — root cause found + source fix applied (build proof PENDING)

**Status:** root cause pinned at the source level. Fix written to `client/src/look.rs`.
**NOT YET BUILD-PROVEN** — per Director, the build is fired detached by the owner (this
lane was told not to light a `cargo build`; the box has limited headroom and the quest
lane is building). This section is honest about that: until the green proof frame lands,
"fixed" is a source claim, not a proven one.

### The root cause (why the dome never reached the fragment)
The dome spawn (`sky_dome()`) was missing a **`Visibility`** component. In Bevy 0.19 an
entity without `Visibility` never gets a `ViewVisibility` computed, and the render world's
mesh extraction only pulls entities whose `ViewVisibility` is set — so the dome geometry
was never extracted, its fragment shader never ran, and every dome knob (`base_color`,
vertex gradient, `sky_gain`, the green emissive) was editing a material nothing drew. The
gold frame is the `ClearColor` showing through where the dome should have been.

This is not a theory — it is the delta between the dome and every working mesh entity in
the crate, all of which spawn with `Visibility::default()`:

| entity | spawn | renders? |
|---|---|---|
| Maren NPC | `Mesh3d, MeshMaterial3d, Transform, Visibility::default()` (`quest.rs:786`) | yes |
| imported model | `transform, Visibility::default()` (`import.rs:205`) | yes |
| **sky dome** | `Mesh3d, MeshMaterial3d, Transform` — **no `Visibility`** (`look.rs` spawn) | **no** |

The dome was the **sole** mesh entity in the crate without `Visibility`.

### The Director's candidate list, ruled in/out at source
- **Visibility / ViewVisibility** — **THE root cause** (above). Fixed.
- **far-plane clip** — ruled OUT. The gameplay camera spawns `Camera3d::default()` with no
  explicit `Projection` (`main.rs:867`), so it runs Bevy 0.19's default
  `PerspectiveProjection { near: 0.1, far: 1000.0 }`. Dome radius **640 < 1000** → not
  clipped. (Only the hero shot sets a custom projection, `hero.rs:743` — different scene.)
- **cull_mode** — already correct (`cull_mode: None`, both faces, visible from inside).
- **alpha mode / render layer** — opaque, default layer 0; not the cause.
- **NotShadowCaster** — not a "won't render" cause, but a correctness follow-on the fix
  adds anyway: a dome that encloses the whole scene would, once it renders, cast its shell
  as a shadow over everything inside it. Added so the fix is complete, not half.

### Why the earlier green probe "did nothing"
Those captures came off a **contested build lane** — the three `_dbg_dome*` frames were
near-identical regardless of which debug flag was set (see the pixel table above), which is
the signature of a binary that never relinked, not of a live material with no effect. The
new proof defeats that trap explicitly: it prints a marker string (`LOOK-A1-PROOF: …ACTIVE`)
that does not exist in any prior binary, so seeing it proves the fresh build ran.

### The fix (source, `client/src/look.rs`)
1. Spawn gains `Visibility::default()` + `NotShadowCaster` (the permanent fix).
2. A **TEMP-A1-PROOF** block (env `VOXELFORGE_LOOK_SKYPROBE=1`) swaps the dome material
   for a nuclear-green HDR-8.0 unlit emissive. Shipped path (env unset) = real gradient.
3. Import: `NotShadowCaster` added to the existing `use bevy::light::{…}` (it lives in
   `bevy_light`, re-exported as `bevy::light`).

### Falsification (the repeated original test) — run by the owner
Build the fixed source, then shoot ONE frame with `VOXELFORGE_LOOK_SKYPROBE=1`:
- **whole frame nuclear green** → the dome now reaches the fragment; the A1 axes were
  measuring the wrong surface, exactly as this doc concluded. **FIX CONFIRMED.**
- **still gold** → the fix is wrong/incomplete; the next suspect is frustum culling of the
  640-unit shell (would add `NoFrustumCulling`), then a render-graph/phase issue.

Then unset `SKYPROBE`, shoot the real gradient dome, and confirm a blue-zenith / warm-horizon
sky replaces the flat gold clear.

### On green confirmation
Revert the TEMP-A1-PROOF block (the env read + the `if sky_probe` branch) → the committed
diff is exactly two lines on the spawn: `Visibility::default()` + `NotShadowCaster`. Clean.
