# VERDICT — star-splat texture bug, fixed (2026-08-20)

Monanisa / art lane. Root cause found and fixed, 14 files regenerated and
promoted to live, contact sheet + manifest in
`docs/evidence/star-splat-fix-2026-08-20/`.

## Root cause

`paint_blobs()` in `scripts/_pixel_blocks_gen64_v3_palette.py` set each accent
blob's radius as `base_r * (1 + wobble * sin(freq*ang + phase))` — a clean
n-petal rose curve. At v3's small radius/partial blend it was invisible; at
v4's larger radius + `max_blend=1.0` (kevin's mip-surviving repaint,
`ae81561`) it renders as a visible pointed star, not an organic moss/mineral
patch. Confirmed by eye on all 14 files below plus in an isolated
old-formula-vs-new-formula render before touching any real texture.

Fix: sum 4 sine octaves at incommensurate frequencies derived from the same
`(freq, phase)` (no new RNG state, so seeded output stays reproducible and
hand-authored 6-tuple blobs elsewhere keep working). Per the
`tile-zoom-check-procedural-texture` skill's guidance: small integer-frequency
sine sums are the classic cause of visible fake-organic periodicity — the fix
follows its recommended direction (non-integer/irrational frequency ratios).

## Commits

| Commit | What |
|---|---|
| `9933662` | Safety: v4 generator writes to `scripts/_out/kevin_v4/` scratch dir, sys.exit() guard if OUT_DIR ever resolves into the live asset folder |
| `be365e9` | Root-cause fix: `paint_blobs()` multi-octave wobble, breaks the rose symmetry |
| `27cb2c5` | Regenerated + promoted the 14 buggy files |

## Files fixed (13 confirmed by earlier B>G QA + clay_plaster, confirmed by eye)

All traced to `ae81561` (the commit that landed kevin's v4 repaint).

| file | before red% | before grn% | after red% | after grn% |
|---|---:|---:|---:|---:|
| clay_plaster | 12.7% | 3.5% | 13.5% | 2.3% |
| dirt | 15.2% | 27.3% | 15.9% | 31.4% |
| floorboards | 22.2% | 21.4% | 20.1% | 19.1% |
| glass | 0.0% | 0.0% | 0.0% | 0.0% |
| lamp | 0.0% | 3.6% | 0.0% | 3.3% |
| metal | 0.0% | 0.4% | 0.0% | 0.2% |
| oak_log_side | 0.0% | 39.5% | 0.0% | 38.9% |
| oak_log_top | 0.0% | 6.7% | 0.0% | 6.3% |
| oak_planks | 0.0% | 21.2% | 0.0% | 17.8% |
| red_sand | 0.0% | 13.8% | 0.0% | 15.3% |
| sand | 0.0% | 1.0% | 0.0% | 0.4% |
| snow | 0.0% | 0.0% | 0.0% | 0.0% |
| stone_bricks | 0.0% | 26.7% | 0.0% | 28.9% |
| water | 0.0% | 0.1% | 0.0% | 0.0% |

Red/green hue-share pixel counts, HSV-thresholded (red: hue 340-360/0-10 &
sat>0.35; green: hue 70-150 & sat>0.25) — per-file, not a blanket B>G check
(that automated check is exactly what missed clay_plaster). Coverage stayed
within a few points of the buggy version on every file: this was a *shape*
fix (rosette → irregular blob), not a palette-breadth regression. The visual
confirmation (2x2-tiled zoom, all 14, before promoting) is the deciding
evidence — see the contact sheet.

glass/metal/sand/snow/water read ~0% on this metric because their accent
hues (blue-grey, olive-gold, teal) sit outside the red/green bands checked
here; their star-splat was equally visible in the 2x2-tiled zoom compare, so
they're fixed on the same evidence, just not on this particular number.

## roof_tile — NOT part of this fix, do not conflate

`roof_tile.png` currently shipping (`5ebe745`) is a smooth diagonal gradient
with **no tile groove pattern at all** — a different, older bug, unrelated to
the star-splat formula. Running the fixed generator (as a side effect of
regenerating the other 14, since `main()` loops over all 19 materials)
produces a version with correct tile grooves, but the moss/gold/red-terracotta
accents are far too dense — it reads as moldy overgrowth, not a roof. That
scratch output was **not promoted**; it's in the contact sheet as a labeled
proposal only. Needs an accent-density pass before it's shippable — CEO call
on whether to pursue now or later.

## Evidence

- `docs/evidence/star-splat-fix-2026-08-20/contact_sheet.png` — all 14
  before/after pairs (verified commit hashes, never hand-typed — resolved via
  `scripts/contact_sheet.py`) + the roof_tile status panel.
- `docs/evidence/star-splat-fix-2026-08-20/manifest.json` — the sheet's source
  manifest, for regenerating or auditing the hash resolution.

No tag/release cut. Awaiting CEO sign-off.
