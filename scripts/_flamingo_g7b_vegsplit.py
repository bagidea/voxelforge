#!/usr/bin/env python3
"""Where did G7's vegetation-saturation win actually come from?

axis C reads vegetation saturation over the WHOLE frame. A2 reads how much the
haze moves the near band. In this framing those are largely the same pixels --
the foreground is grass -- so the two axes can be measuring one effect with
opposite signs. This splits axis C's own mask by the depth ruler
(_flamingo_g7_probe.ps1) and reports saturation per band, which settles it:

  * if G7's win is spread evenly across depth, it was aerial perspective and
    removing the near-field wash cost something real;
  * if it is concentrated in the near band, the "win" WAS the wash -- the exact
    thing A2 was independently flagging as the defect -- and the two gates were
    never both satisfiable by the haze.
"""
import colorsys
import sys

from PIL import Image

G7, G7B, PR = "_flamingo_g7", "_flamingo_g7b", "_flamingo_g7probe"
SLICES = [8, 12, 16, 20, 25, 30, 40, 55, 75, 110]
CHANGED = 6
STEP = 2
ROWS = [("hazeoff", f"{G7B}/hazeoff-nohud2.png"),
        ("g7full  (old default, ExpSq+desat .40)", f"{G7B}/g7full-nohud2.png"),
        ("g7curve (ExpSq + desat .60)", f"{G7B}/g7curve-nohud2.png"),
        ("s16     (ramp from 16)", f"{G7B}/s16-nohud2.png"),
        ("ship    (ramp from 20 + desat .60)", f"{G7B}/ship-nohud2.png")]


def hsv(r, g, b):
    h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
    return h * 360, s * 100, v * 100


def is_veg(r, g, b):
    h, s, v = hsv(r, g, b)
    return 40.0 <= h <= 150.0 and s > 15.0 and v > 10.0


def main():
    ref = Image.open(f"{G7}/hazeoff-nohud2.png").convert("RGB")
    W, H = ref.size
    off = ref.load()
    sl = {s: Image.open(f"{PR}/slice{s}-nohud2.png").convert("RGB").load() for s in SLICES}

    depth = {}
    for y in range(0, H, STEP):
        for x in range(0, W, STEP):
            d = None
            for s in SLICES:
                p, q = off[x, y], sl[s][x, y]
                if max(abs(p[i] - q[i]) for i in range(3)) < CHANGED:
                    d = s
                    break
            depth[(x, y)] = d

    def band(d):
        if d is None:
            return "far 75+"
        if d <= 20:
            return "near <=20"
        if d <= 40:
            return "mid 20-40"
        return "far 40+"

    order = ["near <=20", "mid 20-40", "far 40+", "far 75+"]
    print(f"# vegetation saturation by MEASURED depth (axis C's own mask)\n")
    print(f"{'row':40s} " + " ".join(f"{b:>11s}" for b in order) + f"{'whole':>9s}")
    base = {}
    for label, path in ROWS:
        img = Image.open(path).convert("RGB").load()
        acc = {b: [] for b in order}
        allv = []
        for y in range(0, H, STEP):
            for x in range(0, W, STEP):
                p = img[x, y]
                if not is_veg(*p):
                    continue
                s = hsv(*p)[1]
                acc[band(depth[(x, y)])].append(s)
                allv.append(s)
        cells = []
        for b in order:
            v = acc[b]
            m = sum(v) / len(v) if v else float("nan")
            base.setdefault(b, m)
            cells.append(f"{m:6.1f}({m - base[b]:+5.1f})")
        whole = sum(allv) / len(allv)
        print(f"{label:40s} " + " ".join(f"{c:>11s}" for c in cells) + f"{whole:9.1f}")
    print("\n(brackets = delta vs the hazeoff row, i.e. what the haze bought in that band)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
