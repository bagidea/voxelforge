# Sky-plate audit evidence — 2026-08-14

The frames `docs/flamingo-sky-plate-audit-2026-08-14.md` §1–§3 are computed
from. Committed on purpose: the first copy of this set lived in
`_flamingo_shots/`, which `.gitignore:166` (`_*/`) swallows whole, so the
document quoted numbers that could not be re-run from a clean checkout —
`_flamingo_sky_presence.py` printed `skipped` and exited 0 anyway. It now fails
if any of this is missing.

    POPPY_SKY_SET=proof python scripts/_flamingo_sky_presence.py
    POPPY_SKY_SET=ab    python scripts/_flamingo_sky_presence.py

Everything was shot with `target/release/voxelforge.exe` (2026-08-14 06:15)
except `dbg/`, which is the point of `dbg/`.

| dir | what | how it was made |
|---|---|---|
| `r1/`, `r2/` | two full runs, nothing changed between them — the run-to-run floor every lever reading in §3 is judged against | `BIN=./target/release/voxelforge.exe bash scripts/prove_playable.sh` ×2 |
| `s0618/` | the 06:18 frames **as published**, frozen. `docs/assets/*.png` gets overwritten by the next capture; this copy is what "06:18 vs R1" in §1 actually compares | copied from `docs/assets/` at audit time, md5-identical |
| `dbg/` | the same `edhari-load` shot off the stale 08-11 **debug** exe — the negative control that closes "which binary shot the frames" | `VOXELFORGE_MAP_LOAD=maps/edhari.json ./target/debug/voxelforge.exe --play` |
| `dbg/edhari-load-restamp.png` | that control re-shot with the exe's identity stamped into `edhari-restamp.shotlog.txt` (`142950400 bytes 2026-08-11 01:05:06`). It reproduces the control frame to 0.39 levels, so the control is provably off the debug exe rather than off someone's word | same command, stamped |
| `probe/*-skyhor.png` | `VOXELFORGE_LOOK_SKYHOR=1,0,1` paints the dome magenta. Counted in a single frame, so a moved camera cannot fake it | `VOXELFORGE_LOOK_SKYHOR=1,0,1 VOXELFORGE_SHOT=… <exe> --play[-demo]` |
| `probe/ab-skyhor.png` | the magenta lever's **positive control**: on the fog-off A/B camera it must find the 8,711 px `SKYPROBE` finds. It lands 8,712 | ab env (`FOG=100000,200000`, `VFOG=off`) + `SKYHOR=1,0,1` |
| `probe/ab-fixed-rerun.png` | a same-camera repeat of the A/B frame with **no lever** — that frame's own floor, 0.03 levels | ab env, nothing else |

The `.shotlog.txt` files are the run logs (named to dodge `*.log` in
`.gitignore`); each one carries its `SCENE_READY` line and the shot path.
