#!/usr/bin/env python3
"""G7 — the three axes G1-G6 does not cover: aerial perspective, AO bite, vegetation colour.

WHY A NEW SCRIPT AND NOT MORE ROWS IN grade_gate.py. G3/G5/G6 are LOCKED gates
with signed-off thresholds; a frame that passes all three can still be the flat,
depthless, toy-green frame the CEO rejected on 2026-08-05, because not one of
them measures anything that varies WITH DISTANCE or with geometric occlusion,
and none of them looks at the greens at all (G6 reads sunlit wood by
construction). This grades exactly that gap. It never re-derives a G3/G5/G6
number — `grade_gate.py` stays the authority for those.

TWO OF THE THREE AXES ARE A/B, NOT SINGLE-FRAME, AND THAT IS THE POINT.
"Is there ambient occlusion in this frame" cannot be answered from one PNG
without guessing which dark pixels are AO and which are cast shadow, dark albedo
or a crevice that was always brown. Shooting the SAME scene from the SAME binary
with the layer lifted out and diffing turns a judgement call into a measurement:
whatever changed IS the layer. The `VOXELFORGE_LOOK_SSAO=off` and
`VOXELFORGE_LOOK_HAZE=0` hooks in look.rs exist for this and nothing else.

A SINGLE-FRAME HAZE AXIS WAS TRIED FIRST AND THROWN OUT — it is kept below as
reported context (A1) with no verdict attached, because it PASSED on the
zero-fog G6 baseline: far ground in this scene is disproportionately in shadow,
and shadow desaturates ground exactly the way haze does. An axis that scores a
frame with provably no fog in it as "has aerial perspective" is not a gate. A2
is the gate.

AXIS C (vegetation) already runs in `scripts/regrade.py` on every frame.

A/B CAPTURE RECIPE — paired frames must be shot from the SAME binary:
  # Haze A/B pair (proves aerial perspective is depth-dependent):
  $env:VOXELFORGE_LOOK_HAZE=0; .\voxelforge_perf.exe -- --play   # -> haze-off-nohud2.png
  .\voxelforge_perf.exe -- --play                                 # -> haze-on-nohud2.png
  python scripts/regrade.py --before before/ --after after/ \
      --g7-haze haze-off-nohud2.png haze-on-nohud2.png

  # SSAO A/B pair (proves contact AO is real, not dark albedo):
  $env:VOXELFORGE_LOOK_SSAO=off; .\voxelforge_perf.exe -- --play  # -> ssao-off-nohud2.png
  .\voxelforge_perf.exe -- --play                                 # -> ssao-on-nohud2.png
  python scripts/regrade.py --before before/ --after after/ \
      --g7-ao ssao-off-nohud2.png ssao-on-nohud2.png

The env-var hooks are real: see `VOXELFORGE_LOOK_HAZE` and `VOXELFORGE_LOOK_SSAO`
in client/src/look.rs. Same binary for both shots or the diff is meaningless.

Original Usage (every frame must be de-HUDded — see the guard note in main()):
  grade_g7.py --frame <on-nohud2.png>                       # axis C + A1 context
  grade_g7.py --ab-haze <off-nohud2.png> <on-nohud2.png>    # axis A2
  grade_g7.py --ab-ao   <off-nohud2.png> <on-nohud2.png>    # axis B
Exit 0 iff every graded axis PASSes.
"""
import argparse
import colorsys
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, called in main())

# ---------------------------------------------------------------------------
# REFERENCE + TARGETS
#
# The vegetation numbers are calibrated against `docs/assets/moodboard.png`
# (right panel — the outdoor reference the look bible names on line 7), measured
# with the SAME mask this file uses, not eyeballed:
#
#            vegetation sat    vegetation hue    G-R
#   REF          37.3%             44.2 deg      -16.1   (warm yellow-olive)
#   G6 frame     79.8%             89.1 deg      +36.6   (pure saturated green)
#
# THE GATE IS "CLOSE 40% OF THE MEASURED GAP", FIXED BEFORE THE SWEEP RAN, not a
# number picked afterwards to match what came out. 40% and not 100% because the
# remaining distance is ALBEDO: the block palette's own green
# (`BlockId::base_color`, voxel.rs) is shiba's lane, and the look lane's honest
# reach is the grade + the light + the haze. Whatever gap is left after this is a
# handoff, and it gets reported as one rather than hidden by a softer target.
# ---------------------------------------------------------------------------
REF_VEG_SAT, REF_VEG_HUE = 37.3, 44.2
BASE_VEG_SAT, BASE_VEG_HUE = 79.8, 89.1
CLOSURE = 0.40
T_VEG_SAT = BASE_VEG_SAT - CLOSURE * (BASE_VEG_SAT - REF_VEG_SAT)   # <= 62.8 %
T_VEG_HUE = BASE_VEG_HUE - CLOSURE * (BASE_VEG_HUE - REF_VEG_HUE)   # <= 71.1 deg

T_DEPTH_RATIO = 2.5   # far-band haze delta / near-band haze delta. Proves the
                      # haze is DEPTH-dependent and not a global colour wash — a
                      # flat tint scores 1.0 here however pretty it looks. The
                      # zero-fog baseline scores 0 on both terms by construction.
T_FAR_DELTA = 6.0     # ...and the far band has to actually move, not merely move
                      # more than a near band that moved nothing.

T_AO_BITE = 10.0      # p99 per-pixel darkening (L, 0-100) with SSAO on vs off.
T_AO_COVER = 2.0      # % of frame darkened by >= 2 L. Guards the other way: one
                      # dark seam can carry p99 without AO reading anywhere.


def load(path):
    return Image.open(path).convert("RGB")


def lum(r, g, b):
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255 * 100


def hsv(r, g, b):
    h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
    return h * 360, s * 100, v * 100


def is_veg(r, g, b):
    """Vegetation, by HUE BAND rather than by channel ordering.

    The obvious test (`g > r`) is the one thing that must not be used here: the
    reference's sunlit grass is `R > G` — warm yellow-olive — so a green-dominant
    mask finds ZERO vegetation in the very frame the targets are calibrated
    against, and would then only ever be able to measure frames that already have
    the defect. Hue 40..150 deg spans yellow-olive through green and reads both.
    """
    h, s, v = hsv(r, g, b)
    return 40.0 <= h <= 150.0 and s > 15.0 and v > 10.0


def sample(img, step=2):
    px, (W, H) = img.load(), img.size
    return [px[x, y] for y in range(0, H, step) for x in range(0, W, step)]


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


def axis_c(img, label):
    """Vegetation colour — the 'green ของเล่น' axis."""
    veg = [p for p in sample(img) if is_veg(*p)]
    if not veg:
        raise SystemExit("! no vegetation pixels found")
    hs = [hsv(*p) for p in veg]
    h, s = mean([a[0] for a in hs]), mean([a[1] for a in hs])
    r, g, b = (mean([p[i] for p in veg]) for i in range(3))
    ok = s <= T_VEG_SAT and h <= T_VEG_HUE
    print("## C   vegetation colour vs moodboard reference")
    print(f"   n={len(veg)}   mean RGB=({r:.1f},{g:.1f},{b:.1f})   G-R={g - r:+.1f}")
    print(f"   saturation {s:5.1f}%   need <= {T_VEG_SAT:4.1f}   (REF {REF_VEG_SAT}, G6 base {BASE_VEG_SAT})")
    print(f"   hue        {h:5.1f}deg need <= {T_VEG_HUE:4.1f}   (REF {REF_VEG_HUE}, G6 base {BASE_VEG_HUE})")
    if BASE_VEG_SAT != REF_VEG_SAT:
        cs = 100 * (BASE_VEG_SAT - s) / (BASE_VEG_SAT - REF_VEG_SAT)
        ch = 100 * (BASE_VEG_HUE - h) / (BASE_VEG_HUE - REF_VEG_HUE)
        print(f"   gap closed: saturation {cs:5.1f}%   hue {ch:5.1f}%   (need >= {CLOSURE * 100:.0f}%)")
    print(f"   -> {'PASS' if ok else 'FAIL'}   [{label}]\n")
    return ok


def axis_a1_context(img):
    """REPORTED, NOT GRADED. See the module docstring for why this is not a gate."""
    px, (W, H) = img.load(), img.size
    rows = {}
    for y in range(0, H, 2):
        row = [px[x, y] for x in range(0, W, 2) if is_veg(*px[x, y])]
        if len(row) >= 12:
            rows[y] = row
    if len(rows) < 20:
        print("## A1  (context) not a ground-field framing — skipped\n")
        return
    ys = sorted(rows)
    n = max(1, int(len(ys) * 0.14))
    far = [p for y in ys[:n] for p in rows[y]]
    near = [p for y in ys[-n:] for p in rows[y]]
    s_n, s_f = mean([hsv(*p)[1] for p in near]), mean([hsv(*p)[1] for p in far])
    b_n, b_f = mean([p[2] for p in near]), mean([p[2] for p in far])
    print("## A1  (context only, NOT a gate — confounded by shadow, see docstring)")
    print(f"   ground rows y={ys[0]}..{ys[-1]}   saturation near {s_n:5.1f}% far {s_f:5.1f}%"
          f"   B near {b_n:5.1f} far {b_f:5.1f}\n")


def _delta(off, on):
    if off.size != on.size:
        raise SystemExit(f"! size mismatch {off.size} vs {on.size}")
    a, b, (W, H) = off.load(), on.load(), off.size
    dl, de = [], []
    for y in range(0, H, 2):
        rl, re = [], []
        for x in range(0, W, 2):
            p, q = a[x, y], b[x, y]
            rl.append(lum(*p) - lum(*q))
            re.append(max(abs(p[i] - q[i]) for i in range(3)))
        dl.append(rl)
        de.append(re)
    return dl, de


def axis_a2(off_png, on_png):
    """Depth-dependence proof: the haze must move the far band far more than the near.

    A global colour grade moves both equally (ratio 1.0). Only something that
    reads distance can score high here — which is the entire claim being made
    about aerial perspective, tested directly instead of asserted.
    """
    off, on = load(off_png), load(on_png)
    _, de = _delta(off, on)
    n = max(1, int(len(de) * 0.18))
    far, near = [v for r in de[:n] for v in r], [v for r in de[-n:] for v in r]
    f, nr = mean(far), mean(near)
    ratio = f / nr if nr > 0.05 else float("inf")
    ok = ratio >= T_DEPTH_RATIO and f >= T_FAR_DELTA
    print("## A2  aerial perspective is depth-dependent (A/B, VOXELFORGE_LOOK_HAZE=0 vs shipped)")
    print(f"   off={off_png}\n   on ={on_png}")
    print(f"   mean |delta|  far band {f:6.2f} (need >= {T_FAR_DELTA})   near band {nr:6.2f}")
    print(f"   far/near ratio {ratio:6.2f}   need >= {T_DEPTH_RATIO}")
    print(f"   -> {'PASS' if ok else 'FAIL'}\n")
    return ok


def axis_b(off_png, on_png):
    """AO bite: how much does lifting the SSAO layer out actually change the frame."""
    off, on = load(off_png), load(on_png)
    dl, _ = _delta(off, on)
    flat = sorted(v for r in dl for v in r)
    p99, p50 = flat[int(len(flat) * 0.99)], flat[len(flat) // 2]
    cover = 100.0 * sum(1 for v in flat if v >= 2.0) / len(flat)
    ok = p99 >= T_AO_BITE and cover >= T_AO_COVER
    print("## B   contact AO bite (A/B, VOXELFORGE_LOOK_SSAO=off vs on)")
    print(f"   off={off_png}\n   on ={on_png}")
    print(f"   darkening L: p50 {p50:+5.2f}   p99 {p99:5.2f}   need >= {T_AO_BITE}")
    print(f"   coverage (darkened >= 2 L) {cover:5.2f}%   need >= {T_AO_COVER}")
    print(f"   -> {'PASS' if ok else 'FAIL'}\n")
    return ok


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--frame")
    ap.add_argument("--ab-haze", nargs=2, metavar=("OFF", "ON"))
    ap.add_argument("--ab-ao", nargs=2, metavar=("OFF", "ON"))
    a = ap.parse_args()
    if not (a.frame or a.ab_haze or a.ab_ao):
        ap.error("nothing to grade")

    # HARD GUARD — after parse_args (so --help still works), before anything is
    # loaded or measured. Every axis here is a mean/percentile over a masked
    # BAND of the frame: axis C over hue 40..150 vegetation, A2 over a far band,
    # B over an A/B darkening delta. HUD glyphs land inside those masks and
    # there is no per-pixel result to sanity-check afterwards — one poisoned
    # frame just moves a number a plausible amount. All paths are checked in one
    # call so an A/B pair reports both bad names at once.
    require_nohud2(
        [p for p in ([a.frame] if a.frame else []) + list(a.ab_haze or []) + list(a.ab_ao or [])],
        tool="grade_g7.py",
    )

    results = []
    if a.frame:
        img = load(a.frame)
        print(f"# {a.frame} = {img.size[0]}x{img.size[1]}\n")
        axis_a1_context(img)
        results.append(("C", axis_c(img, a.frame)))
    if a.ab_haze:
        results.append(("A2", axis_a2(*a.ab_haze)))
    if a.ab_ao:
        results.append(("B", axis_b(*a.ab_ao)))
    print("G7 AXES: " + "  ".join(f"{k}={'P' if v else 'F'}" for k, v in results)
          + f"   => {'PASS' if all(v for _, v in results) else 'FAIL'}")
    return 0 if all(v for _, v in results) else 1


if __name__ == "__main__":
    sys.exit(main())
