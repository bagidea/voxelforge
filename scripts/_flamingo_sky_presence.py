#!/usr/bin/env python3
"""How much of the sky plate's "sky" is actually sky — and how repeatable is the shot.

`_poppy_sky_after_plate.sky_mask` calls a pixel sky when it is `R>=250 & B>=170`
in the top 45% of the frame. That predicate cannot tell a dome from a sunlit
sandstone wall, and every frame in play here stands INSIDE a house in Edhari
(spawn (32.5, 2.6, 32.5), campfire at (32.5, 1.0, 29.5)); even the `--play-demo`
walk, 10 blocks north, never leaves the room. So the question gets settled
against the renderer, not against a colour range.

`VOXELFORGE_LOOK_SKYPROBE=1` (look.rs:1988) swaps the dome material for
`base_color: BLACK, unlit: true`. The BLACK is what does the work, not the
`emissive` beside it — an unlit fragment writes base_color verbatim
(`pbr.wgsl:80-84`, the bug 5eba1e8 fixed), so emissive never lands. Every dome
pixel on screen therefore goes near-black and nothing else moves at all.

`VOXELFORGE_LOOK_SKYGAIN` is NOT usable for this: `haze_color()` folds
`sky_gain` in and is also the geometry fog colour, so it repaints the world.
`VOXELFORGE_LOOK_SKYHOR=r,g,b` is a second dome-only lever and agrees with the
probe on every frame here.

Capture (release exe — the one `prove_playable.sh` must be given via BIN):

    BIN=./target/release/voxelforge.exe bash scripts/prove_playable.sh
    BIN=./target/release/voxelforge.exe bash scripts/_poppy_sky_gain_ab.sh
    # then one probe per frame, same binary, same env, plus SKYPROBE=1:
    VOXELFORGE_LOOK_SKYPROBE=1 VOXELFORGE_MAP_LOAD=maps/edhari.json \
      VOXELFORGE_SHOT=docs/assets/_poppy_sky_probe_edhari-load.png \
      ./target/release/voxelforge.exe --play
    VOXELFORGE_LOOK_SKYPROBE=1 \
      VOXELFORGE_SHOT=docs/assets/_poppy_sky_probe_playable-walk-after.png \
      ./target/release/voxelforge.exe --play-demo

    POPPY_SKY_SET=ab    python scripts/_flamingo_sky_presence.py
    POPPY_SKY_SET=proof python scripts/_flamingo_sky_presence.py

Exit 1 if any frame's mask is mostly geometry, because a sky plate built on it
is measuring furniture.
"""
import importlib.util
import os
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
# Optional: two full prove_playable runs off the SAME binary, copied aside. Used
# only for the run-to-run noise floor in section 3. Absent is fine.
RUNS = ROOT / "_flamingo_shots"

_spec = importlib.util.spec_from_file_location(
    "after_plate", Path(__file__).with_name("_poppy_sky_after_plate.py"))
plate = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(plate)


def load(p: Path) -> np.ndarray:
    return np.asarray(Image.open(p).convert("RGB")).astype(np.int16)


def guard(b: np.ndarray, a: np.ndarray) -> float:
    """The plate's angle guard verbatim: mean |b-a| over non-sky, non-HUD."""
    off = ~plate.sky_mask(b)
    off[:plate.HUD_ROWS, :] = False
    return float(np.abs(b - a).mean(axis=2)[off].mean())


def pinned(px: np.ndarray) -> tuple:
    return (100.0 * float((px[:, 0] >= 250).mean()),
            100.0 * float((px >= 255).all(axis=1).mean()))


def main() -> int:
    print(f"POPPY_SKY_SET={os.environ.get('POPPY_SKY_SET', 'ab')}")
    bad = []

    for name, _ in plate.FRAMES:
        after = plate.AFTER_DIR / name
        before = plate.before_path(name)
        probe = plate.probe_path(name)
        print()
        print("=" * 74)
        print(f"{name}")
        print("=" * 74)
        if not probe.exists():
            print(f"  no probe frame ({probe.name}) — see the capture commands above")
            bad.append(f"{name}: no probe")
            continue

        a, p, b = load(after), load(probe), load(before)
        m = plate.sky_mask(b)
        dome = np.abs(a - p).mean(axis=2) > plate.DOME_RESPONSE_MIN
        inter, rest = m & dome, m & ~dome
        purity = inter.sum() / max(m.sum(), 1)

        print(f"  plate mask                {int(m.sum()):>6} px")
        print(f"  dome pixels in the frame  {int(dome.sum()):>6} px "
              f"({100 * dome.mean():.2f}% of frame)")
        print(f"  mask that IS dome         {int(inter.sum()):>6} px "
              f"-> purity {100 * purity:.1f}%")
        print(f"  mask that is NOT          {int(rest.sum()):>6} px "
              f"-> {100 * (1 - purity):.1f}% of the published sky is geometry")
        print(f"  angle guard before/after  {guard(b, a):.2f} levels")

        # The split that matters: geometry cannot unpin, so leaving it in the
        # mask can only ever drag the published percentage toward "unchanged".
        print()
        print("  BEFORE -> AFTER, same numbers the plate publishes, measured on:")
        for tag, sel in (("the mask as published", m),
                         ("ONLY the dome pixels", inter),
                         ("ONLY the rest", rest)):
            if not sel.sum():
                print(f"    {tag:<24} (empty)")
                continue
            pb, wb = pinned(b[sel])
            pa, wa = pinned(a[sel])
            print(f"    {tag:<24} n={int(sel.sum()):>6}  R>=250 {pb:5.1f}% -> {pa:5.1f}%"
                  f"   pure-white {wb:6.3f}% -> {wa:6.3f}%"
                  f"   mean R {b[sel][:, 0].mean():5.1f} -> {a[sel][:, 0].mean():5.1f}")

        if purity < plate.PURITY_MIN:
            bad.append(f"{name}: mask is {100 * (1 - purity):.1f}% geometry")

    print()
    print("=" * 74)
    print("CAPTURE REPEATABILITY — two runs of the same binary, nothing changed")
    print("=" * 74)
    r1, r2 = RUNS / "r1", RUNS / "r2"
    if r1.is_dir() and r2.is_dir():
        for f in sorted(r1.glob("*.png")):
            if (r2 / f.name).exists():
                print(f"  {f.name:<26} R1 vs R2 = {guard(load(f), load(r2 / f.name)):6.2f} levels")
        print("  playable-walk-after is the `--play-demo` shot: `screenshot_once` fires")
        print("  on `time.elapsed_secs() > 3.2` — wall clock — and the driven walk")
        print("  integrates real dt, so its own floor is above any 3.0 guard. The three")
        print("  `--play` shots are effectively deterministic.")
    else:
        print(f"  skipped — put two prove_playable runs in {r1} and {r2}")

    print()
    if bad:
        print("VERDICT: the sky plate cannot stand on these frames")
        for x in bad:
            print(f"  - {x}")
        return 1
    print("VERDICT: every frame's mask is really the dome")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
