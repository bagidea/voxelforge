#!/usr/bin/env python3
"""Grid table for the beauty-axes ladder (flamingo, 2026-08-16).

Reads every plate the ladder shot and prints the FULL pose x rung x axis grid,
plus the per-rung verdict.  It is deliberately not a "winner picker" that hides
its inputs: my own rule from 08-09 (quoted in docs/flamingo-beauty-axes-2026-08-14.md
section 3) is that a rung is reported for EVERY pose and only counts as won if it
clears the target on every admitted pose at once, so the grid IS the deliverable
and the verdict is derived from it in front of the reader.

Axes reported here are the four that stand on any outdoor frame regardless of how
much sky it holds (#2, #3, #5, #6 in the brief).  The sky axes (#7-#9) are printed
separately and are REFUSED, not filled with a number, on any plate whose sky mask
is below the 3%-of-frame floor the brief's footnotes 1-2 set -- an office scar:
a gate must not pass or fail on an empty region.

Reference is the right half of docs/assets/moodboard.png (crop 514,0,1024,1024),
the same ref and the same crop the brief measured its REF column with.

Usage:
  python scripts/_flamingo_beauty_table.py --dir docs/assets/look/beauty/ladder
"""
import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import importlib

axes = importlib.import_module("_flamingo_beauty_axes")

REF = "docs/assets/moodboard.png"
REF_CROP = (514, 0, 1024, 1024)
SKY_FLOOR_PCT = 3.0  # brief footnotes 1-2: below this a sky axis is refused

# axis -> (label, getter, target test, target text)
WHITE = 255.0


def a_value_span(r):
    return r["structure"]["L_p95_minus_p5"]


def a_shadow_p5(r):
    return r["structure"]["L_p"][0]


def a_shadow_pct(r):
    return 100.0 * r["structure"]["L_p"][0] / WHITE


def a_chroma_span(r):
    return r["structure"]["sat_p95_minus_p5"]


def a_sat_p5(r):
    return r["structure"]["sat_p"][0]


AXES = [
    ("#2 value span L(p95-p5)", a_value_span, lambda v: v >= 150.0, ">= 150", "{:.1f}", "{:+.1f}"),
    ("#3 shadow L p5", a_shadow_p5, lambda v: 20.0 <= v <= 28.0, "20-28", "{:.1f}", "{:+.1f}"),
    ("#3 shadow % of white", a_shadow_pct, lambda v: 8.0 <= v <= 11.0, "8-11%", "{:.1f}%", "{:+.1f}"),
    ("#5 chroma span sat(p95-p5)", a_chroma_span, lambda v: v >= 0.62, ">= 0.62", "{:.3f}", "{:+.3f}"),
    ("#6 sat p5", a_sat_p5, lambda v: v <= 0.32, "<= 0.32", "{:.3f}", "{:+.3f}"),
]


def split_name(stem, rungs):
    """`p4-zaxis-level-amb1400` -> ('p4-zaxis-level', 'amb1400').

    Split on the KNOWN rung suffixes rather than the last dash: pose names carry
    dashes too, and a last-dash split silently mangles every one of them.
    """
    for r in sorted(rungs, key=len, reverse=True):
        if stem.endswith("-" + r):
            return stem[: -(len(r) + 1)], r
    return stem, "?"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", default="docs/assets/look/beauty/ladder")
    ap.add_argument("--rungs", default="r0,fog8-45,fog6-55,fog5-70,amb1700,amb1400,amb1100,ev11.3,ev12.0")
    ap.add_argument("--json", default=None)
    ap.add_argument("--md", default=None)
    ap.add_argument("--lit-only", action="store_true",
                    help="measure on lit content only (L >= 12), excluding the dead-black "
                         "sky. Use when the sky renders black: it occupies 11-17%% of these "
                         "frames, which is MORE than the 5th percentile, so an all-pixel p5 "
                         "reads the black sky and not the shade floor at all.")
    args = ap.parse_args()

    if args.lit_only:
        _orig_structure = axes.structure

        def _lit_structure(L, sat, m):
            lit = m & (L >= 12.0)
            # Never hand back a mask so thin the percentiles are noise; fall
            # back and say so rather than quietly scoring an empty region.
            if lit.sum() < 0.2 * m.sum():
                print("REFUSED: lit mask under 20% of frame; not scoring", file=sys.stderr)
                return _orig_structure(L, sat, m)
            return _orig_structure(L, sat, lit)

        axes.structure = _lit_structure

    rungs = [r for r in args.rungs.split(",") if r]
    d = Path(args.dir)
    plates = sorted(d.glob("*.png"))
    if not plates:
        print(f"REFUSED: no plates in {d} -- nothing to measure", file=sys.stderr)
        return 2

    R = axes.grade(REF, REF_CROP, 0, "REF moodboard R-half")

    cells = {}   # (pose, rung) -> graded dict
    poses = []
    for p in plates:
        pose, rung = split_name(p.stem, rungs)
        if pose not in poses:
            poses.append(pose)
        cells[(pose, rung)] = axes.grade(str(p), None, 0, p.stem)

    have_rungs = [r for r in rungs if any((po, r) in cells for po in poses)]

    out = []
    w = 30
    out.append(f"REF = {REF} crop {REF_CROP}   poses={len(poses)}  rungs={len(have_rungs)}  plates={len(plates)}")
    out.append("")
    for label, get, ok, tgt, fmt, dfmt in AXES:
        out.append(f"### {label}   REF {fmt.format(get(R))}   TARGET {tgt}")
        head = f"{'rung':<10}" + "".join(f"{po:>22}" for po in poses) + f"{'all poses':>12}"
        out.append(head)
        out.append("-" * len(head))
        for r in have_rungs:
            row = f"{r:<10}"
            allok = True
            seen = False
            for po in poses:
                c = cells.get((po, r))
                if c is None:
                    row += f"{'-':>22}"
                    allok = False
                    continue
                seen = True
                v = get(c)
                base = cells.get((po, "r0"))
                dv = f" ({dfmt.format(v - get(base))})" if base is not None and r != "r0" else ""
                mark = "P" if ok(v) else "f"
                allok &= ok(v)
                row += f"{fmt.format(v) + dv + ' ' + mark:>22}"
            row += f"{('PASS' if (allok and seen) else 'fail'):>12}"
            out.append(row)
        out.append("")

    out.append("### sky mask (gate for axes #7-#9; brief floor = 3.00% of frame)")
    head = f"{'rung':<10}" + "".join(f"{po:>22}" for po in poses)
    out.append(head)
    out.append("-" * len(head))
    for r in have_rungs:
        row = f"{r:<10}"
        for po in poses:
            c = cells.get((po, r))
            if c is None:
                row += f"{'-':>22}"
                continue
            pct = c["sky_px_pct"]
            row += f"{f'{pct:.2f}%' + (' ok' if pct >= SKY_FLOOR_PCT else ' REFUSED'):>22}"
        out.append(row)
    out.append("")
    out.append("REFUSED = mask below the 3% floor; axes #7-#9 are not scored on that plate at all")

    txt = "\n".join(out)
    print(txt)
    if args.md:
        Path(args.md).write_text(txt + "\n", encoding="utf-8")
        print(f"\nwrote {args.md}")
    if args.json:
        Path(args.json).write_text(json.dumps(
            dict(ref=R, cells={f"{k[0]}|{k[1]}": v for k, v in cells.items()}),
            indent=1, default=float), encoding="utf-8")
        print(f"wrote {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
