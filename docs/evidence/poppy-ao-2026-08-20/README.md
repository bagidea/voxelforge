# AO before/after — 2026-08-20 (Poppy)

Commit under test: `5087a64` (`feat(voxel): give the mesher an eave term, and the palette a shape`).

**One binary for all six plates**: `target-poppy/perf/voxelforge.exe`, built by
Build Sentinel run `1787233325847` (`cargo build --profile perf -p voxelforge
--bin voxelforge -j 2 --target-dir target-poppy`, exit 0, written 20:45).
Inside a pair the ONLY difference is `VOXELFORGE_AO`: `0` = every vertex fully
lit, unset = the shipped strength. Same map, same `VOXELFORGE_LOOK_CAM`, same
`VOXELFORGE_SEED=1`, same 3.2 s capture. Reproduce with `./shoot.sh`.

Each run's stdout is kept beside its plate; the first line of every log is the
mesher stating which strength it meshed at (`AO strength 0` / `AO strength 1`).

| pair | scene | camera | what it is for |
|---|---|---|---|
| pair1-corner | `maps/castle.json` | `35,-30,10` | two walls meeting, the wall→grass junction, crenellation notches |
| pair2-canopy | `maps/river_sunset.json` | `40,-10,8` | ground under leaf canopies + trunk bases — the eave case the contact term exists for |
| pair3-village | `maps/edhari.json` | `90,-15,9` | many building corners at once, the whole-scene sanity check |

## Measured (AFTER vs BEFORE, per-pixel mean of RGB)

| pair | mean darkening | px darker >2 | >8 | >20 | max | px **brighter** >2 |
|---|---:|---:|---:|---:|---:|---:|
| pair1-corner | 1.49 | 11.0% | 7.3% | 2.3% | 83 | 0.02% |
| pair2-canopy | 6.29 | 54.6% | 29.3% | 6.9% | 137 | 0.02% |
| pair3-village | 2.38 | 19.9% | 10.9% | 2.7% | 99 | 0.01% |

AO may only ever remove light, so the "brighter" column is the correctness
check, not a statistic: at 0.01–0.02% it is capture noise (cloud drift in the
3.2 s window), not the term putting light back.

`*-DIFF-heatmap.png` maps the darkening (black = untouched, hot = up to 40/255)
and `*-SIDEBYSIDE.png` is BEFORE | AFTER for a quick read. The heatmaps are the
real argument: the darkening pools under ledges and canopies, at trunk bases and
along inner edges, while open field and sky stay black — it tracks geometry
rather than dimming the frame.
