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
