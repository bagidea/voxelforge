#!/usr/bin/env python3
"""Poppy - re-measure the four sky axes on the painted-sky before/after pair.

WHY THIS WRAPS THE GRADER INSTEAD OF MEASURING ANYTHING ITSELF.
    Every number quoted at me came out of `scripts/_pixel_artgap_grade.py`, and
    that grader's sky mask is a flood-fill from the top edge that stops at hard
    edges - not a luminance threshold. A second implementation of "which pixels
    are sky" would produce numbers that LOOK comparable to the document's and are
    not, which is the exact failure mode `docs/look-acceptance-rubric.md` rule 7
    exists to catch. So this file computes nothing. It runs Flamingo's grader,
    reads its JSON, and lays the six numbers out next to the reference.

THE BEFORE ARM IS TREATED AS A CLAIM, NOT AS A GIVEN.
    `before` is supposed to be yesterday's code path reproduced by env levers on
    today's binary. That is falsifiable: it must land on top of
    `_matmaps_after.png`, which came off a binary (sha e7db71ca) predating every
    line of this change. The mean |diff| against that plate is printed FIRST and
    flagged, because if the before arm is not the baseline it claims to be then
    nothing downstream of it is a before/after at all.

USAGE
    python scripts/_poppy_skymeasure_20260818.py
    python scripts/_poppy_skymeasure_20260818.py --frames _poppy_sky/g*.png
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GRADER = os.path.join(ROOT, "scripts", "_pixel_artgap_grade.py")
REF = os.path.join(ROOT, "docs", "refs", "ceo_ref_sunset_valley.jpg")
BASELINE = os.path.join(ROOT, "_matmaps_after.png")

# key, label, reference value, direction (+1 = higher is better)
AXES = [
    ("sky_ground_ratio", "sky brighter than ground", 1.80, +1),
    ("sky_L_range", "sky tonal gradient (p95-p5)", 172.6, +1),
    ("sky_void_pct", "sky sitting at black (L<10) %", 0.00, -1),
    ("sky_hue_span_deg", "sky hue range (deg)", 60.0, +1),
    ("crush_pct", "crushed blacks %", 0.55, -1),
    # NOT one of the four, and printed anyway: this is the axis a too-high
    # SKY_PAINT_GAIN would buy the other four with. A sky that clips to white
    # scores a huge L_range and a dead hue span, so it has to be in view.
    #
    # The reference's own value is 5.09, NOT zero — the CEO frame has the sun in
    # it and clips there. So this is not a "keep at 0" axis and the direction
    # below is a simplification: read it as "do not run away from 5", and treat
    # anything past ~10 as having bought the other four axes with clipping.
    ("sky_blown_pct", "sky blown to white (L>245) % [guard]", 5.09, -1),
]


def control_diff(a: str, b: str) -> float | None:
    """Mean |diff| in 8-bit levels between two plates, or None if incomparable."""
    try:
        import numpy as np
        from PIL import Image
    except ImportError:
        return None
    if not (os.path.exists(a) and os.path.exists(b)):
        return None
    ia = np.asarray(Image.open(a).convert("RGB"), dtype=float)
    ib = np.asarray(Image.open(b).convert("RGB"), dtype=float)
    if ia.shape != ib.shape:
        return None
    return float(abs(ia - ib).mean())


def grade(frames: list[str], out_json: str) -> dict:
    cmd = [sys.executable, GRADER, *frames, "--ref", REF, "--json", out_json]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    # The grader exits 1/2 to signal GAP/SKIP, which is a VERDICT, not a crash.
    # Only a missing JSON means it actually failed.
    if not os.path.exists(out_json):
        print(r.stdout[-4000:])
        print(r.stderr[-4000:], file=sys.stderr)
        raise SystemExit(f"grader produced no JSON (exit {r.returncode})")
    with open(out_json, encoding="utf-8") as fh:
        return json.load(fh)


def find_frame(data: dict, stem: str) -> dict | None:
    """Pull one frame's metric dict out of the grader's JSON, whatever it nests."""
    def walk(node):
        if isinstance(node, dict):
            if any(k in node for k, *_ in AXES):
                yield node
            for v in node.values():
                yield from walk(v)
        elif isinstance(node, list):
            for v in node:
                yield from walk(v)

    # Prefer an entry whose own recorded path matches this stem.
    for node in walk(data):
        p = str(node.get("path", node.get("frame", node.get("file", ""))))
        if stem in p.replace("\\", "/"):
            return node
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--before", default=os.path.join(ROOT, "_poppy_sky", "before.png"))
    ap.add_argument("--after", default=os.path.join(ROOT, "_poppy_sky", "after.png"))
    ap.add_argument("--json", default=os.path.join(ROOT, "_poppy_sky", "skyaxes.json"))
    args = ap.parse_args()

    for p in (args.before, args.after):
        if not os.path.exists(p):
            raise SystemExit(f"missing plate: {p}")

    print("=" * 78)
    print("CONTROL - is `before` really yesterday's path?")
    d = control_diff(args.before, BASELINE)
    if d is None:
        print("  UNDETERMINED - _matmaps_after.png absent or a different size.")
        print("  The before arm is UNPROVEN; say so wherever these numbers go.")
    else:
        verdict = "PASS" if d < 2.0 else "SUSPECT"
        print(f"  mean |before - _matmaps_after.png| = {d:.3f} levels   {verdict}")
        if d >= 2.0:
            print("  The before arm does not reproduce the graded baseline.")
            print("  Explain that before publishing the pair.")
    print("=" * 78)

    data = grade([args.before, args.after], args.json)
    fb = find_frame(data, "before.png")
    fa = find_frame(data, "after.png")
    if fb is None or fa is None:
        raise SystemExit(f"could not locate both frames in {args.json}")

    print()
    print(f"{'axis':<38} {'REF':>8} {'before':>9} {'after':>9} {'move':>9}")
    print("-" * 78)
    for key, label, ref, direction in AXES:
        b = fb.get(key)
        a = fa.get(key)
        def fmt(v):
            return "  n/a" if v is None else f"{float(v):9.2f}"
        if a is None or b is None:
            move = "      n/a"
        else:
            delta = float(a) - float(b)
            good = (delta * direction) > 0
            move = f"{delta:+8.2f}{'+' if good else '-'}"
        print(f"{label:<38} {ref:8.2f}{fmt(b)}{fmt(a)}{move:>10}")
    print("-" * 78)
    print("move column: trailing + = moved TOWARD the reference, - = away.")
    print(f"raw JSON: {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
