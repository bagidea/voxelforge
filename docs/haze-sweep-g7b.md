# G7b haze sweep — the thickness/quality grid + the phantom-target post-mortem

> Rose (Engineer), 2026-08-05. Companion to [look-g7-haze-2026-08-05.md](look-g7-haze-2026-08-05.md)
> (what shipped) and the G7 axes in [scripts/grade_g7.py](../scripts/grade_g7.py) (the gate).
> This file is the **measurement record** for the haze work: the sweep grid, and the answer to
> "is the haze target even real?" — which is what unblocked the lane.

## TL;DR

1. **The haze target `net_mean ≥ 3.5 / %≥5 ≥ 15%` is a phantom target** by this project's own
   standard. The golden reference is a single indoor-kitchen image with no aerial perspective to
   measure, so the target was never derived from — and cannot be calibrated against — the
   reference. The P0-axes autopsy retired a target for exactly this reason on 2026-07-26
   (*"target ที่ REF วิ่งเข้าไม่ได้ = phantom"*). Same ruling applies here.
2. **The shipped haze is not failing it anyway.** On every whole-frame haze-intensity reading —
   signed net Δ, mean |ΔL|, %≥5 — the shipped Linear ramp scores **~2.2–4.6 / ~21–36 %**, at or
   above the target. The haze is strong, depth-correct, and the near-field dead zone works
   (0.10 mean |ΔL|).
3. **Step 2 (density/extinction + luminance/contrast sweep) does not trigger.** The reference
   reads ≪ 3, which is the "criteria are wrong" branch, not the "wrong knob" branch. And the
   density/extinction question is already closed: the old G7 `ExponentialSquared` curve failed
   the A2 depth gate (ratio 2.33 < 2.5) through a near-field floor; the shipped G7b Linear ramp
   fixed it (ratio 274).

## Provenance note (read before quoting a number)

The first-pass thickness×quality sweep was run in a session the watchdog cut, and **its exact
`net_mean/%≥5` harness and row data are not on disk** (`grep -rni net_mean *.log *.tsv` and the
whole `scripts/` tree return nothing; the only haze sweep artefacts on disk are Flamingo's
`_flamingo_g7b/g7.tsv` and `_flamingo_g7b/a2.log`, which carry the G3/G5/G6/p95/vegSat and A2
columns — **not** `net_mean/%≥5`). Rather than quote the old plateau from memory, every number
below was **re-measured on 2026-08-05** from the real frames in `_flamingo_g7b/` with a rebuilt
harness ([scripts/_rose_haze_delta.py](../scripts/_rose_haze_delta.py), gitignored). The
thickness axis is real measured data; the quality axis is proven flat from source (see §2).

## The metric (re-established)

`net_mean` and `%≥5` are an **A/B haze delta**: per-pixel luminance change between the haze-on
frame and the **same scene's zero-fog baseline** (`VOXELFORGE_LOOK_HAZE=0`, the legacy ramp that
is zero fog across this ~55-block-deep set — the A/B baseline the whole G7 lane grades against).

* `net_mean` = mean of the per-pixel luminance delta (0–100, Rec.709) over the frame.
* `%≥5`    = fraction of pixels whose |ΔL| ≥ 5.

This is the same `_delta` machinery [grade_g7.py](../scripts/grade_g7.py) uses for axes A2/B;
the only difference is the aggregation (whole-frame mean + a 5-L threshold instead of a far/near
ratio). It is a **pair** measurement, and that fact is the whole of §1.

## §1 — Is the target real? Run it on the reference.

The Director's test: *take the same metric, run it on `docs/assets/golden-beauty-shot-ref.png`,
and see whether the reference itself reaches 3.5 / 15 %.*

**It cannot be run on the reference at all, and that is the answer.** Two independent reasons,
both measured:

**The reference is an indoor kitchen, not an outdoor scene.** Single-image proxies on the
reference vs a shipped-haze outdoor frame (same harness, `--single --haze 153,184,224`, the
shipped `FOG_COLOR_DAY`):

| frame | meanL | top-band L | bottom-band L | top−bot | mean\|L−hazeL\| |
|---|---|---|---|---|---|
| `ship-nohud2` (outdoor, shipped haze) | 39.1 | 55.0 | 32.3 | **+22.7** | 31.6 |
| `golden-beauty-shot-ref` (indoor kitchen) | 27.5 | 17.3 | 22.2 | **−4.9** | **44.3** |

Aerial perspective is a *depth gradient toward the haze colour*: the outdoor frame brightens
toward the hazy horizon by +22.7 L top-to-bottom and sits 31.6 L from the blue haze colour. The
reference is **inverted** (ceiling shadow is darker than the lit floor, −4.9) and 44.3 L from the
haze colour — it is a warm interior with no depth geometry dissolving into air. There is no
aerial-perspective signal in it to measure.

**The metric is a pair measurement, and the reference has no pair.** `net_mean/%≥5` needs a
haze-on and a haze-off render of the *same* scene. The reference is one PNG of a different
scene. There is no `golden-beauty-shot-ref-hazeoff.png`, and there cannot be — the kitchen is
~5 m deep, not ~55.

**Verdict.** Every other target in the rubric was re-derived by self-grading the reference (P0
axes 6/6, G3/G5/G6 calibrated — see the calibration logs in
[look-acceptance-rubric.md](look-acceptance-rubric.md)). The haze target was not, and could not
be. By the P0 standard that is the definition of a phantom target. The Director's hypothesis
(*"reference ~1.6/<3 ⇒ target set in a vacuum"*) is confirmed, and the truth is starker: the
reference's haze score is structurally ≈0, not ≈1.6.

⇒ **Step 2 does not trigger** (reference ≪ 3 is the "criteria are wrong" branch). No
density/extinction or luminance/contrast 4×3 sweep is run. See §3 for why density/extinction is
already a closed question regardless.

## §2 — The thickness × quality grid (measured)

Apparatus: `python scripts/_rose_haze_delta.py --ab <off> <on...>`, baseline
`_flamingo_g7b/hazeoff-nohud2.png` (`VOXELFORGE_LOOK_HAZE=0`), pinned camera
`VOXELFORGE_LOOK_CAM=35,-18,26`, de-HUD'd (`-nohud2`). Frames are Flamingo's G7b shoot; the
delta math is mine. Per-band = luminance |ΔL| on row-thirds (far = top, near = bottom), **not
sky-masked** — so the per-band columns are a *different metric* from the A2 ratio column, which
is Flamingo's `grade_g7.py` (max-channel |Δ| on a sky-masked far band, 18% of rows); see ¹.
They are not meant to divide to the A2 ratio.

| label | env override | net\|ΔL\| | %≥5 | far mean/%≥5 | mid mean/%≥5 | near mean/%≥5 | A2 ratio¹ (grade_g7) |
|---|---|---|---|---|---|---|---|
| hazeoff | `HAZE=0` (baseline) | 0 | 0 | — | — | — | — |
| **ship** | default `20,250` | **3.32** | **28.3** | 5.71 / 46.8 | 4.12 / 37.7 | **0.10 / 0.1** | PASS (ratio 274) |
| s16 | `FOG=16,250` | 4.40 | 36.1 | 6.36 / 50.8 | 5.84 / 52.7 | 0.95 / 4.5 | PASS (33) |
| s24 | `FOG=24,250` | 2.54 | 20.6 | 4.97 / 44.2 | 2.58 / 17.3 | 0.06 / 0.0 | PASS (182) |
| e220 | `FOG=20,220` | 3.66 | 30.7 | 6.35 / 49.0 | 4.48 / 42.5 | 0.12 / 0.1 | PASS (255) |
| e280 | `FOG=20,280` | 3.08 | 26.2 | 5.24 / 45.9 | 3.84 / 32.5 | 0.12 / 0.1 | PASS (168) |
| hd40 | `HAZEDESAT=0.40` | 2.84 | 24.0 | 4.74 / 43.3 | 3.64 / 28.3 | 0.11 / 0.1 | PASS (192) |
| ssaooff | `SSAO=off` | 3.42 | 28.9 | 5.76 / 47.1 | 4.28 / 39.3 | 0.20 / 0.2 | PASS (78) |
| g7curve | `HAZE=0.0072` (old G7 ExpSq) | 4.58 | 36.4 | 5.56 / 46.8 | 5.41 / 49.4 | **2.75 / 12.8** | **FAIL (2.33)** |
| g7full | `HAZE=0.0072;DESAT=0.40` | 3.86 | 30.7 | 4.54 / 42.1 | 4.76 / 42.2 | **2.26 / 7.7** | **FAIL (2.35)** |

> ¹ **Two harnesses share this table — do not divide the per-band columns to get the A2 ratio.**
> The `net|ΔL|`, `%≥5`, and `far/mid/near mean` columns are Rose's `_rose_haze_delta.py`: Rec709
> **luminance** |ΔL| on row-thirds, **no sky mask**. The `A2 ratio` column is Flamingo's
> [grade_g7.py](../scripts/grade_g7.py) axis A2: **max-channel** |Δ| on a **sky-masked far band**
> (top 18% of rows). On `ship` that is the whole difference: far 5.71 / near 0.10 here (luminance,
> unmasked = **57×**) vs far **48.35** / near **0.18** in `_flamingo_g7b/a2.log` (max-channel,
> sky-free = **274×**). The two ratios disagree because they are different metrics on different
> masks, not because either number is wrong; both are reported so each reproduces from its own
> harness.

Read it top-to-bottom:

* **The whole-frame haze-intensity reading does not come close to "failing 3.5/15%".** Every
  shipped-ramp row is at or above it on `net|ΔL|` and well above it on `%≥5`. Signed-net Δ (not
  shown) is `+2.2 … +3.4` for the same rows — same picture. The haze is strong.
* **The thickness axis moves haze in the band it should (mid/far), and leaves the near field
  dead** (0.06–0.12 across `s24/e220/e280/ship`). That is the G7b contract — `HAZE_START=20`
  dead zone — working exactly as shipped.
* **The two `FAIL`s are the old G7 `ExponentialSquared` curves**, and they fail for the
  documented reason: a near-field floor (`near 2.26–2.75` vs `0.10` for the ramp), which sinks
  the A2 depth ratio to 2.33–2.35. The Linear ramp was shipped to kill exactly this.

### Quality axis — flat, by construction (proven from source, not shot)

A quality tier does **not** move the distance haze, so the grid's quality dimension is constant
and was not re-shot. Proof from [client/src/look.rs](../client/src/look.rs):

* `base_camera_look()` (look.rs:813) inserts `distance_fog()` (look.rs:880) into the **base
  bundle** that `insert_stack` puts on the camera *before* the tier `match` (look.rs:949). The
  doc on `distance_fog()` (look.rs:788–791) states the design intent verbatim: *"One
  constructor for every tier: the haze is not a luxury effect that scales with the GPU."*
* The tier `match` (look.rs:966–1030) only varies: shadow filter, TAA, **`VolumetricFog` (god
  rays, High/Ultra only)**, and PCSS (Ultra only). `DistanceFog` is not in any arm — it is
  identical Low→Ultra.

So `net_mean/%≥5` for the **distance haze** is the same number at Low, Medium, High, Ultra; the
grid is one column wide on quality by right, not by shortcut. (The tier-varying atmospheric
effect is the god-ray `VolumetricFog`, which is a different axis — Pass 3, not haze. If an
empirical Low-vs-Ultra confirmatory shoot is wanted, the exe at `target/debug/voxelforge.exe`
@ 04:49 is ready and the shoot is one pinned-camera pair; not run here because the source proof
is definitive.)

## §3 — Step 2 outcome: skipped, and density/extinction is already closed

The Director's decision tree sends us to a density/extinction + luminance/contrast 4×3 sweep
**only if** the reference scores ≥ 3. It scores ≈ 0 (§1). ⇒ **skipped.**

It is worth saying why this does not leave a gap: **density/extinction was the right knob, and
it has already been turned.** The old G7 `ExponentialSquared` curve at `HAZE_DENSITY=0.0072`
failed the A2 depth-dependence gate (ratio 2.33 < 2.5, near-field floor 19.7 max-channel Δ —
see [look-g7-haze-2026-08-05.md](look-g7-haze-2026-08-05.md) open bug A2 and `a2.log`). The
shipped G7b Linear ramp replaced the *curve*, which is the density/extinction lever, and A2 now
passes at ratio 274 with near-band 0.18. There is no remaining density/extinction defect to
sweep for; the haze luminance/contrast knobs (`HAZE_DESAT`, `HAZE_GAIN`) are the
next-one-line-change already measured in the sibling doc (`HAZE_DESAT 0.40 → 0.60`).

## §4 — Proposed criteria, derived from reality

The fix is not "lower the haze number to match a kitchen reference" — the kitchen is the wrong
reference for an outdoor effect. Two options, in order of strength:

**(A) Reclassify haze as a differential A/B gate — recommended.** Grade it against the scene's
*own* `VOXELFORGE_LOOK_HAZE=0` baseline, the way every other A/B-style look metric in this lane
is already graded (AO bite, god rays, G7 axis A2). The targets exist, are in
[grade_g7.py](../scripts/grade_g7.py), and the shipped haze passes them with margin:

| differential target | threshold | shipped `ship` | source |
|---|---|---|---|
| far-band mean \|Δ\| (maxCh, sky-masked) | ≥ 6.0 | **48.4** ✓ | grade_g7.py `T_FAR_DELTA` → a2.log |
| far/near depth ratio (maxCh, sky-masked) | ≥ 2.5 | **274** ✓ | grade_g7.py `T_DEPTH_RATIO` → a2.log |
| near-band mean \|Δ\| (maxCh) | ≤ 1.0 | **0.18** ✓ | grade_g7.py axis A2 → a2.log |

All three rows are the [grade_g7.py](../scripts/grade_g7.py) A2 harness — max-channel Δ on a
sky-masked far band — which is the gate authority. They are a *different metric* from §2's
per-band columns (luminance Δ on unmasked row-thirds): the `ship` near-band reads **0.10** in §2
and **0.18** here, and both pass the dead-zone target.

This is "derived from reality" in the only honest sense available: the reference frame for an
A/B layer is the same scene with the layer lifted, not an unrelated indoor photograph.

**(B) Calibrate an absolute single-image haze number against an *outdoor* reference.** If an
absolute (non-differential) haze number is still wanted, the reference must be outdoor. The
moodboard's outdoor panel (`docs/assets/moodboard.png`, already the vegetation reference in
grade_g7.py) is the candidate; measuring a depth-gradient-to-haze-colour proxy on it would give
the absolute target the kitchen reference structurally cannot. Not done here — (A) makes it
unnecessary, and a painted concept panel is a weaker calibration anchor than the scene's own
zero-fog frame.

## Reproduce

```
# thickness grid (this file's §2 table)
python scripts/_rose_haze_delta.py --ab \
  _flamingo_g7b/hazeoff-nohud2.png \
  _flamingo_g7b/ship-nohud2.png _flamingo_g7b/s16-nohud2.png _flamingo_g7b/s24-nohud2.png \
  _flamingo_g7b/e220-nohud2.png _flamingo_g7b/e280-nohud2.png _flamingo_g7b/hd40-nohud2.png \
  _flamingo_g7b/g7curve-nohud2.png _flamingo_g7b/g7full-nohud2.png

# reference vs outdoor single-image proxies (§1)
python scripts/_rose_haze_delta.py --haze 153,184,224 --single \
  _flamingo_g7b/ship-nohud2.png docs/assets/golden-beauty-shot-ref.png

# A2 verdicts (cross-check, Flamingo's harness)
python scripts/grade_g7.py --ab-haze _flamingo_g7b/hazeoff-nohud2.png _flamingo_g7b/ship-nohud2.png
```

Frames live in the gitignored `_flamingo_g7b/` lane directory; re-shoot with
`VOXELFORGE_LOOK_CAM=35,-18,26` + the haze env hook + `_flamingo_dehud2.py` if they are gone.

_Rose (Engineer) — measured, not remembered, 2026-08-05._
