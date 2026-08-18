#!/usr/bin/env python3
# ===========================================================================
# Poppy — three arms, two questions, one null floor.
#
#   head16   -> art64off   what the 64 px ALBEDO bought on its own
#   art64off -> art64on    what the AUTHORED NORMAL/ROUGHNESS maps bought
#   head16   -> art64on    the two together (should not be quoted for either)
#
# Splitting the pass this way is the point of the shoot: the previous report
# could only say "the maps work at 16 px", because the 64 px albedo could not
# reach a frame at all. Now both halves are separable, so neither gets to claim
# the other's credit.
#
# Columns
#   mad    per-pixel mean |A-B| over RGB, 0..255
#   p99    99th percentile of that delta — where the change actually lives
#   chg%   share of pixels whose delta exceeds 1 level
#   hp     mean |Laplacian| of luma = LOCAL contrast. The population stat: per-
#          pixel animation noise cancels in the mean, so this is the tight floor.
#   spec%  share of pixels above the specular gate (luma > 200)
#
# A verdict needs BOTH: mad above the null's mad AND |hp delta| above 3x the
# null's hp delta.
#
# USAGE
#   python scripts/_poppy_tilepx_measure.py [plates-dir] [--json out.json]
# ===========================================================================
import hashlib
import json
import os
import sys

import numpy as np
from PIL import Image

SCENES = ["day", "evening-raking", "night-firelit"]
ARMS = ["head16", "art64off", "art64on"]
PAIRS = [
    ("head16", "art64off", "64px ALBEDO alone"),
    ("art64off", "art64on", "AUTHORED MAPS alone"),
    ("head16", "art64on", "both together"),
]


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


def delta(a, b):
    d = np.abs(a - b).mean(axis=2)
    return {
        "mad": float(d.mean()),
        "p99": float(np.percentile(d, 99)),
        "max": float(d.max()),
        "chg%": 100.0 * float((d > 1.0).mean()),
    }


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    d = args[0] if args else "_poppy_tilepx/plates"
    out_json = None
    if "--json" in sys.argv:
        out_json = sys.argv[sys.argv.index("--json") + 1]

    def p(arm, scene, suffix=""):
        return os.path.join(d, f"{arm}_{scene}{suffix}.png")

    report = {"null": None, "scenes": {}}

    # ---- the floor ---------------------------------------------------------
    null = null_hp = None
    npath, apath = p("art64on", "day", "NULL"), p("art64on", "day")
    if os.path.isfile(npath) and os.path.isfile(apath):
        null = delta(load(apath), load(npath))
        ha, hb = stats(load(apath))["hp"], stats(load(npath))["hp"]
        null_hp = abs(hb - ha) / ha * 100.0
        report["null"] = dict(null, hp_pct=null_hp)
        print(f"NULL PAIR  art64on day vs day   mad={null['mad']:.4f}  p99={null['p99']:.2f}  "
              f"chg%={null['chg%']:.3f}  hp delta={null_hp:+.2f}%   <- capture noise floor\n")
    else:
        print("NULL PAIR  missing — no floor for this session\n")

    # ---- per-arm absolute stats -------------------------------------------
    print(f"{'scene':<15} {'arm':<9} {'hp':>7} {'spec%':>7} {'std':>7} {'luma':>7}   md5")
    for s in SCENES:
        for arm in ARMS:
            f = p(arm, s)
            if not os.path.isfile(f):
                print(f"{s:<15} {arm:<9} MISSING {f}")
                continue
            st = stats(load(f))
            report["scenes"].setdefault(s, {}).setdefault("arms", {})[arm] = dict(st, md5=md5(f), path=f)
            print(f"{s:<15} {arm:<9} {st['hp']:>7.3f} {st['spec%']:>7.3f} {st['std']:>7.2f} "
                  f"{st['luma']:>7.2f}   {md5(f)}")
    print()

    # ---- the two isolated questions ---------------------------------------
    print(f"{'scene':<15} {'A -> B':<22} {'mad':>8} {'p99':>7} {'chg%':>7} {'hp A':>7} {'hp B':>7} "
          f"{'hp d%':>8} {'spec d':>7}  verdict")
    for s in SCENES:
        for a, b, what in PAIRS:
            fa, fb = p(a, s), p(b, s)
            if not (os.path.isfile(fa) and os.path.isfile(fb)):
                continue
            ia, ib = load(fa), load(fb)
            if ia.shape != ib.shape:
                print(f"{s:<15} shape mismatch {ia.shape} vs {ib.shape}")
                continue
            dd = delta(ia, ib)
            sa, sb = stats(ia), stats(ib)
            hp_d = (sb["hp"] - sa["hp"]) / sa["hp"] * 100.0
            spec_d = sb["spec%"] - sa["spec%"]
            if dd["max"] == 0.0:
                verdict = "IDENTICAL — the arm never reached the loader"
            elif null is None:
                verdict = "NO FLOOR"
            elif abs(hp_d) > null_hp * 3.0 and dd["mad"] > null["mad"]:
                verdict = f"REAL CHANGE ({abs(hp_d) / null_hp:.0f}x floor)"
            else:
                verdict = "AT NOISE FLOOR"
            report["scenes"].setdefault(s, {}).setdefault("pairs", {})[f"{a}->{b}"] = dict(
                dd, hp_a=sa["hp"], hp_b=sb["hp"], hp_pct=hp_d, spec_d=spec_d,
                what=what, verdict=verdict)
            print(f"{s:<15} {a + ' -> ' + b:<22} {dd['mad']:>8.4f} {dd['p99']:>7.2f} {dd['chg%']:>7.3f} "
                  f"{sa['hp']:>7.3f} {sb['hp']:>7.3f} {hp_d:>+7.2f}% {spec_d:>+7.3f}  {what}: {verdict}")
        print()

    if out_json:
        with open(out_json, "w", encoding="utf-8") as f:
            json.dump(report, f, indent=2)
        print(f"wrote {out_json}")


if __name__ == "__main__":
    main()
