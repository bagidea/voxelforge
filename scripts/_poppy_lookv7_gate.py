#!/usr/bin/env python3
"""v5's three clauses PLUS the two that say the frame is still measurable.

WHY TWO MORE CLAUSES. This round is asked to accept or reject an EXPOSURE
value, and exposure is the one knob that can make a gate pass by destroying the
thing it grades. Rail every channel white and `warmth` (R-B) goes to 0 on those
pixels; crush every channel to black and `spread` collapses to a floor-vs-floor
comparison. Both read as a number, neither is a measurement. So the exposure
clauses are not "does it look nice" -- they are "does the ESTIMATOR still have
a signal to work with".

  blow    % of pixels with min(R,G,B) >= 250. All three channels railed: hue is
          gone there, so `warmth` cannot see them.
  crush   % of pixels with max(R,G,B) <=   2. All three at the floor: `p05` and
          the dark half of `spread` are reading the clamp, not the shade.
  band    p75 - p35 in luma. The literal width of the midtone band `warmth`
          averages over (`_poppy_lookv3_pairs.warmth` takes [p35, p75]). A band
          narrower than a couple of code values means the estimator is averaging
          one tone and reporting it as a scene.

THE BARS, AND WHERE THEY COME FROM -- none of them invented for this round:
  blow/crush <= 2.00%  is `scripts/colour_gate.py`'s existing 2% frame-fraction
                       convention (the same bar TEMPERATURE_V3's magenta sweep
                       was judged against).
  band >= 2.00         is two code values, i.e. the narrowest band on which a
                       mean R-B is still an average rather than a single tone.

INSTRUMENT CONTROL, PRINTED NOT ASSUMED. Every bar is printed for the BEFORE
plate as well as the after. The before leg is v2 at its own baseline exposure
and is NEVER moved by this round, so if a bar is failed by the before column
the bar is wrong for this plate class -- and you can see that instead of
inferring it. (Bars that were only ever run on the after plate score N/N there
by construction.)

Estimators are imported from `_poppy_lookv3_pairs.py`, and the p05 floor from
`_poppy_lookv5_gate.py`, for the same reason v5 imported rather than re-typed:
a second copy that drifts grades something nobody measured.

Usage: python scripts/_poppy_lookv7_gate.py [dir] [label]
Exit:  0 = all five clauses hold on all three scenes
"""
import sys
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import numpy as np

_pairs = __import__("_poppy_lookv3_pairs")
load, luma, warmth, orient = _pairs.load, _pairs.luma, _pairs.warmth, _pairs.orient
SCENES = _pairs.SCENES

# `_poppy_lookv5_gate.py` cannot be imported -- it grades and `sys.exit`s at
# module level -- so its bar is READ OUT OF THE SOURCE instead of re-typed from
# memory. A literal copied out of another file cannot notice that file moved;
# this raises instead of grading against a stale number.
_V5_SRC = (pathlib.Path(__file__).resolve().parent / "_poppy_lookv5_gate.py").read_text()
_m = __import__("re").search(r"^P05_FLOOR\s*=\s*([0-9.]+)", _V5_SRC, __import__("re").M)
if not _m:
    raise SystemExit("v7 gate: cannot find P05_FLOOR in _poppy_lookv5_gate.py -- "
                     "the v5 bar moved or was renamed; fix this gate, do not guess.")
P05_FLOOR = float(_m.group(1))

BLOW_MAX = 2.00
CRUSH_MAX = 2.00
BAND_MIN = 2.00


def expo(a, l):
    """(blow%, crush%, band) -- the three numbers that say 'still measurable'."""
    blow = float(np.mean(a.min(axis=-1) >= 250.0) * 100.0)
    crush = float(np.mean(a.max(axis=-1) <= 2.0) * 100.0)
    band = float(np.percentile(l, 75) - np.percentile(l, 35))
    return blow, crush, band


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "docs/assets/look")
    label = sys.argv[2] if len(sys.argv) > 2 else out.name
    ok = True

    print(f"--- {label}  ({out}) ---")
    print(f"{'scene':16s} {'leg':>6s} {'p05':>7s} {'p50':>7s} {'p95':>7s} "
          f"{'warm':>7s} {'spread':>7s} {'blow%':>7s} {'crush%':>7s} {'band':>7s}")

    verdicts = []
    for s in SCENES:
        bp, ap = out / f"{s}_before.png", out / f"{s}_after.png"
        if not bp.exists() or not ap.exists():
            print(f"{s:16s} MISSING")
            ok = False
            continue
        b, a = load(bp), load(ap)
        lb, la = luma(b), luma(a)

        stats = {}
        for tag, img, l in (("before", b, lb), ("after", a, la)):
            p05, p50, p95 = (float(np.percentile(l, q)) for q in (5, 50, 95))
            w, sp = warmth(img, l), orient(img, l)
            blow, crush, band = expo(img, l)
            stats[tag] = dict(p05=p05, p50=p50, p95=p95, warm=w, spread=sp,
                              blow=blow, crush=crush, band=band)
            print(f"{s:16s} {tag:>6s} {p05:7.2f} {p50:7.2f} {p95:7.2f} "
                  f"{w:7.2f} {sp:7.2f} {blow:7.3f} {crush:7.3f} {band:7.2f}")

        B, A = stats["before"], stats["after"]
        c = {
            "floor": A["p05"] >= P05_FLOOR,
            "warm": A["warm"] >= B["warm"],
            "sep": A["spread"] >= B["spread"],
            "blow": A["blow"] <= BLOW_MAX,
            "crush": A["crush"] <= CRUSH_MAX,
            "band": A["band"] >= BAND_MIN,
        }
        # `band` is the third measurability clause; folded into the same column
        # group as blow/crush so the printed verdict stays five names wide.
        c["measurable"] = c["blow"] and c["crush"] and c["band"]
        scene_ok = c["floor"] and c["warm"] and c["sep"] and c["measurable"]
        ok &= scene_ok
        verdicts.append((s, c, scene_ok))

    print()
    print(f"{'scene':16s} {'floor':>8s} {'warmth':>8s} {'sep':>8s} "
          f"{'blow':>8s} {'crush':>8s} {'band':>8s} {'SCENE':>8s}")
    m = lambda v: "PASS" if v else "FAIL"
    for s, c, scene_ok in verdicts:
        print(f"{s:16s} {m(c['floor']):>8s} {m(c['warm']):>8s} {m(c['sep']):>8s} "
              f"{m(c['blow']):>8s} {m(c['crush']):>8s} {m(c['band']):>8s} "
              f"{m(scene_ok):>8s}")
    print()
    print(f"bars: p05(after) >= {P05_FLOOR}   warmth/sep: after >= before   "
          f"blow <= {BLOW_MAX}%   crush <= {CRUSH_MAX}%   band >= {BAND_MIN}")
    print(f"V7 GATE [{label}]:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
