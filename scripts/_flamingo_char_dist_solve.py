#!/usr/bin/env python3
"""Is the boom actually AT the length the shot asked for? One measurement, no loop.

WHY. `grade_character.py` builds its mask from an ANALYTIC bbox — it projects the
body cylinder through (yaw, pitch, dist) and trusts `dist`. In `--play` that trust
is misplaced: `camera_boom` re-derives `dist` every frame from `want_dist` MINUS
whatever the collision sweep hits, so a boom asked for 5 m behind a body standing
in a narrow alley renders at a fraction of that. The first char grade was shot
that way — bbox computed at 5.0 m, body in the frame far larger — so the mask
sampled a slice of the character plus a lot of wall, and every number under it was
a statement about the wrong pixels.

WHAT IS MEASURED. The projection ties the body's on-screen half-width to depth by
one equation:

    x_half_px = HALF_W * f_px / (dist + dy_centre * sin(pitch))

so ONE masked frame gives one implied dist. This script measures the half-width in
the bbox built at the REQUESTED dist and reports:

    predicted (what the request implies) vs measured (what rendered)

A ratio near 1.00 means the boom got its full length and `--cam` is the truth, so
the grade that follows is sampling the character. A ratio well above 1.00 means
the body renders bigger than the request can explain — the boom was shortened, and
the frame must be re-shot from a pose with clearance (with the `place_player` fix
in scene.rs, PITCH is finally honoured, so tilting the boom up over the walls is
now a real option) rather than graded as-is.

DELIBERATELY NOT AN ITERATIVE SOLVE. Feeding the implied dist back in grows the
bbox, which lets the per-row background model swallow wall, which grows the mask,
which shrinks the implied dist — it runs away instead of converging. Measured:
5.00 -> 3.89 -> 3.50 -> ... -> 1.68 with no fixed point in sight. One honest
measurement with a stated assumption beats eight that feed on themselves.

Usage:
  _flamingo_char_dist_solve.py --cam 0,-14.32,5.0 [--actor player] <frame>-nohud2.png
Prints RATIO=<x> and IMPLIED_DIST=<m>; exits 0 if the boom is clear, 2 if not.
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import grade_character as gc  # noqa: E402

CLEAR_TOL = 0.12          # |ratio - 1| within this = the boom got its length


def measure(png, yaw, pitch, want, actor, hud_crop, key_tol):
    rgb = np.asarray(Image.open(png).convert("RGB")).astype(np.float32)
    h, w, _ = rgb.shape
    full_h = h + hud_crop
    f_px = (full_h * 0.5) / np.tan(np.radians(gc.FOV_Y_DEG) * 0.5)
    half_w = gc.ACTOR_HALF_W.get(actor, 0.50)
    hgt = gc.ACTOR_HEIGHT.get(actor, gc.PLAYER_HEIGHT)
    dy_c = hgt * 0.5 - (gc.EYE_HEIGHT + gc.PIVOT_UP)   # body mid-height vs pivot
    sp = np.sin(np.radians(pitch))

    predicted = half_w * f_px / (want + dy_c * sp)

    bbox, foot = gc.analytic_bbox(w, h, yaw, pitch, want, actor, hud_crop)
    mask = gc.mask_from_bbox(rgb, bbox, foot, key_tol, True)
    if mask.sum() < 200:
        return predicted, None, None, 0
    xs = np.where(mask)[1]
    # 5-95 percentile, not min/max: one stray masked wall pixel must not set the
    # body's width.
    measured = float(np.percentile(xs, 95) - np.percentile(xs, 5)) * 0.5
    implied = half_w * f_px / max(measured, 1e-3) - dy_c * sp
    return predicted, measured, implied, int(mask.sum())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("--cam", required=True, help="YAW,PITCH,DIST as requested")
    ap.add_argument("--actor", default="player", choices=["player", "husk"])
    ap.add_argument("--hud-crop", type=int, default=80)
    ap.add_argument("--key-tol", type=float, default=26.0)
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    yaw, pitch, want = [float(v) for v in args.cam.split(",")]
    pred, meas, implied, npx = measure(args.image, yaw, pitch, want, args.actor,
                                       args.hud_crop, args.key_tol)
    if meas is None:
        print(f"# {os.path.basename(args.image)}  cam={args.cam}  MASK EMPTY")
        print("RATIO=nan")
        print("IMPLIED_DIST=nan")
        sys.exit(2)

    ratio = meas / pred
    clear = abs(ratio - 1.0) <= CLEAR_TOL
    if not args.quiet:
        print(f"# {os.path.basename(args.image)}  cam={args.cam}  mask {npx} px")
        print(f"   body half-width  predicted {pred:6.1f} px   measured {meas:6.1f} px"
              f"   ratio {ratio:4.2f}")
        print(f"   => implied boom {implied:.2f} m vs requested {want:.2f} m   "
              f"[{'CLEAR — cam is the truth' if clear else 'SHORTENED — do not grade this frame'}]")
    print(f"RATIO={ratio:.3f}")
    print(f"IMPLIED_DIST={implied:.3f}")
    sys.exit(0 if clear else 2)


if __name__ == "__main__":
    main()
