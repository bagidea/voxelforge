#!/usr/bin/env python3
# ===========================================================================
# Poppy — does VOXELFORGE_MAT_MAPS actually change pixels?
#
# Reports, per scene, the on-vs-off delta against the SAME session's null pair
# (`day` shot twice under identical env). A delta at or below the null is the
# renderer breathing; only a delta clearly above it is the authored maps.
#
# Columns
#   mad        per-pixel mean |on-off| over RGB, 0..255
#   p99        99th percentile of that per-pixel delta (where the change lives)
#   chg%       share of pixels whose delta exceeds 1 level
#   spec%      share of pixels above the specular gate (luma > 200) — normal +
#              roughness maps redistribute highlights, so this moves when they
#              are bound and not when they are not
#   hp         mean |Laplacian| of luma = LOCAL contrast. A normal map's whole
#              job is per-texel relief the albedo does not carry, so it shows up
#              here before it shows up in a global std.
#   std        global luma std, for the record
#
# USAGE
#   python scripts/_poppy_matmaps_measure.py <dir-with-plates> [prefix]
# ===========================================================================
import hashlib
import os
import sys

import numpy as np
from PIL import Image

SCENES = ["day", "evening-raking", "night-firelit"]


def load(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)


def luma(a):
    return a[:, :, 0] * 0.2126 + a[:, :, 1] * 0.7152 + a[:, :, 2] * 0.0722


def laplace(y):
    return np.abs(4.0 * y[1:-1, 1:-1] - y[:-2, 1:-1] - y[2:, 1:-1] - y[1:-1, :-2] - y[1:-1, 2:])


def md5(path):
    with open(path, "rb") as f:
        return hashlib.md5(f.read()).hexdigest()[:12]


def stats(a):
    y = luma(a)
    return {
        "spec%": 100.0 * float((y > 200).mean()),
        "hp": float(laplace(y).mean()),
        "std": float(y.std()),
        "luma": float(y.mean()),
    }


def pair(pa, pb):
    a, b = load(pa), load(pb)
    if a.shape != b.shape:
        raise SystemExit(f"shape mismatch {a.shape} vs {b.shape}: {pa} {pb}")
    d = np.abs(a - b).mean(axis=2)
    return {
        "mad": float(d.mean()),
        "p99": float(np.percentile(d, 99)),
        "max": float(d.max()),
        "chg%": 100.0 * float((d > 1.0).mean()),
    }


def main():
    d = sys.argv[1]
    prefix = sys.argv[2] if len(sys.argv) > 2 else ""

    def p(scene, lever):
        return os.path.join(d, f"{prefix}{scene}_{lever}.png")

    # The null pair fixes BOTH floors. `mad` alone is the weaker of the two:
    # this scene animates (fire, foliage, water), so two identical runs already
    # disagree on a few percent of pixels at random. `hp` is a population stat --
    # random per-pixel disagreement cancels in the mean, so the null's hp delta
    # is a much tighter floor, and a normal map's whole signature is local relief
    # the albedo does not carry. A verdict has to clear both.
    null = null_hp = None
    npath = p("day", "onNULL")
    if os.path.isfile(npath) and os.path.isfile(p("day", "on")):
        null = pair(p("day", "on"), npath)
        a, b = stats(load(p("day", "on")))["hp"], stats(load(npath))["hp"]
        null_hp = abs(b - a) / a * 100.0
        print(f"NULL PAIR  day on vs on   mad={null['mad']:.4f}  p99={null['p99']:.2f}  "
              f"max={null['max']:.1f}  chg%={null['chg%']:.3f}  hp delta={null_hp:+.2f}%"
              f"   <- capture noise floor")
        print()
    else:
        print("NULL PAIR  missing - no noise floor for this arm\n")

    print(f"{'scene':<15} {'mad':>8} {'p99':>7} {'max':>7} {'chg%':>7}   "
          f"{'spec% on':>8} {'spec% off':>9} {'hp on':>7} {'hp off':>7} {'hp d%':>7} "
          f"{'std on':>7} {'std off':>7}  verdict")
    for s in SCENES:
        pon, poff = p(s, "on"), p(s, "off")
        if not (os.path.isfile(pon) and os.path.isfile(poff)):
            print(f"{s:<15} MISSING ({os.path.basename(pon)} / {os.path.basename(poff)})")
            continue
        dd = pair(pon, poff)
        son, soff = stats(load(pon)), stats(load(poff))
        hp_d = (son["hp"] - soff["hp"]) / soff["hp"] * 100.0
        if md5(pon) == md5(poff) or dd["max"] == 0.0:
            verdict = "IDENTICAL - lever never reached the loader"
        elif null is None:
            verdict = "NO FLOOR - shoot a null pair before quoting this"
        elif abs(hp_d) > null_hp * 3.0 and dd["mad"] > null["mad"]:
            verdict = f"REAL CHANGE (hp {abs(hp_d) / null_hp:.0f}x the null floor)"
        else:
            verdict = f"AT NOISE FLOOR (null mad {null['mad']:.4f}, hp {null_hp:+.2f}%) - not the maps"
        print(f"{s:<15} {dd['mad']:>8.4f} {dd['p99']:>7.2f} {dd['max']:>7.1f} {dd['chg%']:>7.3f}   "
              f"{son['spec%']:>8.3f} {soff['spec%']:>9.3f} {son['hp']:>7.3f} {soff['hp']:>7.3f} "
              f"{hp_d:>+6.2f}% {son['std']:>7.2f} {soff['std']:>7.2f}  {verdict}")

    print()
    for s in SCENES:
        for lever in ("on", "off"):
            f = p(s, lever)
            if os.path.isfile(f):
                print(f"  md5 {md5(f)}  {f}")


if __name__ == "__main__":
    main()
