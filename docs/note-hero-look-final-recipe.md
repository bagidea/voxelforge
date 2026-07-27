# Hero look-final — recipe & status (Poppy, 2026-07-26)

**Deliverable:** `hero-look-final.png` (= `charm2-C.png`). Compare: `hero-look-compare.png`
(GOLDEN | BEFORE converged-final | AFTER).

## What changed vs the signed-off `hero-converged-final.png`
Fixed the **monochrome-collapse** charm miss (Flamingo note #9 / "ส้มจัดแบนทั้งเฟรม")
WITHOUT touching geometry, materials, or the window (identity = ห้ามแตะ). Pure light-tune:
widen the **tonal range** so sunlit gold punches while the warm shadow floor holds — the
opposite of a flat global desaturate. Golden's sunlit patch is R-B **+133**; we went from
a flat **+50** (converged) to **+118** — matching golden's character. Shadow floor stayed
warm & above gate (p05-L 9.9%, exactly golden's).

## Exact env recipe (main `voxelforge.exe`, VOXELFORGE_HERO=1)
```
VOXELFORGE_CAM=7.6,5.9,-5.2,7.6,3.2,6.0,52
VOXELFORGE_FOG=0.032  VOXELFORGE_DFOG=0.008  VOXELFORGE_DOF=11,3.2
VOXELFORGE_SUN=18,196,22000   # punchier golden key (was 20,195,12000)
VOXELFORGE_AMBIENT=2400        # trimmed flat wash for tonal range (was 2900)
VOXELFORGE_EXPOSURE=9.9        # (was 9.7)
```
Render script: `scripts/render_charm2.sh` (variant C).

## Gate readout (all frames PASS — numeric objective met)
| frame            | G3 p05-L | sunlit R-B | G3 | G5 | G6 |
|------------------|----------|------------|----|----|----|
| golden ref       | 9.9%     | +133       | P  | P  | P  |
| converged-final  | 11.5%    | +50 (flat) | P  | P  | P  |
| **hero-look-final (C)** | **9.9%** | **+118** | **P** | **P** | **P** |

## To BAKE as the new default (needs sign-off — changes what ships)
If approved, fold the SUN/AMBIENT/EXPOSURE values above into the baked defaults in
`src/hero.rs` (lines ~219 sun illum, ~325 ambient brightness, ~290 exposure) so
`VOXELFORGE_HERO=1` reproduces this out-of-box with zero env crutch. NOT done unilaterally:
it changes the reviewer-signed converged frame.

## Remaining charm gaps (Flamingo note — geometry, out of scope for this env pass)
Composition (bowl reads as counter-block → wants ceramic rim-lip + tapered foot, smaller
scale), DOF depth separation, god-ray hard-wedge→soft + dust motes, fridge specular streak.
All need a `hero.rs` geometry/material recompile and risk regressing the signed frame —
recommend a separate approved pass, not bundled here.

---

# WIDE establishing hero — recipe & status (Flamingo, 2026-07-26)

**Deliverable:** `docs/assets/wide-hero-final.png` (tracked; repo `.gitignore` drops root
`*.png`, so the canonical wide lives under `docs/assets/`). Driver: `scripts/render_wide_hero.sh`.
Compare: `wide-final-compare.png` (GOLDEN REF | BEFORE wide-A | AFTER).

## What changed vs the in-progress `wide-A/B/C.png`
The wide-A/B/C candidates passed the G3/G5/G6 gate but were **AMBIENT-DOMINATED**:
`render_wide_shot.sh` cranked `VOXELFORGE_AMBIENT=4500` and left the sun at its `12000`
default → huge floor/wall areas filled flat → **fire-orange, low saturation, high blue**
(1/6 P0 axes). Fix = the SAME key-light-dominant balance the tight hero uses (raise the
sun, cut the flat wash), ported onto the wide-A angle, + **deep DOF** so the establishing
floor stays crisp voxel geometry (G1). Env-only, no `hero.rs` touch, no geometry change.

## Exact env recipe (`voxelforge_shot.exe`; the shot binary always renders setup_hero)
```
VOXELFORGE_WIDE=1
VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58   # wide-A: 3/4 toward the window (ref angle)
VOXELFORGE_DOF=8,10                          # DEEP focus → floor tiles stay hard-edge (G1)
VOXELFORGE_SUN=19,196,20000                  # key up (was 12000 default) — golden-hour direction
VOXELFORGE_AMBIENT=4400                       # trimmed the flat fill (was 4500 ambient-dominant)
VOXELFORGE_BLUESCALE=0.57                     # pull the residual blue leg down toward ref B≈4
VOXELFORGE_EXPOSURE=8.82                      # seats window p95 in the 150..185 band
VOXELFORGE_GRADE=0.02,1.00,1.15              # temp,sat,contrast — key-lit frame carries the punch
VOXELFORGE_AMBCOLOR=0.78,0.63,0.28           # amber fill; G leg up lifts the shadow floor (G3)
```

## Gate + P0-axis readout (measured, `scripts/grade_gate.py` + `scripts/grade_axes.py`)
| check | value | target | verdict |
|---|---|---|---|
| **G3** p05-L | 8.2% (darkest shade R-B +73, warm) | ≥ 8% | **PASS** |
| **G5** window | brightest (248,234,210) min(G,B)=210 · 3-pt spread 78 | ≤245 + spread ≥8 | **PASS** |
| **G6** sunlit wood | (230,209,178) R-B +52 | R>G>B, R-B 40..210 | **PASS** |
| G1 voxel edge / G2 key dir + window pool / G4 soft-shadow+AO | visual | — | **PASS** (see compare) |
| P0 warmth R-B (mid) | 165.0 | ≥110 | PASS |
| P0 blue B (mid) | 1.3 | ≤10 | PASS |
| P0 saturation (mid) | 99.3 | ≥90 | PASS |
| P0 micro-contrast | 5.61 | ≥5 | PASS |
| P0 highlight p95 | 181.5 | 150..185 | PASS |
| P0 DOF fg:bg | 0.95 | ≥3.0 | **FAIL — intrinsic to a wide** |

**On the DOF axis:** `grade_axes.py` measures fg:bg sharpness with fixed boxes tuned for the
TIGHT hero (sharp bowl foreground / bokeh background). A wide establishing frame with deep
focus has fg≈bg sharpness **by design** — the axis is unreachable without a shallow-DOF,
textured-foreground (tight-hero) composition. The grader's own header note flags this. Not a
defect of the wide; the frame passes the actual **G1–G6 gate**, which is what the wide is graded on.

## Geometry pass — DONE (Flamingo, 2026-07-27, CEO-approved WIDE-A locked)
The geometry/silhouette gap the env-only pass parked is now closed, all env-gated under
`VOXELFORGE_WIDE` (narrow hero byte-identical — `wa/wb` wall selector = the shipped pair when
`!wide`; new plank/table/glass materials only spawn in the wide block):

1. **Walls de-checkered** → smooth honey plaster (`wall_a_w/wall_b_w`, ~0.006 spread). The loud
   orange/yellow chessboard was the #1 anti-AAA tell; smoothing it is the biggest single win.
2. **Floor de-checkered** → warm honey-walnut parquet (`plank_h/m/d/seam`, G leg lifted off red,
   boards in Z with staggered joints + interleaved per-board tone).
3. **Tabletop deepened** (`table_h/table_d`, clear 2-tone) so the cream bowl separates; muted moss
   block → **teal glass tumbler** (`glass_teal`, faint emissive) beside the bowl = the ref accent.

### Updated gate + axis readout (geometry-pass frame, `scripts/grade_gate.py` + `grade_axes.py`)
| check | value | target | verdict |
|---|---|---|---|
| **G3 / G5 / G6** | p05-L 8.4% warm · window min(G,B)=210 spread 78 · sunlit R-B +54 | gate | **ALL PASS** |
| P0 warmth R-B | 162 | ≥110 | PASS |
| P0 blue B | 1.1 | ≤10 | PASS |
| P0 saturation | 99 | ≥90 | PASS |
| P0 highlight p95 | 182 | 150..185 | PASS |
| P0 micro-contrast | 4.9 | ≥5 | **near-miss — accepted tradeoff of de-checkering the walls** |
| P0 DOF fg:bg | 0.28 | ≥3.0 | **FAIL — intrinsic to a wide** |

**The two failing axes are both documented, accepted tradeoffs** (see
`golden-beauty-shot.md` §GOLDEN establishing hero): DOF is unreachable for a deep-focus wide, and
micro-contrast is the ~2% cost of choosing smooth honey walls over loud voxel checker — which the
rubric's #1 go/no-go ("Minecraft-no-shader = FAIL") demands. The frame passes the actual **G1–G6
gate**, which is what the wide is graded on.

---

# Hero tilt-B — key-lit, gate PASS 2026-07-27

**Deliverable:** `docs/assets/wide-tiltB.png` (1280×720). The promoted **new hero** — the
straight-on **tilt-down** wide establishing shot. tilt-B passed the gate and is the hero going
forward. Rendered through the **shot binary** `voxelforge_shot.exe` (env-only; `hero.rs` framing
untouched — the camera/look is driven entirely by the env below).

## What this is
The **wide-B** camera (straight-on, looking down at the scene) wearing the **key-lit recipe** —
the SAME key-light-dominant balance as `render_wide_hero.sh`'s wide-A frame, but on the wide-B
angle. The earlier AMBER recipe (`render_wide_finals.sh`, `AMBCOLOR=0.784,0.630,0.200`) crashed
**G3 to 7.8%** (< 8) on the current geometry (the de-checkered walls lowered the shade floor), so
the key-lit recipe was swapped in on the wide-B angle to hold the shadow floor while keeping the
key punch. Base recipe = `scripts/render_wide_hero.sh`; only the camera was changed (wide-A → wide-B).

## Exact env recipe (`target/release/voxelforge_shot.exe`; shot binary always renders `setup_hero`)
```
VOXELFORGE_WIDE=1
VOXELFORGE_CAM=7.6,6.4,-6.0,7.6,2.7,8.0,60   # wide-B: straight-on, TILT-DOWN (eye y6.4 -> target y2.7, dy 3.7), fov 60
VOXELFORGE_DOF=8,10                          # DEEP focus -> floor stays hard-edge voxel geometry (G1)
VOXELFORGE_SUN=19,196,20000                  # elevation,azimuth,illuminance — golden-hour key
VOXELFORGE_AMBIENT=4400                      # trimmed flat fill (was 4500 ambient-dominant)
VOXELFORGE_BLUESCALE=0.57                    # pull residual blue leg down toward ref B~4
VOXELFORGE_EXPOSURE=8.82                     # seats window p95 in the 150..185 band
VOXELFORGE_GRADE=0.02,1.00,1.15              # temperature, saturation, contrast
VOXELFORGE_AMBCOLOR=0.78,0.63,0.28           # amber fill; G leg up lifts the shadow floor (G3)
```

### One-liner to reproduce (from repo root)
```bash
env VOXELFORGE_WIDE=1 \
  VOXELFORGE_CAM=7.6,6.4,-6.0,7.6,2.7,8.0,60 \
  VOXELFORGE_DOF=8,10 \
  VOXELFORGE_SUN=19,196,20000 VOXELFORGE_AMBIENT=4400 \
  VOXELFORGE_BLUESCALE=0.57 VOXELFORGE_EXPOSURE=8.82 \
  VOXELFORGE_GRADE=0.02,1.00,1.15 VOXELFORGE_AMBCOLOR=0.78,0.63,0.28 \
  VOXELFORGE_SHOT=docs/assets/wide-tiltB.png \
  target/release/voxelforge_shot.exe
```
Build the shot binary first if needed: `cargo build --release --bin voxelforge_shot`.
The binary self-exits ~4.4 s after grab (screenshot at t>3.2 s, after TAA/PCSS/SSAO accumulate).

## Gate readout (measured on `wide-tiltB.png` by `scripts/grade_gate.py`, 2026-07-27 — NOT copied from wide-A)
| check | value | target | verdict |
|---|---|---|---|
| **G3** interior p05-L | 8.4% · darkest shade @(682,142) RGB=(78,0,0) R-B **+78** warm | ≥ 8% | **PASS** |
| **G5** window | brightest (248,234,210) **min(G,B)=210** · 3-pt spread 74 | ≤245 + spread ≥8 | **PASS** |
| **G6** sunlit wood | (241,220,185) R-B **+56** L=87.1 | R>G>B, R-B 40..210 | **PASS** |

**Result: G3=P · G5=P · G6=P → all measurable gates PASS.** (G1 voxel-edge / G2 window-bars-on-floor /
G4 soft-shadow+AO are the visual gates — see the frame.) These numbers were re-measured directly on the
deliverable; they differ slightly from the wide-A readout above (wide-A: spread 78, sunlit R-B +54),
confirming tilt-B is a separately-measured passing frame, not a copy.
