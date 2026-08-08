#!/usr/bin/env python3
"""Turn _flamingo_g6/sweep.tsv into the round-1 response-surface report.

Two things the raw TSV does not say on its own:
  * how each row sits against the G6 floor (L >= 55), and
  * which AXIS the row moved and by how much L moved *because of it* — which is
    only meaningful as a delta against `base`, the no-override row shot on the
    same binary.

`base` is therefore also the reproducibility check: if base's L is not the 53.9
the plan was written against, every delta below is measured against a different
frame and the whole round has to be re-read.

Usage: _flamingo_g6_report.py [sweep.tsv]
"""
import sys
from pathlib import Path

FLOOR = 55.0
TSV = Path(sys.argv[1] if len(sys.argv) > 1 else "_flamingo_g6/sweep.tsv")

# label -> (axis, what changed) so the report can group by response surface.
AXIS = {
    "base":  ("-",        "shipped default (no override)"),
    "s13":   ("key",      "sun illuminance 11000 -> 13000 lx"),
    "s16":   ("key",      "sun illuminance 11000 -> 16000 lx"),
    "s22":   ("key",      "sun illuminance 11000 -> 22000 lx"),
    "s30":   ("key",      "sun illuminance 11000 -> 30000 lx"),
    "s45":   ("key",      "sun illuminance 11000 -> 45000 lx"),
    "el24":  ("key",      "sun elevation 17 -> 24 deg"),
    "el30":  ("key",      "sun elevation 17 -> 30 deg"),
    "e105":  ("exposure", "ev100 -> 10.5"),
    "e100":  ("exposure", "ev100 -> 10.0"),
    "a1600": ("fill",     "ambient -> 1600 lx"),
    "a2200": ("fill",     "ambient -> 2200 lx"),
    "skyhi": ("sky",      "ClearColor -> brighter blue"),
    "skylo": ("sky",      "ClearColor -> deeper blue"),
}

rows = []
for line in TSV.read_text(encoding="utf-8").splitlines():
    if not line.strip() or line.startswith("label\t") or line.startswith("==="):
        continue
    f = line.split("\t")
    label = f[0]
    if len(f) < 10 or f[2].startswith("MISS"):
        rows.append({"label": label, "miss": f[2] if len(f) > 2 else "MISS"})
        continue
    rows.append({
        "label": label, "miss": None,
        "p05": f[2], "mingb": f[3], "spread": f[4], "rgb": f[5],
        "L": float(f[6]) if f[6] else None,
        "g3": f[7], "g5": f[8], "g6": f[9],
    })

base = next((r for r in rows if r["label"] == "base" and not r["miss"]), None)
bL = base["L"] if base else None

print(f"# G6 round-1 response surface  ({TSV})")
print(f"# floor: sunlit golden-patch L >= {FLOOR}")
if bL is None:
    print("# !! no usable `base` row — deltas below cannot be trusted")
else:
    print(f"# base L = {bL:.1f}  (plan was written against 53.9)")
print()
print(f"{'row':<7} {'axis':<9} {'L':>6} {'vs floor':>9} {'vs base':>8}  {'G3/G5/G6':<9} {'golden RGB':<14} what changed")
print("-" * 108)

for r in rows:
    if r["miss"]:
        print(f"{r['label']:<7} {AXIS.get(r['label'],('?','?'))[0]:<9} {'--':>6} {'--':>9} {'--':>8}  {'--':<9} {'--':<14} DID NOT RUN: {r['miss']}")
        continue
    axis, what = AXIS.get(r["label"], ("?", ""))
    L = r["L"]
    vsf = f"{L - FLOOR:+.1f}"
    vsb = f"{L - bL:+.1f}" if bL is not None else "--"
    gates = f"{r['g3']}/{r['g5']}/{r['g6']}"
    print(f"{r['label']:<7} {axis:<9} {L:6.1f} {vsf:>9} {vsb:>8}  {gates:<9} {r['rgb']:<14} {what}")

print()
print("## per-axis reach (best row on each axis, vs base)")
if bL is not None:
    for axis in ("key", "fill", "exposure", "sky"):
        cand = [r for r in rows if not r["miss"] and AXIS.get(r["label"], ("?",))[0] == axis]
        if not cand:
            print(f"  {axis:<9} no usable rows")
            continue
        best = max(cand, key=lambda r: r["L"])
        d = best["L"] - bL
        verdict = "CLEARS floor" if best["L"] >= FLOOR else f"still {FLOOR - best['L']:.1f} short"
        print(f"  {axis:<9} best={best['label']:<6} L={best['L']:5.1f}  dL={d:+5.1f}  {verdict}")
