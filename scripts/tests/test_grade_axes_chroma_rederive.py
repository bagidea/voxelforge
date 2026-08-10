#!/usr/bin/env python3
"""Lock the 2026-08-09 chromatic re-derive + terrain-band fix (Rose).

Companion to test_grade_axes_clip_gate.py. That test locks the clip co-gate;
THIS test locks what the re-derive concluded about warmth/blue/sat and the sky
mask, so neither can silently revert:

  A) ADVISORY set is exactly {warmth, blue, sat}. Re-adding any of them as a hard
     gate turns this red -- and it must, because the next two assertions show why.
  B) THE INVERSION IS REAL. A B-clamped wreck scores BETTER on the old warmth/blue
     gates than a healthy outdoor frame. Concretely: N6 grade-vista (@1.90, clip
     99.8%) would PASS the retired warmth>=110 AND blue<=10 (it reads warmth ~153 /
     blue ~0.04), while a healthy s1-vista @1.02 would FAIL warmth>=110 (reads ~85).
     That is a gate that rewards damage -- exactly why it was retired. Pinning the
     numbers means nobody can hand-wave "just lower the blue target" without
     confronting that warmth is inverted too.
  C) clip IS THE SOLE SEPARATOR. Every shipped-good @1.02 outdoor plate clips
     <=22%; every N6 (@1.90) plate clips >=55%. REF clips ~18.8%. No B-dependent
     axis separates good from damaged -- only clip does.
  D) TERRAIN BAND (sky dropped) IS A NO-OP ON REF. REF is indoor, 0% sky, so its
     terrain band == the old whole-frame band bit-for-bit. s1-vista @1.02 (37.9%
     sky inside the old band) now reads a sane terrain clip and PASSES instead of
     false-failing on sky-driven blue ~91.

REF is a tracked asset (always present). The shotset/sweep frames are local
capture, so absent legs SKIP with a reason instead of false-failing a clean
clone -- but on the machine that has them, they assert hard.

Run:   python scripts/tests/test_grade_axes_chroma_rederive.py
Exit 0 only if every present assertion holds. No pytest, no build, no network.
"""
import os
import shutil
import sys
import tempfile

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))      # .../scripts/tests
SCRIPTS = os.path.dirname(HERE)                         # .../scripts
ROOT = os.path.dirname(SCRIPTS)                         # .../Voxelforge
sys.path.insert(0, SCRIPTS)

import grade_axes  # noqa: E402

REF = os.path.join(ROOT, "docs", "assets", "golden-beauty-shot-ref.png")
SWEEP102 = os.path.join(ROOT, "_yama_sat_sweep_102", "after")
N6 = os.path.join(ROOT, "_pixel_shotset_N6", "after")
FAILS = []


def check(cond, msg):
    print(("  [PASS] " if cond else "  [FAIL] ") + msg)
    if not cond:
        FAILS.append(msg)


def measure_legacy_band(path):
    """The OLD whole-frame midtone band L[p35,p75] (no sky drop). Used only to
    prove REF-invariance: on a 0%-sky frame it must equal grade_axes.measure()."""
    im = Image.open(path).convert("RGB").resize((1024, 1024), Image.LANCZOS)
    a = np.asarray(im).astype(np.float32)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    L = 0.2126 * R + 0.7152 * G + 0.0722 * B
    tlo, thi = np.percentile(L, 35), np.percentile(L, 75)
    mm = (L >= tlo) & (L <= thi)
    railed = (R[mm] == 0) | (G[mm] == 0) | (B[mm] == 0) | (R[mm] == 255) | \
             (G[mm] == 255) | (B[mm] == 255)
    clip = float(railed.sum()) / max(int(mm.sum()), 1) * 100.0
    return {"warmth": float((R[mm] - B[mm]).mean()),
            "blue": float(B[mm].mean()), "clip": clip, "n": int(mm.sum())}


def measure_ref():
    """The nohud2 guard refuses the golden REF filename; grade a -nohud2 copy."""
    d = tempfile.mkdtemp(prefix="rederive_")
    p = os.path.join(d, "golden-beauty-shot-ref-nohud2.png")
    shutil.copy(REF, p)
    try:
        return grade_axes.measure(p)
    finally:
        shutil.rmtree(d)


def have(path):
    ok = os.path.isfile(path)
    if not ok:
        print(f"  [SKIP] {path} not present (local capture) -- leg skipped")
    return ok


def main():
    print("chromatic re-derive + terrain-band regression lock")
    print("=" * 62)

    # A) ADVISORY set locked
    print("\nA) ADVISORY == {warmth, blue, sat}  (re-gating any turns this red)")
    check(grade_axes.ADVISORY == {"warmth", "blue", "sat"},
          f"ADVISORY == {{warmth,blue,sat}}  (got {sorted(grade_axes.ADVISORY)})")
    gate_hero = [k for k in grade_axes.PROFILES["hero"] if k not in grade_axes.ADVISORY]
    check(set(gate_hero) == {"clip", "dof", "micro", "p95"},
          f"hero HARD GATES == {{clip,dof,micro,p95}}  (got {sorted(set(gate_hero))})")
    gate_gp = [k for k in grade_axes.PROFILES["gameplay"] if k not in grade_axes.ADVISORY]
    check(set(gate_gp) == {"clip", "micro", "p95"},
          f"gameplay HARD GATES == {{clip,micro,p95}}  (got {sorted(set(gate_gp))})")

    # B) The inversion is real -- pin the numbers that prove warmth/blue are retired
    #    BECAUSE they reward damage, not because the targets were merely "too tight".
    print("\nB) inversion pin: a B-clamped wreck PASSES the retired warmth/blue gates")
    if have(os.path.join(N6, "grade-vista-nohud2.png")):
        n6 = grade_axes.measure(os.path.join(N6, "grade-vista-nohud2.png"))
        check(n6["clip"] > grade_axes.CLIP_THRESH,
              f"N6 grade-vista clip {n6['clip']:.1f}% > 35 (it IS damaged)")
        # The retired gates would have CALLED THIS PASS:
        check(n6["warmth"] >= 110.0,
              f"N6 grade-vista warmth {n6['warmth']:.1f} >= 110  (retired warmth gate "
              f"would PASS a wrecked frame -- the inversion)")
        check(n6["blue"] <= 10.0,
              f"N6 grade-vista blue {n6['blue']:.2f} <= 10  (retired blue gate would "
              f"PASS a wrecked frame -- the inversion)")
    if have(os.path.join(SWEEP102, "s1-vista-nohud2.png")):
        good = grade_axes.measure(os.path.join(SWEEP102, "s1-vista-nohud2.png"))
        check(good["clip"] <= grade_axes.CLIP_THRESH,
              f"s1-vista @1.02 clip {good['clip']:.1f}% <= 35 (it is NOT damaged)")
        check(good["warmth"] < 110.0,
              f"s1-vista @1.02 warmth {good['warmth']:.1f} < 110  (retired warmth gate "
              f"would FAIL a healthy frame) -- together with B above, the gate is inverted")

    # C) clip is the sole separator
    print("\nC) clip separates shipped-good @1.02 from N6 @1.90  (REF passes too)")
    if os.path.isfile(REF):
        r = measure_ref()
        check(r["clip"] <= grade_axes.CLIP_THRESH,
              f"REF clip {r['clip']:.1f}% <= 35  (positive control passes the real gate)")
    good_plates = [os.path.join(SWEEP102, f"{p}-nohud2.png")
                   for p in ("grade-vista", "hero", "s1-vista", "s4-raking")]
    good_hits = 0
    for p in good_plates:
        if not have(p):
            continue
        good_hits += 1
        m = grade_axes.measure(p)
        check(m["clip"] <= grade_axes.CLIP_THRESH,
              f"{os.path.basename(p)} @1.02 clip {m['clip']:.1f}% <= 35 (good frame passes)")
    n6_names = ("gate3-boot", "gate3-combat", "gate3-walk", "grade-vista",
                "hero", "s1-vista", "s3-clash", "s4-raking")
    bad_hits = 0
    for nm in n6_names:
        p = os.path.join(N6, f"{nm}-nohud2.png")
        if not have(p):
            continue
        bad_hits += 1
        m = grade_axes.measure(p)
        check(m["clip"] > grade_axes.CLIP_THRESH,
              f"N6 {nm} @1.90 clip {m['clip']:.1f}% > 35 (damaged frame fails)")
    if good_hits and bad_hits:
        print(f"  (separator proven on {good_hits} good + {bad_hits} N6 plates)")

    # D) terrain band is a no-op on REF (0% sky) and repairs the sky-bearing plate
    print("\nD) terrain band: REF bit-identical (0% sky); s1-vista no longer false-fails")
    if os.path.isfile(REF):
        cur = measure_ref()
        leg = measure_legacy_band(REF)
        # REF has no sky -> terrain band MUST equal the old whole-frame band
        for k in ("warmth", "blue", "clip"):
            d = abs(cur[k] - leg[k])
            check(d < 1e-3,
                  f"REF {k}: terrain {cur[k]:.4f} == legacy-band {leg[k]:.4f}  "
                  f"(sky-mask is a no-op on a 0%-sky frame, |d|={d:.2e})")
    if have(os.path.join(SWEEP102, "s1-vista-nohud2.png")):
        m = grade_axes.measure(os.path.join(SWEEP102, "s1-vista-nohud2.png"))
        # Pre-fix this plate read blue ~91 (sky inside the band) and false-FAILED on
        # the retired blue<=10. It must now PASS the real gate on clip alone.
        check(m["clip"] <= grade_axes.CLIP_THRESH,
              f"s1-vista @1.02 PASSES on clip {m['clip']:.1f}%  (was false-failing on "
              f"sky-driven blue ~{m['blue']:.0f} under the retired <=10 gate)")

    print("\n" + "=" * 62)
    if FAILS:
        print(f"RESULT: FAIL -- {len(FAILS)} assertion(s) broken:")
        for f in FAILS:
            print("   - " + f)
        sys.exit(1)
    print("RESULT: PASS -- chroma re-derive locked: advisory set, inversion pinned, "
          "clip sole separator, terrain band REF-invariant")


if __name__ == "__main__":
    main()
