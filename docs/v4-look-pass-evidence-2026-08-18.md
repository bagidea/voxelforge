# v4 look pass — evidence sheet (Flamingo, 2026-08-18)

The two commits this grades — `a07348e` *look(v4): WIP-**UNPROVEN** exposure/midtone/bloom
pass* and `048fc04` *look(hero): PILE A* — say **unproven** in their own subject lines.
This file is the proving. Nothing here is an opinion about the frames; every number comes
from an instrument that was made to reproduce a published answer first.

## 0. The instrument was calibrated BEFORE anything was graded

`python scripts/_fl_v4_grade_20260818.py control` → **PASS, 27/27**, 2026-08-18 02:24.

| control | input | what it proves |
|---|---|---|
| A | `_kevin_ref_panel_mid.png` | `_kevin_art_gap_measure.measure()` reproduces all 10 published REF values in `docs/art-gap-vs-reference-2026-08-17.md` |
| B | `_fl_lookv4/outdoor-noon-after-nohud2.png` | reproduces all 10 published OURS-outdoor values — which pins the **plate file on disk** too, not just the code path |
| C | `docs/assets/golden-beauty-shot-ref.png` | `grade_axes.measure()` reproduces all 7 values of its own REF column (warmth 120.92 / blue 4.33 / clip 18.79 / sat 95.16 / DOF 3.499 / micro 5.245 / p95 165.83) |

Tolerance on every row is half the last printed digit — a correct re-run lands inside it,
a real drift does not. **If a control fails the script exits 2 and grades nothing**: the
instrument would be wrong, not the image. (This exists because on 2026-08-09 I published a
ladder that was one plate's numbers wearing the whole set's name.)

## 1. What actually changed — every lever, before → after

### 1a. `client/src/look.rs` — `LookGen::V3` → `LookGen::V4` (now the default)

| lever | v3 | **v4** | note |
|---|---|---|---|
| exposure trim (stops subtracted from `ev100`) | 0.00 | **0.55** | brighter; applied *after* `VOXELFORGE_LOOK_EXPOSURE` so a pinned capture recipe still shows the change |
| `ColorGrading.midtones.gain` | 1.00 | **1.12** | first time this section carries a gain |
| `ColorGrading.highlights.gain` | 1.06 | **1.14** | |
| `ColorGrading.shadows.gain` | 1.08 | 1.08 | unchanged |
| `ColorGrading.midtones.contrast` | 1.10 | 1.10 | unchanged — gain, not contrast, on purpose |
| bloom prefilter threshold | 0.85 | **0.72** | |
| bloom intensity | 0.20 | **0.26** | |
| bloom softness / LF boost | 0.6 / 0.35 | 0.6 / 0.35 | unchanged |
| `first_cascade_far_bound` | 10.0 | **8.0** | more shadow texels on the contact range |
| PCSS width | 12.0 | 12.0 | unchanged (the v3 ladder was fitted to this camera) |
| per-block **occlusion map** | not bound | **bound** | `voxel.rs`, gated on the same `v4()` predicate |

New sweep hooks, so every one of the above moves without a relink:
`VOXELFORGE_LOOK_GAIN=<s,m,hl>`, `VOXELFORGE_LOOK_EVTRIM=<stops>`,
`VOXELFORGE_LOOK_BLOOM=<i,t,s,lf>`.

### 1b. `client/src/hero.rs` — PILE A (the interior hero shot)

| lever | before | **after** | direction |
|---|---|---|---|
| `AMBIENT` (flat fill lux) | 2800 | **3900** | more fill |
| `AMBCOLOR` (flat fill colour) | `[0.70, 0.60, 0.44]` amber | **`[0.30, 0.45, 0.80]` sky** | the whole shadow mass changes temperature |
| `BLUESCALE` | 0.85 | **1.00** | stops removing blue (kept as a no-op const so `=0.85` reproduces the old frame) |
| `EXPOSURE` | 9.0 | **8.15** | brighter |
| `GRADE` (lift, gamma, gain) | `[0.02, 1.00, 1.30]` | **`[-0.06, 0.80, 1.16]`** | |
| `SHOULDER` | 0.64 | **0.92** | |
| `BOUNCE2` | 1.7 | **1.9** | |
| `BOUNCE2_COLOR` (back-wall rake) | `[1.0, 0.75, 0.42]` warm | **`[0.42, 0.60, 1.0]` cool** | splits the shade by direction *and* temperature |
| `RIM` (cool window-sky rake) | did not exist | **`[0.46, 0.62, 1.0, 5200]`** | new third card, aimed down/inward |
| `PANE_HI` (zenith band) | `[2.3, 1.85, 1.25]` warm | **`[1.05, 1.95, 3.6]` blue** | |
| `PANE_HI_BASE` | warm | **`[0.66, 0.79, 1.0]`** | |
| `CLEAR` | `[0.05, 0.03, 0.02]` warm | **`[0.03, 0.05, 0.10]` cool** | |
| `SUNCOLOR` / `BOUNCE1_COLOR` / `FOGCOLOR` / `PANE_LO` | — | **unchanged**, hoisted to consts only | the warm half is deliberately left alone |

## 2. Baseline — the shipped v3 set, measured by the calibrated instrument

`python scripts/_fl_v4_grade_20260818.py baseline` → `_fl_v4_20260818/baseline-v3-shipped.{md,json}`

| plate | lum mean | shadow % | midtone % | highlight % | edge | strong % | hue90° | bins/36 | sat mean | sat std | cool % | warm % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| outdoor-noon v3-after | 91.9 | 41.3 | 57.4 | 1.3 | 36.77 | 33.6 | 52 | 8 | 0.718 | 0.230 | 0.0 | 97.5 |
| outdoor-noon v2-before | 97.3 | 32.1 | 66.6 | 1.3 | 32.48 | 27.3 | 51 | 8 | 0.624 | 0.209 | 0.0 | 97.5 |
| evening-raking v3-after | 73.7 | 62.9 | 36.1 | 1.0 | 25.44 | 20.1 | 58 | 9 | 0.702 | 0.187 | 0.0 | 91.5 |
| evening-raking v2-before | 76.1 | 61.2 | 37.9 | 0.9 | 22.65 | 16.6 | 50 | 7 | 0.714 | 0.181 | 0.0 | 95.8 |
| night-firelit v3-after | 63.8 | 77.8 | 17.4 | 4.7 | 16.31 | 11.7 | 57 | 9 | 0.478 | 0.290 | 0.1 | 95.9 |
| night-firelit v2-before | 66.1 | 75.9 | 19.7 | 4.4 | 14.56 | 10.1 | 48 | 8 | 0.526 | 0.273 | 0.0 | 98.7 |
| **REF (target)** | **148.6** | **25.6** | **32.6** | **41.8** | **60.54** | **44.8** | **202** | **25** | **0.589** | **0.322** | **16.9** | **65.2** |

Read that table before reading any v4 delta: **v3 is not one gap from REF, it is three.**
Brightness (lum 92 vs 149, highlight 1.3 % vs 41.8 %), detail (edge 37 vs 61), and colour
(hue spread 52° vs 202°, cool 0.0 % vs 16.9 %). v4's levers in §1a aim at the FIRST of the
three on the play scene; Pile A (§1b) is the first attempt at the THIRD, on the hero scene
only. Nothing in either commit targets edge density, so a v4 plate that does not move
`edge` is behaving as designed, and I will not report that as a miss.

## 3. Correction to a number quoted in the shipped source

`grade::EV_TRIM_V4`'s doc comment justifies itself with

> ours (v3)   lum_mean **95.5**   shadow **35.3 %**   highlight 1.3 %

Those are measured on `docs/assets/look/outdoor-noon_after.png` — the **HUD-bearing**
capture. Re-measured just now on that exact file: lum 95.5, highlight 1.3 % — reproduced,
so the provenance is certain. The de-HUDded plate of the same frame
(`_fl_lookv4/outdoor-noon-after-nohud2.png`) measures **lum 91.9, shadow 41.3 %,
highlight 1.3 %**.

The argument is unchanged and in fact stronger on the clean frame (it is *darker* and has
*more* shadow than the constant claims), so this is a citation fix, not a re-derivation:
the doc comment should quote the `-nohud2` numbers. **Not applied yet** — editing a source
file while its build is linking is how a before/after becomes two builds. Queued for after
the plates land.

## 4. How the after-plates get shot (one binary, plus a null)

`scripts/_fl_v4_shoot_20260818.sh` — same three scenes, same cameras, same hour env as the
shipped v3 recipe (`_poppy_lookv3_shoot.cmd`), each shot twice from **one** exe:
`VOXELFORGE_LOOK_GEN=v3` (before) and `=v4` (after).

Two gates are built into it rather than trusted to me:

* **stale-binary gate** — refuses any exe older than `look.rs` / `hero.rs` / `voxel.rs`,
  and prints the exe's mtime and size on every plate. (`_poppy_shotset_before` was a binary
  hours older than the source it claimed to represent; that before/after was two different
  games.)
* **null pair** — `outdoor-noon` is shot a third time under *identical* env and *identical*
  gen. Its delta is the capture noise floor, and any v3→v4 move smaller than it is reported
  as "within noise", never as a win.

`scripts/_fl_v4_chain_20260818.sh` waits for a fresh, **size-stable** exe (two identical
readings 30 s apart — a still-linking file has a new mtime and a truncated size), then runs
shoot → de-HUD → sheets → table, printing a timestamped line every 30 s so nothing about it
is a silent wait.

## 5. Results

*(filled in when the plates land — one side-by-side PNG per scene plus the measured table;
`_fl_v4_20260818/sheet-<scene>.png`, `_fl_v4_20260818/v4-vs-v3.md`)*
