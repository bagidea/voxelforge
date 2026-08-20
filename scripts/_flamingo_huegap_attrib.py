#!/usr/bin/env python3
"""Flamingo - attribute the hue-gap move to individual constants, and test the
`haze_color()` -> cloud-ambient claim against a falsifying control.

Consumes a ladder directory shot by `_flamingo_huegap_ladder.ps1` on ONE binary
(so every delta is the env lever and nothing else) and prints three blocks:

  1. NOISE FLOOR. N0 and N1 are the same arm shot twice. Every delta below is
     printed with a `>floor?` verdict against 3x this, because a lever that
     moves a metric by less than the capture disagrees with itself is not a
     measurement. Missing N0/N1 => the floor is UNKNOWN and no verdict is
     printed, rather than a floor of 0 being assumed.

  2. BEFORE -> AFTER, plus per-constant attribution. B0 restores all three old
     values; B1/B2/B3 each put ONE constant back to its baked value, so the
     column is that constant's own contribution rather than the bundle's.

  3. THE 2x2. The claim is that `HAZE_COOL` reaches SKY pixels only through
     `CloudKnobs::amb_warm = haze_color()`. With the deck off there is no such
     path, so the sky-region delta must collapse to the floor:

        deck ON : B0 -> B1   (expected: large)
        deck OFF: P0 -> P1   (expected: at the floor)

     PASS requires BOTH. A large ON delta alone proves only that the lever does
     something; if the OFF delta is just as large the sky is being moved by
     something that is not the deck and the claim is REFUTED, which is the
     outcome this block exists to be able to return.

Usage:
    python scripts/_flamingo_huegap_attrib.py <ladder-dir> [--plate=riverbend]
                                              [--json=OUT.json]
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _flamingo_huegap_measure import measure  # noqa: E402

# The axes an attribution is meaningful on. (key, label, digits)
AXES = [
    ("sky_median_RmB", "skyRmB", 1),
    ("sky_hue_deg", "skyHue", 1),
    ("range_p95_p05", "p95-p05", 1),
    ("zone_min_dE00", "minDE", 1),
    ("frame_hue_std_deg", "hueSTD", 1),
]
# A delta must beat this multiple of the measured noise floor to be called real.
FLOOR_K = 3.0


def load(d: Path, plate: str) -> dict:
    """arm id -> measured row, for one plate. Arm id is the bit after the last '-'."""
    out = {}
    for p in sorted(d.glob(f"{plate}-*.png")):
        arm = p.stem.rsplit("-", 1)[-1]
        r = measure(str(p))
        r["_file"] = str(p)
        out[arm] = r
    return out


def delta(a: dict, b: dict, key: str):
    """b[key] - a[key], or None if either side could not be measured."""
    x, y = a.get(key), b.get(key)
    if x is None or y is None:
        return None
    return float(y) - float(x)


def floors(rows: dict) -> dict:
    """Per-axis |N1 - N0|. Empty dict when the pair was not shot."""
    if "N0" not in rows or "N1" not in rows:
        return {}
    f = {}
    for key, _, _ in AXES:
        d = delta(rows["N0"], rows["N1"], key)
        f[key] = None if d is None else abs(d)
    return f


def verdict(d, fl):
    if d is None:
        return "unmeasurable"
    # A floor of exactly 0 means the two noise takes are byte-identical on this
    # axis. Real captures of a TAA + stochastic-SSAO frame are not, so a 0 floor
    # says the pair is degenerate (same file twice, or the arm never re-shot) -
    # NOT that the instrument is infinitely sensitive. Refuse to grade on it.
    if fl is None or fl <= 0.0:
        return "floor-unknown"
    return "REAL" if abs(d) >= FLOOR_K * fl else "at-floor"


def block_deltas(title, rows, base, arms, fl, sink):
    print()
    print(title)
    hdr = f'{"arm":<6}{"what":<30}' + "".join(f'{lab:>10}' for _, lab, _ in AXES)
    print(hdr)
    print("-" * len(hdr))
    if base not in rows:
        print(f"  MISSING baseline arm {base} - nothing in this block can be computed")
        return
    for arm, what in arms:
        if arm not in rows:
            print(f'{arm:<6}{what:<30}' + "".join(f'{"absent":>10}' for _ in AXES))
            continue
        cells, rec = "", {}
        for key, _, nd in AXES:
            d = delta(rows[base], rows[arm], key)
            v = verdict(d, fl.get(key))
            rec[key] = {"delta": d, "verdict": v}
            if d is None:
                cells += f'{"n/a":>10}'
            else:
                mark = "" if v == "REAL" else ("~" if v == "at-floor" else "?")
                cells += f'{d:>+9.{nd}f}{mark:1}'
        print(f'{arm:<6}{what:<30}{cells}')
        sink[f"{base}->{arm}"] = rec
    print('  "~" = inside 3x the capture noise floor, i.e. NOT a real move.')


def main() -> int:
    args = [a for a in sys.argv[1:]]
    d = None
    plates, jout = [], None
    for a in args:
        if a.startswith("--plate="):
            plates.append(a.split("=", 1)[1])
        elif a.startswith("--json="):
            jout = a.split("=", 1)[1]
        elif d is None:
            d = Path(a)
    if d is None or not d.is_dir():
        print(__doc__)
        return 2
    if not plates:
        plates = sorted({p.stem.rsplit("-", 1)[0] for p in d.glob("*.png")})

    report = {}
    worst = 0
    for plate in plates:
        rows = load(d, plate)
        if not rows:
            print(f"\n=== {plate}: no frames in {d}")
            worst = max(worst, 2)
            continue
        sink = {}
        print()
        print("=" * 96)
        print(f"PLATE {plate}   ({len(rows)} arms: {', '.join(sorted(rows))})")
        print("=" * 96)

        # ---- 1. noise floor -------------------------------------------------
        fl = floors(rows)
        print()
        print("NOISE FLOOR  |N1 - N0|, same arm shot twice")
        if not fl:
            print("  UNKNOWN - N0/N1 were not both shot. Deltas below are printed")
            print("  but carry no REAL/at-floor verdict, and must not be read as proof.")
        else:
            print("  " + "  ".join(
                f"{lab}={'n/a' if fl.get(k) is None else format(fl[k], f'.{nd}f')}"
                for k, lab, nd in AXES))
            print(f"  a delta counts as real at >= {FLOOR_K:g}x these.")

        # ---- 2. before/after + per-constant ---------------------------------
        block_deltas(
            "BEFORE(B0) -> ...   positive = the arm reads HIGHER than the old default",
            rows, "B0",
            [("A0", "AFTER: all three baked"),
             ("B1", "only HAZE_COOL  0.35->0.72"),
             ("B2", "only HAZE_FULL  72->240"),
             ("B3", "only AMBIENT    620->420")],
            fl, sink)

        # ---- 3. the 2x2 -----------------------------------------------------
        print()
        print("2x2 CLOUD-AMBIENT PATH TEST   (metric: sky_median_RmB, sky region only)")
        key = "sky_median_RmB"
        on = delta(rows.get("B0", {}), rows.get("B1", {}), key)
        off = delta(rows.get("P0", {}), rows.get("P1", {}), key)
        f_k = fl.get(key)
        print(f"  deck ON   B0 -> B1 : {'n/a' if on is None else format(on, '+.1f')}")
        print(f"  deck OFF  P0 -> P1 : {'n/a' if off is None else format(off, '+.1f')}")
        if on is None or off is None:
            v = "INCONCLUSIVE - an arm of the 2x2 is missing"
        elif f_k is None:
            v = "INCONCLUSIVE - no noise floor was measured"
        elif f_k <= 0.0:
            v = ("INCONCLUSIVE - the two noise takes agree EXACTLY on this axis, "
                 "which a real re-shot capture does not; the floor is degenerate "
                 "so nothing here can be graded against it")
        elif abs(on) < FLOOR_K * f_k:
            v = "INCONCLUSIVE - the lever does not move the sky even with the deck on"
        elif abs(off) < FLOOR_K * f_k:
            v = ("CONFIRMED - the sky move needs the deck; haze_color() reaching the "
                 "sky through CloudKnobs::amb_warm is the path")
        else:
            v = ("REFUTED - the sky moves just as much with no deck in frame, so the "
                 "cloud-ambient path is NOT what carries it")
        print(f"  VERDICT: {v}")
        sink["_2x2"] = {"on": on, "off": off, "floor": f_k, "verdict": v}
        report[plate] = sink
        if v.startswith("REFUTED") or v.startswith("INCONCLUSIVE"):
            worst = max(worst, 1)

    if jout:
        Path(jout).parent.mkdir(parents=True, exist_ok=True)
        Path(jout).write_text(json.dumps(report, indent=2), encoding="utf-8")
    return worst


if __name__ == "__main__":
    raise SystemExit(main())
