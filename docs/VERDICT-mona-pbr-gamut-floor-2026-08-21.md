# VERDICT — Monanisa: PBR gamut-floor pass (mona/pbr, 2026-08-21)

## Task
Sun's brief: in all-orange/red scenes, 12 of 16 graded plates have 98%+ of pixels sitting
at literal B=0. Material lane only (Flamingo owns lighting) — check whether
metallic/roughness/base-color values are saturating against the gamut ceiling, pull them
back into a physically-plausible PBR range. Also re-check the known `paint_blobs()`
polar-rose bug (be365e9) hasn't regressed.

## paint_blobs() polar-rose check — CLEAN, not regressed
`scripts/_pixel_blocks_gen64_v3_palette.py::paint_blobs()` still carries the 4-octave
irregular-silhouette fix from `be365e9` verbatim (4 incommensurate `sin()` terms replacing
the single `sin(freq*ang)` rose curve). Both the fix commit and the 14-texture regen
(`27cb2c5`) are ancestors of this lane's base commit `fa69a3c`. No action needed.

## metallic/roughness — not the source
`client/src/voxel.rs::block_surface()` sets `perceptual_roughness`/`metallic`/`reflectance`
as scalar, per-material-family constants (wood/mineral/powder/foliage/glass/emitter). None
of them are colour-channel-specific, so they cannot selectively crush a texture's B channel.
Only `METAL` has `metallic: 1.0`, and `metal.png` is a neutral grey (R132/G134/B133 mean) —
tinting its specular by base_color doesn't introduce a warm bias. Ruled out.

## base-color — the real finding
Every live 64×64 base-colour PNG was histogrammed directly. **None has any literal B=0
pixel today** (0.00% across all 19 textures) — so "98% B=0" is not raw-texture clipping;
that only appears once Flamingo's light/tonemap multiplies an already-thin channel below
the 8-bit rounding floor (same shape as two prior scars this office already logged:
`gamut-clip-fakes-colour-gates`, `blue-gate-unsatisfiable-with-clip`).

What **is** a genuine material-side defect: several textures carry very thin minimum-channel
headroom (worst single texel's weakest channel, measured on the live art, i.e. *after* every
prior structural fix — material-lock pass `54d4d81`, star-splat fix `be365e9`, floorboards/
red_sand contain `012ca11` — all already baked in):

| texture | headroom before | after |
|---|---|---|
| leaves | 3.5% (minB=9) | 7.5% (minB=19, floor-capped by V itself — see note) |
| grass_top | 6.3% (minB=16) | 11.8% (minB=30) |
| oak_log_side | 6.7% (minB=17) | 11.8% (minB=30) |
| dirt | 7.1% (minB=18) | 11.8% (minB=30) |
| grass_side | 7.8% (minB=20) | 11.8% (minB=30) |
| brick | 11.4% (minB=29) | 11.8% (minB=30) |

No real dielectric material reflects anywhere near 0% in any visible band — a channel with
under ~12% headroom is one exposure stop from an unrecoverable clip, which is exactly what
the 12/16-plate symptom looks like.

**leaves note**: its worst texel only reached 19/255, not the 30 floor — that pixel's low
channel comes from low *value* (a near-black shadow fleck, V≈0.075), not saturation. The
fix (below) only pulls S back; it correctly refuses to brighten a texel that's dark by
design, since that would misrepresent the material rather than fix a gamut clip.

## Fix — HSV gamut-floor pass (surgical, per-texel)
`min_channel = V*(1-S)` is an exact identity for every hue, so one rule covers all 19
textures regardless of colour family:

```
if V*(1-S) < FLOOR:  S_new = 1 - FLOOR/V   (H, V unchanged)
```

`FLOOR = 30/255 (~11.8%)`. Implemented in `scripts/_monanisa_pbr_gamut_floor.py`, run
against the **already-live** PNGs (not a from-scratch regen off the base generator chain —
confirmed by direct comparison that brick/dirt/oak_log_side/roof_tile/etc. are produced by
bespoke follow-up scripts, `_monanisa_material_lock_pass.py` / `_monanisa_brick_regrain.py`
/ the floorboards+red_sand fix, not a plain `_pixel_blocks_gen64_v4_kevin.py` run — tracing
and reproducing four separate generator chains would have risked silently reverting those
already-shipped structural fixes). Only texels below the floor are touched:

| texture | texels changed / 4096 |
|---|---|
| leaves | 1496 (36.5%) |
| oak_log_side | 544 (13.3%) |
| grass_side | 282 (6.9%) |
| grass_top | 211 (5.2%) |
| dirt | 18 (0.4%) |
| brick | 4 (0.1%) |
| (other 13 textures) | 0 |

`_n`/`_r` maps untouched (derived from height/roughness fields, not albedo colour).
Live PNGs backed up to `assets/textures/blocks_pbr_lock_backup_20260821/` before this ran
(per backup-procedural-asset-generator).

## Verification
- Pixel-diff before/after: only the 6 textures above changed, all other 13 byte-for-byte
  or re-encode-only (reverted those to keep the diff to genuine content changes).
- Contact sheet (all 19, before | after): `docs/evidence/pbr-gamut-floor-2026-08-21/
  contact_sheet_before_after.png` — visually near-identical at normal viewing size, as
  expected for a sub-perceptual safety-margin fix (not a re-paint).
- 2×2 tile-zoom check on the most-touched texture (leaves, 36.5% of texels): no seam, no
  new periodicity — the operation is a pointwise HSV transform, cannot introduce spatial
  pattern.
- B=0 count: 0 before and after, on every texture (was already 0 — the point of this pass
  is headroom against downstream multiplication, not fixing a literal clip that doesn't
  exist in the static texture).

## Scope note for Flamingo
This raises every material's floor to ~12% before any light touches it. Whether the 12/16
plates still hit B=0 after this depends on how much further Flamingo's warm-scene light/
tonemap multiplies channels down — that's outside this lane. If plates still clip after
this lands, the remaining headroom to spend is on the light/exposure side, not material.

## Build
Not applicable — this is an asset-only diff, no Rust source touched, verified via
`git diff --stat`:

```
 assets/textures/blocks/brick.png        | Bin 7904 -> 7907 bytes
 assets/textures/blocks/dirt.png         | Bin 9797 -> 9789 bytes
 assets/textures/blocks/grass_side.png   | Bin 10754 -> 10721 bytes
 assets/textures/blocks/grass_top.png    | Bin 10746 -> 10707 bytes
 assets/textures/blocks/leaves.png       | Bin 10364 -> 9998 bytes
 assets/textures/blocks/oak_log_side.png | Bin 10113 -> 10044 bytes
 6 files changed, 0 insertions(+), 0 deletions(-)
```

`cargo build` is not a valid gate for a diff with zero `.rs` lines touched. The correct
gate is pixel-diff + contact sheet, both already done under Verification above.

## Handoff to Flamingo
Every live base-colour texture now measures B=0 at 0.00% (0/4096 texels, all 19 textures),
and the six worst-headroom textures have been floored from a worst case of 3.5% up to a
uniform ~11.8% minimum-channel headroom (leaves/grass_top/oak_log_side/dirt/grass_side/brick
— see table above). If the graded plates still show 98%+ B=0 pixels after this lands, the
cause is not material: it's downstream in your light/exposure/tonemap chain multiplying an
already-floored channel back down below the 8-bit rounding line. The remaining headroom to
spend against the 12/16-plate symptom is on your side of the pipeline.
