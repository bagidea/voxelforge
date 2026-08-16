#!/usr/bin/env python
"""_poppy_fillrig_sheet.py — measure the v1/v2 fill-rig pairs, GATE them, then draw.

MEASURE FIRST, AND THE MEASUREMENTS DECIDE. Every number below has a threshold and
a verdict; a printed number with no threshold is not a gate, it is decoration, and
the first version of this script shipped three of those.

  shade band     the darkest 25 % of the V1 frame, as a MASK, applied to BOTH
                 images. Same pixel set on both sides — that is what makes the
                 delta a delta and not two crops of two different histograms.
  band sigma     std-dev of luminance INSIDE that band, and the one number that
                 separates the two rigs BY CONSTRUCTION: a flat AmbientLight gives
                 every shaded face the same irradiance whichever way it faces, so
                 its shade band is narrow by definition. Two directional fills at
                 different elevations cannot do that — a top face and a side face
                 in the same shade must land apart.
                 GATE: sigma must rise >= +10 %. Anything less and the top/side
                 split did not happen, whatever the mean did.
  p05 L          the crush floor (G3 wants the darkest 5 % off the floor with
                 detail left to grade). Moving fill from flat to directional must
                 not re-crush it.  GATE: p05 must not fall more than 2.0.
  sunlit p90     the key band. NOT expected to be identical — `bounce_lux` at 28°
                 deliberately lands on sun-FACING walls, so some rise is the
                 feature working. What must not happen is the exposure walking off:
                 GATE: |delta| <= 8.0 (~0.2 stop on this band). WARN over 2.0,
                 which is already 5x the 0.28-0.39 repeat-shot floor look.rs
                 measured for this capture path.

Exit code is 0 only if every pair passes every gate.

Usage: python scripts/_poppy_fillrig_sheet.py [OUTDIR]
"""
import sys
import os
import numpy as np
from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1] if len(sys.argv) > 1 else "_poppy_fillrig"
FRAMINGS = ["vista", "horizon", "shade"]
# Rec.709 luma — the same weights every other measuring script in this repo uses.
W = np.array([0.2126, 0.7152, 0.0722], dtype=np.float64)

SIGMA_MIN_GAIN = 1.10   # shade-band sigma must rise at least 10 %
P05_MAX_DROP = 2.0      # shade must not be re-crushed
KEY_MAX_DRIFT = 8.0     # sunlit band: hard ceiling on exposure walk
KEY_WARN_DRIFT = 2.0    # sunlit band: above the repeat-shot floor, worth saying


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)


def lum(a):
    return a @ W


def stats(a, mask):
    """Level / spread / warmth over `mask`, in 0-255 units."""
    L = lum(a)[mask]
    r, g, b = a[..., 0][mask], a[..., 1][mask], a[..., 2][mask]
    return dict(
        mean=L.mean(), sigma=L.std(), p05=np.percentile(L, 5),
        p50=np.percentile(L, 50), rb=(r - b).mean(), n=int(mask.sum()),
    )


def label(img, text, sub=""):
    """Caption bar under a plate. Plain on purpose — this sheet is evidence."""
    w = img.width
    bar = Image.new("RGB", (w, 64 if sub else 40), (14, 14, 16))
    d = ImageDraw.Draw(bar)
    try:
        f1 = ImageFont.truetype("arialbd.ttf", 26)
        f2 = ImageFont.truetype("arial.ttf", 18)
    except OSError:
        f1 = f2 = ImageFont.load_default()
    d.text((14, 6), text, fill=(240, 240, 245), font=f1)
    if sub:
        d.text((14, 38), sub, fill=(150, 155, 168), font=f2)
    out = Image.new("RGB", (w, img.height + bar.height), (14, 14, 16))
    out.paste(img, (0, 0))
    out.paste(bar, (0, img.height))
    return out


rows, report, verdicts, missing = [], [], [], []
for f in FRAMINGS:
    pa, pb = f"{OUT}/{f}-v1.png", f"{OUT}/{f}-v2.png"
    if not (os.path.exists(pa) and os.path.exists(pb)):
        missing.append(f)
        continue
    A, B = load(pa), load(pb)
    if A.shape != B.shape:
        missing.append(f + " (size mismatch)")
        continue

    LA = lum(A)
    # Mask from the V1 side only, then used on both. Never re-derived per image.
    shade = LA <= np.percentile(LA, 25)
    sunlit = LA >= np.percentile(LA, 90)
    sa, sb = stats(A, shade), stats(B, shade)
    ka, kb = stats(A, sunlit), stats(B, sunlit)
    dpix = float((np.abs(lum(B) - LA) > 2.0).mean() * 100.0)

    # --- the gates ---------------------------------------------------------
    g = []
    gain = sb["sigma"] / sa["sigma"] if sa["sigma"] > 0 else float("inf")
    g.append(("sigma +%d%% (need +%d%%)" % (round((gain - 1) * 100), round((SIGMA_MIN_GAIN - 1) * 100)),
              gain >= SIGMA_MIN_GAIN))
    drop = sa["p05"] - sb["p05"]
    g.append(("p05 %+.2f (max drop %.1f)" % (-drop, P05_MAX_DROP), drop <= P05_MAX_DROP))
    drift = abs(kb["mean"] - ka["mean"])
    g.append(("key drift %.2f (max %.1f)" % (drift, KEY_MAX_DRIFT), drift <= KEY_MAX_DRIFT))
    passed = all(ok for _, ok in g)
    verdicts.append((f, passed, g, drift))

    report.append(
        f"{f:8s} shade-band  L {sa['mean']:6.2f} -> {sb['mean']:6.2f}   "
        f"sigma {sa['sigma']:5.2f} -> {sb['sigma']:5.2f}   "
        f"p05 {sa['p05']:5.2f} -> {sb['p05']:5.2f}   "
        f"R-B {sa['rb']:+6.2f} -> {sb['rb']:+6.2f}\n"
        f"{'':8s} sunlit p90  L {ka['mean']:6.2f} -> {kb['mean']:6.2f}\n"
        f"{'':8s} frame moved {dpix:5.2f} % of pixels by >2 L\n"
        f"{'':8s} GATES: " + "  ".join(("PASS " if ok else "FAIL ") + n for n, ok in g)
        + ("" if drift <= KEY_WARN_DRIFT else
           f"\n{'':8s} note: key band moved {drift:.2f} L — inside the ceiling, but "
           f"above the {KEY_WARN_DRIFT:.1f} repeat-shot floor; bounce@28deg does land on sun-facing walls")
    )

    scale = 720 / A.shape[1]
    size = (720, int(A.shape[0] * scale))
    ia = label(Image.open(pa).convert("RGB").resize(size, Image.LANCZOS),
               f"{f} · BEFORE (v1)", "flat fill 2200 lux · no sky/bounce · PCSS off at High")
    ib = label(Image.open(pb).convert("RGB").resize(size, Image.LANCZOS),
               f"{f} · AFTER (v2)",
               f"1150 flat + sky 1400@76d + bounce 1100@28d · PCSS 8 · "
               f"shade sigma {sa['sigma']:.1f} -> {sb['sigma']:.1f}")
    row = Image.new("RGB", (ia.width + ib.width + 12, ia.height), (14, 14, 16))
    row.paste(ia, (0, 0))
    row.paste(ib, (ia.width + 12, 0))
    rows.append(row)

print("=== fill rig v1 -> v2, one exe, env only ===")
for r in report:
    print(r)
if missing:
    print("MISSING (not measured, not drawn): " + ", ".join(missing))
if not rows:
    print("no pairs on disk — nothing measured, nothing drawn")
    sys.exit(1)

W_ = max(r.width for r in rows)
H_ = sum(r.height for r in rows) + 12 * (len(rows) - 1)
sheet = Image.new("RGB", (W_, H_), (14, 14, 16))
y = 0
for r in rows:
    sheet.paste(r, (0, y))
    y += r.height + 12
p = f"{OUT}/fillrig_before_after.png"
sheet.save(p)
print(f"\nsheet -> {p}  ({sheet.width}x{sheet.height})")

bad = [f for f, ok, _, _ in verdicts if not ok]
print("VERDICT: " + ("ALL PAIRS PASS" if not bad and not missing
                     else "FAILED: " + ", ".join(bad + missing)))
sys.exit(0 if (not bad and not missing) else 1)
