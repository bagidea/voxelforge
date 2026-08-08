#!/usr/bin/env python3
"""Turn the character grade into the two things a non-engineer can act on:

  1. `concept-vs-ingame.png` — the approved Monanisa sheet and the SAME character
     as it actually renders in `--play`, side by side at the same body height.
     Nothing is retouched; the right panel is a crop of the graded frame.
  2. A two-sided MATCH table. `grade_character.py` gates most axes one-sided
     (">= 0.85 x concept"), which is right for a gate — a character may not be
     duller than the sheet — but it cannot say "this is 2.5x too orange", and on
     this render three axes pass while sitting far ABOVE the sheet. Here every
     axis is scored `min(v/ref, ref/v)`: 1.00 = on the sheet, and BOTH directions
     of drift cost you. That is the number to quote as "% match".

Usage: _flamingo_char_report.py            (reads _fl_char/*.json, writes into _fl_char/)
"""
import json
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
OUT = os.path.join(ROOT, "_fl_char")
sys.path.insert(0, HERE)

import grade_character as gc  # noqa: E402  (needs HERE on the path)

# Axes that describe the CHARACTER AS ART, in the order a reviewer reads them.
# `pen` is excluded: it measured 0 contact-shadow rows on both frames (the body
# is cropped by the frame bottom), so it has no value to compare.
AXES = [
    ("warmth", "ความอุ่นของสี (R-B)"),
    ("blue",   "ช่องสีฟ้าในเงา"),
    ("sat",    "ความอิ่มสี"),
    ("micro",  "รายละเอียดพื้นผิว (micro-contrast)"),
    ("p95",    "ไฮไลต์สว่างสุด"),
    ("p05L",   "ความลึกของเงา"),
    ("darkRB", "โทนของเงาที่มืดที่สุด"),
    ("edge",   "ความเด้งของ silhouette"),
    ("form",   "ช่วงแสง-เงาบนตัว (form)"),
    ("iso",    "ความซับซ้อนของ silhouette"),
    ("mats",   "จำนวนวัสดุที่แยกออกได้"),
]


def two_sided(v, ref):
    """1.00 = identical to the sheet; both too-little and too-much cost you."""
    if v is None or ref is None:
        return None
    if v == 0 and ref == 0:
        return 1.0
    if v <= 0 or ref <= 0:
        # An axis pinned at zero (e.g. the blue channel fully crushed) is a total
        # miss, not a divide-by-zero to be hidden.
        return 0.0
    return min(v / ref, ref / v)


def font(size):
    # Leelawadee UI / Tahoma first: Segoe UI ships no Thai coverage that PIL can
    # reach without libraqm, and the captions came out as tofu boxes.
    for p in (r"C:\Windows\Fonts\leelawui.ttf", r"C:\Windows\Fonts\tahoma.ttf",
              r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\arial.ttf"):
        if os.path.exists(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def crop_character(frame_png, cam, actor="player", hud_crop=80, pad=0.30):
    """Crop the analytic body box out of a graded frame — same projection the
    grader masks with, so the crop and the numbers describe the same pixels."""
    rgb = np.asarray(Image.open(frame_png).convert("RGB"))
    h, w, _ = rgb.shape
    yaw, pitch, dist = [float(v) for v in cam.split(",")]
    (x0, y0, x1, y1), _foot = gc.analytic_bbox(w, h, yaw, pitch, dist, actor, hud_crop)
    bw, bh = x1 - x0, y1 - y0
    X0 = int(max(0, x0 - bw * pad))
    X1 = int(min(w, x1 + bw * pad))
    Y0 = int(max(0, y0 - bh * pad))
    Y1 = int(min(h, y1 + bh * pad))
    return Image.open(frame_png).convert("RGB").crop((X0, Y0, X1, Y1))


def panel(img, height, title, sub, note=None):
    """Scale to a common body height and caption it."""
    s = height / img.height
    img = img.resize((max(1, int(img.width * s)), height), Image.LANCZOS)
    pad_top = 96
    pad_bot = 44 if note else 0
    out = Image.new("RGB", (img.width, height + pad_top + pad_bot), (24, 24, 28))
    out.paste(img, (0, pad_top))
    d = ImageDraw.Draw(out)
    d.text((18, 18), title, font=font(34), fill=(255, 255, 255))
    d.text((18, 60), sub, font=font(21), fill=(170, 175, 185))
    if note:
        d.text((18, height + pad_top + 10), note, font=font(22), fill=(255, 190, 90))
    return out


def main():
    ref = json.load(open(os.path.join(OUT, "ref-auren-hero.json"), encoding="utf-8"))["metrics"]
    shots = []
    for name in ("char-medium", "char-closeup"):
        p = os.path.join(OUT, f"{name}.json")
        if os.path.exists(p):
            shots.append((name, json.load(open(p, encoding="utf-8"))))

    lines = []
    scores = {}
    for name, sj in shots:
        m = sj["metrics"]
        lines.append(f"\n=== {name}   cam={sj['cam']} ===")
        lines.append(f"  {'แกน':<36}{'ในเกม':>10}{'คอนเซ็ปต์':>12}{'match':>9}   ทิศทางที่หลุด")
        vals = []
        for key, th in AXES:
            v, r = m.get(key), ref.get(key)
            sc = two_sided(v, r)
            if sc is None:
                continue
            vals.append(sc)
            if v is not None and r is not None and r > 0 and v > 0:
                drift = "สูงเกิน" if v > r else "ต่ำเกิน"
                if abs(v - r) / r < 0.10:
                    drift = "ตรง"
            else:
                drift = "ศูนย์/หาย"
            lines.append(f"  {th:<36}{v:10.2f}{r:12.2f}{sc * 100:8.0f}%   {drift}")
        scores[name] = float(np.mean(vals)) * 100
        lines.append(f"  -> MATCH รวม (เฉลี่ยสองทาง {len(vals)} แกน): {scores[name]:.0f} %")

    report = "\n".join(lines)
    print(report)
    with open(os.path.join(OUT, "match-report.txt"), "w", encoding="utf-8") as f:
        f.write(report + "\n")

    # ---- the picture -------------------------------------------------------
    H = 900
    concept = Image.open(os.path.join(ROOT, "docs", "assets", "characters",
                                      "auren-hero-concept.png")).convert("RGB")
    left = panel(concept, H, "CONCEPT (อนุมัติแล้ว)",
                 "docs/assets/characters/auren-hero-concept.png",
                 note="ผม · เคป · เสื้อคลุม · เข็มขัด · โคมไฟ · ดาบ · บูท = 6 วัสดุ")

    game_json = dict(shots)["char-medium"]
    game = crop_character(os.path.join(OUT, "char-medium-nohud2.png"), game_json["cam"])
    right = panel(game, H, "IN-GAME (เรนเดอร์จริงวันนี้)",
                  f"_fl_char/char-medium-nohud2.png  ·  match {scores['char-medium']:.0f} %",
                  note="กล่องหัว + กล่องตัว + แขนสั้น = 4 วัสดุ · ขาหลุดขอบล่างเฟรม")

    gap = 16
    canvas = Image.new("RGB", (left.width + gap + right.width,
                               max(left.height, right.height)), (24, 24, 28))
    canvas.paste(left, (0, 0))
    canvas.paste(right, (left.width + gap, 0))
    dst = os.path.join(OUT, "concept-vs-ingame.png")
    canvas.save(dst)
    print(f"\nwrote {dst}  ({canvas.width}x{canvas.height})")


if __name__ == "__main__":
    main()
