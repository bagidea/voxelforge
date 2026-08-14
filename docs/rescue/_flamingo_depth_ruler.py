#!/usr/bin/env python3
"""Read the depth CDF off the `_flamingo_depth_ruler.sh` step-fog captures.

Each capture `d<S>.png` was shot with `VOXELFORGE_LOOK_FOG=S,S+0.5`, i.e. haze
alpha is a hard step at S. Against the no-haze reference `d9999.png`, a pixel
that MOVED is a pixel whose geometry sits farther than S. So

    beyond(S) = |{p : |frame_S(p) - ref(p)| > THRESH}|

and `beyond(S) / beyond(1)` is P(depth > S) over the fog-receiving geometry in
frame. Percentiles are read by linear interpolation on that curve.

THRESH (6 levels max-channel, ~2x the 8-bit quantisation of the tonemap
shoulder) rejects per-pixel noise. It does NOT reject the OTHER noise floor, and
the first run of this script walked straight into it: from S=32 out to S=320 the
moved count sat flat at 8 465-11 506 px (~1.0-1.2% of frame) instead of falling
to zero. Nothing in this map is 320 blocks from the camera, so that plateau is
not geometry — it is the capture's own frame-to-frame irreproducibility (TAA
history + stochastic SSAO; every capture is a separate process, so no two frames
are bit-identical even at identical settings). Reported raw it inflates every
percentile above p90 and invents a 320-block tail out of nothing.

So the floor is MEASURED, not assumed: `floor` is the median of the flat tail
(the largest suffix of the ladder whose spread is within 2x its own minimum),
every count has it subtracted, and any S whose raw count is inside the tail's
observed range is printed UNMEASURABLE rather than given a number. A percentile
that can only be read from unmeasurable rows is refused (exit 2) instead of
guessed.

EXIT CODES: 0 measured · 2 refused to measure (missing/degenerate captures).
"""
import os
import sys
from pathlib import Path

from PIL import Image

OUT = Path(os.environ.get("OUT", "_flamingo_depthruler"))
THRESH = 6  # max-channel levels; printed below so the reader can judge it
REF = OUT / "d9999.png"


def load(p):
    return Image.open(p).convert("RGB")


def moved(a, b):
    """Count pixels whose max channel difference exceeds THRESH."""
    ap, bp = a.load(), b.load()
    w, h = a.size
    n = 0
    for y in range(h):
        for x in range(w):
            r1, g1, b1 = ap[x, y]
            r2, g2, b2 = bp[x, y]
            if max(abs(r1 - r2), abs(g1 - g2), abs(b1 - b2)) > THRESH:
                n += 1
    return n


def main():
    if not REF.is_file():
        print(f"REFUSED  no no-haze reference at {REF}")
        return 2
    ref = load(REF)
    W, H = ref.size
    total = W * H

    steps = []
    for p in OUT.glob("d*.png"):
        s = p.stem[1:]
        if s == "9999":
            continue
        try:
            steps.append((float(s), p))
        except ValueError:
            continue
    steps.sort()
    if not steps:
        print("REFUSED  no step captures found")
        return 2

    print(f"ref {REF}  {W}x{H} ({total} px)   THRESH={THRESH} levels (max-channel)")
    rows = []
    for s, p in steps:
        img = load(p)
        if img.size != ref.size:
            print(f"REFUSED  {p.name} is {img.size}, ref is {ref.size}")
            return 2
        rows.append((s, moved(ref, img)))

    # ---- the capture's own reproducibility floor, measured off the flat tail --
    # Walk suffixes from the longest down; take the first whose max <= 2x its min
    # AND which is small against the near-field counts. That is the plateau where
    # the ladder has run out of geometry and is only re-measuring capture noise.
    counts = [n for _, n in rows]
    floor_lo = len(rows)
    for i in range(len(rows) - 1, 0, -1):
        tail = counts[i:]
        if len(tail) >= 3 and max(tail) <= 2 * min(tail) and max(tail) < 0.25 * counts[0]:
            floor_lo = i
        else:
            if floor_lo < len(rows):
                break
    if floor_lo >= len(rows):
        floor, floor_hi = 0, 0
        print("no flat tail found — floor taken as 0 (the ladder never ran out of geometry)")
    else:
        tail = sorted(counts[floor_lo:])
        floor = tail[len(tail) // 2]
        floor_hi = tail[-1]
        print(f"capture noise floor: median {floor} px, max {floor_hi} px, measured over "
              f"S>={rows[floor_lo][0]:g} ({len(tail)} rungs, {100.0*floor/total:.2f}% of frame)")

    norm = rows[0][1] - floor
    if norm < total * 0.01:
        print(f"REFUSED  denominator degenerate: S={rows[0][0]} moved only {rows[0][1]} px "
              f"({100.0*rows[0][1]/total:.2f}% of frame) — the step fog is not landing")
        return 2

    print(f"denominator: S={rows[0][0]:g} moved {rows[0][1]} px - floor = {norm} px "
          f"= {100.0*norm/total:.1f}% of frame (all fog-receiving geometry)")
    print()
    print(f"{'S (blocks)':>10} {'moved px':>10} {'-floor':>9} {'P(depth>S)':>11}  note")
    curve = []
    for s, n in rows:
        net = n - floor
        measurable = n > floor_hi
        frac = max(0.0, net) / norm
        if measurable:
            curve.append((s, frac))
        note = "" if measurable else "UNMEASURABLE (inside noise floor)"
        print(f"{s:>10g} {n:>10} {net:>9} {100.0*frac:>10.1f}%  {note}")

    if len(curve) < 2:
        print("REFUSED  fewer than 2 measurable rungs — nothing to interpolate")
        return 2

    deepest_measurable = curve[-1][0]

    def pct(q):
        """Depth d with P(depth > d) == 1-q, or None if only noise rungs reach it."""
        target = 1.0 - q
        prev_s, prev_f = curve[0]
        for s, f in curve[1:]:
            if f <= target:
                if prev_f == f:
                    return s
                t = (prev_f - target) / (prev_f - f)
                return prev_s + t * (s - prev_s)
            prev_s, prev_f = s, f
        return None

    print()
    print("DEPTH PERCENTILES over fog-receiving geometry (blocks):")
    for q in (0.05, 0.25, 0.50, 0.75, 0.90, 0.95, 0.99):
        v = pct(q)
        shown = f"{v:7.1f}" if v is not None else "      ?"
        tail_note = "" if v is not None else f"  UNMEASURABLE (> p-reach of S={deepest_measurable:g})"
        print(f"  p{int(q*100):02d} = {shown}{tail_note}")
    p05, p95 = pct(0.05), pct(0.95)
    print()
    if p05 is not None and p95 is not None:
        print(f"  visible span p05..p95 = {p05:.1f} .. {p95:.1f} blocks")
    print(f"  deepest rung above the noise floor = {deepest_measurable:g} blocks")
    return 0


if __name__ == "__main__":
    sys.exit(main())
