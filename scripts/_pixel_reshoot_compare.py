#!/usr/bin/env python3
"""Did the re-shot plates come back the same? — measured, per plate, per axis.

Compares freshly-shot beauty plates against `docs/assets/look/beauty/_grid.json`,
the 2026-08-16 measurement of the plates `git clean -fd` destroyed.  Same grader
(`_flamingo_beauty_axes.py`), same axes, same all-pixel mask the JSON was written
with, so the two columns are commensurable.

WHAT A DELTA HERE IS NOT ALLOWED TO MEAN, WITHOUT A CONTROL.  Two frames of the
same scene never match bit for bit: TAA/SSAO dither alone moves a good fraction
of pixels between two runs of the identical command.  So a raw delta cannot be
read as "the look changed" until it is put against this capture's own noise
floor.  `--noise DIR` takes a second shot of the same plates on the same binary
and prints, per axis, the floor that any real change has to clear.  `--baseline-exe DIR`
does the third leg: the same plates on the ORIGINAL binary, which separates
"the binary changed" from "my re-shoot recipe is wrong".

Plates with no row in the JSON (the 8 `fill-*` plates were shot after the table
was written, and the probe pass was never tabled) are printed as NO-BASELINE
rather than silently dropped or compared against something else.

Usage:
  python scripts/_pixel_reshoot_compare.py --dir docs/assets/look/beauty/ladder
      [--noise _pixel_reshoot/noise] [--baseline-exe _pixel_reshoot/ctl97cb283]
      [--json OUT.json]
"""
import argparse
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "beauty_axes", HERE / "_flamingo_beauty_axes.py")
axes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(axes)

GRID = Path("docs/assets/look/beauty/_grid.json")

# label, getter, print format.  The five the ladder steers plus the sky mask,
# which is the gate that decides whether the sky axes may be scored at all.
AXES = [
    ("value_span", lambda r: r["structure"]["L_p95_minus_p5"], "{:7.1f}", 0.1),
    ("shadow_p5", lambda r: r["structure"]["L_p"][0], "{:7.1f}", 0.1),
    ("chroma_span", lambda r: r["structure"]["sat_p95_minus_p5"], "{:7.3f}", 0.001),
    ("sat_p5", lambda r: r["structure"]["sat_p"][0], "{:7.3f}", 0.001),
    ("sky_px_pct", lambda r: r["sky_px_pct"], "{:7.2f}", 0.01),
]


def measure_dir(d):
    out = {}
    for p in sorted(Path(d).glob("*.png")):
        out[p.stem] = axes.grade(str(p), None, 0, p.stem)
    return out


def deltas(a, b, stems):
    """max |a-b| per axis over the given stems."""
    m = {}
    for name, get, _, _ in AXES:
        vals = [abs(get(a[s]) - get(b[s])) for s in stems if s in a and s in b]
        m[name] = max(vals) if vals else None
    return m


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--noise", help="second shot of the same plates, same binary")
    ap.add_argument("--baseline-exe", help="same plates on the ORIGINAL binary")
    ap.add_argument("--grid", default=str(GRID))
    ap.add_argument("--json", help="write the full comparison here")
    a = ap.parse_args()

    base = json.loads(Path(a.grid).read_text(encoding="utf-8"))
    baseline = {c["label"]: c for c in base["cells"].values()}
    new = measure_dir(a.dir)
    print(f"new plates: {len(new)} in {a.dir}")
    print(f"baseline  : {len(baseline)} rows in {a.grid}\n")

    # ---- leg 2: this capture's own noise floor --------------------------------
    floor = None
    if a.noise:
        noise = measure_dir(a.noise)
        shared = sorted(set(noise) & set(new))
        floor = deltas(new, noise, shared)
        print(f"--- NOISE FLOOR: same binary, same env, shot twice ({len(shared)} plates) ---")
        for name, _, fmt, _ in AXES:
            v = floor[name]
            print(f"  {name:<12} max|d| {fmt.format(v) if v is not None else '   n/a'}")
        print()

    # ---- leg 3: the original binary, same recipe -----------------------------
    ctl = None
    if a.baseline_exe:
        ctlm = measure_dir(a.baseline_exe)
        shared = sorted(set(ctlm) & set(baseline))
        ctl = deltas(ctlm, baseline, shared)
        print(f"--- CONTROL: ORIGINAL binary re-run through this recipe, vs the 08-16 JSON "
              f"({len(shared)} plates) ---")
        print("    a small delta here means the recipe reproduces the lost plates;")
        print("    a large one means something other than the binary moved.")
        for name, _, fmt, _ in AXES:
            v = ctl[name]
            print(f"  {name:<12} max|d| {fmt.format(v) if v is not None else '   n/a'}")
        print()

    # ---- the grid itself ------------------------------------------------------
    rows = []
    hdr = f"{'plate':<40}" + "".join(f"{n:>26}" for n, _, _, _ in AXES)
    print("--- NEW vs 2026-08-16 (all-pixel mask, the mask the JSON used) ---")
    print(hdr)
    print("-" * len(hdr))
    nobase = []
    for stem in sorted(new):
        if stem not in baseline:
            nobase.append(stem)
            continue
        line = f"{stem:<40}"
        rec = dict(plate=stem)
        for name, get, fmt, _ in AXES:
            nv, ov = get(new[stem]), get(baseline[stem])
            d = nv - ov
            over = ""
            if floor and floor.get(name) is not None:
                over = "*" if abs(d) > floor[name] else " "
            line += f"{fmt.format(ov) + ' ->' + fmt.format(nv) + f' ({d:+.3g})' + over:>26}"
            rec[name] = dict(old=ov, new=nv, delta=d)
        rows.append(rec)
        print(line)

    if nobase:
        print(f"\nNO-BASELINE ({len(nobase)}) — never in _grid.json, measured only, not compared:")
        for stem in nobase:
            r = new[stem]
            print(f"  {stem:<40} " + "  ".join(
                f"{n}=" + fmt.format(get(r)).strip() for n, get, fmt, _ in AXES))

    print("\n* = |delta| exceeds this capture's own noise floor (a real move, not dither)"
          if floor else "\n(no --noise leg: deltas are unqualified — do not call any of them real)")

    if a.json:
        Path(a.json).write_text(json.dumps(
            dict(dir=a.dir, floor=floor, control=ctl, rows=rows, no_baseline=nobase),
            indent=1, default=float), encoding="utf-8")
        print(f"\nwrote {a.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
