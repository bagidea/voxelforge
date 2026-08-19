#!/usr/bin/env python3
# ===========================================================================
# Poppy — the MAT_MAPS contact sheet, shot on the SHIPPED 64px art.
#
# Three scenes down, three panels across: OFF (derived maps only), ON (the
# authored _n/_r), and a x8 amplified |on-off| so the difference is visible at
# a glance instead of only in a number. The 16px-proxy sheet
# (`_poppy_matmaps_sheet.py`) has two panels and no diff column; the whole
# question this time is "did the real art flip the result", and a reader who
# has to squint between two thumbnails cannot answer it.
#
# EVERY NUMBER IS RECOMPUTED HERE from the PNGs named on the sheet, and the
# verdict strip is read out of the judge's own matmaps64-verdict.json rather
# than retyped — a caption that can disagree with the gate is a caption that
# eventually will (2026-08-18 lesson: the sheet outlived the run that made it).
#
# USAGE
#   python scripts/_poppy_matmaps64_sheet.py [out.png]
# ===========================================================================
import datetime
import hashlib
import json
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUB = os.path.join(ROOT, "docs", "assets", "look")
VER = os.path.join(ROOT, "_poppy_matmaps", "verdict64")
EXE = os.path.join(ROOT, "target-poppy", "release", "voxelforge.exe")

SCENES = [
    ("day", "outdoor noon  SUN=66,205,20000  EXPOSURE=10.6"),
    ("evening-raking", "raking evening  (look.rs hour default)"),
    ("night-firelit", "night, lit by fire  LOOK_NIGHT=1"),
]
COL_W = 470
DIFF_GAIN = 8.0


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)


def luma(a):
    return a[:, :, 0] * 0.2126 + a[:, :, 1] * 0.7152 + a[:, :, 2] * 0.0722


def lap(y):
    return np.abs(4 * y[1:-1, 1:-1] - y[:-2, 1:-1] - y[2:, 1:-1] - y[1:-1, :-2] - y[1:-1, 2:])


def font(sz):
    for f in ("C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/arial.ttf"):
        if os.path.isfile(f):
            return ImageFont.truetype(f, sz)
    return ImageFont.load_default()


def fit(im, w):
    return im.resize((w, int(im.height * w / im.width)), Image.LANCZOS)


def exe_line():
    if not os.path.isfile(EXE):
        return "binary: MISSING"
    h = hashlib.sha256(open(EXE, "rb").read()).hexdigest()[:16].upper()
    t = datetime.datetime.fromtimestamp(os.path.getmtime(EXE))
    return ("binary: target-poppy/release/voxelforge.exe  built %s  sha256 %s…"
            % (t.strftime("%Y-%m-%d %H:%M:%S"), h))


def verdict_of(scene):
    p = os.path.join(VER, scene, "matmaps-verdict.json")
    if not os.path.isfile(p):
        return None
    return json.load(open(p))


def crit(v, prefix):
    """The one criterion row whose name starts with `prefix`, or None."""
    if not v:
        return None
    for c in v["criteria"]:
        if c["name"].startswith(prefix):
            return c
    return None


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(PUB, "matmaps64_contact_sheet.png")
    f_hdr, f_cap, f_sm = font(23), font(17), font(14)

    rows = []
    for scene, hour in SCENES:
        off_p = os.path.join(PUB, f"matmaps64_{scene}_off.png")
        on_p = os.path.join(PUB, f"matmaps64_{scene}_on.png")
        off_i, on_i = Image.open(off_p).convert("RGB"), Image.open(on_p).convert("RGB")
        a, b = load(on_p), load(off_p)
        d = np.abs(a - b)
        diff_i = Image.fromarray(np.clip(d * DIFF_GAIN, 0, 255).astype(np.uint8))
        ha, hb = lap(luma(a)).mean(), lap(luma(b)).mean()
        rows.append({
            "scene": scene, "hour": hour,
            "thumbs": [fit(off_i, COL_W), fit(on_i, COL_W), fit(diff_i, COL_W)],
            "mad": d.mean(axis=2).mean(),
            "chg": 100.0 * (d.mean(axis=2) > 1.0).mean(),
            "ha": ha, "hb": hb, "hpd": 100.0 * (ha - hb) / hb,
            "v": verdict_of(scene),
        })

    tw, th = rows[0]["thumbs"][0].size
    pad, head, cap_h, foot = 14, 104, 64, 76
    W = pad * 4 + tw * 3
    H = head + sum(cap_h + th + pad for _ in rows) + foot
    sheet = Image.new("RGB", (W, H), (17, 17, 20))
    dr = ImageDraw.Draw(sheet)

    dr.text((pad, 10), "VOXELFORGE_MAT_MAPS on the SHIPPED 64px art — authored _n/_r vs derived, ONE binary",
            (240, 240, 240), font=f_hdr)
    dr.text((pad, 40), exe_line(), (170, 170, 175), font=f_sm)
    dr.text((pad, 58), "shared env: PLAY=1 NOHUD=1 LOOK_QUALITY=ultra CINE_START=1.0  |  "
                       "ATLAS_DIR unset -> assets/textures/blocks (manifest tile_px 64)", (170, 170, 175), font=f_sm)
    dr.text((pad, 76), "left = MAT_MAPS=off (derived only)   middle = MAT_MAPS unset (authored)   "
                       f"right = |on-off| x{DIFF_GAIN:.0f}", (170, 170, 175), font=f_sm)

    y = head
    for r in rows:
        v = r["v"]
        m1, m4 = crit(v, "M1"), crit(v, "M4")
        tag = v["verdict"] if v else "not judged"
        col = (150, 220, 150) if tag == "PASS" else ((235, 140, 140) if tag == "FAIL" else (170, 170, 175))
        dr.text((pad, y + 2), f"{r['scene']}   {r['hour']}", (235, 235, 235), font=f_cap)
        dr.text((pad + tw * 2 + pad * 2, y + 2), f"VERDICT: {tag}", col, font=f_cap)
        dr.text((pad, y + 23), f"mean|on-off| {r['mad']:.2f} lv   pixels changed {r['chg']:.1f}%   "
                               f"local contrast (mean |Laplacian| of luma) {r['hb']:.2f} -> {r['ha']:.2f} "
                               f"= {r['hpd']:+.1f}%", (150, 200, 150), font=f_sm)
        dr.text((pad, y + 41), ("M1 " + m1["detail"]) if m1 else "M1 —",
                (150, 220, 150) if (m1 and m1["ok"]) else (235, 150, 150), font=f_sm)
        y += cap_h
        for i, im in enumerate(r["thumbs"]):
            sheet.paste(im, (pad + i * (tw + pad), y))
        for i, (lbl, c) in enumerate((("MAT_MAPS=off", (255, 180, 180)),
                                      ("MAT_MAPS on (authored)", (255, 235, 140)),
                                      (f"|on-off| x{DIFF_GAIN:.0f}", (170, 210, 255)))):
            dr.text((pad + i * (tw + pad) + 8, y + 6), lbl, c, font=f_cap)
        dr.text((pad + 8, y + th - 24), ("M4 " + m4["detail"]) if m4 else "M4 —",
                (150, 220, 150) if (m4 and m4["ok"]) else (235, 150, 150), font=f_sm)
        y += th + pad

    dr.text((pad, y + 4), "shoot  scripts/_poppy_matmaps64_shoot.sh   judge  scripts/_fl_matmaps_judge.py (608eb60/d7f95bd, unmodified)"
                          "   sheet  scripts/_poppy_matmaps64_sheet.py", (140, 140, 145), font=f_sm)
    dr.text((pad, y + 24), "plates docs/assets/look/matmaps64_*.png   logs _poppy_matmaps/plates64/*.log   "
                           "verdicts _poppy_matmaps/verdict64/<scene>/matmaps-verdict.json", (140, 140, 145), font=f_sm)
    dr.text((pad, y + 44), "Each row's floor is its OWN null pair (on shot twice, identical env) — no borrowed floors.",
            (140, 140, 145), font=f_sm)

    sheet.save(out)
    print(f"{out}  {sheet.size[0]}x{sheet.size[1]}")


if __name__ == "__main__":
    main()
