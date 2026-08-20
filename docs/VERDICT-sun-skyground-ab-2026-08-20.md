# VERDICT — Sun: sky-brighter-than-ground A/B (2026-08-20)

> ⚠️ **Plate provenance**: this plate set was captured before commit `27cb2c5` (star-splat regen), so the textures still carry the polar-rose pattern — do NOT use these frames to judge image quality; valid only for the sky/ground light-ratio measurement.

## Task
"ฟ้าสว่างกว่าพื้น 47%" (sky_ground_ratio stuck below target) must move toward the golden
ref ratio **1.80**. The sky dome is saturated (unlit, writes base_color verbatim into HDR,
never passes Exposure — look.rs:4178-4193), so the sky lever CANNOT raise the ratio. The
correct path = **darken the lit ground** (raise ev100, or lower ambient/fill), same camera,
same hour.

## Control — golden ref (run BEFORE any fix; exit 0 = metric pipeline sane)
`docs/refs/ceo_ref_sunset_valley.jpg` — measure exit **0**

| axis | value |
|---|---|
| sky_ground_ratio | **1.798**  (target 1.80) |
| sky_median_L    | 149.66 |
| ground_median_L | 83.24  |
| sky_L_mean      | 153.7  |
| sky_frac_pct    | 21.85  |
| sky_void_pct    | 0.0    |
| sky_blown_pct   | 5.09   |

## Before (current shipped look, no fix)
- exe: `_sun_before_voxelforge.exe`  (safety copy of clean HEAD `target-sun/perf/voxelforge.exe`, 102 758 400 bytes)
- clean HEAD before (authoritative for A/B): `target-sun/perf/voxelforge.exe` — build done, BUILD_EXIT=0, 0 error lines, HEAD `18ef061`
- frame: `_sun_skyground/before.png`
- axes:

| axis | value |
|---|---|
| sky_ground_ratio | **0.789** |
| sky_median_L / ground_median_L | 74.98 / 95.01 |
| sky_L_mean      | 75.58  |
| sky_frac_pct    | 17.82  |
| sky_void_pct    | 0.0    |
| sky_blown_pct   | 0.0    |

## Fix (candidate — ground-darkening, sky untouched)
- lever: `VOXELFORGE_LOOK_EXPOSURE` = `11.8`
- mechanism: unlit dome exposure-invariant → raising ev100 darkens lit ground only → raises ratio toward 1.80

## After (fix applied)
- exe: `target-sun/perf/voxelforge.exe`  (same binary, same HEAD `18ef061` — lever is env-only, no source change)
- frame: `_sun_skyground/ev11.8.png`
- axes:

| axis | value |
|---|---|
| sky_ground_ratio | **0.907** |
| sky_median_L / ground_median_L | 75.05 / 82.73 |
| sky_L_mean      | 75.26  |
| sky_frac_pct    | 18.08  |
| sky_void_pct    | 0.0    |
| sky_blown_pct   | 0.0    |

## Sweep (all 6 arms, one binary, env-lever only)
| arm | sky_ground_ratio | sky_median_L | ground_median_L | sky_frac_pct | void | blown |
|---|---|---|---|---|---|---|
| before (defaults) | 0.789 | 74.98 | 95.01 | 17.82 | 0.0 | 0.0 |
| ev11.2 | 0.868 | 75.05 | 86.44 | 18.00 | 0.0 | 0.0 |
| **ev11.8** | **0.907** | 75.05 | 82.73 | 18.08 | 0.0 | 0.0 |
| amb420 | 0.797 | 74.98 | 94.09 | 17.81 | 0.0 | 0.0 |
| amb300 | 0.802 | 74.98 | 93.50 | 17.81 | 0.0 | 0.0 |
| fill800 | 0.797 | 74.98 | 94.06 | 17.82 | 0.0 | 0.0 |

## Pair evidence
- contact sheet: `_sun_skyground/contact_sheet.png`  (assembled via `scripts/contact_sheet.py`, both sides verified against HEAD `18ef061`)
- % sky-ground difference: **+14.96%**  (0.789 → 0.907)

## Verdict
- status: **NEGATIVE — target 1.80 NOT reached**
- notes:
  - Build green (BUILD_EXIT=0, 0 `^error` lines); control re-measured this run = **1.798** (metric pipeline sane).
  - Mechanism CONFIRMED directionally: every ground-darkening lever raises the ratio; strongest = ev11.8 → 0.907 (+14.96%). sky_void 0, sky_blown 0 (saturated dome never blows).
  - **Baseline mismatch**: `before.png` = 0.789 (sky DARKER than ground, 74.98 vs 95.01) — NOT the ~1.47 "sky brighter" the plan assumed. On beach_dusk vista the unlit dome reads darker than the sunlit ground, so the A/B starts on the wrong side of 1.0.
  - **Lever range insufficient**: even ev11.8 lands at 0.907, ~50% short of 1.80. Need either a much stronger ground-darkening lever (ev100 well past 11.8), or a scene where the sky is genuinely the bright element before tuning.
  - **Metric (exposure-invariant re-measure)**: sky_frac_pct now holds 17.82→18.08 across the sweep — this re-measure uses the contrast-normalised mask in `scripts/_sun_skyground_measure.py`. The earlier leak (19.90→34.12, exposure-sensitive `sky_mask`) is recorded under Superseded below.

## Superseded — leaked-mask numbers (do not cite)

The numbers below came from the OLD sky mask (`_pixel_artgap_grade.sky_mask`), which
thresholds the absolute Sobel magnitude of luma — exposure-sensitive. Dark frames (ev11.8)
had weaker edges, so the flood-from-top leaked past the horizon and inflated sky_frac
(19.90 → 34.12), dragging sky_ground_ratio with it. Replaced by the exposure-invariant mask
(contrast-normalised Sobel, `scripts/_sun_skyground_measure.py`), which holds sky_frac
stable (17.82 → 18.08). Do not cite these numbers.

Before: ratio 0.792 · sky/ground 75.77/95.62 · sky_L_mean 76.31 · sky_frac 19.90
After (ev11.8): ratio 0.981 · sky/ground 78.83/80.33 · sky_L_mean 81.7 · sky_frac 34.12
% sky-ground difference (leaked): +23.86% (0.792 → 0.981)

| arm | sky_ground_ratio | sky_median_L | ground_median_L | sky_frac_pct | void | blown |
|---|---|---|---|---|---|---|
| before (defaults) | 0.792 | 75.77 | 95.62 | 19.90 | 0.0 | 0.0 |
| ev11.2 | 0.874 | 75.98 | 86.98 | 22.37 | 0.0 | 0.0 |
| **ev11.8** | **0.981** | 78.83 | 80.33 | 34.12 | 0.0 | 0.0 |
| amb420 | 0.798 | 75.67 | 94.82 | 20.23 | 0.0 | 0.0 |
| amb300 | 0.804 | 75.69 | 94.16 | 20.00 | 0.0 | 0.0 |
| fill800 | 0.800 | 75.77 | 94.74 | 20.00 | 0.0 | 0.0 |
