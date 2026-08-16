#!/usr/bin/env python3
"""A1 dome close-out grader — does the dome reach the fragment shader?

Four plates, ONE binary, one env lever apart each:

  d0_atmos-on     probe set, atmosphere ON   -> dome suppressed by design
  d1_atmos-off    probe set, atmosphere off  -> dome material forced flat
  d2_gradient     no probe,  atmosphere off  -> the real gradient dome   (AFTER)
  d3_skygrad-off  SKYGRAD=off, atmosphere off -> flat ClearColor sky      (BEFORE)

THE BAND IS THE TOP 90 ROWS, NOT THE TOP 40%. The 40% band this scene's earlier
grader used runs past the skyline into the hazed distance, where terrain, not
sky, sets the numbers -- the dome's whole contribution washes out to ~1% and the
frame scores "no change" while the dome is plainly drawing. The top 90 rows are
the band that is unbroken sky in the flat-sky control, so it is the band where
"is this the dome or the ClearColor" is actually answerable.

THE PROBE READS BLACK, NOT GREEN. `sky_dome`'s probe material sets
`emissive: rgb(0,8,0)` WITH `unlit: true`, and Bevy's unlit path returns
`base_color` and never adds emissive (look.rs's own comment at the top of
`build_sky_dome_mesh` transcribes that exact shader line). So the probe paints
the dome's `base_color`, which is `Color::BLACK`. That is still a total
falsification -- nothing else in this scene can black out the sky -- but the
`REVERT before shipping` block's promise that "the frame MUST go green" is
wrong, and a later reader must not score a black frame as "dome still dead".
Gate D below is what keeps that honest: it demands the probe frame still have
LIT GEOMETRY in it, so "black sky" can never be confused with "black screen".
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

S = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(
    r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_dome\shots"
)
PLATES = ["d0_atmos-on", "d1_atmos-off", "d2_gradient", "d3_skygrad-off"]
BAND = 90


def load(name):
    return np.asarray(Image.open(S / f"{name}.png").convert("RGB"), dtype=np.float32) / 255.0


def luma(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


im = {p: load(p) for p in PLATES}
st = {}
for p in PLATES:
    b = luma(im[p][:BAND])
    st[p] = {
        "mean": tuple(round(float(im[p][:BAND][..., i].mean()), 4) for i in range(3)),
        "luma": float(b.mean()),
        # column-std of the column means: a flat single-colour sky has none; a
        # dome has the sphere's own left/right falloff, so this separates them
        # without any mask derived from the frames being compared.
        "colstd": float(b.mean(axis=0).std()),
        "rowstd": float(b.mean(axis=1).std()),
        "lit_frac": float((luma(im[p]) > 0.02).mean()),
    }

print(f"{'plate':<16} {'band meanRGB':>26} {'luma':>8} {'colstd':>9} {'rowstd':>9} {'lit_frac':>9}")
for p in PLATES:
    v = st[p]
    print(f"{p:<16} {str(v['mean']):>26} {v['luma']:>8.4f} {v['colstd']:>9.5f} "
          f"{v['rowstd']:>9.5f} {v['lit_frac']:>9.4f}")

d23 = float(np.abs(im["d2_gradient"] - im["d3_skygrad-off"]).mean())
d13 = float(np.abs(im["d1_atmos-off"] - im["d3_skygrad-off"]).mean())
print(f"\nfull-frame mean|d2-d3| = {d23:.4f}   mean|d1-d3| = {d13:.4f}")

checks = [
    ("A  RENDER PHASE: probe blacks out the sky band (d1 luma < 0.01, control > 0.02)",
     st["d1_atmos-off"]["luma"] < 0.01 and st["d3_skygrad-off"]["luma"] > 0.02),
    ("B  RENDER PHASE: probe moves the whole frame (mean|d1-d3| >= 0.10)",
     d13 >= 0.10),
    ("C  ATMOS GATE: atmosphere ON suppresses the dome (d0 luma > 0.10, probe inert)",
     st["d0_atmos-on"]["luma"] > 0.10),
    ("D  NOT-A-BLACK-SCREEN: lit geometry survives in the probe frame",
     0.0005 < st["d1_atmos-off"]["lit_frac"] < 0.20),
    ("E  BEFORE/AFTER: control sky is genuinely flat (d3 colstd < 0.0005)",
     st["d3_skygrad-off"]["colstd"] < 0.0005),
    ("F  BEFORE/AFTER: dome sky has real lateral structure (>= 10x the control)",
     st["d2_gradient"]["colstd"] >= 10 * max(st["d3_skygrad-off"]["colstd"], 1e-6)),
    ("G  BEFORE/AFTER: the two plates are not the same frame (mean|d2-d3| >= 0.01)",
     d23 >= 0.01),
]
print()
ok = True
for label, passed in checks:
    print(f"{'PASS' if passed else 'FAIL'}  {label}")
    ok &= passed
print(f"\nVERDICT: {'ALL PASS' if ok else 'FAILURES ABOVE'}")
sys.exit(0 if ok else 1)
