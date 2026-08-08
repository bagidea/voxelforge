#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Pick the best before/after visual pair for the Director to show the CEO.

Criteria (in priority order):
1. Biggest warmth improvement (R-B delta is the most human-visible change)
2. G3 p05-L improvement (dark interior getting brighter is immediately noticeable)
3. Saturation improvement
4. Frame where the most FAIL→PASS transitions happened

Outputs: path to the after PNG (abs), one per line — caller picks first.
Also copies the best pair files to the recap dir for easy discovery.
"""

import argparse
import json
import shutil
import sys
from pathlib import Path


def score_pair(pair_data):
    """Score a pair — higher is more dramatic/visible improvement."""
    b = pair_data.get("before", {})
    a = pair_data.get("after", {})
    score = 0.0

    # Warmth delta (most visible axis)
    axes_b = b.get("grade_axes.py", {})
    axes_a = a.get("grade_axes.py", {})
    for axis_label in ["warmth R-B (mid)", "saturation (mid)"]:
        bv = axes_b.get(axis_label, {}).get("value")
        av = axes_a.get(axis_label, {}).get("value")
        if isinstance(bv, (int, float)) and isinstance(av, (int, float)):
            score += abs(av - bv) * (3.0 if "warmth" in axis_label else 1.5)

    # p05-L delta (G3 dark interior)
    g3_b = b.get("grade_g3.py", {})
    g3_a = a.get("grade_g3.py", {})
    for k in ["p05", "p10"]:
        bv = g3_b.get(k)
        av = g3_a.get(k)
        if isinstance(bv, (int, float)) and isinstance(av, (int, float)):
            score += abs(av - bv) * 2.0  # big multiplier — darkness to visible is dramatic

    # FAIL→PASS transitions bonus
    transitions = 0
    for axis_label in ["warmth R-B (mid)", "blue B (mid)", "saturation (mid)",
                       "micro-contrast", "highlight p95"]:
        bp = axes_b.get(axis_label, {}).get("pass")
        ap = axes_a.get(axis_label, {}).get("pass")
        if bp is False and ap is True:
            transitions += 1
    score += transitions * 5.0

    # G3 gate transition
    g3b = g3_b.get("p05_pass") and g3_b.get("warm_pass")
    g3a = g3_a.get("p05_pass") and g3_a.get("warm_pass")
    if g3b is False and g3a is True:
        score += 10.0
    elif g3b is not True and g3a is True:
        score += 5.0

    # Penumbra change
    pen_b = b.get("measure_penumbra.py", {}).get("mean_px")
    pen_a = a.get("measure_penumbra.py", {}).get("mean_px")
    if isinstance(pen_b, (int, float)) and isinstance(pen_a, (int, float)):
        score += abs(pen_a - pen_b) * 0.5

    return score


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", required=True)
    ap.add_argument("--pair-dir", required=True)
    ap.add_argument("--recap-dir", required=True)
    args = ap.parse_args()

    data = json.loads(Path(args.json).read_text(encoding="utf-8"))
    pairs = data.get("pairs", [])

    if not pairs:
        print("NO_PAIRS", file=sys.stderr)
        sys.exit(1)

    # Score all pairs
    scored = [(score_pair(p), p) for p in pairs]
    scored.sort(key=lambda x: x[0], reverse=True)

    best_score, best = scored[0]
    label = best["label"]

    # Find the actual after and before files
    pair_dir = Path(args.pair_dir)
    recap_dir = Path(args.recap_dir)

    after_file = pair_dir / "after" / f"{label}-nohud2.png"
    before_file = pair_dir / "before" / f"{label}-nohud2.png"

    # Copy best pair to recap root for easy discovery
    if after_file.exists():
        shutil.copy2(after_file, recap_dir / f"BEST_AFTER_{label}-nohud2.png")
    if before_file.exists():
        shutil.copy2(before_file, recap_dir / f"BEST_BEFORE_{label}-nohud2.png")

    # Print results
    print(f"BEST_PAIR={label}  score={best_score:.1f}")
    print(f"BEFORE={before_file}")
    print(f"AFTER={after_file}")

    # Also print runner-up for reference
    if len(scored) > 1:
        s2, p2 = scored[1]
        print(f"RUNNER_UP={p2['label']}  score={s2:.1f}")

    # Print all scores for transparency
    print()
    for s, p in scored:
        print(f"  {p['label']:<20} score={s:.1f}")


if __name__ == "__main__":
    main()
