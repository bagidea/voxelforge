#!/usr/bin/env python3
"""Grade ONE pair -- night-firelit before/after -- for the Hour::NIGHT.ev100 move.

WHY A SECOND GATE FILE AND NOT `_poppy_lookv7_gate.py` DIRECTLY. That one walks
`SCENES` (three plates) and prints `MISSING` + FAILs for any it cannot find. This
round moves ONE constant that only `Hour::NIGHT` reads, so shooting noon and
evening would spend four boots proving two frames did not change. Everything that
decides a verdict here is IMPORTED from the v7 gate -- the estimators, the three
measurability bars, and `P05_FLOOR` (which the v7 gate itself reads out of
`_poppy_lookv5_gate.py`'s source rather than re-typing). Nothing is re-typed, so
this file cannot drift away from the bar the other plates are graded against.

TWO EXTRA CHECKS THE V7 GATE DOES NOT HAVE, because this round's failure mode is
different -- the v7 ladder always relinked between rungs, this one does not:

  md5      the two frames must not be byte-identical. Both legs come out of ONE
           binary and the AFTER leg sets no env lever at all, so if the source
           edit never reached this exe the harness would happily shoot the same
           frame twice and every delta below would read 0.00 as "no regression".
  %diff    how much of the frame actually moved. A one-pixel difference also
           clears the md5 check.

p95 IS PRINTED, NOT GATED. The 150 p95 floor in look.rs belongs to the daylight
plates: raising ev100 lowers p95 by construction, and a night frame whose
brightest 5% is a campfire has no business being held to a noon bar. It is in
the table so the cost is visible rather than hidden.

Usage: python scripts/_poppy_ev100_night_gate.py [dir] [label]
Exit:  0 = every bar holds on the after leg
"""
import sys
import pathlib
import hashlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import numpy as np

_v7 = __import__("_poppy_lookv7_gate")
load, luma, warmth, orient = _v7.load, _v7.luma, _v7.warmth, _v7.orient
expo, P05_FLOOR = _v7.expo, _v7.P05_FLOOR
BLOW_MAX, CRUSH_MAX, BAND_MIN = _v7.BLOW_MAX, _v7.CRUSH_MAX, _v7.BAND_MIN

SCENE = "night-firelit"


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_poppy_ev100night")
    label = sys.argv[2] if len(sys.argv) > 2 else out.name
    bp, ap = out / f"{SCENE}_before.png", out / f"{SCENE}_after.png"

    print(f"--- {label}  ({out}) ---")
    for p in (bp, ap):
        if not p.exists():
            print(f"MISSING {p}")
            return 1

    md5 = {p.name: hashlib.md5(p.read_bytes()).hexdigest() for p in (bp, ap)}
    for n, h in md5.items():
        print(f"  {n:28s} {h}")
    distinct = md5[bp.name] != md5[ap.name]

    b, a = load(bp), load(ap)
    lb, la = luma(b), luma(a)
    if b.shape != a.shape:
        print(f"  shape mismatch {b.shape} vs {a.shape} -- not comparable")
        return 1
    pct = float(np.mean(np.any(b.astype(np.int16) != a.astype(np.int16), axis=-1)) * 100.0)

    print()
    print(f"{'leg':>7s} {'p05':>7s} {'p50':>7s} {'p95':>7s} {'warm':>7s} "
          f"{'spread':>7s} {'blow%':>7s} {'crush%':>7s} {'band':>7s}")
    stats = {}
    for tag, img, l in (("before", b, lb), ("after", a, la)):
        p05, p50, p95 = (float(np.percentile(l, q)) for q in (5, 50, 95))
        w, sp = warmth(img, l), orient(img, l)
        blow, crush, band = expo(img, l)
        stats[tag] = dict(p05=p05, p50=p50, p95=p95, warm=w, spread=sp,
                          blow=blow, crush=crush, band=band)
        print(f"{tag:>7s} {p05:7.2f} {p50:7.2f} {p95:7.2f} {w:7.2f} "
              f"{sp:7.2f} {blow:7.3f} {crush:7.3f} {band:7.2f}")

    B, A = stats["before"], stats["after"]
    print(f"{'delta':>7s} {A['p05']-B['p05']:7.2f} {A['p50']-B['p50']:7.2f} "
          f"{A['p95']-B['p95']:7.2f} {A['warm']-B['warm']:7.2f} "
          f"{A['spread']-B['spread']:7.2f} {A['blow']-B['blow']:7.3f} "
          f"{A['crush']-B['crush']:7.3f} {A['band']-B['band']:7.2f}")

    c = {
        "distinct": distinct,
        "floor": A["p05"] >= P05_FLOOR,
        "warm": A["warm"] >= B["warm"],
        "sep": A["spread"] >= B["spread"],
        "blow": A["blow"] <= BLOW_MAX,
        "crush": A["crush"] <= CRUSH_MAX,
        "band": A["band"] >= BAND_MIN,
    }
    ok = all(c.values())
    m = lambda v: "PASS" if v else "FAIL"
    print()
    print(f"  frames differ on {pct:.2f}% of pixels")
    for k in ("distinct", "floor", "warm", "sep", "blow", "crush", "band"):
        print(f"  {k:9s} {m(c[k])}")
    print()
    print(f"bars: distinct md5   p05(after) >= {P05_FLOOR}   warmth/sep: after >= before   "
          f"blow <= {BLOW_MAX}%   crush <= {CRUSH_MAX}%   band >= {BAND_MIN}")
    print(f"EV100-NIGHT GATE [{label}]:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
