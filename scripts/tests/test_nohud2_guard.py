"""Regression harness for the de-HUD grading guard (scripts/nohud2_guard.py).

Run: python scripts/tests/test_nohud2_guard.py    (exit 0 = all green, 1 = failure)
No pytest in this repo's env — plain asserts, plain exit code, like test_quest_demo.py.

It guards three things that each already went wrong once:

  1. THE GUARD SHIPS.  The guard was first written as `scripts/_nohud2_guard.py`,
     and `.gitignore`'s `_[!_]*.py` scratch rule swallowed it while two TRACKED
     graders imported it at module top. On a fresh clone both would have died on
     ModuleNotFoundError before printing a number. So: assert the guard — and the
     de-HUD tool its error message tells you to run — are tracked by git.

  2. COVERAGE IS COMPLETE.  The guard was wired into 2 of 10 grading entry
     points while its own docstring claimed it "cannot be routed around". Every
     tracked grading entry point must now be classified GUARDED or EXEMPT, so a
     new grader cannot be added without deciding, in writing, which it is.

  3. IT STILL FAILS CLOSED.  Each guarded script, handed a frame that is not
     `*-nohud2.png`, must exit 2 and print NOTHING on stdout. Stdout silence is
     the real assertion: the whole point is that there is no number for a human
     to read out of context.
"""

import contextlib
import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPTS = REPO / "scripts"
sys.path.insert(0, str(SCRIPTS))

import nohud2_guard as G  # noqa: E402

failures = []


def check(cond, msg):
    if not cond:
        failures.append(msg)
    return cond


def tracked(rel):
    """Is `rel` carried by git? (`git ls-files` lists tracked paths only.)"""
    p = subprocess.run(["git", "ls-files", "--error-unmatch", rel],
                       cwd=REPO, capture_output=True, text=True)
    return p.returncode == 0


# ---------------------------------------------------------------------------
# 1. The guard chain ships with the repo
# ---------------------------------------------------------------------------
for rel in ("scripts/nohud2_guard.py", "scripts/_flamingo_dehud2.py"):
    check(tracked(rel),
          f"{rel} is NOT tracked by git — a tracked grader requires it, so a fresh "
          f"clone cannot grade anything. Check .gitignore's `_` scratch rules.")

# Every module a guarded grader imports from must ship too — this catches the
# next `_probe_helper.py` before it becomes the same bug.
check(not (SCRIPTS / "_nohud2_guard.py").exists(),
      "scripts/_nohud2_guard.py is back — that underscore name is the one .gitignore "
      "swallows. The guard lives at scripts/nohud2_guard.py.")

# ... and nobody may still be importing the old name. The rename to escape
# .gitignore left `_flamingo_halo_probe.py` importing `_nohud2_guard` — the exact
# bug the rename existed to fix, inverted, and invisible to a normal grep because
# the probe is gitignored. So this walks the DISK, not `git ls-files`.
for p in sorted(SCRIPTS.rglob("*.py")):
    src = p.read_text(encoding="utf-8", errors="replace")
    for line in src.splitlines():
        s = line.strip()
        if s.startswith(("import _nohud2_guard", "from _nohud2_guard")):
            failures.append(
                f"{p.relative_to(REPO)} imports the OLD guard name `_nohud2_guard` — that "
                f"module does not exist, so this file dies on ModuleNotFoundError. "
                f"Import `nohud2_guard`.")


# ---------------------------------------------------------------------------
# 2. Every tracked grading entry point is classified, and the classification is true
# ---------------------------------------------------------------------------
# Discovery is BEHAVIOURAL, not by filename — see nohud2_guard.is_entry_point().
# The name-glob version reported "13 entry points classified" while
# `dof_crop_compare.py` (hardcoded frame, prints a sharpness table and an fg:bg
# ratio) sat outside both lists, because its name does not start with `grade_`.
entry_points = set()
for p in sorted(SCRIPTS.glob("*.py")):
    if not tracked(f"scripts/{p.name}"):
        continue
    src = p.read_text(encoding="utf-8", errors="replace")
    if G.is_entry_point(p.name, src):
        entry_points.add(p.name)

check(bool(entry_points), "found no grading entry points at all — the detector is wrong")

# The detector must be doing more than re-stating the globs, or we are back to
# classifying by filename with extra steps.
import fnmatch  # noqa: E402

by_glob = {n for n in entry_points if any(fnmatch.fnmatch(n, g) for g in G.ENTRY_POINT_GLOBS)}
check(len(entry_points - by_glob) >= 3,
      f"behavioural discovery found only {len(entry_points - by_glob)} entry point(s) the "
      f"name-globs miss — either the detector regressed to matching names, or someone "
      f"deleted the scripts it was written to catch (dof_crop_compare, dof_decision_sheet, "
      f"make_gate3_verdict_card).")

# Regression pin: the three the glob version let through by name.
for name in ("dof_crop_compare.py", "dof_decision_sheet.py", "make_gate3_verdict_card.py"):
    if (SCRIPTS / name).exists():
        check(name in entry_points,
              f"{name} is no longer detected as a grading entry point — it measures frame "
              f"pixels and prints the numbers; it must stay classified.")

classified = set(G.GUARDED) | set(G.EXEMPT)
unclassified = entry_points - classified
check(not unclassified,
      f"grading entry point(s) in neither GUARDED nor EXEMPT: {sorted(unclassified)}. "
      f"Decide and record it in scripts/nohud2_guard.py — an unclassified grader is an "
      f"open door, which is how the first version of this guard covered 2 of 10 scripts.")

stale = classified - entry_points
check(not stale, f"classified but no longer a tracked entry point: {sorted(stale)}")

both = set(G.GUARDED) & set(G.EXEMPT)
check(not both, f"listed as GUARDED *and* EXEMPT: {sorted(both)}")

for name, reason in G.EXEMPT.items():
    check(len((reason or "").strip()) > 40,
          f"EXEMPT[{name}] has no real reason written — an exemption without a stated "
          f"reason is how a gate gets quietly loosened.")

# The registry must describe reality, not intent: read the source.
for name in G.GUARDED:
    src = (SCRIPTS / name).read_text(encoding="utf-8", errors="replace")
    check("require_nohud2(" in src,
          f"{name} is listed GUARDED but never calls require_nohud2()")
for name in G.EXEMPT:
    src = (SCRIPTS / name).read_text(encoding="utf-8", errors="replace")
    check("require_nohud2(" not in src,
          f"{name} is listed EXEMPT but calls require_nohud2() — fix the list or the file")

# The docstring promises no bypass. Keep it true.
guard_src = (SCRIPTS / "nohud2_guard.py").read_text(encoding="utf-8")
body = guard_src.split('"""', 2)[-1]           # skip the module docstring
check("getenv" not in body and "environ" not in body,
      "nohud2_guard.py reads the environment — an env override turns the guard into a "
      "comment. Remove it.")


# ---------------------------------------------------------------------------
# 3. Behaviour: accepts a real de-HUDded frame, refuses everything else
# ---------------------------------------------------------------------------
with tempfile.TemporaryDirectory() as td:
    good = Path(td) / "probe-nohud2.png"
    good.write_bytes(b"")                       # the guard checks the NAME + existence
    missing = Path(td) / "gone-nohud2.png"
    raw = Path(td) / "probe.png"
    raw.write_bytes(b"")

    try:
        G.require_nohud2([str(good)], tool="selftest")
    except SystemExit:
        failures.append("guard refused a valid *-nohud2.png that exists on disk")

    for bad, why in ((raw, "wrong suffix"), (missing, "missing on disk"), (None, "no args")):
        try:
            # The refusal text is asserted through the subprocess runs below; in
            # here it would just bury this harness's own report.
            with open(os.devnull, "w") as devnull, contextlib.redirect_stderr(devnull):
                G.require_nohud2([] if bad is None else [str(bad)], tool="selftest")
            failures.append(f"guard ACCEPTED a bad input ({why})")
        except SystemExit as e:
            check(e.code == G.EXIT_REFUSED,
                  f"guard exited {e.code} on {why}, expected {G.EXIT_REFUSED}")

    # Each guarded script, end to end, on a raw frame name. argv differs per tool;
    # the assertion does not: exit 2, and not one character on stdout.
    argv_for = {
        "grade_g7.py": ["--frame", str(raw)],
        "grade_web_parity.py": ["--web", str(raw), "--native", str(raw)],
        "dof_decision_sheet.py": [str(raw), str(raw)],
        # No frame on argv by default — it reads a fixed gate3 set. `--frames`
        # exists so the refusal is testable end to end instead of asserted by
        # reading the source, which is how a guard rots.
        "make_gate3_verdict_card.py": ["--frames", str(raw), str(raw), str(raw)],
    }
    for name in G.GUARDED:
        cmd = [sys.executable, str(SCRIPTS / name)] + argv_for.get(name, [str(raw)])
        p = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True, timeout=120)
        check(p.returncode == G.EXIT_REFUSED,
              f"{name} exited {p.returncode} on a non-de-HUDded frame, expected "
              f"{G.EXIT_REFUSED}\n    stdout: {p.stdout[:200]!r}\n    stderr: {p.stderr[:200]!r}")
        check(p.stdout.strip() == "",
              f"{name} printed to stdout while refusing — a refusal must leave NO number "
              f"behind to be misread:\n    {p.stdout[:300]!r}")
        check("REFUSED" in p.stderr,
              f"{name} refused without saying REFUSED (stderr: {p.stderr[:200]!r})")

    # render_grade.sh is the producer side of the same rule: it renders with the
    # HUD-free shot exe, so it must demand a -nohud2 output name instead of
    # handing the graders a frame they will refuse three lines later.
    bash = os.environ.get("BASH", "bash")
    try:
        p = subprocess.run([bash, "scripts/render_grade.sh", "out.png"],
                           cwd=REPO, capture_output=True, text=True, timeout=60)
        check(p.returncode == 2 and "REFUSED" in (p.stdout + p.stderr),
              f"render_grade.sh accepted a non-nohud2 output name "
              f"(rc={p.returncode}): {(p.stdout + p.stderr)[:200]!r}")
    except (FileNotFoundError, subprocess.TimeoutExpired):
        print("SKIP  render_grade.sh check (no bash on PATH)")


if failures:
    print(f"\nFAILED ({len(failures)}):")
    for f in failures:
        print(f"  - {f}")
    sys.exit(1)

print(f"OK  guard chain tracked · {len(entry_points)} entry points classified "
      f"({len(G.GUARDED)} guarded / {len(G.EXEMPT)} exempt) · all refusals fail closed")
