#!/usr/bin/env python3
"""EV-ladder report: G5 / G6 / p95 / warmth per plate per rung, WITH the pixel
coordinates the grader actually sampled.

Why the coordinates are mandatory output and not a debug flag: every axis here is
a single patch or a single pixel chosen by a search, and a gate verdict is only as
meaningful as the place it was read. A G6 that "passes" off a patch sitting on a
lit rim edge, or a p95 dominated by sky, is a number about the wrong thing — and
that failure is invisible in the verdict alone. Print the site with the score, so
a bad site can be seen rather than inferred.

Usage:  _grade_ev_ladder.py <label>=<dir-of-nohud2-frames> [<label>=<dir> ...]
"""
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Interior/exterior split for the G6 sanity note. A patch in the top band of a
# wide frame is sky or skyline, not the "sunlit wood" G6 is about.
SKY_BAND = 0.30


def run(script, *args):
    p = subprocess.run([sys.executable, str(HERE / script), *map(str, args)],
                       capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def grade(frame):
    """-> dict of the four axes + the sites they were read at."""
    out = {}
    rc, g = run("grade_gate.py", frame)
    out["_gate_raw"] = g

    m = re.search(r"## G5.*?brightest px @\((\d+),(\d+)\) RGB=\(([^)]*)\).*?"
                  r"min\(G,B\)=(\d+).*?spread=([\d.]+).*?-> (PASS|FAIL)", g, re.S)
    if m:
        out["G5"] = {"x": int(m.group(1)), "y": int(m.group(2)),
                     "rgb": m.group(3), "gb": int(m.group(4)),
                     "spread": float(m.group(5)), "pass": m.group(6) == "PASS"}

    m = re.search(r"## G6.*?golden patch @\((\d+),(\d+)\) RGB=\(([^)]*)\)\s*"
                  r"R-B=([+-][\d.]+)\s*L=([\d.]+).*?-> (PASS|FAIL)", g, re.S)
    if m:
        out["G6"] = {"x": int(m.group(1)), "y": int(m.group(2)),
                     "rgb": m.group(3), "rb": float(m.group(4)),
                     "L": float(m.group(5)), "pass": m.group(6) == "PASS"}
    elif "NO warm patch found" in g:
        out["G6"] = {"none": True, "pass": False}

    m = re.search(r"## G3.*?darkest shade @\((\d+),(\d+)\)", g, re.S)
    if m:
        out["G3site"] = (int(m.group(1)), int(m.group(2)))

    rc, a = run("grade_axes.py", "--profile", "gameplay", frame)
    out["_axes_raw"] = a
    for key, label in (("p95", r"highlight p95"), ("warmth", r"warmth R-B \(mid\)")):
        m = re.search(r"\[(PASS|FAIL|SKIP)\]\s+" + label + r"\s+([\d.-]+)", a)
        if m:
            out[key] = {"v": float(m.group(2)), "pass": m.group(1) == "PASS"}
    m = re.search(r"warmth\(global\)\s+([+-][\d.]+)", a)
    if m:
        out["warmth_global"] = float(m.group(1))
    return out


def frame_h(frame):
    from PIL import Image
    with Image.open(frame) as im:
        return im.size


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    rungs = []
    for spec in sys.argv[1:]:
        label, _, d = spec.partition("=")
        rungs.append((label, Path(d)))

    plates = sorted({f.name for _, d in rungs for f in d.glob("*-nohud2.png")})
    tick = {True: "PASS", False: "FAIL"}

    for plate in plates:
        print("=" * 100)
        print(f"PLATE  {plate}")
        print("=" * 100)
        print(f"{'ev100':>7} | {'G5':^5} {'G6':^5} | {'p95':>7} {'warmth':>7} "
              f"| {'G6 patch':>13} {'L':>5} {'R-B':>6} | {'G5 px':>13} {'sprd':>5}")
        print("-" * 100)
        for label, d in rungs:
            f = d / plate
            if not f.exists():
                print(f"{label:>7} | (frame missing: {f})")
                continue
            r = grade(f)
            W, H = frame_h(f)
            g5, g6 = r.get("G5", {}), r.get("G6", {})
            p95, wm = r.get("p95", {}), r.get("warmth", {})
            g6site = "none" if g6.get("none") else f"({g6.get('x')},{g6.get('y')})"
            g5site = f"({g5.get('x')},{g5.get('y')})"
            note = ""
            if not g6.get("none") and g6.get("y", H) < SKY_BAND * H:
                note = "  <-- G6 patch in TOP BAND (sky/skyline?)"
            print(f"{label:>7} | {tick[bool(g5.get('pass'))]:^5} {tick[bool(g6.get('pass'))]:^5} "
                  f"| {p95.get('v', float('nan')):7.2f} {wm.get('v', float('nan')):7.2f} "
                  f"| {g6site:>13} {g6.get('L', float('nan')):5.1f} {g6.get('rb', float('nan')):+6.1f} "
                  f"| {g5site:>13} {g5.get('spread', float('nan')):5.1f}{note}")
        print()


if __name__ == "__main__":
    main()
