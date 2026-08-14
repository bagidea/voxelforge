# Sky-overexposure fix verification — 2026-08-14 (sky-verify)

Verifying commit `5eba1e8` (`fix(look): the sky was 1513x overexposed`), which sets
`exp_comp = 1.0` in `client/src/look.rs:1897` (confirmed present at current HEAD,
`git status --porcelain client/src/look.rs` is clean).

**UPDATE 13:40 — the provenance gap below is closed. See `postfix/README.md`:
a freshly built, freshly captured, checksum-verified-different frame off this
exact source still measures FAIL. `5eba1e8` does not visibly fix the sky.**

## [ORIGINAL 13:25 report] Bottom line: NOT CONFIRMED FIXED — provenance gap, not a verified pass

The only frames available on disk (`docs/assets/playable-walk-after.png`,
`docs/assets/edhari-load.png`, mtime 2026-08-14 06:36:41) still show the sky-band
pixel signature almost identical to the **broken** numbers the fix commit itself
quotes as evidence of the bug:

| | commit's quoted "before" (05:35 capture) | measured here, now |
|---|---|---|
| `playable-walk-after.png` R/G/B span | R 0.8 / G 6.0 / B 11.0 | R 0.7 / G 4.8 / B 7.3 |
| `edhari-load.png` R/G/B span | R 1.0 / G 6.0 / B 9.9 | R 0.9 / G 5.8 / B 9.7 |

Both are the "flat near-clipped cream" pattern the commit describes as the bug — a
live dome gradient should move far more than ~1 level of R across 170+ rows of
elevation. These are either stale, un-refreshed frames (mtime updated by a
checkout/copy, not a fresh render) or the fix genuinely isn't landing visually —
I cannot tell which from pixels alone, and I am not claiming either. There is no
`.shotlog.txt` / exe-identity stamp next to these two files, so unlike the
`sky-2026-08-14/` evidence set (which stamps every frame's producing binary),
these two have no provenance I can verify.

**What is confirmed:**
- The source fix (`exp_comp = 1.0`) is committed and present at HEAD.
- A binary built from post-fix source did exist: Build Sentinel run
  `id=1786662166672` started 06:02:46 (after the 05:53:05 fix commit), finished
  06:15:23, verdict `green`, exit 0, artifact `target/release/voxelforge.exe`
  fresh at that time.
- That binary no longer exists on disk — deleted by an accidental
  `cargo clean -p voxelforge` earlier in this task. `target/release/voxelforge.exe`
  is absent as of this report.
- Two redundant `cargo build --release --bin voxelforge -j 2` processes are
  currently rebuilding it (PIDs 22624/4704 started 13:14:45, PIDs 19992/21648
  started 13:15:06 — a duplicate that should be avoided next time). Not waited on
  for this report per instruction.

## What was actually measured (real numbers, no waiting)

Ran `scripts/_poppy_sky_exposure_probe.py` (no rebuild required, read-only) against
the two frames present on disk:

```
$ python scripts/_poppy_sky_exposure_probe.py
```
Full output: `probe_output.txt`.

- `playable-walk-after.png` (1280x720): sky-band (y46..324) **%R>=250 = 14.33%**,
  true burnout (R,G,B all >=250) **= 0.00%**. Row span R0.7/G4.8/B7.3 over 177 rows.
- `edhari-load.png` (1280x720): sky-band **%R>=250 = 5.33%**, true burnout **= 0.00%**.
  Row span R0.9/G5.8/B9.7 over 62 rows.
- Whole-frame true burnout (R,G,B>=250 every pixel, not just sky band):
  `playable-walk-after.png` 0.16%, `edhari-load.png` 0.16% — this matches the
  commit's own aside ("only 0.2% of it is [255,255,255]") almost exactly, another
  sign these are the pre-fix reference frames rather than a fresh capture.

Full per-file numbers: `burnout_measurements.txt`.

## Gate scripts — real exit codes

`grade_gate.py` / `grade_g3.py` refuse any frame not named `*-nohud2.png`
(hard guard, `scripts/nohud2_guard.py`). Ran `scripts/_flamingo_dehud2.py` on both
frames to produce gradable copies, then graded:

| frame | grade_gate.py exit | grade_g3.py exit | grade_gate verdict |
|---|---|---|---|
| `playable-walk-after-nohud2.png` | 0 | 0 | G3=PASS G5=PASS G6=PASS |
| `edhari-load-nohud2.png` | 0 | 0 | G3=PASS G5=PASS G6=PASS |

**Caveat: these gates score interior shade/window/wood tone — none of the three
measurable gates (G3/G5/G6) touch the sky dome.** A PASS here says the interior
look is fine; it says nothing about whether the sky overexposure is fixed. Do not
read this table as "sky fix verified" — it isn't what it measures.

Raw output: `gate_grading_output.txt`.

## Files in this folder

- `playable-walk-after.png`, `edhari-load.png` — copies of the two frames as found
  on disk at report time (mtime 06:36:41), unmodified.
- `playable-walk-after-nohud2.png`, `edhari-load-nohud2.png` — de-HUDded copies
  used for gate grading.
- `probe_output.txt` — full stdout of `_poppy_sky_exposure_probe.py`.
- `burnout_measurements.txt` — %R>=250 / %true-burnout, whole-frame and sky-band,
  raw and nohud2 versions.
- `gate_grading_output.txt` — full stdout + exit codes of `grade_gate.py` /
  `grade_g3.py` on both nohud2 frames.
- `process_evidence.txt` — process list proving the two in-flight rebuilds, and the
  absence of `target/release/voxelforge.exe` at report time.

## What still needs to happen

Once a rebuild finishes (not waited on here), capture a **stamped** frame — same
recipe as `docs/evidence/sky-2026-08-14/` (`.shotlog.txt` naming the exe's mtime +
size) — so provenance is provable, then re-run this same probe + gate pair. If the
fix landed, sky-band %R>=250 should collapse toward single digits and the row span
should widen well past 1 level (the commit's own math predicts ~1500x less
radiance, nowhere near the tonemapper's shoulder). If the numbers come back
unchanged from this report, the fix does not render as intended and needs a second
look at `look.rs` beyond the `exp_comp` literal.
