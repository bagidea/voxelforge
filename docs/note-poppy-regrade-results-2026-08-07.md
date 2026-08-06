# Regrade shotset — numbers for Flamingo (2026-08-07)

Numbers only. I am not calling this better or worse; that read is Flamingo's.

## What was shot

- 8/8 plates, `DONE=FINISH_OK` — shot-fails 0 · pairs 8 · unpaired 0 · map-drift 0
- every plate logged `map 8838 blocks` = baseline, so no world edit is inside these deltas
- exe: sha `CC414845…E521B`, mtime 2026-08-07 05:38:59, commit `fc329c183`, `stale=False`.
  No `cargo` was run for this shotset — the binary did not move.
- before side = the 19:53 exe plates (02:33); after side = target-flamingo 05:38 exe

Artefacts, in the repo: `_poppy_shotset/regrade.json`, `regrade_table.txt`,
`regrade_gates_crosscheck.txt` — the three every number below is read from.
On disk only, not tracked: `regrade_console.txt` (33 KB raw measurer log, superseded by
`regrade.json`), `before-after-sheet.png` (1276x3264, 8 rows, 3 MB) and the per-plate
PNGs. `.gitignore` allowlists the three by name; adding a PNG to that dir cannot reach
the repo.

## Axis deltas (AFTER − BEFORE, `beforePass->afterPass`)

See `_poppy_shotset/regrade_table.txt` for the full grid. Headlines:

- warmth R-B is **negative on all 8 plates** (−4.32 … −21.66); it flips a gate only on
  `hero` (P->F)
- vegHue is **positive on all 8** (+4.8 … +25.3)
- hi p95 `+0.00` on `s1-vista` and `s4-raking` is real, not a stuck measurer: ~12.9% of
  those frames is flat sky, so p95 lands in the same band both times
- P0 axes PASS, all 8 plates: **28/48 -> 24/48**

## Gates — BOTH measurers, because they do not agree

`regrade_table.txt` carries two independent gate readouts and they disagree on **5 of 8
plates**, including the direction of the total. Reporting one of them alone would be
half a verdict, so both are here and neither is dropped:

| plate | grade_gate G3/G5/G6 | grade_hero G3/G5/G6 | disagree |
|---|---|---|---|
| gate3-boot | P->P  P->P  P->P | P->P  P->P  F->F | G6 |
| gate3-combat | P->P  F->P  P->P | P->P  P->P  P->P | G5 |
| gate3-walk | P->P  F->F  F->F | P->P  P->P  F->F | G5 |
| grade-vista | P->P  P->P  P->P | P->P  P->P  P->P | — |
| hero | P->P  P->F  P->P | P->P  P->F  P->P | — |
| s1-vista | P->P  F->F  F->P | P->P  F->F  F->P | — |
| s3-clash | P->P  P->P  P->P | P->P  P->P  P->F | G6 |
| s4-raking | P->P  P->P  F->F | P->P  F->F  P->P | G5, G6 |

- `grade_gate.py` totals: **18/24 -> 19/24** (up 1)
- `grade_hero.py` totals: **19/24 -> 18/24** (down 1)

Who measures what:

- **`grade_gate.py`** auto-*locates* the window and the warm patch anywhere in the frame,
  so it runs on any plate. This is the column I reported first.
- **`grade_hero.py`** uses *fixed* eyedrop regions tuned for the golden kitchen hero shot
  (window assumed screen-left, sunlit wood assumed in the lower 55%). On a vista or a
  clash plate those regions point at whatever happens to be there — which is my best
  guess at why the disagreement clusters on non-hero plates, but it is a guess, not a
  measurement. On `hero` itself the two agree exactly.
- `grade_look.py` gives a third G3/G5 opinion and agrees with `grade_gate.py` on all 8
  plates, both sides (checked cell by cell: 0 mismatches). So the split is
  `grade_hero.py` against the other two, not a three-way scatter.

The hero column only became visible this round — it was silently dead until the regex fix
below. It has not been cross-validated against anything; treat it as a second opinion that
just came online, not as a correction to the first.

## Two bugs fixed in the tooling this round

Both in scripts, none in the build. The binary was never rebuilt.

1. `scripts/regrade.py` — `_run(*G7_SCRIPT[0], …)` splatted a string one character at a
   time, so it invoked `scripts\g` and returned rc=2. Every G7 vegetation axis was
   missing. Fixed to `_run(script, *args)`.
2. `scripts/regrade.py` — `_parse_hero`'s regex was inside an f-string, so `({{PASS|FAIL}})`
   compiled to a literal `{PASS|FAIL}` and never matched. That is why the entire
   `grade_hero.py` column read empty until now.
3. `scripts/_poppy_shotset_sheet.py` — the canvas height summed
   `t[3] + HEAD_H + PAD*2` per row while the draw loop advanced by
   `HEAD_H + 18 + th + PAD*2`, so the sheet came up 18px short per row = 144px over
   8 rows and the last row (`s3-clash`) was cropped. Both sides now read one `CAP_GAP`
   constant, and a layout self-check fails the run loudly instead of saving a short sheet.

Re-running the identical regrade command after fix 1+2 gave `errored measurers: none` and
byte-identical deltas on the 6 pre-existing axes, so the fixes added columns without
moving anything that already worked.

## Still open

- interpreting these numbers is Flamingo's call — in particular which measurer a given
  plate should be graded by. I am not picking one.
- `grade_hero.py`'s fixed eyedrop regions are still only a *hypothesis* for the
  disagreement (see above). Nobody has measured where those regions actually land on a
  vista or clash plate.

The `regrade.py` / sheet fixes and the three numeric artefacts land in the same commit as
this note, so a fresh clone can re-derive the table above.
