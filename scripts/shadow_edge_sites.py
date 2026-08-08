#!/usr/bin/env python3
"""Where do you put the `--at` probes, and who chose the plate they were locked on?

`scripts/cast_shadow_penumbra.py` measures the edge a human points at. It is
honest about everything except the pointing, and the pointing is where the
2026-08-08 N6 note went wrong. Fourteen sites were snapped to the gradient ridge
of the **N5** plate and the same pixel coordinates were then read off `before`.
Thirteen of them came back "no gradeable ramp", and that was written up as "the
relight created the edge". It cannot support that sentence, for two reasons this
file exists to remove:

  1. THE SITE SET IS A FILTER FAVOURING ITS OWN PLATE. A site is kept only if it
     was gradeable on the lock plate, so the lock plate scores 14/14 by
     construction and every other plate pays for any disagreement. Run the lock
     the other way and the winner flips. So this tool refuses to lock once: it
     locks on EVERY plate in turn and prints the whole square. A claim survives
     only if it holds in the direction that does not favour it.
  2. THE EDGE MOVED. 8303db5 took Hour::GOLDEN's elev_deg 17 -> 22, which slides
     every cast shadow across its receiver. "No ramp at (x,y)" is then
     indistinguishable from "the ramp is at (x+d,y)". `--track` walks the
     gradient normal out to +/-TRACK px looking for the same edge somewhere else
     and reports the displacement, so a moved edge reads as moved, not as absent.

And the defect underneath both: the 13:53 coordinates were never written to
disk, so the round that tried to re-check them had to invent new ones. Every run
of this tool dumps its site list to --out and echoes a paste-ready `--at`
string. Derivation is a pure function of (plate, roi, orient, n) -- no random
seed, no interactive picking -- so the JSON is a convenience, not the only copy.

Usage:
  shadow_edge_sites.py --roi X0,Y0,X1,Y1 [--n 14] [--orient 30,46] [--track 24]
                       --out sites.json  <label>=<plate>-nohud2.png ...
"""
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cast_shadow_penumbra import Frame, by_orient, collect, thin  # noqa: E402
from cast_shadow_penumbra import MAX_EDGES, ORIENT_BINS          # noqa: E402
from nohud2_guard import require_nohud2                          # noqa: E402


def derive(fr, roi, orient, n):
    """The n sites of this plate's strongest coherent edge inside `roi`.

    Deterministic: ridge pixels, filtered to ones that actually yield a ramp in
    the requested orientation band, ordered by (y, x) and thinned by a fixed
    stride. Same inputs -> same list, on any box, in any order of runs.
    """
    x0, y0, x1, y1 = roi
    box = np.zeros((fr.H, fr.W), bool)
    box[y0:y1, x0:x1] = True
    cand = []
    for y, x in thin(fr.ridge(~fr.sky & box), MAX_EDGES):
        r = fr.profile(int(x), int(y))
        if r and orient[0] <= r[1] < orient[1]:
            cand.append((int(x), int(y)))
    if len(cand) <= n:
        return cand
    cand.sort(key=lambda p: (p[1], p[0]))
    step = len(cand) / float(n)
    return [cand[int(i * step)] for i in range(n)]


def track(fr, x, y, nx, ny, dist):
    """Nearest gradeable ramp to (x,y) along the lock plate's own normal.

    Returns (signed offset px, width, orientation) or None. This is what tells a
    shadow that MOVED apart from a shadow that vanished.
    """
    best = None
    for d in range(0, int(dist) + 1):
        for s in ((0,) if d == 0 else (+1, -1)):
            xx, yy = int(round(x + nx * d * s)), int(round(y + ny * d * s))
            if not (0 <= xx < fr.W and 0 <= yy < fr.H):
                continue
            r = fr.profile(xx, yy)
            if r:
                return d * s, r[0], r[1]
    return best


def population(fr, roi, orient):
    """Every gradeable edge in the ROI in the orientation band -- no site list.

    The site table above still has to pick 14 pixels somewhere; this does not
    pick at all, so it survives the edge moving. It is the number to quote when
    the question is "is this plate's edge wider than that plate's edge".
    """
    x0, y0, x1, y1 = roi
    box = np.zeros((fr.H, fr.W), bool)
    box[y0:y1, x0:x1] = True
    w = [r[0] for y, x in thin(fr.ridge(~fr.sky & box), 4000)
         for r in [fr.profile(int(x), int(y))]
         if r and orient[0] <= r[1] < orient[1]]
    return np.array(w)


def control(fr):
    """This plate's own sky silhouette: geometry, zero penumbra, per orientation."""
    w, o = collect(fr, thin(fr.ridge(fr.sky), MAX_EDGES), allow_sky=True)
    return w, by_orient(w, o)


def measure(fr, sites, normals, ctl_w, ctl_b, dist):
    got, widths, offs, soft = 0, [], [], 0
    rows = []
    for (x, y), (nx, ny) in zip(sites, normals):
        r = fr.profile(x, y)
        if r:
            got += 1
            widths.append(r[0])
            pool = next((ctl_b[b] for b in ORIENT_BINS if b[0] <= r[1] < b[1]), np.array([]))
            if pool.size and r[0] >= 1.5 * np.median(pool):
                soft += 1
            rows.append((x, y, 0, r[0]))
        elif dist:
            t = track(fr, x, y, nx, ny, dist)
            if t:
                offs.append(abs(t[0]))
                rows.append((x, y, t[0], t[1]))
            else:
                rows.append((x, y, None, None))
        else:
            rows.append((x, y, None, None))
    return {
        "hit": got,
        "median": float(np.median(widths)) if widths else None,
        "control": float(np.median(ctl_w)) if ctl_w.size else None,
        "ratio": (float(np.median(widths) / np.median(ctl_w))
                  if widths and ctl_w.size else None),
        "soft": soft,
        "found_nearby": len(offs),
        "median_offset": float(np.median(offs)) if offs else None,
        "rows": rows,
    }


def cell(m, n):
    if m["hit"]:
        return f"{m['hit']:2d}/{n}  {m['median']:5.2f}px {m['ratio']:.2f}x  SOFT {m['soft']}"
    if m["found_nearby"]:
        return f" 0/{n}  edge found {m['median_offset']:.0f}px away ({m['found_nearby']}/{n})"
    return f" 0/{n}  --"


def main(argv=None):
    argv = list(argv if argv is not None else sys.argv[1:])
    roi, n, orient, dist, out, plates = None, 14, (30, 46), 24, None, []
    while argv:
        a = argv.pop(0)
        if a == "--roi":
            roi = tuple(int(v) for v in argv.pop(0).split(","))
        elif a == "--n":
            n = int(argv.pop(0))
        elif a == "--orient":
            orient = tuple(float(v) for v in argv.pop(0).split(","))
        elif a == "--track":
            dist = int(argv.pop(0))
        elif a == "--out":
            out = argv.pop(0)
        else:
            plates.append(a.split("=", 1) if "=" in a else [os.path.basename(a), a])
    if not roi or not plates:
        print(__doc__)
        return 2
    require_nohud2([p for _, p in plates], tool="shadow_edge_sites.py")

    fr = {k: Frame(p) for k, p in plates}
    ctl = {k: control(fr[k]) for k, _ in plates}
    print(f"ROI {roi}  n={n}  orient {orient[0]:.0f}-{orient[1]:.0f}deg off-axis  "
          f"track +/-{dist}px")
    for k, _ in plates:
        print(f"   {k:8s} control sky silhouette median "
              f"{np.median(ctl[k][0]):5.2f}px (n={ctl[k][0].size})")

    dump = {"roi": list(roi), "n": n, "orient": list(orient), "track": dist,
            "plates": {k: p for k, p in plates}, "population": {}, "locks": {}}

    print("\nPOPULATION -- every gradeable edge in the ROI, no site picking:")
    for k, _ in plates:
        w = population(fr[k], roi, orient)
        cb = next((ctl[k][1][b] for b in ORIENT_BINS
                   if b[0] <= orient[0] < b[1]), np.array([]))
        r = float(np.median(w) / np.median(cb)) if w.size and cb.size else None
        print(f"   {k:8s} n={w.size:4d} median={np.median(w) if w.size else float('nan'):5.2f}px "
              f"p25={np.percentile(w, 25) if w.size else float('nan'):5.2f} "
              f"p75={np.percentile(w, 75) if w.size else float('nan'):5.2f}"
              + (f"  vs same-orientation control {np.median(cb):5.2f}px = {r:.2f}x"
                 if r else "  (no control at this orientation)"))
        dump["population"][k] = {
            "n": int(w.size),
            "median": float(np.median(w)) if w.size else None,
            "ratio": r,
        }
    print(f"\n{'lock \\ measured':>18s} | " +
          " | ".join(f"{k:^34s}" for k, _ in plates))
    for lk, _ in plates:
        sites = derive(fr[lk], roi, orient, n)
        normals = []
        for x, y in sites:
            g = np.hypot(fr[lk].gx[y, x], fr[lk].gy[y, x])
            normals.append((fr[lk].gx[y, x] / g, fr[lk].gy[y, x] / g))
        row, cells = {}, []
        for mk, _ in plates:
            m = measure(fr[mk], sites, normals, ctl[mk][0], ctl[mk][1], dist)
            row[mk] = m
            cells.append(f"{cell(m, len(sites)):^34s}")
        print(f"{'locked on ' + lk:>18s} | " + " | ".join(cells))
        dump["locks"][lk] = {
            "sites": sites,
            "at": ";".join(f"{x},{y}" for x, y in sites),
            "measured": {k: {kk: vv for kk, vv in v.items() if kk != "rows"}
                         for k, v in row.items()},
            "rows": {k: v["rows"] for k, v in row.items()},
        }

    for lk, _ in plates:
        print(f"\n--at for sites locked on {lk}:\n  {dump['locks'][lk]['at']}")
    if out:
        with open(out, "w") as f:
            json.dump(dump, f, indent=1)
        print(f"\nwritten: {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
