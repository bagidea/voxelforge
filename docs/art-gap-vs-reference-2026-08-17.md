# Art gap vs reference — measured 2026-08-17 (corrected)

Goal: turn "the art feels flat" into numbers we can chase. Measurement lane only —
**no build, no cargo**. All numbers are re-runnable:
`python scripts/_kevin_art_gap_measure2.py` (measure),
`python scripts/_kevin_art_gap_plot.py` (figure), and
`python scripts/_kevin_sky_verify.py` (sky* cool-share + sky-band RGB).

## ⚠️ Correction (this is a re-issue of the same-day report)

The first pass shipped two premises that are **wrong**, found by opening the reference
with eyes instead of trusting the histogram:

1. **REF is not a photo.** It is a *Minecraft + shader render*, stacked **vertically in
   3 near-identical panels** (wooden house by the sea, sunset). The old caveat "edge
   density will never equal a sensor" and target #5 "do NOT chase REF's 55 — that is
   photo noise" are both false: edge 55.7 (and 60.5 on the clearest panel) comes from
   **block texture + foliage + set dressing**, which a renderer can absolutely produce.
2. **REF is a 3-panel stack, not one frame.** Averaging three variants hides the panel
   that is actually the sharpest. This pass crops the clearest single panel and
   re-measures it (below).

Everything else in the numbers was correct and is reproduced unchanged.

## Inputs

| label | file | px |
|---|---|---|
| REF (CEO) | `uploads\1786952280558_…_n.jpg` | 768×1376 — **3-panel vertical stack** of a Minecraft+shader render |
| **REF single panel** (primary) | `_kevin_ref_panel_mid.png` | 768×455 — the middle panel, the sharpest of the three |
| OURS beauty | `_fl_beauty_20260816\beauty-wide-nohud2.png` | 1280×720 |
| OURS g4 | `_fl_g2g4_20260816\g4-only-nohud2.png` | 1280×720 |
| OURS outdoor (like-for-like) | `_fl_lookv4\outdoor-noon-after-nohud2.png` | 1280×720 — the shipped outdoor plate (the blue-sky plates live in the `_flamingo_g6`/`_g7` sky* experiments, §below) |
| OURS outdoor sky crop (secondary) | `_fl_lookv4\crop-outdoor-noon-far-terrain-sky.png` | 840×480 |

`beauty` and `g4` measure essentially identically (hue90 24° vs 24°, sat 0.863 vs 0.863,
edge 7.12 vs 7.13), so "OURS interior" below is one stable look, not a noisy measurement.

## Method (pinned definitions — unchanged)

| metric | definition |
|---|---|
| hue spread (°) | HSV hue 0–360, saturated px only (sat ≥ 0.08). Minimal circular arc holding 90% of hue mass. |
| occupied bins | 10°-bins (of 36) holding ≥ 0.5% of saturated-pixel mass. |
| saturation mean/std | over all px, 0–1. |
| luminance | Rec.601 luma 0–255. shadow < 85, midtone 85–170, highlight > 170. |
| edge density | mean Sobel gradient magnitude per px (gray float 0–255, gaussian σ=0.8) = detail per area. |
| strong edge | fraction of px with Sobel magnitude > 40. |
| dominant colors | KMeans k=8 on RGB, 20k samples, random_state=0. |
| warm / cool | warm hue ∈ [0,70) ∪ [340,360); cool ∈ [170,270); else neutral. Over saturated px. |

## REF panel structure (why we re-measured)

The reference is 768×1376 with two near-white horizontal separators (row std < 8) at
rows **459–460** and **916–917**, giving three panels of 459 / 455 / 458 px. Per-panel
Sobel edge density (the same metric as the report):

| panel | rows | height | edge mean | strong-edge % |
|---|---|---|---|---|
| top | 0–458 | 459 | 48.6 | 33.3 |
| **middle** | 461–915 | 455 | **60.5** | **44.8** |
| bottom | 918–1375 | 458 | 57.1 | 39.1 |

The **middle panel is the sharpest**, so it is the single-frame reference for the rest of
this report. Cropping it out and re-measuring shows how much the stack average hides:

| metric | REF full stack | REF single panel | Δ |
|---|---|---|---|
| edge mean | 55.7 | **60.5** | **+8.7%** |
| strong-edge % | 39.1 | **44.8** | **+14.4%** |
| hue spread 90% | 213° | 202° | −5.2% |
| occupied bins | 27 | 25 | −7.4% |
| shadow % | 23.2 | 25.6 | +10.6% |
| warm : cool | 2.5 : 1 | 3.85 : 1 | +53% |
| sat mean / std | 0.589 / 0.309 | 0.589 / 0.322 | ~0 / +4% |
| highlight % / lum | 42.4 / 151.0 | 41.8 / 148.6 | −1.4% / −1.6% |

Several metrics move **> 5%**, so per the correction rule the **single-panel value is the
primary** below; the full-stack number is kept only for provenance. The important one:
the reference's true edge density is **60.5, not 55.7** — the clearest panel is *more*
detailed than the stack average, which makes the "don't chase 55" caveat doubly wrong.

## REF vs OURS

REF column = single middle panel (primary). OURS interior = beauty / g4. OURS outdoor is
the like-for-like outdoor plate.

| metric | REF (single) | OURS interior | OURS outdoor | gap (interior → REF) |
|---|---|---|---|---|
| hue spread 90% (°) | 202 | **24 / 24** | 52 | −178° |
| occupied hue bins (/36) | 25 | **5 / 5** | 8 | −20 |
| saturation mean | 0.589 | **0.863 / 0.863** | 0.718 | +0.27 (over-saturated) |
| saturation std | 0.322 | **0.173 / 0.173** | 0.230 | −0.15 (flat) |
| shadow % | 25.6 | **60.3 / 60.3** | 41.3 | +35 |
| midtone % | 32.6 | 30.9 / 31.0 | 57.4 | −2 |
| highlight % | 41.8 | **8.7 / 8.7** | 1.3 | −33 |
| luminance mean | 148.6 | **79.5 / 79.5** | 91.9 | −69 (too dark) |
| edge density (mean) | **60.5** | **7.1 / 7.1** | **36.8** | −53 (≈8.5× lower) |
| strong-edge % | 44.8 | **4.1 / 4.2** | 33.6 | −41 |
| warm : cool ratio | 3.85 : 1 | **∞ (no cool px)** | **∞ (no cool px)** | — |
| warm / cool / neutral % | 65 / 17 / 18 | **97 / 0 / 3** | 97 / 0 / 3 | — |

### Dominant colors (k=8)

- **REF (single panel)** — balanced warm + cool, bright: `#39410c` 21% (foliage green),
  `#8c5812` 17% (wood), `#fbe68e` 16% (sunlit glow), `#f6efda` 16% (cream), `#cf943c` 9%,
  `#aab7cb` 9% (**sky blue**), `#3d7784` 7% (**sea teal**), `#10c1ce` 5% (**water cyan**).
- **OURS interior** — monochrome dark warm: `#48230c` 33%, `#5d2603` 19%, `#a53901` 17%,
  `#b54802` 15%, `#d6a65c` 6%, `#eec781` 4%, `#5c945e` 3% (the only green, dim), `#df9e25` 3%.
- **OURS outdoor** — warm/olive, no sky blue: `#8d5736` 22%, `#676c04` 18% (olive),
  `#844207` 15%, `#604c31` 15%, `#b27f5b` 14%, `#6b2300` 9%, `#2a2015` 6%, `#f1c79c` 1%.

## Scene mismatch — the indoor plates lack a sky, but outdoor cool 0% is a *grade* choice, not a renderer ceiling

The two OURS interior plates are an indoor room with an all-orange grade, so a naive read
would blame "there's no sky in a room" for the missing cool channel. **That is only half
of it.** The project has outdoor plates too, and they split into two groups.

The `_fl_lookv4` like-for-like outdoor set is warm-graded across the board:

| plate | scene | cool % | sky-region (top 10%) RGB |
|---|---|---|---|
| `outdoor-noon-after-nohud2` | outdoor, noon | **0.0** | (129, 79, 54) — warm, R>G>B |
| `outdoor-noon-before-nohud2` | outdoor, noon | 0.0 | (122, 82, 64) — warm |
| `evening-raking-after-nohud2` | outdoor, sunset | 0.0 | (109, 68, 47) — warm |
| `night-firelit-after-nohud2` | outdoor, night | 0.1 | (128, 67, 43) — warm |
| `crop-outdoor-noon-far-terrain-sky` | sky + far terrain | 0.0 | (139, 98, 77) — warm |

But the project also ships **blue-sky renders**. The `_flamingo_g6` / `_flamingo_g7` sky
experiments (our renders — the `VOXELFORGE_LOOK_SKY` grade lives in each run's `.grade.log`
sweep record, **column 2**, e.g. `skyh15.grade.log` line 1 carries
`VOXELFORGE_LOOK_SKY=0.54,0.90,1.35`; the `.out.log` file is the engine log and carries no
grade) already produce a *blue* sky at or near REF's cool share. That lever is read
**in-engine** by `client/src/look.rs:1808`
(`if let Some([r, g, b]) = env_floats::<3>("VOXELFORGE_LOOK_SKY") { h.sky = [r, g, b]; }`),
so it is a render-grade change, not a post-process filter:

| plate | cool % | sky-region (top 10%) RGB |
|---|---|---|
| `_flamingo_g6/skyh15-nohud2` | 13.4 | (116, 133, 167) — blue, B>R |
| `_flamingo_g6/skyh25-nohud2` | 19.1 | (158, 171, 193) — blue, B>R |
| `_flamingo_g7/sky2-nohud2` | 13.4 | (111, 127, 159) — blue, B>R |
| `_flamingo_g7/sky3-nohud2` | 13.6 | (123, 140, 171) — blue, B>R |
| `_flamingo_g7/sky4-nohud2` | 14.1 | (134, 150, 179) — blue, B>R |
| `_flamingo_g7/ev097sky3-nohud2` | 13.4 | (139, 154, 182) — blue, B>R |
| `_flamingo_g7/ev100sky3-nohud2` | 13.4 | (136, 151, 180) — blue, B>R |

(REF single panel = 17% cool, sky cluster `#aab7cb`.) So the correction: there **is** blue
sky in the project, and the sky* experiments already hit **13.4–19.1%** cool — the
pipeline reaches REF's cool channel *today*. The 0% is specific to the indoor plates and
to the `_fl_lookv4` outdoor set, whose sky is graded warm. The cool deficit is therefore a
**grade / lighting gap**, not a scene-selection artifact and not a renderer limitation:

- **Indoor scenes**: the cool channel must come from window light / rim / fill, *not* the
  sea. That path does not exist yet in `beauty`/`g4` — hence cool 0% there.
- **Outdoor `_fl_lookv4` scenes**: cool should come from the sky, and there the sky is
  graded warm — hence cool 0% there. The sky* experiments show this is a *grade choice*,
  not a ceiling: the same pipeline grades a sky blue and lands at 13–19%.

The one metric where scene *does* matter is **edge density**. The interior plate (7.1) is
a dark, mostly-empty room; the outdoor plate reaches **36.8** with the same renderer,
purely from terrain + vegetation + far detail. So the interior's ~8× edge gap is *partly*
"empty dark room", and the reference-level gap (36.8 → 60.5) is the part only **asset
detail** (block texture, foliage, set dressing) closes.

## Gap, in one paragraph

Our interior plate is a **97% warm, 0% cool, single-hue** image squeezed into a 24° slice
of orange/brown (REF single panel spans 202°). It is **over-saturated but flat** (mean
0.86, std 0.17), **crushed to shadow** (60% under luma 85, 8.7% highlight vs REF 41.8%),
and carries **~8× less edge detail** than the reference. The single biggest lever is still
the absence of a cool channel — but it is now clear this is a **grade/lighting problem,
not a photo-vs-render or scene-selection excuse**: the cool fill/sky/sea that REF gets
from a *shader render* is achievable in our pipeline — the `_flamingo_g6`/`_g7` sky*
experiments already land at 13–19% cool — and is simply turned **off** in the shipped
beauty/interior and `_fl_lookv4` outdoor plates. Flipping it on opens the hue range, lifts
the luminance, and breaks the muddy monotone in one move; closing the remaining edge-detail
gap is a separate, larger **asset** job.

## 5 measurable targets — split into two piles

**Pile A — fixable by grade / lighting** (no new assets; edit the beauty-shot grade
`hero.rs`/`shot_main.rs` — pixel's lane — and the shipping post stack `look.rs` — poppy's
lane):

1. **Hue spread** `hue90`: **24° → ≥ 90°** (REF single 202°); occupied bins **5 → ≥ 15**
   (REF 25). Cool fill is what opens the wheel; our own outdoor plate already reaches 52°
   on scene content alone, so 90° is a realistic first milestone.
2. **Cool presence**: cool share **0% → ≥ 12%** (REF single 17%); warm:cool **∞ → ≤ 6:1**
   (REF 3.85:1). Already proven in-project — the `_flamingo_g6`/`_g7` sky* experiments
   hit **13.4–19.1%** cool with a blue sky, so 12% is the value the sky plates already
   produce, not a stretch goal. The work is to *port* that grade onto the shipped plates:
   cool fill/rim/window light (interior) and a cool blue-sky grade (the `_fl_lookv4`
   outdoor set).
3. **Luminance balance**: highlight **8.7% → ≥ 20%** (REF 41.8%); shadow **60% → ≤ 45%**
   (REF 25.6%); mean luma **79.5 → ≥ 105** (REF 148.6).
4. **Saturation spread**: sat std **0.17 → ≥ 0.22** (REF 0.32) while holding sat mean
   **≤ 0.75** (stop over-saturating; our outdoor plate already shows 0.230 is reachable).

**Pile B — needs new assets** (this is the source of the edge detail, and it is the
**bigger job**):

5. **Edge detail**: edge mean **7.1 → ≥ 30** and strong-edge **4.1% → ≥ 25%**. These
   targets are not pulled from a photo — they are what our *own renderer already proves*
   (`outdoor-noon` hits 36.8 / 33.6% with terrain + foliage). The long-term ceiling is
   REF's single-panel **60.5 / 44.8%**, closed by **block texture detail, foliage/plants,
   and set dressing** — not by any grade knob. File lanes: block palette/texture
   (`sim/src/block.rs` — poppy, colour values monanisa's call) and level props/foliage
   (`maps/`, `scripts/gen_edhari.py` — shiba). Staffing is the Director's call; these are
   the lanes that own the assets.

> Caveats that still bound the numbers: (a) REF and OURS are both **renders** now, so
> edge density is a legitimate apples-to-apples target — but REF is 768×455 portrait and
> OURS is 1280×720; the histogram/ratio metrics are per-pixel and resolution-invariant,
> while absolute edge magnitude is content-sensitive, so always re-measure on the same
> plate. (b) The single-panel REF is one of three near-identical variants; treat its
> numbers as "a render of this scene", not a hand-picked ideal — if we re-derive a
> reference, re-crop the same middle panel so runs stay comparable.
