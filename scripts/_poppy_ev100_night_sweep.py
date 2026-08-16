#!/usr/bin/env python3
"""Grade the Hour::NIGHT.ev100 ladder -- every rung against the SHIPPED 7.5 baseline.

WHY NOT `_poppy_ev100_night_gate.py`. That file grades ONE pair and hardcodes the
names `night-firelit_before/after.png`. It answered the only question a two-leg
shoot can answer ("is the compiled-in value better than the shipped one") and its
answer for 8.6 was FAIL on p05 / warmth / separation. Picking a replacement value
needs the whole ladder scored against one fixed reference, which is what this does.

EVERY BAR AND ESTIMATOR IS IMPORTED, NEVER RE-TYPED. `P05_FLOOR`, `BLOW_MAX`,
`CRUSH_MAX`, `BAND_MIN` and the `luma/warmth/orient/expo` estimators all come from
`_poppy_lookv7_gate` via `_poppy_ev100_night_gate`, which is the same chain the
single-pair gate uses. A rung that passes here therefore passes the identical bar
the pair gate applied to 8.6 -- this file cannot quietly lower the bar to make a
rung look good.

THE BASELINE IS A ROW IN THE TABLE, NOT A HIDDEN CONSTANT. `ev75` is graded
against itself and must come out all-zero deltas; that row is the positive
control for the harness. If it ever prints a nonzero delta the comparison is
broken and no other row means anything.

RUNTIME PROOF PER RUNG. The lever is read back out of `_sweep.log` and matched
against the rung's nominal value, so a rung whose env var never reached look.rs
is reported as a mismatch instead of being silently graded as if it had.

Usage: python scripts/_poppy_ev100_night_sweep.py [dir]
Exit:  0 = at least one rung clears every bar
"""
import sys
import re
import pathlib
import hashlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import numpy as np

_gate = __import__("_poppy_ev100_night_gate")
_v7 = _gate._v7
load, luma, warmth, orient = _v7.load, _v7.luma, _v7.warmth, _v7.orient
expo, P05_FLOOR = _v7.expo, _v7.P05_FLOOR
BLOW_MAX, CRUSH_MAX, BAND_MIN = _v7.BLOW_MAX, _v7.CRUSH_MAX, _v7.BAND_MIN

SCENE = "night-firelit"
BASE = "ev75"
# (tag, nominal ev100) -- must match the rungs _poppy_ev100_night_sweep.cmd shoots.
RUNGS = [("ev86", 8.6), ("ev75", 7.5), ("ev70", 7.0), ("ev65", 6.5), ("ev60", 6.0)]


def levers_from_log(log):
    """Rung tag -> the ev100 look.rs actually applied, straight out of the run log."""
    if not log.exists():
        return {}
    txt = log.read_text(errors="replace")
    seen, tag = {}, None
    for line in txt.splitlines():
        m = re.search(r"\[sweep\]\s+\S+\s+(\w+)\s+gen=v3\s+ev=([0-9.]+)", line)
        if m:
            tag = m.group(1)
            continue
        m = re.search(r"LOOK_FILL .*?ev100=([0-9.]+)", line)
        if m and tag is not None and tag not in seen:
            seen[tag] = float(m.group(1))
    return seen


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_poppy_ev100night_sweep")
    applied = levers_from_log(out / "_sweep.log")

    imgs, md5 = {}, {}
    for tag, _ in RUNGS:
        p = out / f"{SCENE}_{tag}.png"
        if not p.exists():
            print(f"MISSING {p}")
            return 1
        md5[tag] = hashlib.md5(p.read_bytes()).hexdigest()
        imgs[tag] = load(p)

    if len(set(md5.values())) != len(md5):
        print("  NOT ALL RUNGS DISTINCT -- the lever did not move between shoots:")
        for t, h in md5.items():
            print(f"    {t:6s} {h}")
        return 1

    base = imgs[BASE]
    lb = luma(base)
    Bp05, Bp50, Bp95 = (float(np.percentile(lb, q)) for q in (5, 50, 95))
    Bw, Bsp = warmth(base, lb), orient(base, lb)

    print(f"--- Hour::NIGHT.ev100 ladder vs shipped {BASE} ({out}) ---")
    print(f"  bars: p05 >= {P05_FLOOR}   warmth/sep >= baseline   "
          f"blow <= {BLOW_MAX}%   crush <= {CRUSH_MAX}%   band >= {BAND_MIN}")
    print()
    hdr = (f"{'rung':>6s} {'ev':>5s} {'ran':>5s} {'p05':>7s} {'p50':>7s} {'p95':>7s} "
           f"{'warm':>7s} {'spread':>7s} {'blow%':>7s} {'crush%':>7s} {'band':>7s} "
           f"{'d.warm':>7s} {'d.sep':>7s} {'%diff':>6s}  verdict")
    print(hdr)

    winners = []
    for tag, ev in RUNGS:
        img = imgs[tag]
        l = luma(img)
        p05, p50, p95 = (float(np.percentile(l, q)) for q in (5, 50, 95))
        w, sp = warmth(img, l), orient(img, l)
        blow, crush, band = expo(img, l)
        pct = float(np.mean(np.any(img.astype(np.int16) != base.astype(np.int16),
                                   axis=-1)) * 100.0)
        ran = applied.get(tag)
        ok_lever = ran is not None and abs(ran - ev) < 1e-6

        c = {
            "floor": p05 >= P05_FLOOR,
            "warm": w >= Bw,
            "sep": sp >= Bsp,
            "blow": blow <= BLOW_MAX,
            "crush": crush <= CRUSH_MAX,
            "band": band >= BAND_MIN,
        }
        failed = [k for k, v in c.items() if not v]
        verdict = "PASS" if not failed else "fail:" + ",".join(failed)
        if not ok_lever:
            verdict = f"LEVER-MISMATCH(ran={ran})"
        elif not failed:
            winners.append((tag, ev, w - Bw, sp - Bsp, p05))

        print(f"{tag:>6s} {ev:5.1f} {('%.2f' % ran) if ran is not None else '   ?':>5s} "
              f"{p05:7.2f} {p50:7.2f} {p95:7.2f} {w:7.2f} {sp:7.2f} "
              f"{blow:7.3f} {crush:7.3f} {band:7.2f} "
              f"{w-Bw:7.2f} {sp-Bsp:7.2f} {pct:6.2f}  {verdict}")

    print()
    print(f"  positive control: {BASE} must read d.warm=0.00 d.sep=0.00 %diff=0.00")
    if not winners:
        print("NIGHT EV100 LADDER: NO RUNG CLEARS EVERY BAR -- keep the shipped 7.5")
        return 1

    # Rank by the axes this round is actually trying to buy back: warmth first,
    # then separation. Ties broken by the brighter floor.
    winners.sort(key=lambda r: (r[2] + r[3], r[4]), reverse=True)
    best = winners[0]
    print(f"  rungs clearing every bar: {', '.join(w[0] for w in winners)}")
    print(f"NIGHT EV100 LADDER: PICK {best[0]} (ev100={best[1]}) "
          f"d.warm={best[2]:+.2f} d.sep={best[3]:+.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
