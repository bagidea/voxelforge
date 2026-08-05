#!/usr/bin/env python3
"""HARD GUARD - every grader in this repo may only speak about a de-HUDded frame.

Why this exists as code and not as a sentence in a doc: the rule ("grade only
`*-nohud2.png`") was already written down in docs and in
`scripts/_poppy_grade_nohud2.sh`, and it was still broken - a raw `--play`
capture from a stale exe got graded by eye and the numbers were reported as if
they meant something. The `[E]` prompt glyphs sit at ~250 on the ground, so a
raw frame reads "window blown out" on G5 and drags the p95 axis up to the UI.

The bash wrapper only guards the ONE path that goes through it. Anyone calling
`python scripts/grade_gate.py <frame>` directly walked straight past it. So the
refusal now lives inside the Python entry points themselves, where it cannot be
routed around, and it fails CLOSED: wrong suffix -> `sys.exit`, no numbers
printed, nothing for a human to misread.

There is deliberately NO env override. A guard with a bypass is a comment.

WHY THIS FILE IS NOT NAMED `_nohud2_guard.py`
    It was, for one afternoon, and `.gitignore`'s `_[!_]*.py` rule (which exists
    to keep throwaway per-agent probes out of the repo) swallowed it - while two
    TRACKED graders imported it at module top. A fresh clone would have had
    `grade_gate.py` and `grade_axes.py` die on `ModuleNotFoundError` before
    printing anything. The `_` prefix in `scripts/` means "scratch"; this file is
    shipped infrastructure, so it carries a shipped name. Same reason
    `test_nohud2_guard.py` asserts this file is tracked by git.

WHAT THE SUFFIX ACTUALLY CLAIMS
    `*-nohud2.png` means "there is no HUD in these pixels" - not "this file was
    produced by one specific tool". There are exactly two honest producers:
      1. `scripts/_flamingo_dehud2.py` run over a `--play` capture (main.rs draws
         `hud.rs`, so a raw capture always needs this pass);
      2. `target/release/voxelforge_shot.exe` (`client/src/shot_main.rs`), which
         renders `hero.rs` only and has no HUD to draw - its frames are HUD-free
         by construction, so `scripts/render_grade.sh` names them accordingly.
    Anything else - including a raw `--play` capture someone renamed by hand - is
    lying to the guard, and no filename check can catch that. This is a speed
    bump against the accident that actually happened, not a proof of provenance.

Usage (at the head of every grading script):

    from nohud2_guard import require_nohud2
    require_nohud2(paths, tool="grade_gate.py")
"""
import os
import sys

SUFFIX = "-nohud2.png"
EXIT_REFUSED = 2

# ---------------------------------------------------------------------------
# COVERAGE REGISTRY - the single source of truth for "which graders are gated".
# ---------------------------------------------------------------------------
# The first version of this guard was wired into 2 of the 10 tracked grading
# entry points, which made the docstring above ("cannot be routed around")
# false: `grade_g3.py`, `grade_look.py` and `grade_hero.py` measured the same
# HUD-sensitive numbers with the door wide open, each still carrying an implicit
# `argv[1] or "some-old-frame.png"` default - exactly how a stale capture gets
# graded without anyone naming it.
#
# So the classification is now explicit and enforced: `tests/test_nohud2_guard.py`
# fails if a tracked grading entry point is in neither list, which means a NEW
# grader cannot be added without deciding, in writing, which side it is on.
#
# Grading entry points = tracked `scripts/*.py` matching these globs.
ENTRY_POINT_GLOBS = ("grade_*.py", "colour_gate.py", "measure_penumbra.py")

# Gated: every number they print is measured off frame pixels the HUD corrupts.
GUARDED = (
    "grade_axes.py",
    "grade_beauty.py",
    "grade_g3.py",
    "grade_g7.py",
    "grade_gate.py",
    "grade_hero.py",
    "grade_look.py",
    "grade_midtone.py",
    "grade_web_parity.py",
    "measure_penumbra.py",
)

# Exempt: these do NOT grade a capture, so demanding the suffix would be a
# ritual, not a safeguard. Each entry states why - an exemption without a reason
# is how a gate gets quietly loosened.
EXEMPT = {
    "grade_ref.py":
        "grades the curated golden reference (docs/assets/golden-beauty-shot-ref.png), "
        "an artwork file that never had a HUD; requiring the suffix would mean renaming "
        "the golden ref itself.",
    "grade_ref2.py":
        "same input as grade_ref.py - the golden reference, not a capture.",
    "colour_gate.py":
        "audits committed docs/assets PNGs (verdict cards, gate3 evidence, and frames "
        "kept BECAUSE they are broken); it has its own allowlist "
        "(docs/assets/.colour-gate-allow) and a suffix rule would reject the evidence "
        "it exists to measure.",
}


def require_nohud2(paths, tool=None):
    """Refuse anything that is not an existing `*-nohud2.png`.

    Exits the process (code 2) on the first violation set - it never returns a
    boolean a caller could ignore. Checks ALL paths before exiting so a batch
    call reports every bad argument in one go instead of one per re-run.
    """
    who = f"{tool}: " if tool else ""
    if isinstance(paths, (str, bytes, os.PathLike)):
        paths = [paths]
    paths = list(paths)

    if not paths:
        print(f"REFUSED  {who}no frame given (need <frame>{SUFFIX})", file=sys.stderr)
        sys.exit(EXIT_REFUSED)

    bad = []
    for p in paths:
        s = os.fspath(p)
        if not s.lower().endswith(SUFFIX):
            bad.append((s, f"not de-HUDded - filename must end in {SUFFIX}"))
        elif not os.path.isfile(s):
            bad.append((s, "missing on disk"))

    if bad:
        for s, why in bad:
            print(f"REFUSED  {who}{why}: {s}", file=sys.stderr)
        print(
            f"\nNothing was graded. Every number in this repo is only meaningful on a\n"
            f"de-HUDded frame: the [E] prompt glyphs are ~250 and sit on the ground, so a\n"
            f"raw --play capture false-FAILs G5 and drags the p95 axis up to the UI.\n"
            f"Run the frame through scripts/_flamingo_dehud2.py first, then grade the\n"
            f"*{SUFFIX} it writes.  (A voxelforge_shot.exe frame draws no HUD at all -\n"
            f"scripts/render_grade.sh already names those *{SUFFIX}.)",
            file=sys.stderr,
        )
        sys.exit(EXIT_REFUSED)
