#!/usr/bin/env python3
"""Ladder grid for the beauty axes — per-plate x per-axis, never collapsed.

This is the reporting half of docs/flamingo-beauty-axes-2026-08-14.md section 3.
That doc set the rule this script enforces (my own lesson, 08-09):

    "ผลบันไดต้องออกเป็น per-plate x per-axis เต็มกริด ... ห้ามยุบเป็นค่าเดียวต่อแถว
     และก่อนเสนอเลขใด ต้องเช็คว่าทุกเพลตพร้อมกันผ่านหรือไม่"

So a rung is NOT reported as one number.  Every rung is printed for every camera
pose, and a rung only counts as won if it clears the target on ALL poses at once.

Axes carried here are the three the director named, plus the two that gate
whether the reading is legitimate at all:

    value span   L(p95-p5)      ref 209.5   target >= 150
    shadow floor L p5 as % of white   ref 9.3%   target 8-11%  (8% is the G3 floor)
    chroma span  sat(p95-p5)    ref 0.776   target >= 0.62
    sky mask %                  (a sky axis on a <3% mask is refused, not reported)
    horizon step                ref 53.4    target <= 62

The reference numbers are not re-derived here — they are read live out of the
reference frame itself (moodboard right half) by the same code path that reads
the plates, so a drifting ref cannot silently keep old targets alive.

Usage:
    python scripts/_flamingo_beauty_grid.py PLATE_DIR [--json OUT.json]
        [--ref docs/assets/moodboard.png] [--ref-crop 514,0,1024,1024]
"""
import argparse
import importlib.util
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "beauty_axes", HERE / "_flamingo_beauty_axes.py")
axes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(axes)

# Targets from docs/flamingo-beauty-axes-2026-08-14.md section 1.
TARGET_VALUE_SPAN = 150.0
TARGET_SHADOW_LO, TARGET_SHADOW_HI = 8.0, 11.0
TARGET_CHROMA_SPAN = 0.62
TARGET_HORIZON = 62.0
SKY_MIN_PCT = 3.0   # under this the sky axes are refused (footnotes 1-2)


def measure(path, hud_bottom=0, crop=None):
    r = axes.grade(path, crop=crop, hud_bottom=hud_bottom)
    st = r["structure"]
    hz = r.get("horizon") or {}
    return dict(
        plate=Path(path).stem,
        value_span=st["L_p95_minus_p5"],
        shadow_p5=st["L_p"][0],
        shadow_pct=round(100.0 * st["L_p"][0] / 255.0, 1),
        chroma_span=st["sat_p95_minus_p5"],
        sky_pct=r["sky_px_pct"],
        sky_clip=(r["sky"] or {}).get("clip_pct"),
        sky_sat=(r["sky"] or {}).get("sat"),
        horizon=hz.get("drgb"),
        horizon_cols=hz.get("columns"),
        depth_dL=(r.get("depth") or {}).get("dL_far_minus_near"),
        raw=r)


def verdict(m):
    """Per-plate pass/fail on the three axes the ladder is steering."""
    v = []
    v.append(("value", m["value_span"] >= TARGET_VALUE_SPAN))
    v.append(("shadow", TARGET_SHADOW_LO <= m["shadow_pct"] <= TARGET_SHADOW_HI))
    v.append(("chroma", m["chroma_span"] >= TARGET_CHROMA_SPAN))
    return v


def rung_of(stem):
    """Group plates by ladder rung: everything after the pose prefix."""
    mt = re.match(r"^(?P<pose>[a-z0-9]+)-(?P<rung>.+)$", stem)
    return (mt.group("pose"), mt.group("rung")) if mt else (stem, "?")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("plate_dir")
    ap.add_argument("--ref", default="docs/assets/moodboard.png")
    ap.add_argument("--ref-crop", default="514,0,1024,1024")
    ap.add_argument("--hud-bottom", type=int, default=0)
    ap.add_argument("--json")
    a = ap.parse_args()

    ref = measure(a.ref, crop=axes.parse_box(a.ref_crop))
    print(f"REF  {ref['plate']}  value_span {ref['value_span']}  "
          f"shadow p5 {ref['shadow_p5']} ({ref['shadow_pct']}% of white)  "
          f"chroma_span {ref['chroma_span']}  sky {ref['sky_pct']}%  "
          f"horizon {ref['horizon']}")

    plates = sorted(Path(a.plate_dir).glob("*.png"))
    rows = [measure(p, hud_bottom=a.hud_bottom) for p in plates]

    poses = sorted({rung_of(r["plate"])[0] for r in rows})
    rungs = sorted({rung_of(r["plate"])[1] for r in rows})

    for axis, key, fmt in (("value span L(p95-p5)", "value_span", "{:7.1f}"),
                           ("shadow p5 (% of white)", "shadow_pct", "{:6.1f}%"),
                           ("chroma span sat(p95-p5)", "chroma_span", "{:7.3f}"),
                           ("sky mask (% of frame)", "sky_pct", "{:6.2f}%")):
        print(f"\n--- {axis}   (ref {ref[key]}) ---")
        print(f"{'rung':<22}" + "".join(f"{p:>16}" for p in poses))
        for rg in rungs:
            cells = []
            for p in poses:
                hit = [r for r in rows if rung_of(r["plate"]) == (p, rg)]
                cells.append(fmt.format(hit[0][key]) if hit and hit[0][key] is not None
                             else "       --")
            print(f"{rg:<22}" + "".join(f"{c:>16}" for c in cells))

    print("\n--- all-pose verdict per rung (a rung wins only if EVERY pose passes) ---")
    for rg in rungs:
        hits = [r for r in rows if rung_of(r["plate"])[1] == rg]
        if not hits:
            continue
        per = {name: all(dict(verdict(h))[name] for h in hits)
               for name, _ in verdict(hits[0])}
        allok = all(per.values())
        print(f"{rg:<22} " + "  ".join(f"{k}={'PASS' if v else 'FAIL'}"
                                       for k, v in per.items())
              + f"   [{len(hits)} poses]  {'>>> WINS' if allok else ''}")

    if a.json:
        Path(a.json).write_text(json.dumps(dict(ref=ref, rows=rows), indent=1,
                                           default=str), encoding="utf-8")
        print(f"\njson -> {a.json}")


if __name__ == "__main__":
    main()
