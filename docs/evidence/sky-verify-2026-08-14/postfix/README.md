# Sky-overexposure fix verification — POST-FIX capture, 2026-08-14 13:40

## Bottom line: `5eba1e8` does NOT visibly fix the sky. All three measured axes are unchanged.

This closes the provenance gap the parent README (`../README.md`) flagged: that
report could not tell whether the frames on disk were stale or the fix wasn't
landing. This one can, because every step is freshly stamped:

- Killed the stray duplicate build (`19992`/`21648`, blocked on the target lock
  since 13:15:06); left the real build (`22624`→`4704`→`23376` rustc) running.
- Confirmed via Build Sentinel (not `tail`, not a pipe's exit code):
  `verdict: green, exitCode: 0, error-scan pass across 331 lines, artifact fresh`.
- Confirmed `client/src/look.rs` is clean at HEAD (`eea8bdd7`) and `5eba1e8` is
  an ancestor — `exp_comp = 1.0` (the fix) is genuinely compiled into this exe.
- Captured fresh via `BIN=./target/release/voxelforge.exe bash scripts/prove_playable.sh`
  — `PROVE_PLAYABLE: PASS`, all 4 shots clean.
- Checksummed the new frames against the OLD (pre-fix-reported) copies in the
  parent folder — **different bytes**, so this is a genuinely new render, not a
  stale file being re-graded:
  - `playable-walk-after.png`: old `1508bf17…` vs new `248a1c27…`
  - `edhari-load.png`: old `63659690…` vs new `6a4be6c1…`
- Full provenance in `.shotlog.txt` (exe mtime, commit sha, capture command,
  capture time, log files).

## The numbers — new (post-fix) vs. documented pre-fix baseline

`scripts/grade_sky.py` (new, direct sky-only gate — calibrated with `--selftest`
against synthetic ground truth before being trusted here, and independently
checked: it reproduces the documented pre-fix numbers almost exactly when run
on the pre-fix copies in the parent folder):

| frame | metric | pre-fix (documented) | **post-fix (this capture)** | moved? |
|---|---|---|---|---|
| playable-walk-after.png | sky-band R-span | 0.7–0.8 | **0.67** | NO |
| playable-walk-after.png | sky-band %R>=250 | 14.33–14.46% | **14.45%** | NO |
| edhari-load.png | sky-band R-span | 0.9–1.0 | **0.55** | NO (still <=1.5 FAIL bar) |
| edhari-load.png | sky-band %R>=250 | 1.27–5.33% | **1.27%** | NO |
| both | whole-frame %true-burnout | 0.16% | **0.16%** | NO — exact match |

`grade_sky.py <frame>.png` verdict on both: **FAIL (exit 1)** — matches the
documented pre-fix blown signature.

Independent cross-check with the team's own already-validated
`scripts/_poppy_sky_exposure_probe.py` (output: `probe_output.txt`), run
against these same fresh frames (it reads `docs/assets/*.png`, which
`prove_playable.sh` had just overwritten):

- `playable-walk-after.png`: R span **0.7**, sky candidates 14.1% of band,
  mean sky RGB ≈ [252.9, 229.9, 196.0] — the exact "flat near-clipped cream"
  the commit describes as the bug.
- `edhari-load.png`: R span **0.9**, sky candidates 5.7% of band, mean sky RGB
  ≈ [252.1, 224.2, 186.3] — same signature.

Both tools, built independently and calibrated separately, agree: **the sky
gradient did not widen, the clipped-pixel fraction did not collapse, and the
whole-frame burnout did not move even in the second decimal place.**

## Visual confirmation

Both new frames were opened and inspected directly — the sky visible through
the gaps in the rooftops is still a flat, blown, near-white patch in both
(`playable-walk-after.png` top-centre/top-right, `edhari-load.png`
top-left/top-centre), visually indistinguishable from the pre-fix screenshots
already on file. This is not a measurement artifact of either grading tool.

## What this means

`exp_comp = 1.0` is genuinely in the binary that shot these frames (confirmed
by commit ancestry + a clean working tree + a green, freshly-verified build),
and the visible result is unchanged from the documented pre-fix state. Two
honest possibilities, not distinguished by anything measured here:

1. `build_sky_dome_mesh` / `haze_color()` has a second multiplier still
   applying something close to the old `exp_comp` (or an equivalent factor)
   that this one-line change didn't reach.
2. The `--play` / `--play-demo` render path that `prove_playable.sh` exercises
   doesn't go through the code path `5eba1e8` touched at all (e.g. a different
   sky material variant, a cached/precomputed mesh, or a feature-gated branch).

Either way: **do not report the sky as fixed.** The next step is in
`client/src/look.rs` itself, not in more captures — the fix needs a second
look at what else feeds the dome's material colour beyond `exp_comp`.
