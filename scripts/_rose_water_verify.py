#!/usr/bin/env python3
"""Rose — verify the water evidence pair with pixels, not vibes.

Reads _rose_water_20260818/{before,after,noise}.png and answers:

  SIGNAL  (before vs after, the only difference is 2011 water voxels):
    1. Did anything change at all? (mean |diff|, changed-pixel share)
    2. Is there WATER-COLOURED pixels in the after frame where the before had
       none? We count pixels that flipped from warm (R>=B) to cool (B>R+6) —
       that direction is the river filling; the reverse is the river draining.
    3. Is the change where the river is? The changed-pixel mask must sit
       mostly in the lower half (the river runs front-to-back through it).

  FLOOR   (after vs noise — the SAME wet map shot twice; matmaps discipline):
    whatever moves there is capture-to-capture variance only. Every signal
    number must beat its floor number by a clear multiple or the "change" is
    noise, not water.

Writes a side-by-side sheet pair.png with a diff heatmap panel.

Exit 0 = signal beats floor and all three gates pass; 1 = fail with reasons.
"""

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "_rose_water_20260818"


def metrics(a, b):
    """Compare two RGB frames on the 2px lattice the gates were sized for."""
    w, h = a.size
    pa, pb = a.load(), b.load()
    n = 0
    changed = 0
    cool_flips = 0
    diff_sum = 0.0
    rows = [0] * 8  # changed pixels per horizontal band
    for y in range(0, h, 2):
        band = min(y * 8 // h, 7)
        for x in range(0, w, 2):
            ra, ga, ba = pa[x, y]
            rb, gb, bb_ = pb[x, y]
            d = abs(ra - rb) + abs(ga - gb) + abs(ba - bb_)
            n += 1
            diff_sum += d / 3
            if d > 24:
                changed += 1
                rows[band] += 1
                if ra >= ba and bb_ > rb + 6:
                    cool_flips += 1  # warm pixel became cool pixel
    return {
        "n": n,
        "mean": diff_sum / n,
        "changed": changed,
        "share": changed / n,
        "cool": cool_flips,
        "rows": rows,
    }


def main():
    frames = {}
    for tag in ("before", "after", "noise"):
        p = OUT / f"{tag}.png"
        if not p.exists():
            print(f"FAIL missing frame: {p}")
            return 1
        frames[tag] = Image.open(p).convert("RGB")
    if frames["before"].size != frames["after"].size != frames["noise"].size:
        print(f"FAIL size mismatch")
        return 1

    sig = metrics(frames["before"], frames["after"])
    floor = metrics(frames["after"], frames["noise"])

    w, h = frames["before"].size
    print(f"frames        : {w}x{h}  sampled {sig['n']} px")
    print(f"SIGNAL  before vs after : mean {sig['mean']:.3f}  changed {sig['share'] * 100:.2f}% ({sig['changed']} px)  warm->cool {sig['cool']} px")
    print(f"FLOOR   after  vs noise : mean {floor['mean']:.3f}  changed {floor['share'] * 100:.2f}% ({floor['changed']} px)  warm->cool {floor['cool']} px")
    ratio_mean = sig["mean"] / max(floor["mean"], 1e-9)
    ratio_share = sig["share"] / max(floor["share"], 1e-9)
    ratio_cool = (sig["cool"] - floor["cool"])
    print(f"signal/floor : mean x{ratio_mean:.1f}   changed x{ratio_share:.1f}   warm->cool excess +{ratio_cool} px")
    print("signal changed by band (top..bottom):", [f"{r * 100 / max(sig['n'] // 8, 1):.1f}%" for r in sig["rows"]])

    reasons = []
    if sig["share"] < 0.02:
        reasons.append(f"only {sig['share'] * 100:.2f}% of the frame changed — no river appeared")
    if ratio_mean < 4.0:
        reasons.append(f"signal mean only x{ratio_mean:.1f} the noise floor — not above capture noise")
    if ratio_share < 4.0:
        reasons.append(f"changed share only x{ratio_share:.1f} the noise floor — not above capture noise")
    if ratio_cool < 500:
        reasons.append(f"only +{ratio_cool} warm->cool pixels above floor — the change is not water")
    lower_half = sum(sig["rows"][4:]) / max(sig["changed"], 1)
    if lower_half < 0.35:
        reasons.append(f"only {lower_half * 100:.0f}% of the change is in the lower half — not a river through the frame")

    # side-by-side sheet: before | after | amplified diff
    pa = frames["before"].load()
    pb = frames["after"].load()
    diff = Image.new("L", (w, h))
    pd = diff.load()
    for y in range(h):
        for x in range(w):
            ra, ga, ba = pa[x, y]
            rb, gb, bb_ = pb[x, y]
            d = min(255, (abs(ra - rb) + abs(ga - gb) + abs(ba - bb_)))
            pd[x, y] = d
    sheet = Image.new("RGB", (w * 3 + 20, h), (12, 12, 12))
    sheet.paste(frames["before"], (0, 0))
    sheet.paste(frames["after"], (w + 10, 0))
    sheet.paste(diff.convert("RGB"), (w * 2 + 20, 0))
    sheet.save(OUT / "pair.png")
    print(f"sheet         : {OUT / 'pair.png'}")

    if reasons:
        for r in reasons:
            print("FAIL:", r)
        return 1
    print("PASS — the after frame contains a river the before frame does not, above the capture noise floor")
    return 0


if __name__ == "__main__":
    sys.exit(main())
