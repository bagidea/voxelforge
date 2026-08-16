#!/usr/bin/env python3
"""Pick the ladder's camera poses from the probe plates — on measured sky, not taste.

The beauty doc refused two of its own plates for exactly this reason: a sky axis
read off a 208 px (0.01%) or 8,118 px (0.99%) mask is a number with nothing under
it.  So poses are admitted here by measurement, and the rejects are PRINTED rather
than dropped quietly — a pose list that only shows its winners looks like every
pose worked.

Admission rules:
  * sky mask >= 3.0% of frame      (else the sky axes cannot be carried at all)
  * far band and near band both non-empty  (else the depth axis is degenerate)
  * horizon measurable over >= 100 columns (else the horizon step is noise)

Prints every probe plate with its numbers and PASS/REJECT + the reason.

Usage:  python scripts/_flamingo_beauty_pick.py docs/assets/look/beauty/probe [--top 4]
"""
import argparse
import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "beauty_axes", HERE / "_flamingo_beauty_axes.py")
axes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(axes)

SKY_MIN_PCT = 3.0
HORIZON_MIN_COLS = 100


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe_dir")
    ap.add_argument("--top", type=int, default=4)
    a = ap.parse_args()

    ref = axes.grade("docs/assets/moodboard.png", crop=(514, 0, 1024, 1024))
    print(f"REF moodboard-right  sky {ref['sky_px_pct']:.2f}%  "
          f"horizon cols {(ref.get('horizon') or {}).get('columns')}\n")

    rows = []
    for p in sorted(Path(a.probe_dir).glob("*.png")):
        r = axes.grade(p)
        hz = r.get("horizon") or {}
        why = []
        if r["sky_px_pct"] < SKY_MIN_PCT:
            why.append(f"sky {r['sky_px_pct']:.2f}% < {SKY_MIN_PCT}%")
        if not r["far"]:
            why.append("far band empty")
        if not r["near"]:
            why.append("near band empty")
        if (hz.get("columns") or 0) < HORIZON_MIN_COLS:
            why.append(f"horizon only {hz.get('columns')} cols")
        rows.append((p, r, why))
        print(f"{p.stem:<26} sky {r['sky_px_pct']:>6.2f}%  "
              f"clip {(r['sky'] or {}).get('clip_pct', float('nan')):>6.2f}%  "
              f"hcols {str(hz.get('columns')):>5}  "
              f"span {r['structure']['L_p95_minus_p5']:>6.1f}  "
              + ("PASS" if not why else "REJECT: " + "; ".join(why)))

    ok = [(p, r) for p, r, why in rows if not why]
    # Rank admitted poses by how close their sky share sits to the reference's,
    # so the ladder is judged on framings comparable to what it is graded against.
    ok.sort(key=lambda t: abs(t[1]["sky_px_pct"] - ref["sky_px_pct"]))
    print(f"\n{len(ok)} pose(s) admitted, {len(rows) - len(ok)} rejected.")
    print(f"top {a.top} by closeness to ref sky share ({ref['sky_px_pct']:.2f}%):")
    for p, r in ok[:a.top]:
        print(f"  {p.stem}   sky {r['sky_px_pct']:.2f}%")


if __name__ == "__main__":
    main()
