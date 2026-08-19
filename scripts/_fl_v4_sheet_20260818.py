#!/usr/bin/env python3
"""Flamingo — v4 look pass: one side-by-side PNG per scene, plus the null floor.

WHY A SHEET AND NOT TWO FILES
    A before and an after in two files is a claim the reader has to reconstruct by
    flipping windows. One image with both halves under one caption is the claim
    itself. The CEO does not accept a prose report for a look change, and he is
    right not to: the whole subject is pixels.

WHAT IS ON EACH SHEET
    left  = VOXELFORGE_LOOK_GEN=v3 (shipped)   right = v4 (this pass)
    caption strip = the axes that moved, measured by the CALIBRATED instrument
    (scripts/_fl_v4_grade_20260818.py control must PASS first — this script
    imports the same measure()), each with the REF (golden target) value, so a
    number that moved the WRONG way is as visible as one that moved the right way.

THE NULL LINE
    Every sheet carries the null-pair delta (outdoor-noon shot twice, identical
    env, identical gen) as the capture noise floor. An axis whose v3→v4 move is
    inside that floor is reported as "within noise", not as a win.

USAGE
    python scripts/_fl_v4_sheet_20260818.py [--plates DIR]
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)

import importlib  # noqa: E402
_g = importlib.import_module("_fl_v4_grade_20260818")

SCENES = ["outdoor-noon", "evening-raking", "night-firelit"]
AXES = [
    ("lum_mean", "lum mean", 1.0, "{:.1f}", "up"),
    ("highlight", "highlight %", 100.0, "{:.1f}", "up"),
    ("shadow", "shadow %", 100.0, "{:.1f}", "down"),
    ("edge_mean", "edge density", 1.0, "{:.2f}", "up"),
    ("strong", "strong-edge %", 100.0, "{:.1f}", "up"),
    ("hue90", "hue spread", 1.0, "{:.0f}", "up"),
    ("occupied", "hue bins/36", 1.0, "{:.0f}", "up"),
    ("sat_std", "sat std", 1.0, "{:.3f}", "up"),
    ("cool_pct", "cool %", 1.0, "{:.1f}", "up"),
]
PAD, STRIP, GAP = 14, 250, 10


def rms_delta(a_path, b_path):
    a = np.asarray(Image.open(a_path).convert("RGB"), dtype=np.float64)
    b = np.asarray(Image.open(b_path).convert("RGB"), dtype=np.float64)
    if a.shape != b.shape:
        return float("nan"), float("nan")
    d = np.abs(a - b)
    return float(np.sqrt((d ** 2).mean())), float(d.mean())


def sheet(scene, plates, ref, null_axis, out):
    bp = os.path.join(plates, f"{scene}_before-nohud2.png")
    ap = os.path.join(plates, f"{scene}_after-nohud2.png")
    if not (os.path.isfile(bp) and os.path.isfile(ap)):
        print(f"SKIP {scene}: missing plate(s)")
        return None
    mb = _g.art_measure(f"{scene} v3", bp)
    ma = _g.art_measure(f"{scene} v4", ap)
    rms, mad = rms_delta(bp, ap)

    ib, ia = Image.open(bp).convert("RGB"), Image.open(ap).convert("RGB")
    w, h = ib.size
    W = PAD * 2 + w * 2 + GAP
    H = PAD * 2 + h + STRIP + 26
    im = Image.new("RGB", (W, H), (18, 18, 22))
    d = ImageDraw.Draw(im)
    im.paste(ib, (PAD, PAD + 22))
    im.paste(ia, (PAD + w + GAP, PAD + 22))
    d.text((PAD, PAD + 4), f"{scene}  —  BEFORE  (VOXELFORGE_LOOK_GEN=v3, shipped)", fill=(210, 210, 220))
    d.text((PAD + w + GAP, PAD + 4), f"{scene}  —  AFTER  (v4, this pass)", fill=(255, 190, 120))

    y = PAD + 22 + h + 10
    d.text((PAD, y), f"{'axis':<16}{'v3 before':>11}{'v4 after':>11}{'delta':>10}{'REF':>9}   verdict", fill=(235, 235, 240))
    y += 16
    lines = []
    for key, label, scale, fmt, want in AXES:
        vb, va = float(mb[key]) * scale, float(ma[key]) * scale
        vr = float(ref[key]) * scale
        dv = va - vb
        floor = null_axis.get(key, 0.0) * scale
        if abs(dv) <= floor:
            verd = f"within noise (null {floor:.3g})"
            col = (150, 150, 155)
        else:
            toward = (dv > 0) == (vr > vb)
            verd = "toward REF" if toward else "AWAY from REF"
            col = (140, 220, 150) if toward else (240, 130, 120)
        d.text((PAD, y), f"{label:<16}{fmt.format(vb):>11}{fmt.format(va):>11}{dv:>+10.2f}{fmt.format(vr):>9}   {verd}", fill=col)
        lines.append((label, vb, va, dv, vr, verd))
        y += 15

    y += 6
    d.text((PAD, y), f"pixel delta v3->v4: RMS {rms:.2f} / MAD {mad:.2f}   "
                     f"|   capture noise floor (null pair): RMS {null_axis.get('_rms', 0.0):.2f}",
           fill=(200, 200, 210))
    im.save(out)
    print(f"wrote {out}")
    return {"scene": scene, "before": mb, "after": ma, "rms": rms, "mad": mad, "lines": lines}


def main():
    ap_ = argparse.ArgumentParser()
    ap_.add_argument("--plates", default="_fl_v4_20260818/plates")
    a = ap_.parse_args()

    if not _g.control():
        print("\nrefusing to build a sheet on an uncalibrated instrument.")
        sys.exit(2)
    print()

    plates = a.plates
    # --- the null floor: same scene, same env, same gen, shot twice ---------
    null_axis = {}
    n1 = os.path.join(plates, "outdoor-noon_after-nohud2.png")
    n2 = os.path.join(plates, "outdoor-noon_afterNULL-nohud2.png")
    if os.path.isfile(n1) and os.path.isfile(n2):
        r, _m = rms_delta(n1, n2)
        null_axis["_rms"] = r
        m1, m2 = _g.art_measure("null1", n1), _g.art_measure("null2", n2)
        for key, _l, _s, _f, _w in AXES:
            null_axis[key] = abs(float(m1[key]) - float(m2[key]))
        print(f"NULL FLOOR  pixel RMS {r:.3f}   per-axis: " +
              ", ".join(f"{k}={null_axis[k]:.4g}" for k, _l, _s, _f, _w in AXES))
    else:
        print("NOTE: no null pair on disk — every delta below is reported without a noise floor.")
    print()

    ref = _g.art_measure("REF_mid", _g.CONTROL_ART["REF_mid"]["path"])
    os.makedirs("_fl_v4_20260818", exist_ok=True)
    for s in SCENES:
        sheet(s, plates, ref, null_axis, os.path.join("_fl_v4_20260818", f"sheet-{s}.png"))


if __name__ == "__main__":
    main()
