#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Unified regrade — one command to grade ALL look axes on before/after frame pairs.

Shells out to every grade script (the single source of truth for each number) and
prints a comparison table so you see every axis move in one glance.

Usage:
    # Compare two frame folders (matches frames by filename):
    scripts/regrade.py --before frames/before/ --after frames/after/

    # Quick single-pair compare:
    scripts/regrade.py before-nohud2.png after-nohud2.png

    # Gameplay profile (skips DOF, which fails on non-hero framing by design):
    scripts/regrade.py --before before/ --after after/ --profile gameplay

    # Master Table — one cross-tabulated table (axes × frames, ✅/❌ every cell):
    scripts/regrade.py --before frames/ --all

    # Output JSON for automated scorecard generation:
    scripts/regrade.py --before before/ --after after/ --json report.json

Scripts run (8 active + 1 supplemental):
    grade_axes.py          P0 6-axis PASS/FAIL grid
    grade_gate.py          G3/G5/G6 canonical gates
    grade_g3.py            G3 deep-dive (p01..p99 + darkest shade)
    grade_beauty.py        full-frame delta vs golden ref
    grade_midtone.py       midtone-band R-B + saturation
    grade_hero.py          G6 cross-check, G3/G5 detail
    grade_look.py          G4a penumbra, G2 key-dir, G1 visual flag
    measure_penumbra.py    raw penumbra px widths
    grade_g7.py            vegetation colour (axis C); A2/B need A/B pairs

NOT run (separate workflows):
    grade_web_parity.py    RETIRED from native regrade — needs wasm build + Chrome+Dawn
    grade_ref.py           grades only the golden ref (doesn't change)
    grade_ref2.py          grades only the golden ref (doesn't change)

G7 A/B axes (aerial perspective, AO bite) need paired frames shot with
VOXELFORGE_LOOK_HAZE=0 / VOXELFORGE_LOOK_SSAO=off. See --g7-haze and --g7-ao.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import OrderedDict
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent

# Windows console may not support UTF-8; force it.
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

# ---------------------------------------------------------------------------
# Per-script parser: each returns a dict of extracted values (or None on failure)
# ---------------------------------------------------------------------------

def _run(script, *args):
    """Run a grade script, return (returncode, stdout)."""
    cmd = [sys.executable, str(SCRIPTS / script), *args]
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    return p.returncode, p.stdout + p.stderr


def _parse_axes(output):
    """Parse grade_axes.py output -> {axis: {value, pass, target}}."""
    axes = {}
    for line in output.splitlines():
        # [PASS] warmth R-B (mid)     128.80   target      >= 110   (REF 120.9)
        # [FAIL] blue B (mid)           16.10   target       <= 10   (REF 4.3)
        m = re.search(
            r"\[(PASS|FAIL|SKIP)\]\s+(.+?)\s{2,}([\d.]+)\s+target\s+(.+?)\s{2,}\(REF\s+([\d.]+)\)",
            line,
        )
        if m:
            tag, label, value, target, ref = m.groups()
            axes[label.strip()] = {
                "value": float(value),
                "pass": tag == "PASS",
                "target": target.strip(),
                "ref": float(ref),
            }
    return axes


def _parse_gate(output):
    """Parse grade_gate.py output -> {G3/G5/G6: {pass, detail...}}."""
    gates = {}
    # G3 block
    m = re.search(r"## G3\b.*?-> (PASS|FAIL)", output, re.S)
    if m:
        g3 = {"pass": m.group(1) == "PASS"}
        for k, pat in [
            ("p05", r"interior p05-L=([\d.]+)%"),
            ("p50", r"p50-L=([\d.]+)%"),
            ("dark_r", r"RGB=\((\d+),"),
            ("dark_g", r"RGB=\(\d+,(\d+),"),
            ("dark_b", r"RGB=\(\d+,\d+,(\d+)\)"),
            ("dark_rb", r"R-B=([+-]?\d+\.?\d*)"),
        ]:
            dm = re.search(pat, output)
            if dm:
                g3[k] = float(dm.group(1))
        gates["G3"] = g3

    # G5 block
    m = re.search(r"## G5\b.*?-> (PASS|FAIL)", output, re.S)
    if m:
        gates["G5"] = {"pass": m.group(1) == "PASS"}

    # G6 block
    m = re.search(r"## G6\b.*?-> (PASS|FAIL)", output, re.S)
    if m:
        gates["G6"] = {"pass": m.group(1) == "PASS"}

    return gates


def _parse_g3(output):
    """Parse grade_g3.py output -> {p01..p99, darkest shade detail, g3_pass}."""
    result = {}
    for p in [1, 5, 10, 50, 90, 99]:
        m = re.search(rf"p{p:02d} L =\s*([\d.]+)%", output)
        if m:
            result[f"p{p:02d}"] = float(m.group(1))
    m = re.search(r"RGB=\(([\d.]+),([\d.]+),([\d.]+)\)\s+L=([\d.]+)%\s+warm.*R-B=([+-]\d+\.?\d*)", output)
    if m:
        result["dark_r"] = float(m.group(1))
        result["dark_g"] = float(m.group(2))
        result["dark_b"] = float(m.group(3))
        result["dark_L"] = float(m.group(4))
        result["dark_RB"] = float(m.group(5))
    # frame means
    m = re.search(r"R=([\d.]+) G=([\d.]+) B=([\d.]+)\s+\(R-B=([+-]\d+\.?\d*)\)", output)
    if m:
        result["mean_R"] = float(m.group(1))
        result["mean_G"] = float(m.group(2))
        result["mean_B"] = float(m.group(3))
        result["mean_RB"] = float(m.group(4))
    # G3 verdict
    m = re.search(r"p05-L >= 8%.*-> (PASS|FAIL)", output)
    if m:
        result["p05_pass"] = m.group(1) == "PASS"
    m = re.search(r"darkest shade warm.*-> (PASS|FAIL)", output)
    if m:
        result["warm_pass"] = m.group(1) == "PASS"
    return result


def _parse_beauty(output):
    """Parse grade_beauty.py output -> {ref: {…}, cur: {…}}."""
    result = {}
    current_section = None
    current = {}
    for line in output.splitlines():
        if line.startswith("===== GOLDEN REF ====="):
            current_section = "ref"
            current = {}
        elif line.startswith("===== CURRENT"):
            if current_section == "ref":
                result["ref"] = current
            current_section = "cur"
            current = {}
        elif current_section:
            # Lum: mean 70.1  median 68.0 ...
            m = re.match(r"Lum:\s+mean\s+([\d.]+)", line)
            if m:
                current["lum_mean"] = float(m.group(1))
            m = re.search(r"p05\s+([\d.]+)\s+\(([\d.]+)%\)", line)
            if m:
                current["lum_p05"] = float(m.group(1))
                current["lum_p05_pct"] = float(m.group(2))
            m = re.search(r"p95\s+([\d.]+)", line)
            if m:
                current["lum_p95"] = float(m.group(1))
            m = re.search(r"Warmth R-B:\s+mean\s+([+-]?[\d.]+)", line)
            if m:
                current["warmth_RB"] = float(m.group(1))
            m = re.search(r"Saturation:\s+mean\s+([\d.]+)%", line)
            if m:
                current["sat_mean"] = float(m.group(1))
            m = re.search(r"Shadow.*R-B\s+([+-]?[\d.]+)", line)
            if m:
                current["shadow_RB"] = float(m.group(1))
            m = re.search(r"Shadow.*L\s+([\d.]+)\s+\(([\d.]+)%\)", line)
            if m:
                current["shadow_L"] = float(m.group(1))
                current["shadow_L_pct"] = float(m.group(2))
            m = re.search(r"Highlight:\s+p99\s+([\d.]+)", line)
            if m:
                current["highlight_p99"] = float(m.group(1))
            m = re.search(r"Micro-contrast.*?:\s+([\d.]+)", line)
            if m:
                current["micro_contrast"] = float(m.group(1))
            m = re.search(r"Edge energy.*?:\s+([\d.]+)", line)
            if m:
                current["edge_energy"] = float(m.group(1))
    if current_section == "cur":
        result["cur"] = current
    return result


def _parse_midtone(output):
    """Parse grade_midtone.py output -> {R, G, B, R-B, sat}."""
    result = {}
    for line in output.splitlines():
        m = re.match(
            r"\[.+\] band L\[\d+-\d+pct\].*?"
            r"R\s+([\d.]+)\s+G\s+([\d.]+)\s+B\s+([\d.]+)\s+"
            r"R-B\s+([+-]?[\d.]+)\s+sat\s+([\d.]+)%",
            line,
        )
        if m:
            result["R"] = float(m.group(1))
            result["G"] = float(m.group(2))
            result["B"] = float(m.group(3))
            result["RB"] = float(m.group(4))
            result["sat"] = float(m.group(5))
            return result
    return result


def _parse_hero(output):
    """Parse grade_hero.py output -> {G3, G5, G6 verdicts + detail}."""
    result = {}
    for g in ("G3", "G5", "G6"):
        m = re.search(rf"\[{g}\].*?({{PASS|FAIL}})", output)
        if m:
            result[f"{g}_pass"] = "PASS" in m.group(1)
    return result


def _parse_look(output):
    """Parse grade_look.py output -> {G3, G4a (penumbra info), G5}."""
    result = {}
    for g in ("G3", "G5"):
        m = re.search(rf"## {g}\b.*?-> (PASS|FAIL)", output, re.S)
        if m:
            result[f"{g}_pass"] = m.group(1) == "PASS"
    # G4a penumbra line
    m = re.search(r"## G4a.*?penumbra[^\n]*", output, re.S)
    if m:
        result["G4a_info"] = m.group(0).strip()
    return result


def _parse_penumbra(output):
    """Parse measure_penumbra.py output -> {mean_px, edges, rows}."""
    result = {}
    m = re.search(r"mean penumbra = ([\d.]+)px", output)
    if m:
        result["mean_px"] = float(m.group(1))
    m = re.search(r"edges=(\d+)", output)
    if m:
        result["edges"] = int(m.group(1))
    return result


def _parse_g7(output):
    """Parse grade_g7.py --frame output -> {veg_sat, veg_hue, veg_pass, ...}."""
    result = {}
    # Axis C
    m = re.search(r"saturation\s+([\d.]+)%\s+need", output)
    if m:
        result["veg_sat"] = float(m.group(1))
    m = re.search(r"hue\s+([\d.]+)deg\s+need", output)
    if m:
        result["veg_hue"] = float(m.group(1))
    m = re.search(r"gap closed:\s+saturation\s+([\d.]+)%\s+hue\s+([\d.]+)%", output)
    if m:
        result["veg_gap_closed_sat"] = float(m.group(1))
        result["veg_gap_closed_hue"] = float(m.group(2))
    m = re.search(r"## C\b.*?-> (PASS|FAIL)", output, re.S)
    if m:
        result["veg_pass"] = m.group(1) == "PASS"
    # Axis A2
    m = re.search(r"## A2\b.*?-> (PASS|FAIL)", output, re.S)
    if m:
        result["haze_pass"] = m.group(1) == "PASS"
    m = re.search(r"far/near ratio\s+([\d.]+)", output)
    if m:
        result["haze_ratio"] = float(m.group(1))
    # Axis B
    m = re.search(r"## B\b.*?-> (PASS|FAIL)", output, re.S)
    if m:
        result["ao_pass"] = m.group(1) == "PASS"
    m = re.search(r"darkening L:.*?p99\s+([\d.]+)", output)
    if m:
        result["ao_p99"] = float(m.group(1))
    return result


# ---------------------------------------------------------------------------
# Main regrade logic
# ---------------------------------------------------------------------------

# Scripts run on every frame (no special args)
PER_FRAME_SCRIPTS = [
    ("grade_axes.py",     _parse_axes,     []),
    ("grade_gate.py",     _parse_gate,     []),
    ("grade_g3.py",       _parse_g3,       []),
    ("grade_beauty.py",   _parse_beauty,   []),
    ("grade_midtone.py",  _parse_midtone,  []),
    ("grade_hero.py",     _parse_hero,     []),
    ("grade_look.py",     _parse_look,     []),
    ("measure_penumbra.py", _parse_penumbra, []),
]

# grade_g7.py runs with --frame (axis C only; A2/B need paired frames)
G7_SCRIPT = ("grade_g7.py", _parse_g7)


def grade_one_frame(path, profile="gameplay", extra_args=None):
    """Run all grade scripts on one frame, return aggregated dict."""
    path = str(path)
    results = {}
    for script, parser, fixed_args in PER_FRAME_SCRIPTS:
        args = list(fixed_args)
        if script == "grade_axes.py":
            args.extend(["--profile", profile])
        args.append(path)
        try:
            rc, out = _run(script, *args)
            parsed = parser(out) if rc in (0, 1) else None  # exit 1 = FAIL but data is valid
            if parsed:
                results[script] = parsed
            else:
                results[script] = {"_error": f"parse failed (rc={rc})", "_raw": out[:500]}
        except Exception as e:
            results[script] = {"_error": str(e)}
    # G7 axis C (vegetation) — same frame
    try:
        rc, out = _run(*G7_SCRIPT[0], "--frame", path)
        parsed = G7_SCRIPT[1](out) if rc in (0, 1) else None
        if parsed:
            results["grade_g7.py"] = parsed
        else:
            results["grade_g7.py"] = {"_error": f"parse failed (rc={rc})", "_raw": out[:500]}
    except Exception as e:
        results["grade_g7.py"] = {"_error": str(e)}
    return results


def find_pairs(before_dir, after_dir):
    """Find matching frame pairs by filename (strip -nohud2 suffix for matching)."""
    before_dir = Path(before_dir)
    after_dir = Path(after_dir)
    before_files = {}
    after_files = {}

    for p in before_dir.glob("*-nohud2.png"):
        key = p.name.replace("-nohud2.png", "")
        before_files[key] = p
    for p in after_dir.glob("*-nohud2.png"):
        key = p.name.replace("-nohud2.png", "")
        after_files[key] = p

    # Only pairs where both exist
    common = sorted(set(before_files) & set(after_files))
    pairs = [(before_files[k], after_files[k]) for k in common]

    only_before = sorted(set(before_files) - set(after_files))
    only_after = sorted(set(after_files) - set(before_files))

    return pairs, only_before, only_after


# ---------------------------------------------------------------------------
# Table formatting
# ---------------------------------------------------------------------------

def _v(d, *keys, default="—"):
    """Walk nested dict keys, return value or default."""
    for k in keys:
        if isinstance(d, dict):
            d = d.get(k, {})
        else:
            return default
    return d if d != {} else default


def _fmt(v, width=7, dec=1):
    """Format a number for table column."""
    if v is None or v == "—":
        return f"{'—':>{width}}"
    if isinstance(v, float):
        return f"{v:{width}.{dec}f}"
    if isinstance(v, bool):
        return f"{'✅' if v else '❌':>{width}}"
    return f"{str(v):>{width}}"


def _delta(before, after, key_path, pct=False):
    """Compute delta: after - before."""
    b = _v(before, *key_path)
    a = _v(after, *key_path)
    if isinstance(b, (int, float)) and isinstance(a, (int, float)):
        d = a - b
        if pct:
            return f"{d:+.1f}pp"
        return f"{d:+.1f}"
    return "—"


def print_comparison(before_results, after_results, labels=("BEFORE", "AFTER")):
    """Print a comprehensive before/after comparison table."""

    def g(script, *keys):
        """Get value from results dict."""
        return _v(before_results.get(script, {}), *keys), _v(after_results.get(script, {}), *keys)

    def row(name, script, *keys, fmt_spec="7.1f", pct=False, cmp=None):
        b, a = g(script, *keys)
        b_s = f"{b:{fmt_spec}}" if isinstance(b, (int, float)) else f"{'—':>7}"
        a_s = f"{a:{fmt_spec}}" if isinstance(a, (int, float)) else f"{'—':>7}"
        d = None
        d_s = "—"
        if isinstance(b, (int, float)) and isinstance(a, (int, float)):
            d = a - b
            d_s = f"{d:+.1f}{'pp' if pct else ''}"
        # Direction marker
        if cmp == "higher_better" and isinstance(d, float):
            dir_mark = "▲" if d > 0 else ("▼" if d < 0 else "—")
        elif cmp == "lower_better" and isinstance(d, float):
            dir_mark = "▼" if d < 0 else ("▲" if d > 0 else "—")
        else:
            dir_mark = " "
        print(f"  {name:<28} {b_s:>8} → {a_s:>8}   Δ {d_s:>8}  {dir_mark}")

    def pass_row(name, script, *keys):
        b, a = g(script, *keys)
        b_s = "✅" if b is True else ("❌" if b is False else "—")
        a_s = "✅" if a is True else ("❌" if a is False else "—")
        print(f"  {name:<28} {b_s:>8} → {a_s:>8}")

    b_label, a_label = labels
    # Truncate labels for column headers (use basename, max 20 chars)
    def _short(s):
        bn = os.path.basename(str(s))
        return bn if len(bn) <= 22 else bn[:19] + "..."

    print(f"\n{'='*80}")
    print(f"  REGRADE: {b_label} -> {a_label}")
    print(f"{'='*80}")

    # ---- P0 Axes (from grade_axes.py) ----
    print(f"\n{'-- P0 Axes (grade_axes.py)':<66} {'-'*10}")
    print(f"  {'Axis':<28} {_short(b_label):>8} -> {_short(a_label):>8}   {'Δ':>8}")
    print(f"  {'-'*28} {'-'*8}   {'-'*8}   {'-'*8}")
    for axis_key, axis_label, cmp_dir in [
        ("warmth R-B (mid)", "warmth R-B (mid)", "higher_better"),
        ("blue B (mid)", "blue B (mid)", "lower_better"),
        ("saturation (mid)", "saturation (mid)", "higher_better"),
        ("DOF fg:bg ratio", "DOF fg:bg ratio", "higher_better"),
        ("micro-contrast", "micro-contrast", "higher_better"),
        ("highlight p95", "highlight p95", "band"),
    ]:
        row(axis_label, "grade_axes.py", axis_label, "value", cmp=cmp_dir)

    # ---- Gates (from grade_gate.py) ----
    print(f"\n── Gates G3/G5/G6 (grade_gate.py) {'─'*48}")
    for gate in ("G3", "G5", "G6"):
        pass_row(f"{gate}", "grade_gate.py", gate, "pass")
    row("G3 p05-L", "grade_gate.py", "G3", "p05", cmp="higher_better")
    row("G3 darkest R-B", "grade_gate.py", "G3", "dark_rb", cmp="higher_better")

    # ---- G3 Deep Dive (from grade_g3.py) ----
    print(f"\n── G3 Deep Dive (grade_g3.py) {'─'*52}")
    for p in [1, 5, 10, 50, 99]:
        row(f"  interior p{p:02d}-L%", "grade_g3.py", f"p{p:02d}", fmt_spec="5.1f", cmp="higher_better")
    row("  darkest shade R-B", "grade_g3.py", "dark_RB", cmp="higher_better")
    row("  frame mean R-B", "grade_g3.py", "mean_RB", cmp="higher_better")

    # ---- Midtone (from grade_midtone.py) ----
    print(f"\n── Midtone Band (grade_midtone.py) {'─'*47}")
    row("  midtone R-B", "grade_midtone.py", "RB", cmp="higher_better")
    row("  midtone sat%", "grade_midtone.py", "sat", pct=True, cmp="higher_better")

    # ---- Beauty Delta (from grade_beauty.py) ----
    print(f"\n── Beauty vs Golden Ref (grade_beauty.py) {'─'*42}")
    for key, label, cmp_dir in [
        ("warmth_RB", "warmth R-B (global)", "higher_better"),
        ("sat_mean", "saturation mean%", "higher_better"),
        ("shadow_L_pct", "shadow L%", "higher_better"),
        ("shadow_RB", "shadow R-B", "higher_better"),
        ("highlight_p99", "highlight p99", "band"),
        ("micro_contrast", "micro-contrast", "higher_better"),
        ("edge_energy", "edge energy", "higher_better"),
    ]:
        row(f"  {label}", "grade_beauty.py", "cur", key, cmp=cmp_dir)

    # ---- Penumbra (from measure_penumbra.py) ----
    print(f"\n── Penumbra (measure_penumbra.py) {'─'*49}")
    row("  mean penumbra px", "measure_penumbra.py", "mean_px", fmt_spec="5.1f", cmp="higher_better")

    # ---- G7 Vegetation (from grade_g7.py) ----
    g7 = before_results.get("grade_g7.py", {})
    if g7 and "veg_sat" in g7:
        print(f"\n── G7 Vegetation (grade_g7.py) {'─'*50}")
        row("  veg saturation%", "grade_g7.py", "veg_sat", fmt_spec="5.1f", cmp="lower_better")
        row("  veg hue deg", "grade_g7.py", "veg_hue", fmt_spec="5.1f", cmp="lower_better")
        pass_row("  veg PASS", "grade_g7.py", "veg_pass")

    # ---- Summary ----
    print(f"\n── Summary {'─'*64}")
    # Count PASS axes from grade_axes
    def count_pass(results):
        axes = results.get("grade_axes.py", {})
        return sum(1 for v in axes.values() if isinstance(v, dict) and v.get("pass"))
    b_pass = count_pass(before_results)
    a_pass = count_pass(after_results)
    print(f"  P0 axes PASS:     {b_pass}/6 → {a_pass}/6")
    def gate_pass(results):
        gt = results.get("grade_gate.py", {})
        return sum(1 for g in ("G3", "G5", "G6") if gt.get(g, {}).get("pass"))
    b_g = gate_pass(before_results)
    a_g = gate_pass(after_results)
    print(f"  Gates PASS:       {b_g}/3 → {a_g}/3")
    print(f"{'='*80}\n")


# ---------------------------------------------------------------------------
# Master Table — "regrade all": cross-tabulated axes × frames with ✅/❌
# ---------------------------------------------------------------------------

# Each entry: (row_label, extractor_fn, target_str)
# The extractor receives the full results dict and returns (value, is_pass) or None.
def _extract_p0(results, axis_key):
    """Extract (value, pass) from grade_axes.py for a P0 axis."""
    axes = results.get("grade_axes.py", {})
    entry = axes.get(axis_key, {})
    if not entry or "value" not in entry:
        return None
    return (entry["value"], entry.get("pass", False))


def _extract_gate(results, gate):
    """Extract (True/False, pass_bool) from grade_gate.py."""
    gates = results.get("grade_gate.py", {})
    gd = gates.get(gate, {})
    if isinstance(gd, dict):
        p = gd.get("pass")
        if p is not None:
            return (p, p)
    return None


def _extract_penumbra(results):
    """Extract (px, px>=5) from measure_penumbra.py."""
    pen = results.get("measure_penumbra.py", {})
    px = pen.get("mean_px")
    if px is None:
        return None
    return (px, px >= 5)


MASTER_AXES = [
    # P0 axes
    ("warmth R−B",       lambda r: _extract_p0(r, "warmth R-B (mid)"),   "≥ +110"),
    ("blue B",           lambda r: _extract_p0(r, "blue B (mid)"),       "≤ 10"),
    ("saturation",       lambda r: _extract_p0(r, "saturation (mid)"),   "≥ 90%"),
    ("micro-contrast",   lambda r: _extract_p0(r, "micro-contrast"),     "≥ 5.0"),
    ("highlight p95",    lambda r: _extract_p0(r, "highlight p95"),      "150–185"),
    ("DOF fg:bg",        lambda r: _extract_p0(r, "DOF fg:bg ratio"),   "≥ 3.0"),
    # Gates
    ("G3 (shade)",       lambda r: _extract_gate(r, "G3"),              "p05≥8%+warm"),
    ("G5 (window)",      lambda r: _extract_gate(r, "G5"),              "G/B≤245+grad"),
    ("G6 (warm wood)",   lambda r: _extract_gate(r, "G6"),              "R−B 40–210"),
    # Penumbra
    ("G4a (penumbra)",   _extract_penumbra,                              "≥ 5px"),
]


def _fmt_master_cell(val, is_pass):
    """Format one cell: value with ✅/❌ verdict."""
    if val is None:
        return "—"
    if isinstance(val, bool):
        return "✅" if val else "❌"
    if isinstance(val, float):
        mark = "✅" if is_pass else "❌"
        if abs(val) >= 100:
            return f"{val:.0f}{mark}"
        elif abs(val) >= 10:
            return f"{val:.1f}{mark}"
        else:
            return f"{val:.2f}{mark}"
    if isinstance(val, int):
        mark = "✅" if is_pass else "❌"
        return f"{val}{mark}"
    return str(val)[:10]


def print_master_table(all_results, labels=None):
    """Print a cross-tabulated Master Table: axes × frames with ✅/❌."""
    if not all_results:
        print("No results to tabulate.")
        return

    if labels is None:
        labels = [r.get("label", os.path.basename(r.get("file", "?"))).replace("-nohud2.png", "")
                  for r in all_results]

    LABEL_W = 19
    TARGET_W = 15
    COL_W = 12

    print(f"\n{'=' * (LABEL_W + TARGET_W + len(labels) * COL_W + 4)}")
    print(f"  REGRADE ALL — Master Table")
    print(f"{'=' * (LABEL_W + TARGET_W + len(labels) * COL_W + 4)}")
    header = f"  {'Axis':<{LABEL_W}} {'Target':<{TARGET_W}}"
    for lbl in labels:
        short = lbl[:COL_W-2] if len(lbl) > COL_W - 2 else lbl
        header += f" {short:>{COL_W - 1}}"
    print(header)
    sep = f"  {'─' * LABEL_W} {'─' * TARGET_W}"
    for _ in labels:
        sep += f" {'─' * (COL_W - 1)}"
    print(sep)

    # One row per axis
    for axis_label, extractor, target_str in MASTER_AXES:
        row_str = f"  {axis_label:<{LABEL_W}} {target_str:<{TARGET_W}}"
        for r in all_results:
            pair = extractor(r["results"])
            if pair is None:
                cell = "—"
            else:
                val, is_pass = pair
                cell = _fmt_master_cell(val, is_pass)
            row_str += f" {cell:>{COL_W - 1}}"
        print(row_str)

    # Divider
    print(f"  {'─' * LABEL_W} {'─' * TARGET_W}", end="")
    for _ in labels:
        print(f" {'─' * (COL_W - 1)}", end="")
    print()

    # Summary rows
    pass_row = f"  {'P0 axes PASS':<{LABEL_W}} {'':<{TARGET_W}}"
    for r in all_results:
        axes = r["results"].get("grade_axes.py", {})
        n = sum(1 for v in axes.values() if isinstance(v, dict) and v.get("pass"))
        pass_row += f" {f'{n}/6':>{COL_W - 1}}"
    print(pass_row)

    gate_row = f"  {'Gates PASS':<{LABEL_W}} {'':<{TARGET_W}}"
    for r in all_results:
        gt = r["results"].get("grade_gate.py", {})
        n = sum(1 for g in ("G3", "G5", "G6") if gt.get(g, {}).get("pass"))
        gate_row += f" {f'{n}/3':>{COL_W - 1}}"
    print(gate_row)

    print(f"{'=' * (LABEL_W + TARGET_W + len(labels) * COL_W + 4)}\n")


def main():
    ap = argparse.ArgumentParser(
        description="Unified regrade — grade ALL look axes on before/after frames"
    )
    ap.add_argument("--before", help="Folder of BEFORE frames (*-nohud2.png)")
    ap.add_argument("--after", help="Folder of AFTER frames (*-nohud2.png)")
    ap.add_argument("--profile", choices=["hero", "gameplay"], default="gameplay",
                    help="hero = all 6 axes; gameplay = skip DOF (default: gameplay)")
    ap.add_argument("--json", help="Write full results as JSON to this path")
    ap.add_argument("--all", action="store_true",
                    help="Master Table mode: grade every frame in --before folder, "
                         "print cross-tabulated axes×frames with ✅/❌")
    ap.add_argument("--g7-haze", nargs=2, metavar=("OFF", "ON"),
                    help="G7 A/B: haze-off and haze-on frames for axis A2")
    ap.add_argument("--g7-ao", nargs=2, metavar=("OFF", "ON"),
                    help="G7 A/B: SSAO-off and SSAO-on frames for axis B")
    ap.add_argument("pos", nargs="*", help="Two frame paths for quick single-pair compare")
    args = ap.parse_args()

    # Determine mode
    single_pair = None
    if len(args.pos) == 2:
        single_pair = (args.pos[0], args.pos[1])
    elif len(args.pos) == 1 and args.before and not args.after:
        # --before dir + one after frame = grade the single frame as "after"
        pass
    elif len(args.pos) > 0 and len(args.pos) != 2:
        ap.error("Positional args: exactly 2 frames for single-pair mode, or use --before/--after")

    # Folder mode
    if args.before and args.after:
        pairs, only_before, only_after = find_pairs(args.before, args.after)
        if not pairs:
            print("No matching frame pairs found.", file=sys.stderr)
            if only_before:
                print(f"  Only in before/: {only_before}", file=sys.stderr)
            if only_after:
                print(f"  Only in after/: {only_after}", file=sys.stderr)
            sys.exit(1)

        print(f"Found {len(pairs)} frame pair(s):")
        for b, a in pairs:
            print(f"  {b.name}  ←→  {a.name}")
        if only_before:
            print(f"  (no after match: {only_before})")
        if only_after:
            print(f"  (no before match: {only_after})")

        all_results = []
        for b_path, a_path in pairs:
            label = b_path.name.replace("-nohud2.png", "")
            print(f"\n▶ Grading: {label} ...", flush=True)
            before = grade_one_frame(b_path, profile=args.profile)
            after = grade_one_frame(a_path, profile=args.profile)
            print_comparison(before, after, labels=(b_path.name, a_path.name))
            all_results.append({
                "label": label,
                "before_file": str(b_path),
                "after_file": str(a_path),
                "before": before,
                "after": after,
            })

        # G7 A/B if requested
        if args.g7_haze:
            print("\n▶ G7 A/B: aerial perspective (haze) ...", flush=True)
            rc, out = _run("grade_g7.py", "--ab-haze", *args.g7_haze)
            print(out)
        if args.g7_ao:
            print("\n▶ G7 A/B: ambient occlusion ...", flush=True)
            rc, out = _run("grade_g7.py", "--ab-ao", *args.g7_ao)
            print(out)

        if args.json:
            report = {
                "profile": args.profile,
                "pairs": all_results,
            }
            Path(args.json).write_text(json.dumps(report, indent=2, default=str), encoding="utf-8")
            print(f"\nJSON → {args.json}")

    elif single_pair:
        b_path, a_path = single_pair
        print(f"▶ Single-pair regrade: {b_path} → {a_path}", flush=True)
        before = grade_one_frame(b_path, profile=args.profile)
        after = grade_one_frame(a_path, profile=args.profile)
        print_comparison(before, after, labels=(str(b_path), str(a_path)))
        if args.json:
            report = {
                "profile": args.profile,
                "before_file": str(b_path),
                "after_file": str(a_path),
                "before": before,
                "after": after,
            }
            Path(args.json).write_text(json.dumps(report, indent=2, default=str), encoding="utf-8")
            print(f"\nJSON → {args.json}")

    elif args.before and not args.after:
        # Grade a single folder
        b_dir = Path(args.before)
        frames = sorted(b_dir.glob("*-nohud2.png"))
        if not frames:
            print(f"No *-nohud2.png frames found in {args.before}", file=sys.stderr)
            sys.exit(1)
        print(f"Grading {len(frames)} frame(s) in {args.before}:")
        all_results = []
        for f in frames:
            print(f"\n▶ {f.name} ...", flush=True)
            result = grade_one_frame(f, profile=args.profile)
            all_results.append({"label": f.name.replace("-nohud2.png", ""), "file": str(f), "results": result})

        if args.all:
            # Master Table mode — single cross-tabulated table
            print_master_table(all_results)
        else:
            # Default: per-frame vertical print
            for r in all_results:
                print(f"\n── {r['label']} ──")
                _print_single(r["results"], r["label"])

        if args.json:
            Path(args.json).write_text(json.dumps(all_results, indent=2, default=str), encoding="utf-8")
            print(f"\nJSON → {args.json}")

    else:
        ap.print_help()
        sys.exit(1)


def _print_single(results, label):
    """Print a single-frame grade summary."""
    axes = results.get("grade_axes.py", {})
    gates = results.get("grade_gate.py", {})
    pen = results.get("measure_penumbra.py", {})
    g7 = results.get("grade_g7.py", {})

    print(f"\n  {'Axis':<28} {'Value':>8}   Target")
    print(f"  {'─'*28} {'─'*8}   {'─'*16}")
    for axis_label in [
        "warmth R-B (mid)", "blue B (mid)", "saturation (mid)",
        "DOF fg:bg ratio", "micro-contrast", "highlight p95",
    ]:
        entry = axes.get(axis_label, {})
        v = entry.get("value", "—")
        t = entry.get("target", "—")
        p = "✅" if entry.get("pass") else ("❌" if "pass" in entry else "—")
        v_s = f"{v:7.1f}" if isinstance(v, (int, float)) else f"{'—':>7}"
        print(f"  {p} {axis_label:<25} {v_s}   {t}")

    print(f"\n  Gates: G3={'✅' if gates.get('G3',{}).get('pass') else '❌'}  "
          f"G5={'✅' if gates.get('G5',{}).get('pass') else '❌'}  "
          f"G6={'✅' if gates.get('G6',{}).get('pass') else '❌'}")
    if pen.get("mean_px"):
        print(f"  Penumbra: {pen['mean_px']:.1f}px")
    if g7.get("veg_sat"):
        print(f"  G7 veg: sat {g7['veg_sat']:.1f}%  hue {g7['veg_hue']:.1f}deg  "
              f"{'✅' if g7.get('veg_pass') else '❌'}")


if __name__ == "__main__":
    main()
