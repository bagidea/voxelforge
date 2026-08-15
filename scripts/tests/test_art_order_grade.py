"""Regression harness for the art-order grader (scripts/art_order_grade.py).

Run: python scripts/tests/test_art_order_grade.py   (exit 0 = all green, 1 = failure)
No pytest in this repo's env — plain asserts, plain exit code, like test_nohud2_guard.py.

It guards the four things that make the grader evidence instead of decoration:

  1. THE CONTROL SUITE PASSES.  Every gate is fed inputs whose answer is known
     (a ramp of exactly 30 L, a flat plate, synthetic shafts, an isotropic glow,
     an exposure bump) plus the approved reference frames. When this suite was
     first written it caught four real defects in one run, including two synthetic
     controls that were lying about their OWN amplitude. If it ever goes red the
     grader's verdicts stop meaning anything, so this is assertion #1.

  2. THE CUT LINES IN THE CODE MATCH THE CUT LINES IN THE ORDER.  A threshold
     that drifts on one side without the other is how a gate quietly stops
     grading what the order asked for. Both A1 numbers the Director named
     explicitly (sky gradient >= 25 L) are pinned here.

  3. UNMEASURABLE NEVER READS AS PASS.  A frame with no A/B pair, no sky and no
     hero box must come back SKIP -> exit 4 (INCOMPLETE), never 0.

  4. THE GRADER IS TRACKED.  `.gitignore`'s `_[!_]*.py` scratch rule has already
     eaten two lane tools in this repo. The order document tells four people to
     run this file; on a fresh clone it has to exist.
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import art_order_grade as G  # noqa: E402

FAILURES = []


def check(label, cond, detail=""):
    print(f"  {'ok  ' if cond else 'FAIL'}  {label}" + (f"   {detail}" if detail else ""))
    if not cond:
        FAILURES.append(label)


print("1. control suite")
c = G.calibrate(verbose=False)
bad = [f"{r[0]}/{r[1]}" for r in c.rows if r[5] != "ok"]
check("every control lands on its own side of the cut line", c.ok,
      f"{len(c.rows)} controls" + (f" · broken: {bad}" if bad else ""))
check("the suite is not empty (an empty suite passes vacuously)", len(c.rows) >= 25,
      f"{len(c.rows)} controls")

print("2. cut lines match the order document")
order = (ROOT / "docs" / "art-order-2026-08-09-composition.md").read_text(encoding="utf-8")
check("A1 sky gradient is 25 L in code", G.T["sky_grad_L"] == 25.0)
check("A1 sky gradient is 25 L in the order", "≥ 25 L" in order or ">= 25 L" in order)
check("A2 needs a same-camera A/B pair, not one frame", "--before" in order)
check("A6 silhouette bar is 40 in code and in the order",
      G.T["silhouette"] == 40.0 and "≥ 40" in order)
check("A8 island share is 40% in code and in the order",
      G.T["island_share_pct"] == 40.0 and "≥ 40%" in order)
check("A4 aerial ratio is 0.80 in code and in the order",
      G.T["aerial_ratio"] == 0.80 and "< 0.80" in order)
check("A4 frame class is documented in the order", "--frame-class" in order)

print("3. unmeasurable is not a pass")
boot = ROOT / "docs/assets/gate3/gate3-after-boot-nohud2.png"
ref = ROOT / "docs/assets/golden-beauty-shot-ref.png"
v = G.Verdict()
G.grade(str(ref), None, str(ref), None, None, "environment", v)
states = {r[1]: r[2] for r in v.rows}
check("no A/B pair -> A2 SKIP", states.get("god ray A/B") == G.SKIP)
check("no sky -> A1 SKIP", states.get("sky gradient") == G.SKIP)
check("no hero box -> A6 SKIP", states.get("silhouette complexity") == G.SKIP)
check("a run holding SKIPs exits 4 (INCOMPLETE), never 0", v.exit_code() == 4,
      f"exit {v.exit_code()}")
v2 = G.Verdict()
G.grade(str(boot), None, None, None, None, "environment", v2)
check("a run holding a FAIL exits 1", v2.exit_code() == 1, f"exit {v2.exit_code()}")

print("3b. A4 frame class — a portrait never leaks into the environment gate")
pa = G.aerial(G.from_image(G.synth_portrait()))
check("a sharp portrait falsely FAILs the environment reading (why 0.80 stays)",
      pa is not None and pa["contrast"] > G.T["aerial_ratio"],
      f"far/near {pa['contrast']:.2f}" if pa else "unmeasurable")
check("portrait class -> A4 SKIP", G.a4_verdict(pa, "portrait") == G.SKIP)
check("environment class -> A4 FAIL (gate not weakened)",
      G.a4_verdict(pa, "environment") == G.FAIL)

print("4. the grader ships")
tracked = subprocess.run(["git", "ls-files", "--error-unmatch", "scripts/art_order_grade.py"],
                         cwd=ROOT, capture_output=True, text=True)
check("scripts/art_order_grade.py is tracked by git", tracked.returncode == 0,
      tracked.stderr.strip())

print()
if FAILURES:
    print(f"FAILED: {len(FAILURES)} check(s) — {FAILURES}")
    raise SystemExit(1)
print("all green")
