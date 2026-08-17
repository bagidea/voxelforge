# The 0.00% cool reading — control first, verdict second

**Date:** 2026-08-18 · **Branch:** `poppy/native-only` · **Lane:** look

The after-plates in this folder read `cool px % = 0.00` and `hue concentration
= 0.98`. The instruction was: calibrate the instrument before concluding the
render is broken. That was the right order, and it changed the answer twice.

## 1. The instrument is not broken — proven, not assumed

`scripts/_poppy_grader_control.py` runs the **same** `extras()` function that
produced the suspect numbers (imported, not reimplemented) over plates whose
answer is known before the run.

| plate | cool_all% | hue_r | expected | |
|---|---|---|---|---|
| `synth_cool` — solid hue 220° | 100.00 | 1.000 | 100% | PASS |
| `synth_warm` — solid hue 30° | 0.00 | 1.000 | 0% | PASS |
| `synth_half` — half 220 / half 30 | 50.00 | 0.087 | 50% | PASS |
| `synth_spread` — full 0–360° ramp | 27.73 | **0.000** | hue_r ≈ 0 | PASS |
| `golden-beauty-shot-ref.png` (signed) | **0.00** | **0.989** | ≤ 0.5 | PASS |
| `_kevin_ref_panel_mid.png` (outdoor ref) | 16.26 | 0.560 | ≥ 1.0 | PASS |

Exit 0. The negative control (`synth_warm`) matters as much as the positive
one — an instrument that answers 100% to everything is equally broken, and only
that row catches it. `synth_spread` clears the *other* suspect number: a full
hue ramp scores `hue_r = 0.000`, so the concentration metric is real.

**A 0.00 reading is therefore a fact about the plate.**

## 2. But 0.00% cool is not by itself a defect

The first version of this control gated the signed golden at `cool > 0` and it
**FAILED** — the approved `golden-beauty-shot-ref.png` scores cool `0.00%` and
`hue_r 0.989`, the exact signature under investigation. That gate was wrong,
not the plate: the golden is an **interior firelit kitchen with no sky in
frame**, and zero cool pixels is its correct answer. It is now kept as a
real-frame *negative* control.

Cool% is only a defect signal on a plate **that has sky in it**. The corrected
control gives an interior a ceiling and an outdoor a floor.

## 3. The real defect: this A/B is void — it never tested look-v4

`_shoot.log` pins the provenance, and it is not a look comparison at all:

| | binary | mtime | knows `VOXELFORGE_LOOK_GEN`? | gen run |
|---|---|---|---|---|
| BEFORE | `target-flamingo\release\voxelforge.exe` | **2026-08-08** 16:25 | **0 occurrences** | n/a |
| AFTER | `target\release\voxelforge.exe` | 2026-08-18 00:18 | 1 occurrence | **V3** |

Three independent confirmations that no v4 was involved:

1. `_run_after_*.log` prints `LOOK_IBL gen=V3` and `LOOK_FILL gen=V3`, with
   `ev100=10.60` — the **untrimmed** recipe value. `EV_TRIM_V4` would have
   moved it. v4 was not selected.
2. `_poppy_ab_20260818_shoot.ps1` **never sets `VOXELFORGE_LOOK_GEN`** — it
   sets PLAY / NOHUD / QUALITY / CINE / SUN / LIGHT / EXPOSURE / SHOT and
   nothing else. The generation lever was never touched, so AFTER took the
   default, which in that binary was V3.
3. The BEFORE binary contains **zero** occurrences of the string
   `VOXELFORGE_LOOK_GEN` — it predates the generation switch entirely.

So the two plates straddle **ten days of unrelated work across every lane**
(authored PBR maps, atlas re-authoring, map regeneration, hero/enemy work), not
a look generation. The dark sky and the orange-sepia cast in
`after_outdoor-noon.png` are real and visible, but nothing here attributes them
to v4.

> **Follow-up, and a correction:** the sky *was* attributable, by a lever that
> was already in the same binary. `SKY-FINDINGS.md` has the three-arm
> `LOOK_ATMOS`/`LOOK_SKYGRAD` control. It lands the opposite of the guess the
> README first published: the dome is suppressed **by design**, it renders fine
> when re-enabled, and the darkness is this lane's grade. Note also that the
> atlas re-authoring listed above never reached a pixel — `BLOCK_ART tile_px=64
> != 16` in every run log in this folder.

`target-poppy/release/voxelforge_shot.exe` (71.6 MB, 01:53) did **not** shoot
these plates either: it contains 0 occurrences of `LOOK_IBL`/`LOOK_FILL`, while
`occlusion` hits 167× in the same scan, so the scan works and the strings are
genuinely absent.

## 4. What is actually needed

A one-binary A/B from a binary that **contains** v4, with the lever set
explicitly — `VOXELFORGE_LOOK_GEN=v3` vs `=v4`, never unset and never `0`/`1`.
Everything else (camera, sun, light, exposure) stays verbatim from
`_poppy_pbr_shoot.ps1` so the pair remains like-for-like.

## 5. Reproduce

```
python scripts/_poppy_grader_control.py          # exit 0 = instrument trusted
python scripts/_poppy_ab_20260818_measure.py REF=_kevin_ref_panel_mid.png ...
```

Run the control **first**, every time. If it exits non-zero, no cool%/hue
number below it means anything.
