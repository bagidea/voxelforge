#!/usr/bin/env python3
"""Lock the clip-fraction co-gate + honest-saturation controls (Rose 2026-08-09).

Companion regression to commit 3511582. The clip co-gate exists to kill the most
dangerous class of gate -- one whose PASS comes from the DAMAGE it should detect:
legacy m["sat"] = (max-min)/max returns 1.0 for any pixel with a channel at 0, so
a midtone gamut-clipped to B=0 read "100% saturated = PASS" on every wrecked N6
plate (clip 54-99.95%). This test freezes BOTH controls so nobody can relax the
threshold or re-grade on the legacy value without editing this file in plain sight:

  1. CLIP_THRESH is exactly 35.0 (bumping it relaxes the gate -> red test).
  2. golden REF -> ALL hero axes PASS (clip ~18.8%, honest sat ~95.2).
  3. N6 hero + gate3-boot -> clip FAIL (>35) and sat reports N-A, not a number.
  4. grade_axes.py as a black box: REF exits 0 ALL AXES PASS; N6 exits 1 with
     '[FAIL] mid clip %' and '[N-A]' saturation.

The REF is a tracked asset (always present in a clone). The N6 shotset is local
capture, so its leg SKIPS with a clear reason when absent instead of false-failing
a clean clone -- but on the machine that has the shotset, it asserts hard.

Run:   python scripts/tests/test_grade_axes_clip_gate.py
Exit 0 only if every present assertion holds. No pytest, no build, no network.
"""
import os
import sys
import shutil
import subprocess
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))   # .../scripts/tests
SCRIPTS = os.path.dirname(HERE)                      # .../scripts
ROOT = os.path.dirname(SCRIPTS)                      # .../Voxelforge
sys.path.insert(0, SCRIPTS)

import grade_axes  # noqa: E402

REF = os.path.join(ROOT, "docs", "assets", "golden-beauty-shot-ref.png")
N6_HERO = os.path.join(ROOT, "_pixel_shotset_N6", "after", "hero-nohud2.png")
N6_BOOT = os.path.join(ROOT, "_pixel_shotset_N6", "after", "gate3-boot-nohud2.png")
PY = sys.executable

FAILS = []


def check(cond, msg):
    print(("  [PASS] " if cond else "  [FAIL] ") + msg)
    if not cond:
        FAILS.append(msg)


def grade_all_axes(m, profile):
    """Mirror grade_axes.main()'s verdict loop: only HARD-GATE axes judge. The
    ADVISORY set (warmth/blue/sat) is printed for tuning but never fails -- clip
    is the sole chromatic-damage gate (RE-DERIVE 2026-08-09, see grade_axes.py)."""
    active = grade_axes.PROFILES[profile]
    results = {}
    for key, _, cmp, bound, _ in grade_axes.TARGETS:
        if key not in active or key in grade_axes.ADVISORY:
            continue
        results[key] = grade_axes.verdict(cmp, bound, m[key])
    return results


def measure_ref():
    """The nohud2 guard refuses the golden REF's filename; grade a -nohud2 copy
    in a temp dir, exactly as the control is measured for real."""
    d = tempfile.mkdtemp(prefix="clipgate_")
    p = os.path.join(d, "golden-beauty-shot-ref-nohud2.png")
    shutil.copy(REF, p)
    try:
        return grade_axes.measure(p), p
    finally:
        shutil.rmtree(d)


def main():
    print("clip-fraction co-gate + honest-saturation regression lock")
    print("=" * 62)

    # 1. threshold locked -- this is the whole point: a silent relax turns red here
    print("\n1) CLIP_THRESH locked at 35.0 (relaxing it turns this test red)")
    check(grade_axes.CLIP_THRESH == 35.0,
          f"CLIP_THRESH == 35.0  (got {grade_axes.CLIP_THRESH!r})")

    # 2. golden REF -> ALL hero axes PASS (positive control)
    print("\n2) golden REF -> ALL hero axes PASS  (positive control)")
    check(os.path.isfile(REF), f"golden REF present: {REF}")
    if os.path.isfile(REF):
        r, _ = measure_ref()
        check(r["clip"] < grade_axes.CLIP_THRESH,
              f"REF clip {r['clip']:.2f}% below {grade_axes.CLIP_THRESH:g} "
              f"(so sat is judged, not N-A)")
        check(r["sat_honest"] >= 90.0,
              f"REF honest sat {r['sat_honest']:.2f} >= 90  (judged value)")
        res = grade_all_axes(r, "hero")
        for k, v in res.items():
            check(v, f"REF hero axis '{k}' PASS")
        check(all(res.values()),
              f"REF -> ALL AXES PASS  ({sum(res.values())}/{len(res)} axes)")

    # 3. N6 wrecked plates -> clip FAIL + sat N-A (negative control)
    print("\n3) N6 wrecked plates -> clip FAIL (>35) and sat N-A  (negative control)")
    for tag, path in (("hero", N6_HERO), ("gate3-boot", N6_BOOT)):
        if not os.path.isfile(path):
            print(f"  [SKIP] {tag}: {path} not present (local shotset) -- N6 leg skipped")
            continue
        m = grade_axes.measure(path)
        check(m["clip"] > grade_axes.CLIP_THRESH,
              f"N6 {tag}: clip {m['clip']:.2f}% > {grade_axes.CLIP_THRESH:g}  (FAIL)")
        ok, txt = grade_axes.sat_status(m)
        check(ok is None and txt == "N-A",
              f"N6 {tag}: sat reports N-A under the clip co-gate, not a fake number "
              f"(got ok={ok!r}, txt={txt!r})")

    # 4. integration: grade_axes.py as a black box (the real pipeline a human runs)
    print("\n4) grade_axes.py black-box: REF exit0 ALL PASS | N6 exit1 FAIL + N-A")
    grade_axes_py = os.path.join(SCRIPTS, "grade_axes.py")
    if os.path.isfile(REF):
        d = tempfile.mkdtemp(prefix="clipgate_cli_")
        p = os.path.join(d, "golden-beauty-shot-ref-nohud2.png")
        shutil.copy(REF, p)
        out = subprocess.run([PY, grade_axes_py, p], capture_output=True, text=True)
        check(out.returncode == 0, f"grade_axes REF exit 0  (got {out.returncode})")
        check("ALL AXES PASS" in out.stdout, "grade_axes REF stdout contains 'ALL AXES PASS'")
        check("[PASS] mid clip %" in out.stdout, "grade_axes REF stdout contains '[PASS] mid clip %'")
        shutil.rmtree(d)
    if os.path.isfile(N6_HERO):
        out = subprocess.run([PY, grade_axes_py, N6_HERO], capture_output=True, text=True)
        check(out.returncode == 1, f"grade_axes N6 hero exit 1  (got {out.returncode})")
        check("[FAIL] mid clip %" in out.stdout, "grade_axes N6 stdout contains '[FAIL] mid clip %'")
        check("[N-A]" in out.stdout and "saturation" in out.stdout,
              "grade_axes N6 stdout contains a '[N-A]' saturation line")

    print("\n" + "=" * 62)
    if FAILS:
        print(f"RESULT: FAIL — {len(FAILS)} assertion(s) broken:")
        for f in FAILS:
            print("   - " + f)
        sys.exit(1)
    print("RESULT: PASS — clip co-gate + honest sat locked; REF passes, N6 fails + N-A")


if __name__ == "__main__":
    main()
