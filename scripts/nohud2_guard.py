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
# WHY THE REGISTRY IS NOT DISCOVERED BY FILENAME
#     It was, and the number it reported ("13 entry points classified") was 13 of
#     a GLOB, not 13 of "ways to print a number about a frame". `dof_crop_compare.py`
#     — hardcoded `hero-look-final.png`, no argv, prints a sharpness table and an
#     fg:bg ratio against the golden ref — is the `grade_beauty.py` defect
#     verbatim, and it sailed past because its name does not start with `grade_`.
#     The globs themselves admitted the flaw: `measure_penumbra.py` had to be
#     hand-added to the tuple for exactly that reason.
#
#     So discovery now reads what a script DOES. A tracked `scripts/*.py` is a
#     grading entry point if it either
#       (a) opens an image AND computes a statistic over the pixels, or
#       (b) imports one of this repo's grading modules (the import path is how
#           `make_gate3_verdict_card.py` reached `grade_axes.measure()` while the
#           guard sat in `grade_axes.main()` — a door beside the guarded one).
#     The globs are kept as a union term, not as the definition: a file named
#     `grade_*.py` must be classified even if it is currently a stub.
ENTRY_POINT_GLOBS = ("grade_*.py", "colour_gate.py", "measure_penumbra.py")

# (a) reads pixels ...
_READS_PIXELS = r"Image\.open|imread|imageio"
# ... and reduces them to a number a human could quote.
_MEASURES = (r"\.(var|mean|std|sum|min|max|median|histogram|getextrema)\(|"
             r"np\.(var|mean|std|median|percentile|count_nonzero|histogram)")
# (b) reaches a measurement function through an import instead of a CLI.
_GRADING_MODULES = ("grade_axes", "grade_gate", "grade_beauty", "grade_g3", "grade_g7",
                    "grade_hero", "grade_look", "grade_midtone", "grade_ref", "grade_ref2",
                    "grade_web_parity", "colour_gate", "measure_penumbra")


def is_entry_point(name, src):
    """Does this script have a way to print a number measured off frame pixels?

    `name` is the bare filename, `src` its source text. Deliberately over-eager:
    a false positive costs one line in EXEMPT with a reason, a false negative
    costs a HUD-graded number on a card someone believes.
    """
    import fnmatch
    import re

    if any(fnmatch.fnmatch(name, g) for g in ENTRY_POINT_GLOBS):
        return True
    if re.search(_READS_PIXELS, src) and re.search(_MEASURES, src):
        return True
    stem = name[:-3] if name.endswith(".py") else name
    return any(re.search(rf"^\s*(?:import|from)\s+{m}\b", src, re.M)
               for m in _GRADING_MODULES if m != stem)

# Gated: every number they print is measured off frame pixels the HUD corrupts.
GUARDED = (
    "dof_crop_compare.py",
    "dof_decision_sheet.py",
    "grade_axes.py",
    "grade_beauty.py",
    "grade_g3.py",
    "grade_g7.py",
    "grade_gate.py",
    "grade_hero.py",
    "grade_look.py",
    "grade_midtone.py",
    "grade_sunsplit.py",
    "grade_web_parity.py",
    "grass_bimodality.py",
    "make_gate3_verdict_card.py",
    "measure_penumbra.py",
    "cast_shadow_penumbra.py",
    "sun_locked_edges.py",
    "band_map.py",
    # per-agent probes that print a frame-measured number (diagnostic graders):
    "_flamingo_halo_probe.py",
    "_flamingo_char_dist_solve.py",
    "_poppy_halo_probe.py",
    "_poppy_sweep_report.py",
    "_rose_gamut_clip_probe.py",
)

# Exempt: these do NOT grade a capture, so demanding the suffix would be a
# ritual, not a safeguard. Each entry states why - an exemption without a reason
# is how a gate gets quietly loosened.
EXEMPT = {
    "_flamingo_dehud2.py":
        "the de-HUD tool itself — a HUD-bearing capture is its INPUT by definition, and "
        "the *-nohud2.png every guarded script demands is its output. Guarding it would "
        "mean no frame could ever be produced.",
    "make_vfx_pair_sheet.py":
        "its plates come from target/*/voxelforge_shot.exe (render_vfx_pairs.sh), which "
        "draws no HUD, and the only number it prints is a before-vs-after changed-pixel "
        "fraction measured against a control floor captured from the SAME source — a "
        "relative delta, not a look-acceptance axis. It never claims a frame is HUD-free.",
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
    # --- 2026-08-09 batch (Rose) — 22 tracked entry points the behavioural
    # detector found unclassified. GUARDED ones (grade_sunsplit, grass_bimodality,
    # halo probes, sweep report, char-dist, gamut-clip probe, band_map, and the
    # already-wired cast_shadow/sun_locked) now call require_nohud2(). The rest
    # are EXEMPT for the concrete reason on each line.
    "palette_break.py":
        "audits COMMITTED look artwork (gate3 evidence frames + golden ref + wide "
        "hero) against docs/look-bible.md palette; it grades curated assets, not a "
        "live capture -- exactly the colour_gate/grade_ref case.",
    "depth_layering.py":
        "audits COMMITTED gate3 evidence frames + the golden ref for depth layering; "
        "like colour_gate/grade_ref it reads curated artwork PNGs, never a live capture.",
    "_review_steam_capsules.py":
        "audits COMMITTED Steam capsule artwork (docs/assets/steam/*) against Valve "
        "tone/legibility/safe-area spec; the art never had a HUD, like grade_ref.",
    "_review_steam_confirm.py":
        "confirms shipped Steam capsules are pure crops of the committed master "
        "artwork (docs/assets/steam/*); it measures derivation, not capture quality.",
    "_review_steam_safearea.py":
        "checks the committed library-hero artwork (docs/assets/steam/*) against "
        "Valve's safe-area spec; audits curated art, not a HUD-bearing capture.",
    "_flamingo_sat_ladder_sheet.py":
        "composite SHEET maker -- tiles sweep plates into one image and labels them "
        "via the already-guarded grade_axes.measure(); it emits a picture, not a verdict.",
    "_poppy_ba_sheet.py":
        "before/after composite SHEET for the magenta-fix A/B; prints descriptive "
        "mean-RGB labels for documentation, not a look-acceptance grade of a capture.",
    "_poppy_shotset_sheet.py":
        "composite SHEET from a manifest; its per-channel |delta| is a null-A/B tell "
        "for the sheet, not a look-quality verdict on a capture.",
    "grade_compare_sheet.py":
        "composite SHEET maker -- lays frames side by side and labels them via "
        "grade_axes.py run as a guarded subprocess; it builds a picture, not a verdict.",
    "_flamingo_char_report.py":
        "concept-vs-ingame comparison SHEET built from already-graded JSON + committed "
        "concept art (docs/assets/characters/*); the capture grading happened upstream "
        "in grade_character, not here.",
    "_poppy_depromptify.py":
        "a DE-HUD producer -- cv2.inpaint erases the residual [E] prompt glyph FROM a "
        "HUD-bearing capture to output *-nohud.png; the dirty frame is its INPUT, by "
        "definition, exactly like _flamingo_dehud2.",
    "_poppy_showcase_dehud.py":
        "a DE-HUD producer -- strips crosshair/prompt/text FROM a raw showcase frame "
        "to output *-nohud.png; a dirty frame is its INPUT, like _flamingo_dehud2.",
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
