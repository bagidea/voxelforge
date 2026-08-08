#!/usr/bin/env python3
"""Collateral check #2: what the HAZE default does to the pixels I let it off on.

`_flamingo_g7_sky.py` was written to convict the GRADE lever and it printed a
stone column that the G7 recommendation then never mentioned — the same
blind-spot standard applied to one lever and not the other. `hazeoff` -> `ship`
moves stone sat 56.2 -> 38.1 and stone hue 28.0 -> 44.8, and "the haze is free"
was measured only in the sky region where the haze happens to be cheap.

So: is that stone shift aerial perspective (which is the POINT of the haze and
should be strongest far away and near-zero underfoot), or a flat wash over the
whole frame (which is the A2 failure, and would mean the haze default is buying
its vegetation win partly by draining the near-field terracotta)?

DEPTH PROXY, AND ITS LIMIT. There is no depth buffer here, only the shot PNGs.
In THIS pinned framing (`VOXELFORGE_LOOK_CAM=35,-18,26`, camera above the set
looking down) image row is monotonic-ish with distance: higher on the screen =
further away. So the stone mask is split into terciles by row and each band is
measured separately. That is a proxy, not a depth read: an upper-band pixel can
be a near wall that happens to be tall. It is reported as bands, and no single
"ratio" is quoted off it -- same caveat A2 carries.

Mask is built ONCE from the reference frame and reused pixel-for-pixel, so every
frame is compared on the identical pixel set (camera is pinned on every row).

Usage: _flamingo_g7_stone.py <ref.png> <frame.png> [frame.png ...]
"""
import colorsys
import sys

from PIL import Image


def hsv(p):
    h, s, v = colorsys.rgb_to_hsv(p[0] / 255, p[1] / 255, p[2] / 255)
    return h * 360, s * 100, v * 100


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


ref = Image.open(sys.argv[1]).convert("RGB")
rp, (W, H) = ref.load(), ref.size

stone = []
for y in range(0, H, 2):
    for x in range(0, W, 2):
        h, s, v = hsv(rp[x, y])
        # same warm-material predicate as _flamingo_g7_sky.py, verbatim
        if not (180 <= h <= 260 and s > 12 and v > 40):
            if (h < 40 or h > 330) and s > 12 and v > 25:
                stone.append((x, y))

ys = sorted(y for _, y in stone)
c1, c2 = ys[len(ys) // 3], ys[2 * len(ys) // 3]
bands = {
    "far (top 1/3 rows)": [p for p in stone if p[1] <= c1],
    "mid": [p for p in stone if c1 < p[1] <= c2],
    "near (bottom 1/3)": [p for p in stone if p[1] > c2],
}

print(f"stone mask from {sys.argv[1]}: n={len(stone)}  row cuts y<={c1} / y<={c2} (H={H})")
for k, v in bands.items():
    print(f"  {k:<20} n={len(v)}")
print()

hdr = f"{'frame':<12}"
for k in bands:
    hdr += f" | {k:>19} sat  hue"
print(hdr)

rows = {}
for path in sys.argv[2:]:
    im = Image.open(path).convert("RGB")
    if im.size != (W, H):
        raise SystemExit(f"! size mismatch {im.size} vs {(W, H)}")
    p = im.load()
    name = path.replace("\\", "/").split("/")[-1].replace("-nohud2.png", "").replace(".png", "")
    line, vals = f"{name:<12}", {}
    for k, pts in bands.items():
        m = [hsv(p[x, y]) for x, y in pts]
        s, h = mean([a[1] for a in m]), mean([a[0] for a in m])
        vals[k] = (s, h)
        line += f" | {s:22.1f} {h:5.1f}"
    rows[name] = vals
    print(line)

if "hazeoff" in rows and "ship" in rows:
    print("\nhazeoff -> ship, per band (this is the haze default's own collateral):")
    for k in bands:
        s0, h0 = rows["hazeoff"][k]
        s1, h1 = rows["ship"][k]
        print(f"  {k:<20} sat {s0:5.1f} -> {s1:5.1f} ({(s1 - s0) / s0 * 100:+5.1f}%)"
              f"   hue {h0:5.1f} -> {h1:5.1f} ({h1 - h0:+5.1f} deg)")

    # --- A2's OWN metric, on a mask that contains no sky -------------------
    # grade_g7.axis_a2 bands the whole frame by image row, so its top band eats
    # sky (REPORT s.4 flags this). Same metric here -- mean per-pixel
    # max-channel |delta|, `grade_g7.py:160-166` -- but only over stone pixels,
    # i.e. ground material at both ends. This does not replace the gate; it says
    # what the gate's number would have been without the sky in it.
    off = Image.open(sys.argv[1]).convert("RGB").load()
    on = Image.open([p for p in sys.argv[2:] if "ship" in p][0]).convert("RGB").load()
    d = {}
    for k, pts in bands.items():
        d[k] = mean([max(abs(off[x, y][i] - on[x, y][i]) for i in range(3)) for x, y in pts])
    f, nr = d["far (top 1/3 rows)"], d["near (bottom 1/3)"]
    print("\nA2 metric (mean max-channel |delta|) on STONE ONLY -- no sky in this mask:")
    for k in bands:
        print(f"  {k:<20} {d[k]:6.2f}")
    print(f"  far/near = {f / nr:.2f}   (gate T_DEPTH_RATIO = 2.5; whole-frame row bands gave 0.97)")
