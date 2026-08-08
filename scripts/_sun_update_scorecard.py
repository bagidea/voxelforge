#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Update the AAA Gap Scorecard with regrade AFTER results.

Reads the regrade JSON (from regrade.py --json) and writes an updated scorecard
that honestly labels every axis PASS/FAIL against the canonical thresholds:
warmth R-B >= 110, blue B <= 10, sat >= 90%, p05-L >= 8%.

Usage:
    python scripts/_sun_update_scorecard.py \
        --json _rose_recap_20260806/regrade_after.json \
        --scorecard docs/aaa-gap-scorecard-2026-08-06.md \
        --out _rose_recap_20260806/aaa-gap-scorecard-after-2026-08-06.md \
        --recap-dir _rose_recap_20260806
"""

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

# Canonical thresholds
THRESHOLDS = {
    "warmth": {"label": "warmth R−B (mid)", "op": ">=", "value": 110, "unit": ""},
    "blue":   {"label": "blue B (mid)",     "op": "<=", "value": 10,  "unit": ""},
    "sat":    {"label": "saturation (mid)",  "op": ">=", "value": 90,  "unit": "%"},
    "p05":    {"label": "interior p05-L",    "op": ">=", "value": 8,   "unit": "%"},
    "penumbra":{"label":"penumbra px",       "op": ">=", "value": 5,   "unit": "px"},
    "micro":  {"label": "micro-contrast",    "op": ">=", "value": 5.0, "unit": ""},
    "p95":    {"label": "highlight p95",     "op": "between", "lo": 150, "hi": 185, "unit": ""},
}


def check(value, spec):
    """Return (PASS/FAIL/SKIP, reason_string)."""
    if value is None:
        return ("SKIP", "N/A")
    op = spec["op"]
    if op == ">=":
        ok = value >= spec["value"]
        return ("✅ PASS" if ok else "❌ FAIL", f"{value:.1f}{spec.get('unit','')} vs ≥{spec['value']}{spec.get('unit','')}")
    elif op == "<=":
        ok = value <= spec["value"]
        return ("✅ PASS" if ok else "❌ FAIL", f"{value:.1f}{spec.get('unit','')} vs ≤{spec['value']}{spec.get('unit','')}")
    elif op == "between":
        ok = spec["lo"] <= value <= spec["hi"]
        return ("✅ PASS" if ok else "❌ FAIL", f"{value:.1f} vs {spec['lo']}–{spec['hi']}")
    return ("SKIP", "unknown op")


def extract_key_values(pair_data):
    """Extract the key numeric values from regrade JSON for a single frame pair."""
    b = pair_data.get("before", {})
    a = pair_data.get("after", {})
    result = {}

    # P0 axes (from grade_axes.py)
    axes_b = b.get("grade_axes.py", {})
    axes_a = a.get("grade_axes.py", {})
    for axis_label in ["warmth R-B (mid)", "blue B (mid)", "saturation (mid)",
                       "micro-contrast", "highlight p95"]:
        bv = axes_b.get(axis_label, {}).get("value")
        av = axes_a.get(axis_label, {}).get("value")
        result[f"axes_{axis_label}"] = {"before": bv, "after": av}

    # G3 depth (from grade_g3.py)
    g3_b = b.get("grade_g3.py", {})
    g3_a = a.get("grade_g3.py", {})
    for k in ["p01", "p05", "p10", "p50", "p99", "dark_RB", "mean_RB"]:
        result[f"g3_{k}"] = {"before": g3_b.get(k), "after": g3_a.get(k)}

    # G3 verdicts
    result["g3_p05_pass"] = {"before": g3_b.get("p05_pass"), "after": g3_a.get("p05_pass")}
    result["g3_warm_pass"] = {"before": g3_b.get("warm_pass"), "after": g3_a.get("warm_pass")}

    # Gates (from grade_gate.py)
    gt_b = b.get("grade_gate.py", {})
    gt_a = a.get("grade_gate.py", {})
    for gate in ["G3", "G5", "G6"]:
        result[f"gate_{gate}"] = {
            "before": gt_b.get(gate, {}).get("pass"),
            "after": gt_a.get(gate, {}).get("pass"),
        }

    # Midtone (from grade_midtone.py)
    mt_b = b.get("grade_midtone.py", {})
    mt_a = a.get("grade_midtone.py", {})
    for k in ["RB", "sat"]:
        result[f"midtone_{k}"] = {"before": mt_b.get(k), "after": mt_a.get(k)}

    # Beauty (from grade_beauty.py)
    beauty_b = b.get("grade_beauty.py", {}).get("cur", {})
    beauty_a = a.get("grade_beauty.py", {}).get("cur", {})
    for k in ["warmth_RB", "sat_mean", "shadow_L_pct", "shadow_RB",
              "highlight_p99", "micro_contrast", "edge_energy"]:
        result[f"beauty_{k}"] = {"before": beauty_b.get(k), "after": beauty_a.get(k)}

    # Penumbra (from measure_penumbra.py)
    pen_b = b.get("measure_penumbra.py", {})
    pen_a = a.get("measure_penumbra.py", {})
    result["penumbra_px"] = {"before": pen_b.get("mean_px"), "after": pen_a.get("mean_px")}

    # G7 vegetation (from grade_g7.py)
    g7_b = b.get("grade_g7.py", {})
    g7_a = a.get("grade_g7.py", {})
    for k in ["veg_sat", "veg_hue", "veg_pass"]:
        result[f"g7_{k}"] = {"before": g7_b.get(k), "after": g7_a.get(k)}

    return result


def delta_str(before, after):
    """Format delta: after - before."""
    if isinstance(before, (int, float)) and isinstance(after, (int, float)):
        d = after - before
        return f"{d:+.1f}"
    return "—"


def pass_str(value, spec_key):
    """Render a PASS/FAIL badge for a value against a threshold spec."""
    if value is None:
        return "—"
    spec = THRESHOLDS.get(spec_key, {})
    if not spec:
        return f"{value:.1f}" if isinstance(value, float) else str(value)
    verdict, _ = check(value, spec)
    return verdict


def build_scorecard(pairs_data, recap_dir):
    """Build the after scorecard markdown from regrade results."""
    today = datetime.now().strftime("%Y-%m-%d")
    lines = []

    lines.append(f"# 🔥 AAA Gap Scorecard — AFTER Rose's fix (bb5c21e)")
    lines.append("")
    lines.append(f"> **วันที่เกรด:** {today} | **ผู้เกรด:** Sun (Specialist, Positive Psychology)  ")
    lines.append(f"> **สคริปต์ที่รัน:** `regrade.py` → `grade_axes.py` · `grade_gate.py` · `grade_g3.py` · `grade_beauty.py` · `grade_midtone.py` · `grade_hero.py` · `grade_look.py` · `measure_penumbra.py` · `grade_g7.py` (axis C)  ")
    lines.append(f"> **สคริปต์ที่ไม่ได้รันรอบนี้:** `grade_web_parity.py` (retired from native regrade — เป็น workflow ของ web store QA ต้องการ wasm build + Chrome+Dawn แยกต่างหาก) · `grade_ref.py`/`grade_ref2.py` (เกรดเฉพาะ golden ref — ref ไม่เปลี่ยน)  ")
    lines.append(f"> **เฟรมที่เกรด:** {len(pairs_data)} คู่ before→after จาก Rose's finisher (`target-rose/release/voxelforge.exe`)  ")
    lines.append("")
    lines.append("---")
    lines.append("")

    # Build frame data
    frames = {}
    for pair in pairs_data:
        label = pair["label"]
        kv = extract_key_values(pair)
        frames[label] = kv

    frame_labels = sorted(frames.keys())

    # ---- Master Table ----
    lines.append("## 📊 ตาราง AFTER — ค่าที่วัดได้ vs เกณฑ์ผ่าน")
    lines.append("")

    # Header
    header = "| แกน | เกณฑ์ผ่าน |"
    for fl in frame_labels:
        header += f" {fl} AFTER | Δ |"
    lines.append(header)

    sep = "|---|"
    for _ in frame_labels:
        sep += "---|"  # simplified — no individual column count needed for md
    lines.append(sep)

    # Build rows
    row_specs = [
        ("warmth R−B (mid)", "warmth", lambda kv: kv.get("axes_warmth R-B (mid)", {}).get("before"),
         lambda kv: kv.get("axes_warmth R-B (mid)", {}).get("after")),
        ("blue B (mid)", "blue", lambda kv: kv.get("axes_blue B (mid)", {}).get("before"),
         lambda kv: kv.get("axes_blue B (mid)", {}).get("after")),
        ("saturation (mid)", "sat", lambda kv: kv.get("axes_saturation (mid)", {}).get("before"),
         lambda kv: kv.get("axes_saturation (mid)", {}).get("after")),
        ("micro-contrast", "micro", lambda kv: kv.get("axes_micro-contrast", {}).get("before"),
         lambda kv: kv.get("axes_micro-contrast", {}).get("after")),
        ("highlight p95", "p95", lambda kv: kv.get("axes_highlight p95", {}).get("before"),
         lambda kv: kv.get("axes_highlight p95", {}).get("after")),
    ]

    for label, spec_key, get_before, get_after in row_specs:
        spec = THRESHOLDS.get(spec_key, {})
        criteria = ""
        if spec:
            if spec["op"] == ">=":
                criteria = f"≥ {spec['value']}{spec.get('unit','')}"
            elif spec["op"] == "<=":
                criteria = f"≤ {spec['value']}{spec.get('unit','')}"
            elif spec["op"] == "between":
                criteria = f"{spec['lo']}–{spec['hi']}{spec.get('unit','')}"

        row = f"| **{label}** | {criteria} |"
        for fl in frame_labels:
            kv = frames[fl]
            bv = get_before(kv)
            av = get_after(kv)
            d = delta_str(bv, av)
            av_str = f"{av:.1f}" if isinstance(av, (int, float)) else "—"
            verdict, _ = check(av, spec) if spec else ("—", "")
            row += f" {av_str} {verdict} | {d} |"
        lines.append(row)

    lines.append("")

    # ---- G3 Deep Dive ----
    lines.append("## 🔬 G3 Deep Dive — Interior Darkness")
    lines.append("")
    header2 = "| เฟรม | p01-L% | p05-L% ≥8%? | p10-L% | p50-L% | darkest R−B | G3 PASS? |"
    lines.append(header2)
    lines.append("|---|---|---|---|---|---|---|")
    for fl in frame_labels:
        kv = frames[fl]
        p05_after = kv.get("g3_p05", {}).get("after")
        p05_v, _ = check(p05_after, THRESHOLDS["p05"]) if p05_after is not None else ("—", "")
        dark_rb_after = kv.get("g3_dark_RB", {}).get("after")
        g3_pass_after = kv.get("g3_p05_pass", {}).get("after") and kv.get("g3_warm_pass", {}).get("after")

        def fmt_v(v):
            return f"{v:.1f}" if isinstance(v, (int, float)) else "—"

        row = (f"| {fl} | {fmt_v(kv.get('g3_p01', {}).get('after'))} "
               f"| {fmt_v(p05_after)} {p05_v} "
               f"| {fmt_v(kv.get('g3_p10', {}).get('after'))} "
               f"| {fmt_v(kv.get('g3_p50', {}).get('after'))} "
               f"| {fmt_v(dark_rb_after)} "
               f"| {'✅ PASS' if g3_pass_after else '❌ FAIL'} |")
        lines.append(row)

    lines.append("")

    # ---- Summary ----
    lines.append("## 📋 สรุปผล AFTER — เทียบเกณฑ์ CEO")
    lines.append("")
    lines.append("| เฟรม | warmth ≥110 | blue ≤10 | sat ≥90% | p05-L ≥8% | G3 | penumbra ≥5px |")
    lines.append("|---|---|---|---|---|---|---|")

    for fl in frame_labels:
        kv = frames[fl]
        w_after = kv.get("axes_warmth R-B (mid)", {}).get("after")
        b_after = kv.get("axes_blue B (mid)", {}).get("after")
        s_after = kv.get("axes_saturation (mid)", {}).get("after")
        p05_after = kv.get("g3_p05", {}).get("after")
        pen_after = kv.get("penumbra_px", {}).get("after")
        g3_pass = kv.get("g3_p05_pass", {}).get("after") and kv.get("g3_warm_pass", {}).get("after")

        w_v, _ = check(w_after, THRESHOLDS["warmth"])
        b_v, _ = check(b_after, THRESHOLDS["blue"])
        s_v, _ = check(s_after, THRESHOLDS["sat"])
        p_v, _ = check(p05_after, THRESHOLDS["p05"]) if p05_after is not None else ("—", "")
        pen_v, _ = check(pen_after, THRESHOLDS["penumbra"]) if pen_after is not None else ("—", "")

        lines.append(f"| {fl} | {w_v} | {b_v} | {s_v} | {p_v} | {'✅' if g3_pass else '❌'} | {pen_v} |")

    lines.append("")

    # ---- Web parity note ----
    lines.append("---")
    lines.append("")
    lines.append("## 📝 หมายเหตุ")
    lines.append("")
    lines.append("- **web parity ไม่ได้วัดรอบนี้** — `grade_web_parity.py` ต้องการ wasm build + Chrome+Dawn + console log + matching native build ซึ่งเป็น workflow แยกของ web store QA ไม่ใช่ native regrade pipeline (ดูรายละเอียดใน header ของ scripts/grade_web_parity.py)")
    lines.append("- **G7 A/B (haze, AO)** — ต้องการ paired frames แบบ toggle layer (`VOXELFORGE_LOOK_HAZE=0` / `VOXELFORGE_LOOK_SSAO=off`) — ไม่ได้อยู่ในชุด recap นี้ (ใช้ `--g7-haze` / `--g7-ao` ใน regrade.py เมื่อพร้อม)")
    lines.append("- **DOF** — skip โดยดีไซน์บนโปรไฟล์ gameplay (fg:bg zones ถูก calibrate สำหรับ indoor 1024² hero shot เท่านั้น; บน gameplay frames ค่า DOF เป็น false FAIL)")
    lines.append("")
    lines.append(f"_เกรดด้วย `regrade.py` (9 สคริปต์) — Sun (Specialist), {today}._")
    lines.append("")

    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", required=True, help="regrade JSON output")
    ap.add_argument("--scorecard", required=True, help="original scorecard (for reference)")
    ap.add_argument("--out", required=True, help="output path for updated scorecard")
    ap.add_argument("--recap-dir", required=True, help="recap directory for context")
    args = ap.parse_args()

    data = json.loads(Path(args.json).read_text(encoding="utf-8"))
    pairs = data.get("pairs", [])

    if not pairs:
        print("ERROR: no pairs in regrade JSON", file=sys.stderr)
        sys.exit(1)

    print(f"Found {len(pairs)} frame pair(s) in regrade JSON")

    # Build and write updated scorecard
    md = build_scorecard(pairs, args.recap_dir)
    Path(args.out).write_text(md, encoding="utf-8")
    print(f"Scorecard written → {args.out}")
    print(f"  {len(md.splitlines())} lines")


if __name__ == "__main__":
    main()
