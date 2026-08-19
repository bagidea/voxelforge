#!/usr/bin/env python3
"""Poppy - calibrate SKY_PAINT_GAIN against the real frame, one binary, one camera.

WHY A LADDER AND NOT ARITHMETIC.
    The painted dome is UNLIT, so its `base_color` is written into the HDR target
    verbatim - and then passes `Tonemapping::TonyMcMapface` (a 3-D LUT) and the
    whole `ColorGrading` stack. There is no closed form for "what gain lands the
    sky's median luminance at 1.80x the ground's" that this lane can write down
    honestly. So it is measured: sweep the hook, grade each rung with Flamingo's
    grader, and read the answer off the frame.

WHAT IT WATCHES BESIDES THE TARGET.
    `sky_blown_pct` is printed at every rung and it is the reason this is a ladder
    rather than a bisection on one number. A sky pushed until it clips scores a
    BIGGER `sky_L_range` and a DEAD `sky_hue_span_deg` - it would buy three of the
    four axes and quietly destroy the fourth. The rung to ship is the one that
    reaches the ratio with the hue span intact, not the one with the biggest
    numbers.

USAGE
    python scripts/_poppy_skygain_ladder_20260818.py
    python scripts/_poppy_skygain_ladder_20260818.py --gains 2.0,2.6,3.2
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "_poppy_sky", "ladder")
GRADER = os.path.join(ROOT, "scripts", "_pixel_artgap_grade.py")
REF = os.path.join(ROOT, "docs", "refs", "ceo_ref_sunset_valley.jpg")

# The pinned `beach_dusk` capture, byte-identical to
# scripts/_poppy_matmaps_ab_final.ps1 - the recipe that produced the frame every
# number in docs/art-gap-vs-ceo-ref-2026-08-18.md was measured on.
CAPTURE_ENV = {
    "VOXELFORGE_PLAY": "1",
    "VOXELFORGE_NOHUD": "1",
    "VOXELFORGE_LOOK_QUALITY": "ultra",
    "VOXELFORGE_CINE_START": "1.0",
    "VOXELFORGE_MAP_LOAD": "maps/beach_dusk.json",
    "VOXELFORGE_CINE": "41,15,37, 41,15,37, 29,3,20, 1",
}
# Anything that would make a rung incomparable to the others.
CLEAR_ENV = [
    "VOXELFORGE_ATLAS_DIR",
    "VOXELFORGE_LOOK_SUN",
    "VOXELFORGE_LOOK_LIGHT",
    "VOXELFORGE_LOOK_EXPOSURE",
    "VOXELFORGE_MAT_MAPS",
    "VOXELFORGE_SKY_DIR",
    "VOXELFORGE_LOOK_ATMOS",
    "VOXELFORGE_CLOUDS",
    "VOXELFORGE_SUN_DISC",
    "VOXELFORGE_LOOK_LIFT",
    "VOXELFORGE_SKY_PAINT",
]

TARGET_RATIO = 1.798  # the reference's own


def shoot(exe: str, gain: float) -> tuple[str, str]:
    png = os.path.join(OUT, f"gain_{gain:.2f}.png")
    log = os.path.join(OUT, f"gain_{gain:.2f}.log")
    if os.path.exists(png):
        os.remove(png)
    env = dict(os.environ)
    for k in CLEAR_ENV:
        env.pop(k, None)
    env.update(CAPTURE_ENV)
    env["VOXELFORGE_SKY_GAIN"] = f"{gain}"
    env["VOXELFORGE_SHOT"] = png
    with open(log, "w", encoding="utf-8", errors="replace") as fh:
        subprocess.run([exe, "--play"], cwd=ROOT, env=env, stdout=fh, stderr=subprocess.STDOUT)
    return png, log


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", default=os.path.join(ROOT, "target-poppy", "release", "voxelforge.exe"))
    ap.add_argument("--gains", default="1.4,1.8,2.2,2.6,3.2,4.0")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        raise SystemExit(f"REFUSED  {args.exe} not found")
    # Same string-scan gate the shoot script uses: mtime says when a file was
    # written, not what is in it.
    with open(args.exe, "rb") as fh:
        blob = fh.read()
    for marker in (b"VOXELFORGE_SKY_GAIN", b"sky-dome PAINTED"):
        if marker not in blob:
            raise SystemExit(f"REFUSED  exe does not contain {marker!r} - it predates the painted sky")
    print(f"binary gate PASS  {args.exe}")

    os.makedirs(OUT, exist_ok=True)
    gains = [float(g) for g in args.gains.split(",") if g.strip()]

    frames = []
    for g in gains:
        png, log = shoot(args.exe, g)
        ok = os.path.exists(png) and os.path.getsize(png) > 0
        painted = "sky-dome PAINTED" in open(log, encoding="utf-8", errors="replace").read()
        print(f"  shot gain={g:<5} frame={'yes' if ok else 'NO'}  painted_dome={'yes' if painted else 'NO'}")
        if ok and painted:
            frames.append((g, png))
        else:
            print(f"    dropping rung {g}: it is not a painted-sky frame")

    if not frames:
        raise SystemExit("no usable rungs - nothing to calibrate against")

    out_json = os.path.join(OUT, "ladder.json")
    subprocess.run(
        [sys.executable, GRADER, *[p for _, p in frames], "--ref", REF, "--json", out_json],
        cwd=ROOT, capture_output=True, text=True,
    )
    if not os.path.exists(out_json):
        raise SystemExit("grader produced no JSON")
    with open(out_json, encoding="utf-8") as fh:
        data = json.load(fh)
    by_file = {os.path.basename(str(f.get("file", ""))): f for f in data.get("frames", [])}

    print()
    print(f"{'gain':>6} {'sky/ground':>11} {'L_range':>9} {'void%':>8} {'hue span':>9} {'blown%':>8}")
    print("-" * 56)
    best = None
    for g, png in frames:
        m = by_file.get(os.path.basename(png))
        if not m:
            print(f"{g:>6.2f}   (not in grader output)")
            continue
        r = m.get("sky_ground_ratio")
        blown = m.get("sky_blown_pct") or 0.0
        hue = m.get("sky_hue_span_deg") or 0.0
        print(f"{g:>6.2f} {('n/a' if r is None else f'{r:.3f}'):>11} "
              f"{(m.get('sky_L_range') or 0):>9.2f} {(m.get('sky_void_pct') or 0):>8.2f} "
              f"{hue:>9.1f} {blown:>8.2f}")
        # Ship the rung closest to the reference ratio that has NOT bought it with
        # clipping (blown past twice the reference's own 5.09) and still has a hue
        # span worth calling a gradient.
        if r is not None and blown <= 10.0 and hue >= 30.0:
            d = abs(float(r) - TARGET_RATIO)
            if best is None or d < best[0]:
                best = (d, g, float(r))
    print("-" * 56)
    print(f"reference: sky/ground {TARGET_RATIO:.3f}, hue span 60.0, blown 5.09")
    if best:
        print(f"\nPICK: VOXELFORGE_SKY_GAIN={best[1]}  (ratio {best[2]:.3f}, "
              f"off target by {best[0]:.3f})")
        print("Set SKY_PAINT_GAIN in client/src/look.rs to this, or keep the env override.")
    else:
        print("\nNO RUNG QUALIFIES - every one either clipped or lost its hue span.")
        print("That is a result: the ramp cannot reach the ratio at this exposure,")
        print("and the next lever is ev100, not the sky gain.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
