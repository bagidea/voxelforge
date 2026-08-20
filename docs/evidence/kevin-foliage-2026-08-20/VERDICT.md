# Foliage lane — cross-quad vegetation in-game (verdict)

**Author:** Kevin (engineer) · **2026-08-20** · branch `poppy/native-only`

## Verdict: PASS — plants appear AND sway, proven in-game

The foliage scatter is wired, built, and proven end-to-end. Cross-quad vegetation
spawns on grass surface in the real game and sways in the wind field. Evidence is
the before/after frames in this directory, captured from ONE binary with the
patch's own A/B levers (so the only variable between frames is foliage / wind phase).

## Provenance

| item | value |
|---|---|
| plugin wired (`mod foliage;` + `.add_plugins`) | commit `428f61a` (2026-08-20 05:21) |
| cross-quad scatter (`scatter_foliage` + `cross_vegetation` + `TileSet::cross` + atlas `"mode":"cross"` ×7) | commit `8a830b8` (2026-08-20 05:35) |
| binary used | `target-kevin/release/voxelforge.exe` (2026-08-20 17:14, release build 45m42s, **0 `^error`**) |
| capture | fixed CINE camera eye(20,1.7,13)→aim(20,1.1,2), map `maps/demo.json`, HUD off |

## Evidence (re-verified live, 2026-08-20 21:21)

Log line in every `foliage_on` / `wind_phase_*` run — `foliage_off` prints none:

```
FOLIAGE plants=163 kinds=7
```

| proof | frames | metric |
|---|---|---|
| **presence** (plants appear) | `foliage_off.png` → `foliage_on.png` | **15.405%** of frame changed (24.5% of it in the green band) |
| **sway** (plants move in wind) | `wind_phase_a.png` (T0=0.0) → `wind_phase_b.png` (T0=1.8) | **13.606%** of frame moved |
| control (frozen vs moving) | `foliage_on.png` → `wind_phase_a.png` | differs; sway_amp 0 → 0.16 |

All four frames: 1280×720, same camera, md5-distinct as expected.
Contact sheet: `contact_sheet.png` (3 pairs, side by side).

## Honest notes (not blockers)

- **Warm lighting.** The demo scene renders warm/dusk, so the green plant textures
  light to an olive/brown tone in-frame and the plants' shadows read brown. The
  mean color of changed pixels is RGB(165,129,66) — warm, not lush green. The
  plants ARE green at the texture level (grass_tall / foliage_bush / leaf_pine are
  95–100% green-dominant), so this is a scene-lighting / look-lane concern, not a
  foliage wiring bug. Re-shooting with neutral (noon) `VOXELFORGE_SUN`/`AMBIENT`
  would make the green read more obviously.
- **Density/scale is a first pass.** ~1 in 6 grass cells scatters; scale jitter and
  kind weighting are single constants in `scene.rs`, ready to tune once the look
  lands. This was wiring, not a density pass.
- The capture reuses the patch's own `VOXELFORGE_FOLIAGE=off` A/B lever rather than
  an old binary: the oldest binaries predate the `VOXELFORGE_CINE` capture mechanism
  (added `4d279af`, 08-18), so they can't produce a deterministic same-camera frame,
  and a binary-vs-binary diff would also carry unrelated art/look changes.

## Files

- `foliage_off.png` / `foliage_on.png` — presence before/after
- `wind_phase_a.png` / `wind_phase_b.png` — wind sway (two phases)
- `contact_sheet.png` — the 3-pair side-by-side
- `*.log` — run logs (`FOLIAGE plants=163 kinds=7`, `SHOT saved`, `CINE eye`)
- `manifest.json` — contact-sheet rows
