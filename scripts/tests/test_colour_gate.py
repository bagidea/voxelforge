#!/usr/bin/env python3
"""Self-test for scripts/colour_gate.py — the gate that must fail the magenta bug.

Guard against the most dangerous class of gate: one that never fails. The
known-magenta canary (fixtures/gate3-after-boot-MAGENTA.png, pulled from git at
db25758 — the shoot BEFORE the look.rs TEMPERATURE 0.10->0.02 fix) is 39.99%
magenta with sky gain R x1.98 G x0.54 B x0.55. If this gate ever PASSes it, the
gate is broken. The current fixed shoot (docs/assets/gate3/*.png) must PASS.

What this proves, in order:
  1. the two signature detectors behave on synthetic colours (amber/grass PASS,
     magenta FAIL) — independent of any PNG;
  2. the canary frame FAILS the overall gate, and specifically via the new
     sky-gain gate (not merely lucky that Gate A also trips);
  3. every frame in the approved set PASSES.
  4. the prefix verdict card (pre-fix, 6.99% magenta) MUST fail; the current
     regenerated card (0.22%) MUST pass — the negative test canary that the
     CEO can reproduce.

Run:    python scripts/tests/test_colour_gate.py
Exit 0 only if every assertion holds. No build, no network.
"""
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))   # .../scripts/tests
SCRIPTS = os.path.dirname(HERE)                      # .../scripts
ROOT = os.path.dirname(SCRIPTS)                      # .../Voxelforge
sys.path.insert(0, SCRIPTS)

import colour_gate as cg  # noqa: E402

FIXTURE = os.path.join(HERE, "fixtures", "gate3-after-boot-MAGENTA.png")
GATE3 = os.path.join(ROOT, "docs", "assets", "gate3")
APPROVED = ("gate3-after-boot.png", "gate3-after-walk.png", "gate3-after-combat.png")

failures = []


def expect(cond, msg):
    tag = "PASS" if cond else "FAIL"
    print(f"  [{tag}] {msg}")
    if not cond:
        failures.append(msg)


# --------------------------------------------------------------------------
print("1) signature detectors on synthetic colours (no PNG needed)")
expect(cg.sky_gain_is_magenta([1.98, 0.54, 0.55]),
       "magenta sky gain 1.98/0.54/0.55 (R lifted, G<=B) -> FAIL")
expect(not cg.sky_gain_is_magenta([0.81, 0.66, 0.65]),
       "approved sky gain 0.81/0.66/0.65 (no red lift) -> PASS")
expect(not cg.sky_gain_is_magenta([1.00, 0.75, 0.39]),
       "amber target gain 1.00/0.75/0.39 (R=1, but G>>B) -> PASS")
expect(cg.sunlit_is_magenta([211.3, 154.4, 154.4]),
       "magenta sunlit 211/154/154 (R-dom, G=B) -> FAIL")
expect(not cg.sunlit_is_magenta([244.0, 184.0, 96.0]),
       "amber sunlit 244/184/96 (R-dom, G>>B) -> PASS")
expect(not cg.sunlit_is_magenta([121.3, 199.6, 112.8]),
       "green grass 121/199/112 (G is max) -> PASS")
expect(not cg.sunlit_is_magenta([170.9, 170.3, 169.5]),
       "mid-gray 171/170/170 (R-G=0.6 < 5.0 margin, not meaningfully red) -> PASS")
expect(not cg.sunlit_is_magenta(None),
       "no lit surface -> PASS (skip, not fail)")

# --------------------------------------------------------------------------
print("\n2) canary: the known-magenta fixture MUST fail the gate")
expect(os.path.exists(FIXTURE),
       f"fixture present: {os.path.relpath(FIXTURE, ROOT)}")
if os.path.exists(FIXTURE):
    m = cg.measure(FIXTURE)
    expect(not cg.check(FIXTURE),
           "fixture overall verdict -> FAIL (the canary — if this PASSes, the gate is broken)")
    expect(m["sky"] is not None and cg.sky_gain_is_magenta(m["sky_gain"]),
           "fixture caught by the NEW sky-gain gate (R lifted + G<=B), not just Gate A")
    expect(m["mag_frac"] > cg.MAGENTA_MAX_FRACTION,
           f"fixture is genuinely magenta: {m['mag_frac'] * 100:.2f}% > {cg.MAGENTA_MAX_FRACTION * 100:g}%")

# --------------------------------------------------------------------------
print("\n3) approved set: the fixed shoot MUST pass")
present = [n for n in APPROVED if os.path.exists(os.path.join(GATE3, n))]
expect(len(present) == len(APPROVED),
       f"all {len(APPROVED)} gate3 frames present (found {len(present)} in {os.path.relpath(GATE3, ROOT)})")
for name in APPROVED:
    p = os.path.join(GATE3, name)
    if os.path.exists(p):
        expect(cg.check(p), f"{name} -> PASS")

# --------------------------------------------------------------------------
print("\n4) verdict cards: prefix (pre-fix, 6.99% magenta) MUST fail; current MUST pass")
PREFIX_CARD = os.path.join(GATE3, "gate3-colour-verdict-prefix.png")
VERDICT_CARD = os.path.join(GATE3, "gate3-colour-verdict.png")
if os.path.exists(PREFIX_CARD):
    expect(not cg.check(PREFIX_CARD),
           "prefix verdict card -> FAIL (6.99% magenta, proof the bug existed)")
if os.path.exists(VERDICT_CARD):
    expect(cg.check(VERDICT_CARD),
           "current verdict card -> PASS (regenerated at 16:25, 0.22% magenta)")

# --------------------------------------------------------------------------
print()
if failures:
    print(f"RESULT: FAIL — {len(failures)} assertion(s) broken")
    sys.exit(1)
print("RESULT: PASS — gate fails the magenta canary and passes the approved set")
sys.exit(0)
