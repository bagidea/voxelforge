#!/usr/bin/env python3
"""Flamingo - grade a whole ladder directory into one table.

Reads pixels only. Every column comes from `_flamingo_huegap_measure.measure`,
so the ladder and the final before/after are scored by the SAME instrument.

Usage:
    python scripts/_flamingo_huegap_table.py <dir-or-files...> [--json=OUT.json]
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _flamingo_huegap_measure import measure  # noqa: E402

HDR = (f'{"plate":<26}{"skyHue":>7}{"skyC":>7}{"skyRmB":>7}{"skyL":>6}'
       f'{"grassGE":>8}{"p95-p05":>8}{"p99-p01":>8}{"minDE":>7}{"hueSTD":>7}  gates')


def row(r: dict) -> str:
    def f(k, w, nd=1):
        v = r.get(k)
        return f'{"n/a":>{w}}' if v is None else f'{v:>{w}.{nd}f}'
    g = "".join(
        {True: "P", False: "F", None: "?"}[r.get(k)]
        for k in ("a_sky_cool", "b_grass_green", "c_range", "d_three_zones"))
    return (f'{r["label"][:26]:<26}{f("sky_hue_deg",7)}{f("sky_chroma",7,3)}'
            f'{f("sky_median_RmB",7)}{f("sky_median_L",6)}{f("grass_green_excess",8)}'
            f'{f("range_p95_p05",8)}{f("range_p99_p01",8)}{f("zone_min_dE00",7)}'
            f'{f("frame_hue_std_deg",7)}  {g}')


def main() -> int:
    files, jout = [], None
    for a in sys.argv[1:]:
        if a.startswith("--json="):
            jout = a.split("=", 1)[1]
        elif Path(a).is_dir():
            files += sorted(Path(a).glob("*.png"))
        else:
            files.append(Path(a))
    if not files:
        print(__doc__)
        return 2
    print(HDR)
    print("-" * len(HDR))
    rows = []
    for p in files:
        r = measure(str(p))
        r["label"] = p.stem
        rows.append(r)
        print(row(r))
    print()
    print("gates = a_sky_cool / b_grass_green / c_range(p95-p05>=150) / d_three_zones")
    print("        P pass   F fail   ? UNMEASURABLE (mask too small - never scored as pass)")
    if jout:
        Path(jout).parent.mkdir(parents=True, exist_ok=True)
        Path(jout).write_text(json.dumps(rows, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
