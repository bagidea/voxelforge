#!/usr/bin/env python3
"""Pick a night ev100 by DISTANCE TO THE APPROVED NIGHT REFERENCE, not by "brightest".

WHY THIS FILE HAD TO EXIST. `_poppy_ev100_night_sweep.py` grades every rung
against the v7 bars, and those bars are monotone in exposure: p05, warmth,
separation and band all rise as ev100 falls, so the sweep will always nominate
the BOTTOM RUNG OF WHATEVER RANGE IS SWEPT. That is not the sweep being wrong --
`_poppy_lookv7_gate.py` says so itself in its own docstring: its clauses are
"does the ESTIMATOR still have a signal to work with", explicitly NOT "does it
look nice". They can REJECT a value (they rejected 8.6 on p05/warmth/sep) but
they cannot PICK one. Picking with them would ride the exposure down until the
night frame stopped being a night frame and still print PASS.

SO THE ANCHOR IS THE REFERENCE look.rs ITSELF NAMES. `Hour::NIGHT`'s doc comment
calls `_poppy_lookv2/ref/bsl-01.jpeg` "the clearest case for it in the whole
set" and describes the property it wants: rooftops catching a cold sky, wall
faces below falling away into near-black, and the ONLY warm light in frame being
the torches. That is a statement about TONAL DISTRIBUTION -- most of the mass
dark, a small isolated bright tail -- which is exactly what a luma histogram
measures. So each rung is scored by how far its luma histogram sits from the
reference's.

WHAT THIS IS AND IS NOT. The reference is a different SCENE, so this is not and
cannot be a pixel match; it is a match of tonal shape. Absolute distances have
no meaning outside this script (a metric pipeline does not travel between
graders). Only the RANKING of rungs against one fixed reference under one
grader is used, which is what the number is fit to support.

NEGATIVE CONTROL, PRINTED NOT ASSUMED. A day plate is scored against the same
night reference. If a noon frame does not land clearly FARTHER from the night
reference than every night rung, this metric is not measuring night-ness and
its ranking means nothing -- so the control is printed on every run and the
script refuses to name a pick when the control does not separate.

Usage: python scripts/_poppy_ev100_night_anchor.py [sweepdir]
Exit:  0 = control separated and a rung was picked
"""
import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import numpy as np

_pairs = __import__("_poppy_lookv3_pairs")
load, luma = _pairs.load, _pairs.luma

SCENE = "night-firelit"
RUNGS = [("ev86", 8.6), ("ev75", 7.5), ("ev70", 7.0), ("ev65", 6.5), ("ev60", 6.0)]
REFS = ["_poppy_lookv2/ref/bsl-01.jpeg", "_poppy_lookv2/ref/bsl-02.jpeg"]
# A DAY frame. Must land farther from the night refs than any night rung.
CONTROL = "docs/assets/look/outdoor-noon_after.png"

BINS = 64
DARK = 32.0   # "falls away into near-black"
HI = 200.0    # "the only warm light in frame" -- the emissive tail


def hist(l):
    h, _ = np.histogram(l, bins=BINS, range=(0.0, 256.0))
    return h.astype(np.float64) / max(h.sum(), 1)


def emd(p, q):
    """Earth-mover distance on a 1-D histogram = L1 of the CDF difference.

    Chosen over chi-square because it is ORDERED: moving mass from luma 20 to 30
    must score as a smaller change than moving it from 20 to 200. A bin-wise
    chi-square treats those the same, which is the whole axis in question here.
    """
    return float(np.abs(np.cumsum(p) - np.cumsum(q)).sum())


def describe(path):
    l = luma(load(path))
    return dict(
        h=hist(l),
        dark=float(np.mean(l < DARK) * 100.0),
        hi=float(np.mean(l > HI) * 100.0),
        p50=float(np.percentile(l, 50)),
        p95=float(np.percentile(l, 95)),
    )


def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_poppy_ev100night_sweep")

    refs = {}
    for r in REFS:
        p = root / r
        if not p.exists():
            print(f"MISSING reference {p}")
            return 1
        refs[pathlib.Path(r).name] = describe(p)

    print(f"--- night ev100 anchored to the reference look.rs names ({out}) ---")
    print(f"  dark% = luma < {DARK:.0f}   hi% = luma > {HI:.0f}   "
          f"EMD = |CDF-CDF| over {BINS} luma bins (lower = closer to the reference)")
    print()
    print(f"{'plate':>10s} {'ev':>5s} {'dark%':>7s} {'hi%':>7s} {'p50':>7s} {'p95':>7s} "
          + " ".join(f"{('EMD:'+n.split('.')[0]):>13s}" for n in refs))

    for n, d in refs.items():
        print(f"{n.split('.')[0]:>10s} {'ref':>5s} {d['dark']:7.2f} {d['hi']:7.2f} "
              f"{d['p50']:7.2f} {d['p95']:7.2f}"
              + "".join(f" {emd(d['h'], o['h']):13.3f}" for o in refs.values()))
    print()

    scored = []
    for tag, ev in RUNGS:
        p = out / f"{SCENE}_{tag}.png"
        if not p.exists():
            print(f"MISSING {p}")
            return 1
        d = describe(p)
        ds = [emd(d["h"], o["h"]) for o in refs.values()]
        scored.append((tag, ev, float(np.mean(ds)), d))
        print(f"{tag:>10s} {ev:5.1f} {d['dark']:7.2f} {d['hi']:7.2f} "
              f"{d['p50']:7.2f} {d['p95']:7.2f}" + "".join(f" {x:13.3f}" for x in ds))

    cp = root / CONTROL
    if not cp.exists():
        print(f"\n  MISSING day control {cp} -- refusing to pick without it")
        return 1
    dc = describe(cp)
    dcs = [emd(dc["h"], o["h"]) for o in refs.values()]
    ctrl = float(np.mean(dcs))
    print(f"{'DAY-ctrl':>10s} {'--':>5s} {dc['dark']:7.2f} {dc['hi']:7.2f} "
          f"{dc['p50']:7.2f} {dc['p95']:7.2f}" + "".join(f" {x:13.3f}" for x in dcs))

    print()
    best = min(scored, key=lambda r: r[2])
    worst_night = max(scored, key=lambda r: r[2])
    print(f"  day control mean EMD {ctrl:.3f} vs worst night rung {worst_night[2]:.3f}")
    if ctrl <= worst_night[2]:
        print("  CONTROL DID NOT SEPARATE -- a noon frame scores as close to the night "
              "reference as a night rung does. This metric is not measuring night-ness; "
              "its ranking is meaningless. NO PICK.")
        return 1
    print(f"  control separates by {ctrl - worst_night[2]:.3f} -- the metric distinguishes "
          f"day from night, so the ranking is meaningful")
    print()
    for tag, ev, s, d in sorted(scored, key=lambda r: r[2]):
        print(f"    {tag:>6s} ev={ev:<4.1f} meanEMD={s:8.3f}")
    print()
    print(f"NIGHT EV100 ANCHOR: PICK {best[0]} (ev100={best[1]}) meanEMD={best[2]:.3f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
