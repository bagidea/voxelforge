#!/usr/bin/env python3
"""Do the re-shot plates still support what the ladder doc PUBLISHED?

`docs/flamingo-beauty-ladder-2026-08-16.md` §2 is the only surviving record of
the lit-only grid -- `_grid_lit.md` was destroyed and, unlike `_grid.md`, cannot
be replayed from `_grid.json` (the JSON holds the all-pixel mask only).  So the
doc's own table IS the baseline for the lit axis, and this script diffs the
re-shot plates against it instead of against a number typed into a report.

It checks axis #2 (value span), the one axis the doc enumerates for all 13 rungs
on all 4 poses.  The other axes are collapsed in the doc ("amb / fill / ev ทุกขั้น
| 0.92-0.94 | ...") and a range cannot be diffed against a value, so they are
NOT scored here -- they are listed as unchecked rather than quietly skipped.

Both sides are parsed, never retyped:
  * doc  -> the markdown table in section 2, axis #2
  * new  -> `_grid_lit.md`, written by the shipped `_flamingo_beauty_table.py`

Usage:
  python scripts/_pixel_ladder_doc_check.py
      [--doc docs/flamingo-beauty-ladder-2026-08-16.md]
      [--lit docs/assets/look/beauty/_grid_lit.md]
"""
import argparse
import re
from pathlib import Path

POSES = ["p1-east-level", "p2-east-up", "p3-east-down", "p8-abelev-eye-horizon"]
NUM = re.compile(r"-?\d+\.\d+")


def parse_doc(path):
    """Section 2, axis #2 table: | rung | v | v | v | v | verdict |."""
    lines = Path(path).read_text(encoding="utf-8").splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("### แกน #2"))
    out = {}
    for l in lines[start:]:
        if l.startswith("###") and not l.startswith("### แกน #2"):
            break
        if not l.startswith("|"):
            continue
        cells = [c.strip() for c in l.strip("|").split("|")]
        rung = cells[0].replace("*", "").strip()
        if rung in ("ขั้น", "---") or rung.startswith("-"):
            continue
        vals = []
        for c in cells[1:5]:
            m = NUM.search(c.replace("**", ""))
            if m:
                vals.append(float(m.group()))
        if len(vals) == 4:
            out[rung] = vals
    return out


def parse_lit(path):
    """`_grid_lit.md` axis #2 block: `rung  V f  V (+d) f  ...` per pose column."""
    lines = Path(path).read_text(encoding="utf-8").splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("### #2 value span"))
    out = {}
    for l in lines[start + 3:]:
        if not l.strip():
            break
        rung = l.split()[0]
        # one value per pose: the first number in each 22-wide column
        body = l[10:]
        cols = [body[i:i + 22] for i in range(0, 22 * len(POSES), 22)]
        vals = [float(NUM.search(c).group()) for c in cols if NUM.search(c)]
        if len(vals) == len(POSES):
            out[rung] = vals
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--doc", default="docs/flamingo-beauty-ladder-2026-08-16.md")
    ap.add_argument("--lit", default="docs/assets/look/beauty/_grid_lit.md")
    ap.add_argument("--target", type=float, default=150.0)
    a = ap.parse_args()

    doc, new = parse_doc(a.doc), parse_lit(a.lit)
    print(f"doc §2 axis #2 : {len(doc)} rungs   {a.doc}")
    print(f"re-shot lit    : {len(new)} rungs   {a.lit}\n")

    missing = [r for r in doc if r not in new]
    extra = [r for r in new if r not in doc]

    hdr = f"{'rung':<14}" + "".join(f"{p:>26}" for p in POSES)
    print("### axis #2 value span — 2026-08-16 PUBLISHED -> 2026-08-17 RE-SHOT")
    print(hdr)
    print("-" * len(hdr))
    worst = 0.0
    for rung, old in doc.items():
        if rung not in new:
            continue
        row = f"{rung:<14}"
        for o, n in zip(old, new[rung]):
            worst = max(worst, abs(n - o))
            row += f"{f'{o:.1f} -> {n:.1f} ({n - o:+.1f})':>26}"
        print(row)

    print(f"\nlargest single-plate move: {worst:.1f} L")
    if missing:
        print(f"in doc but not re-shot: {missing}")
    if extra:
        print(f"re-shot but not in doc: {extra}")

    # The doc's headline verdict, re-tested on the new pixels rather than assumed.
    hits = [(r, max(v)) for r, v in new.items() if max(v) >= a.target]
    print(f"\ndoc §3 verdict was: no rung reaches value span {a.target:.0f} on any pose.")
    print(f"re-shot plates    : {len(hits)} rung(s) reach it"
          + (f" -> {hits}" if hits else "  => VERDICT HOLDS"))
    print(f"best re-shot rung : " + max(
        ((r, max(v)) for r, v in new.items()), key=lambda t: t[1]).__str__())

    print("\nNOT CHECKED (the doc collapses these into ranges, which cannot be diffed "
          "against a value): axis #3 rows 'amb1700 / amb1400 / amb1100' and 'fog ทั้ง 3 ขั้น', "
          "axis #5 row 'amb / fill / ev ทุกขั้น', axis #6 (quoted as a 0.05-0.31 range only).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
