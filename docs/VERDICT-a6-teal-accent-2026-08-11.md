# A6 item 2 (teal cool accent) — z-offset fix re-shot, still OPEN, real occluder found

**Yamamoto (Eng) · 2026-08-11 · re-grades `docs/VERDICT-a6-teal-accent-2026-08-10.md`
after `anim.rs` commit `4d24d30` ("fix(look): push cloak clasp z-offset outside the
cloak slab") landed on top of `90ace55`.**

## Binary used

`target/release/voxelforge.exe`, mtime **2026-08-10 04:01:29**, which is after both
`4d24d30` (2026-08-10 02:19:47) and `4bf81f1` (2026-08-10 03:37:06) — no source file
under `client/src/**/*.rs` is newer than this exe (`gate3_shoot.sh`'s own freshness
guard would have failed otherwise; ran with `GATE3_SKIP_WAIT=1` anyway per the "no
build" constraint on this pass, since Rose holds the build lock).

**Did not rebuild.** Confirmed the fix is actually IN this exe by more than the
timestamp: binary-scanned `target/release/voxelforge.exe` for the literal f32 byte
patterns of `0.1225` and `0.1375` (the commit's new housing/gem z constants) — each
appears **exactly once**, consistent with a single source-literal use each. The build
Yamamoto's 2026-08-10 doc used (mtime 01:30:39) predates `4d24d30`; this is a
**different, later** exe that does carry the fix.

## Method (same as the 2026-08-10 doc, so numbers are comparable)

- `scripts/gate3_shoot.sh` (`GATE3_SKIP_WAIT=1`, no rebuild) → 3/3 frames PASS
  (`docs/assets/gate3-a6-teal-zfix-2026-08-11/gate3-after-{boot,walk,combat}.png`).
- De-HUD: `scripts/_flamingo_dehud2.py` → `-nohud2.png` siblings.
- Camera: same solved boom as the baseline, `--cam 0,-14.32,5.289`.
- Whole-frame teal hue-band scan: new `scripts/_flamingo_clasp_hue_scan.py`, same
  window as the 2026-08-10 doc's repro (hue 150°-220°, delta>15, max>30), re-derived
  from `colorsys.rgb_to_hsv` on the literal gem `Color::srgb(0.310, 0.788, 0.839)`
  constant in `anim.rs` (still `#4FC9D6`, hue 185.8°, unchanged by the z-only fix).
- Hue delta: new `scripts/_flamingo_hue_delta.py`, importing `grade_character.py`'s
  own `analytic_bbox` + `mask_from_bbox` for the character mask (not a hand-rolled
  threshold) and a padded ring outside the bbox for background — same
  `colorsys.rgb_to_hsv` calc.

## Result: still 0 px, hue delta still not improved

```
gate3-after-boot-nohud2.png     (2560x1360, 3,481,600 px): 0 matching px
gate3-after-walk-nohud2.png     (2560x1360, 3,481,600 px): 0 matching px
gate3-after-combat-nohud2.png   (2560x1360, 3,481,600 px): 0 matching px

boot:    hero hue 28.2°  bg hue 29.4°  delta 1.1°
combat:  hero hue 27.6°  bg hue 30.1°  delta 2.5°
```

Both within the 1.1°-3.3° noise band the 2026-08-10 doc already established as "no
measured change." The `4d24d30` fix moved 0 pixels.

**Not just the aggregate stats** — projected the clasp's exact torso-local point
`(0.1785, 0.475, ~0.175)` through the same camera math `grade_character.py`'s
`analytic_bbox` uses (`EYE_HEIGHT`/`PIVOT_UP`/`FOV_Y_DEG` from that script, hip_y=0.84
+ clasp Y-offset for the height term) to predict its screen pixel for the boot frame:
**(1337, 842)** on the 2560×1360 de-HUDded frame. Cropped a tight 160×160 window
there and blew it up 5×:

- `docs/assets/gate3-a6-teal-zfix-2026-08-11/_tight-right.png`
- `docs/assets/gate3-a6-teal-zfix-2026-08-11/_tight-left.png` (mirrored side, in case
  the local-axis sign was flipped — checked both)
- `docs/assets/gate3-a6-teal-zfix-2026-08-11/_full-body-boot.png` (context crop)

Flat, single-colour cloak brown at 5× zoom. No seam, no bump, no colour break —
literally nothing rendering at the predicted location, not "rendering faint."

## Root cause: the fix cleared the cloak, not the actual occluder

The `4d24d30` fix computed clearance against the **cloak**'s own 0.05-thick slab
(front face at z=0.105) and pushed the housing/gem to 0.1225/0.1375. That math was
correct *for the cloak* — but the clasp's `PartSpec`s use `bone: BoneName::Torso` with
no `secondary`, which skins them **directly onto the torso joint**, the same joint the
**torso body mesh itself** (`anim.rs:764`, `Cuboid(0.46, 0.58, 0.28)`, skinned with
`Transform::from_xyz(0.0, neck_y*0.5, 0.0)` — zero z-offset) hangs off. That box's own
back face sits at `0.28/2 = 0.14` in the exact same torso-local frame — **0.035
further out than the cloak's 0.105**, and the clasp's XY (`0.1785, 0.475`) is inside
the torso box's own XY footprint (half-x 0.23, y-range 0-0.58) at every z. So the
character's own body is the real occluder here, not the cloak — and it was invisible
to the first fix because it isn't `cloak_anchor`, it's the torso bone the clasp
(deliberately, per the original design note) is rigidly attached to instead.

Checked against the post-fix spans:
- housing (0.1225 ± 0.0175 → [0.105, 0.14]): front face lands **exactly on** the
  torso's own 0.14 back face — flush, not clear of it.
- gem (0.1375 ± 0.01 → [0.1275, 0.1475]): only **0.0075 m** (7.5 mm) pokes past the
  torso surface — at this camera's ~5.45 m point-depth that's roughly **2 screen
  pixels**, thin enough to be lost to AA/rounding even before considering whether the
  depth buffer resolves a near-flush pair consistently frame to frame.

This is a second instance of the same failure class as the original bug (flush
clearance against the wrong/only-partial occluder), just against a different mesh.

## Patch written, NOT built (per instruction — Rose holds the build lock)

`client/src/anim.rs:679-698` (commit pending, this session): re-targets clearance at
`max(cloak front 0.105, torso back 0.14) = 0.14`, with an explicit **0.01 m margin**
(deliberately not flush, to not repeat the exact bug this is fixing):

```
housing_z = 0.14 + 0.01 + housing_half_z(0.0175) = 0.1675   (was 0.1225)
gem_z     = housing_z + 0.015 (original proud-of-housing gap) = 0.1825   (was 0.1375)
```

Gem front face lands at `0.1825 + 0.01 = 0.1925` — 0.0525 m (5.25 cm) proud of the
torso's own back surface, comfortably clear of both the torso body and the cloak.

**Not build-verified.** This pass only got as far as the pixel-projection crop
confirming the *previous* fix still renders 0 px and the source-level occluder
analysis above — it did not build+reshoot the new offsets, per this session's explicit
no-build constraint. Next step: queue a build, re-run this same doc's method (`gate3_
shoot.sh` + `_flamingo_dehud2.py` + `_flamingo_clasp_hue_scan.py` +
`_flamingo_hue_delta.py`) against the new exe.

## Status

| | status | evidence |
|---|---|---|
| A6.2 hue-separated from background | **STILL OPEN** — `4d24d30`'s z-offset fix shipped in the tested binary (byte-confirmed) but moved 0 px in 3/3 real frames; hue delta 1.1°/2.5° (no improvement, same noise band as the 2026-08-10 baseline); root cause is the torso body mesh (not the cloak) occluding the clasp at its rigid torso-bone attachment point; a second patch is written in `anim.rs` targeting the correct occluder but is unbuilt | this doc |

— Yamamoto
