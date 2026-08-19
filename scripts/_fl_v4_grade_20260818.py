#!/usr/bin/env python3
"""Flamingo — the v4 look pass: CALIBRATE the grader, then grade.

WHY THIS FILE EXISTS AT ALL — the scar it is built against
----------------------------------------------------------
2026-08-09 I published a saturation ladder that was ONE plate's numbers wearing
the whole set's name, and the CEO caught it by re-running the shot himself. The
lesson was not "measure more", it was: **a number is worth nothing until the
instrument that produced it has been shown to reproduce a known answer.** So
this script refuses to grade anything until `control` passes.

CONTROL = re-measure inputs whose answers are already published, and diff.
  A. `_kevin_art_gap_measure.measure()` on the REF middle panel
     (`_kevin_ref_panel_mid.png`) must reproduce the table in
     `docs/art-gap-vs-reference-2026-08-17.md` §"REF vs OURS".
  B. the same function on the shipped outdoor plate
     (`_fl_lookv4/outdoor-noon-after-nohud2.png`) must reproduce the OURS-outdoor
     column of that same table. This control pins TWO things at once: the code
     path AND the plate file — if someone re-shot the plate on disk, B fails and
     I find out before I quote a delta against it.
  C. `grade_axes.measure()` on the golden beauty reference must reproduce the
     REF column baked into `scripts/grade_axes.py`'s TARGETS table.

If a control fails, THE INSTRUMENT IS BROKEN, NOT THE IMAGE — exit 2 ("refused
to measure", per the office's separate-unmeasurable-from-failed rule), print the
drift, and grade nothing.

THE `-nohud2` COPY IN CONTROL C
    `grade_axes` refuses any filename not ending `-nohud2.png`. The golden
    reference is curated artwork that never had a HUD — `nohud2_guard.EXEMPT`
    says so in writing for `grade_ref.py`, whose input is this same file. So the
    copy this script makes is that same exemption spelled with a filename, not a
    frame being smuggled past the gate. Nothing else is ever copied.

USAGE
    python scripts/_fl_v4_grade_20260818.py control
    python scripts/_fl_v4_grade_20260818.py measure <label>=<path> [...] [--out DIR]
    python scripts/_fl_v4_grade_20260818.py baseline      # control + every shipped plate
"""
import argparse
import json
import os
import shutil
import sys

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)

from _kevin_art_gap_measure import measure  # noqa: E402
import grade_axes  # noqa: E402

OUTDIR = os.path.join(ROOT, "_fl_v4_20260818")
EXIT_REFUSED = 2

# ---------------------------------------------------------------------------
# CONTROL A/B — published in docs/art-gap-vs-reference-2026-08-17.md
# ---------------------------------------------------------------------------
# Values as PRINTED in that doc. tol = half of the last printed digit, so a
# correct re-run lands inside it and a real drift does not. (A doc rounds; a
# control has to tolerate exactly the rounding and nothing more.)
CONTROL_ART = {
    "REF_mid": {
        "path": "_kevin_ref_panel_mid.png",
        "expect": {
            "hue90": (202, 0.5), "occupied": (25, 0.5),
            "sat_mean": (0.589, 0.0005), "sat_std": (0.322, 0.0005),
            "shadow": (0.256, 0.0005), "midtone": (0.326, 0.0005),
            "highlight": (0.418, 0.0005), "lum_mean": (148.6, 0.05),
            "edge_mean": (60.5, 0.05), "strong": (0.448, 0.0005),
        },
    },
    "OURS_outdoor_v3": {
        "path": os.path.join("_fl_lookv4", "outdoor-noon-after-nohud2.png"),
        "expect": {
            "hue90": (52, 0.5), "occupied": (8, 0.5),
            "sat_mean": (0.718, 0.0005), "sat_std": (0.230, 0.0005),
            "shadow": (0.413, 0.0005), "midtone": (0.574, 0.0005),
            "highlight": (0.013, 0.0005), "lum_mean": (91.9, 0.05),
            "edge_mean": (36.8, 0.05), "strong": (0.336, 0.0005),
        },
    },
}

# ---------------------------------------------------------------------------
# CONTROL C — the REF column of scripts/grade_axes.py TARGETS, read from the
# table itself rather than retyped, so the control cannot drift from the gate.
# ---------------------------------------------------------------------------
GOLDEN_REF = os.path.join("docs", "assets", "golden-beauty-shot-ref.png")
GOLDEN_COPY = os.path.join("_fl_v4_20260818", "golden-beauty-shot-ref-nohud2.png")
AXES_TOL = {"warmth": 0.05, "blue": 0.05, "clip": 0.05, "sat": 0.05,
            "dof": 0.005, "micro": 0.005, "p95": 0.05}

# Plates the v4 pass is graded against (shipped v3 set, already de-HUDded).
SHIPPED_PLATES = [
    ("outdoor-noon_v3-after",   "_fl_lookv4/outdoor-noon-after-nohud2.png"),
    ("outdoor-noon_v2-before",  "_fl_lookv4/outdoor-noon-before-nohud2.png"),
    ("evening-raking_v3-after", "_fl_lookv4/evening-raking-after-nohud2.png"),
    ("evening-raking_v2-before", "_fl_lookv4/evening-raking-before-nohud2.png"),
    ("night-firelit_v3-after",  "_fl_lookv4/night-firelit-after-nohud2.png"),
    ("night-firelit_v2-before", "_fl_lookv4/night-firelit-before-nohud2.png"),
]

ART_COLS = [
    ("lum_mean", "lum mean", "{:.1f}", 1.0),
    ("shadow", "shadow %", "{:.1f}", 100.0),
    ("midtone", "midtone %", "{:.1f}", 100.0),
    ("highlight", "highlight %", "{:.1f}", 100.0),
    ("edge_mean", "edge", "{:.2f}", 1.0),
    ("strong", "strong %", "{:.1f}", 100.0),
    ("hue90", "hue90 deg", "{:.0f}", 1.0),
    ("occupied", "bins/36", "{:.0f}", 1.0),
    ("sat_mean", "sat mean", "{:.3f}", 1.0),
    ("sat_std", "sat std", "{:.3f}", 1.0),
    ("cool_pct", "cool %", "{:.1f}", 1.0),
    ("warm_pct", "warm %", "{:.1f}", 1.0),
]


def warm_cool_split(path):
    """warm / cool / neutral % over saturated px — the doc's own definition."""
    import numpy as np
    from PIL import Image
    from _kevin_art_gap_measure import hue_sat_val
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    h, s, _ = hue_sat_val(a)
    m = s >= 0.08
    n = int(m.sum())
    if n == 0:
        return 0.0, 0.0, 0.0
    warm = float((((h < 70) | (h >= 340)) & m).sum()) / n * 100
    cool = float((((h >= 170) & (h < 270)) & m).sum()) / n * 100
    return warm, cool, 100.0 - warm - cool


def art_measure(label, path):
    m = measure(path, label)
    w, c, nu = warm_cool_split(path)
    m["warm_pct"], m["cool_pct"], m["neutral_pct"] = w, c, nu
    m["path"] = path
    # measure() also returns the raw per-pixel arrays (h360 / sat / L) and the
    # kmeans mass vector — useful in-process, not serialisable, and not part of
    # any number quoted on a card. Dropped here rather than defaulted away, so a
    # future key that IS a number cannot be silently stringified into a report.
    m["mass"] = [float(x) for x in m.get("mass", [])]
    for k in ("h360", "sat", "L"):
        m.pop(k, None)
    return m


def control():
    os.makedirs(OUTDIR, exist_ok=True)
    ok = True
    print("=" * 78)
    print("CONTROL — does the instrument reproduce answers that are already published?")
    print("=" * 78)

    for label, spec in CONTROL_ART.items():
        p = spec["path"]
        if not os.path.isfile(p):
            print(f"REFUSED  control input missing: {p}")
            return False
        m = measure(p, label)
        print(f"\n[{label}]  {p}")
        for key, (want, tol) in spec["expect"].items():
            got = float(m[key])
            hit = abs(got - want) <= tol
            ok = ok and hit
            print(f"   {'ok ' if hit else 'DRIFT'}  {key:<10} got {got:9.4f}   published {want:<8}  tol +-{tol}")

    # --- control C: the look-acceptance gate against its own REF column -----
    if not os.path.isfile(GOLDEN_REF):
        print(f"REFUSED  golden ref missing: {GOLDEN_REF}")
        return False
    if not os.path.isfile(GOLDEN_COPY):
        shutil.copyfile(GOLDEN_REF, GOLDEN_COPY)
    m = grade_axes.measure(GOLDEN_COPY)
    m["sat"] = m["sat_honest"]  # the gate judges sat_honest; REF column is that value
    print(f"\n[golden-beauty-shot-ref]  {GOLDEN_REF}   (grade_axes REF column)")
    for key, _label, _cmp, _bound, ref in grade_axes.TARGETS:
        got = float(m[key])
        tol = AXES_TOL[key]
        hit = abs(got - ref) <= tol
        ok = ok and hit
        print(f"   {'ok ' if hit else 'DRIFT'}  {key:<10} got {got:9.4f}   REF column {ref:<8} tol +-{tol}")

    print()
    print("CONTROL: " + ("PASS — instrument reproduces every published answer, numbers below are trustworthy"
                         if ok else
                         "FAIL — the GRADER is wrong, not the image. Nothing graded."))
    return ok


def md_table(rows, ref_row=None):
    hdr = "| plate | " + " | ".join(lbl for _k, lbl, _f, _s in ART_COLS) + " |"
    sep = "|---" * (len(ART_COLS) + 1) + "|"
    out = [hdr, sep]
    for r in rows:
        cells = [fmt.format(float(r[k]) * scale) for k, _l, fmt, scale in ART_COLS]
        out.append("| " + r["name"] + " | " + " | ".join(cells) + " |")
    if ref_row:
        cells = [fmt.format(float(ref_row[k]) * scale) for k, _l, fmt, scale in ART_COLS]
        out.append("| **REF (target)** | " + " | ".join(cells) + " |")
    return "\n".join(out)


def run_measure(pairs, out_prefix):
    os.makedirs(OUTDIR, exist_ok=True)
    rows = []
    for label, path in pairs:
        if not os.path.isfile(path):
            print(f"SKIP (missing): {label} -> {path}")
            continue
        r = art_measure(label, path)
        rows.append(r)
        print(f"measured {label:<26} {r['w']}x{r['h']}  lum {r['lum_mean']:6.1f}  "
              f"hl {r['highlight']*100:5.1f}%  edge {r['edge_mean']:6.2f}  "
              f"hue90 {r['hue90']:3.0f}  cool {r['cool_pct']:4.1f}%")
    ref = art_measure("REF_mid", CONTROL_ART["REF_mid"]["path"])
    jpath = os.path.join(OUTDIR, out_prefix + ".json")
    mpath = os.path.join(OUTDIR, out_prefix + ".md")
    with open(jpath, "w", encoding="utf-8") as fh:
        json.dump({"rows": rows, "ref": ref}, fh, indent=1, default=float)
    with open(mpath, "w", encoding="utf-8") as fh:
        fh.write(md_table(rows, ref) + "\n")
    print("\n" + md_table(rows, ref))
    print(f"\nwrote {jpath}\nwrote {mpath}")
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["control", "measure", "baseline"])
    ap.add_argument("pairs", nargs="*", help="label=path")
    ap.add_argument("--out", default=None, help="output file prefix under _fl_v4_20260818/")
    a = ap.parse_args()

    if a.mode == "control":
        sys.exit(0 if control() else EXIT_REFUSED)

    if not control():
        print("\nrefusing to grade on an uncalibrated instrument.")
        sys.exit(EXIT_REFUSED)
    print()

    if a.mode == "baseline":
        run_measure(SHIPPED_PLATES, a.out or "baseline")
    else:
        pairs = []
        for s in a.pairs:
            label, _, path = s.partition("=")
            pairs.append((label, path))
        run_measure(pairs, a.out or "measured")


if __name__ == "__main__":
    main()
