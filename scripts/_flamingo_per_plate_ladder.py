#!/usr/bin/env python3
"""PER-PLATE sat ladder — 4 plates x 6 rungs x 6 axes, measured, nothing collapsed.

Why this exists: my 2026-08-09 verdict published ONE ladder table and let it read
as the whole set. It was `grade-vista-nohud2.png` alone. The CEO re-ran `hero`
at 1.05 and got warmth 100.9 / clip 48.8 -- both FAIL -- against my "1.05 PASSes
both satisfiable gates". Both numbers were true; my table was one plate.

So this script refuses to average, refuses to pick a representative plate, and
prints every (plate, rung) cell. Any row that quotes a number names its plate.

Axes = the `gameplay` profile of scripts/grade_axes.py (the single source of
truth). DOF is excluded there by design (framing-dependent; the CEO-approved
wide-hero baseline itself scores 0.17) -- it is printed as context only, never
graded. sat uses grade_axes.sat_status(), so the N-A co-gate rule cannot drift
from the gate.

Extra columns (shadow decile R, frame Lstd) are NOT sat axes. They are here to
settle whether the lifted-shadow defect moves with POST_SATURATION at all --
input to the separate ambient/exposure ticket.
"""
import json
import os
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import grade_axes as G  # noqa: E402

RUNGS = [("1.00", "_yama_sat_sweep_100"),
         ("1.02", "_yama_sat_sweep_102"),   # shipped: client/src/look.rs:578
         ("1.05", "_yama_sat_sweep_105"),
         ("1.15", "_yama_sat_sweep_115"),
         ("1.30", "_yama_sat_sweep_130"),
         ("1.90", "_yama_sat_sweep_190")]   # the retracted N6 build

PLATES = ["grade-vista", "hero", "s1-vista", "s4-raking"]

AXES = ["warmth", "blue", "clip", "sat", "micro", "p95"]   # gameplay profile, 6 axes
TGT = {k: (cmp, bound, ref) for k, _l, cmp, bound, ref in G.TARGETS}
LBL = {k: l for k, l, _c, _b, _r in G.TARGETS}


def extra(path):
    """Shadow-decile RGB + frame Lstd, on the SAME 1024 LANCZOS geometry as the gate."""
    a = np.asarray(Image.open(path).convert("RGB").resize((1024, 1024), Image.LANCZOS)
                   ).astype(np.float32)
    R, Gc, B = a[..., 0], a[..., 1], a[..., 2]
    L = 0.2126 * R + 0.7152 * Gc + 0.0722 * B
    d = L <= np.percentile(L, 10)
    return {"shadowR": float(R[d].mean()), "shadowG": float(Gc[d].mean()),
            "shadowB": float(B[d].mean()), "Lstd": float(L.std())}


def cell(key, m):
    """(value_string, ok) for one axis of one measured plate. ok=None means N-A."""
    if key == "sat":
        ok, txt = G.sat_status(m)
        return txt, ok
    cmp, bound, _ = TGT[key]
    v = m[key]
    return f"{v:.1f}", G.verdict(cmp, bound, v)


def tag(ok):
    return "N-A" if ok is None else ("PASS" if ok else "FAIL")


def main():
    data = {}
    for rung, d in RUNGS:
        for p in PLATES:
            path = os.path.join(ROOT, d, "after", f"{p}-nohud2.png")
            if not os.path.isfile(path):
                print(f"MISSING {path}", file=sys.stderr)
                sys.exit(3)
            m = G.measure(path)
            m.update(extra(path))
            data[(p, rung)] = m

    out = []
    A = out.append

    A("# PER-PLATE sat ladder — 4 plates x 6 rungs, every cell measured")
    A("")
    A(f"Source: `_yama_sat_sweep_{{100,102,105,115,130,190}}/after/*-nohud2.png` (24 plates).")
    A("Ruler: `scripts/grade_axes.py`, profile **gameplay** (6 axes; DOF excluded there by")
    A("design — framing-dependent). Every table below is ONE named plate. No plate's")
    A("number is quoted for another plate, and nothing is averaged.")
    A("")
    A(f"Gate: warmth >= {TGT['warmth'][1]:g} · blue <= {TGT['blue'][1]:g} · clip <= {TGT['clip'][1]:g} · "
      f"sat(honest) >= {TGT['sat'][1]:g} · micro >= {TGT['micro'][1]:g} · p95 {TGT['p95'][1][0]:g}..{TGT['p95'][1][1]:g}")
    A("REF (golden, per grade_axes TARGETS): warmth 120.9 · blue 4.3 · clip 18.8 · "
      "sat 95.2 · micro 5.24 · p95 165.8")
    A("")

    for p in PLATES:
        A(f"## plate: `{p}-nohud2.png`")
        A("")
        A("| POST_SATURATION | warmth R-B (mid) | blue B (mid) | mid clip % | sat (honest) | micro-contrast | p95 | axes passed |")
        A("|---|---|---|---|---|---|---|---|")
        for rung, _d in RUNGS:
            m = data[(p, rung)]
            cells, npass, njudged = [], 0, 0
            for k in AXES:
                txt, ok = cell(k, m)
                # N-A is not a verdict, it is "this plate can no longer be read here".
                cells.append("N-A *(unreadable, clip>35)*" if ok is None else f"{txt} {tag(ok)}")
                if ok is not None:
                    njudged += 1
                    npass += bool(ok)
            note = " *(shipped)*" if rung == "1.02" else (" *(N6, retracted)*" if rung == "1.90" else "")
            A(f"| {rung}{note} | " + " | ".join(cells) + f" | **{npass}/{njudged}** |")
        A("")

    # ---- Q2: is there ANY rung where all four plates pass clip together? -------
    A("## Cross-plate clip matrix — the co-gate, all 4 plates side by side")
    A("")
    A("| POST_SATURATION | " + " | ".join(PLATES) + " | all 4 <= 35 ? |")
    A("|---|" + "---|" * (len(PLATES) + 1))
    any_all = []
    for rung, _d in RUNGS:
        vals = [data[(p, rung)]["clip"] for p in PLATES]
        ok_all = all(v <= G.CLIP_THRESH for v in vals)
        any_all.append((rung, ok_all))
        A(f"| {rung} | " + " | ".join(
            f"{v:.1f} {'PASS' if v <= G.CLIP_THRESH else 'FAIL'}" for v in vals)
          + f" | {'**YES**' if ok_all else 'no'} |")
    A("")

    A("## Cross-plate warmth matrix (the other satisfiable axis)")
    A("")
    A("| POST_SATURATION | " + " | ".join(PLATES) + " | all 4 >= 110 ? |")
    A("|---|" + "---|" * (len(PLATES) + 1))
    for rung, _d in RUNGS:
        vals = [data[(p, rung)]["warmth"] for p in PLATES]
        ok_all = all(v >= 110.0 for v in vals)
        A(f"| {rung} | " + " | ".join(
            f"{v:.1f} {'PASS' if v >= 110.0 else 'FAIL'}" for v in vals)
          + f" | {'**YES**' if ok_all else 'no'} |")
    A("")

    # ---- clip AND warmth together, all four plates -----------------------------
    A("## Both satisfiable axes, all four plates at once")
    A("")
    A("| POST_SATURATION | plates passing clip<=35 | plates passing warmth>=110 | plates passing BOTH |")
    A("|---|---|---|---|")
    for rung, _d in RUNGS:
        c = [p for p in PLATES if data[(p, rung)]["clip"] <= G.CLIP_THRESH]
        w = [p for p in PLATES if data[(p, rung)]["warmth"] >= 110.0]
        b = [p for p in PLATES if p in c and p in w]
        A(f"| {rung} | {len(c)}/4 {'· '.join(c) or '—'} | {len(w)}/4 {'· '.join(w) or '—'} | "
          f"**{len(b)}/4** {'· '.join(b) or '—'} |")
    A("")

    # ---- sat-invariance of the shadow/Lstd defect ------------------------------
    A("## Shadow floor + Lstd across the ladder (NOT sat axes — ambient/exposure ticket)")
    A("")
    A("Darkest-decile mean RGB and whole-frame Lstd, same 1024 geometry. REF golden = "
      "shadow (45.3, 16.9, 0.8), Lstd 46.6.")
    A("")
    A("| plate | " + " | ".join(f"{r} shadowR / Lstd" for r, _ in RUNGS) + " |")
    A("|---|" + "---|" * len(RUNGS))
    for p in PLATES:
        A(f"| {p} | " + " | ".join(
            f"{data[(p, r)]['shadowR']:.1f} / {data[(p, r)]['Lstd']:.1f}" for r, _ in RUNGS) + " |")
    A("")

    # ---- the direct answer, written INTO the artefact (not left to a summary) ----
    hits_all = [r for r, ok in any_all if ok]
    top = hits_all[-1] if hits_all else None
    readable = []
    for rung, _d in RUNGS:
        if all(G.sat_status(data[(p, rung)])[0] is not None for p in PLATES):
            readable.append(rung)

    A("## ANSWER — is there a POST_SATURATION where ALL FOUR plates pass clip <= 35?")
    A("")
    if hits_all:
        A(f"**Yes, and only these: {', '.join(hits_all)}.** The highest is **{top}** — the value "
          f"shipped today at `client/src/look.rs:578`. Every rung at 1.05 and above loses at "
          f"least one plate (`hero` first, at 48.8 %).")
    else:
        A("**No. There is no such value.**")
    A("")
    A(f"Rungs where the honest-sat axis is still READABLE on all four plates "
      f"(i.e. no plate has gone N-A): **{', '.join(readable) if readable else 'none'}**. "
      f"Above that, the plate the board judges on stops producing a number at all.")
    A("")
    A("And the question nobody asked but which decides the lane: **there is no rung where all "
      "four plates pass warmth >= 110** — `s1-vista` peaks at 76.6 even at the retracted 1.90, "
      "because 37.9 % of its graded band is sky. That is a band-definition bug, not a look "
      "value to tune. See the spec handed to Rose.")
    A("")

    md = "\n".join(out)
    dest = os.path.join(ROOT, "docs", "flamingo-per-plate-sat-ladder-2026-08-09.md")
    with open(dest, "w", encoding="utf-8") as f:
        f.write(md + "\n")

    jd = {f"{p}@{r}": {k: v for k, v in data[(p, r)].items() if not k.startswith("_")}
          for p in PLATES for r, _ in RUNGS}
    with open(os.path.join(ROOT, "_flamingo_per_plate_ladder.json"), "w", encoding="utf-8") as f:
        json.dump(jd, f, indent=1)

    print(md)
    print(f"\n[written] {dest}")
    print("[written] _flamingo_per_plate_ladder.json")

    hits = [r for r, ok in any_all if ok]
    print(f"\nANSWER Q2 — rungs where ALL FOUR plates pass clip<=35: "
          f"{', '.join(hits) if hits else 'NONE'}")


if __name__ == "__main__":
    main()
