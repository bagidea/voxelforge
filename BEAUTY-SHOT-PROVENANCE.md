> ## ⚠️ SUPERSEDED ON THE DOF AXIS — read this first (2026-07-26, Flamingo)
>
> This doc was written 2026-07-25 and, on the **DOF axis only**, it is now WRONG.
> It is kept for the record but corrected here. Single current source of truth for DOF:
> **`client/src/hero.rs:321` → `cfg.dof.unwrap_or([10.0, 2.8])`** and the brief
> `docs/note-to-poppy-dof-fix.md`. Where this doc and the brief disagree, the brief wins.
>
> Three claims below are false as written:
> 1. **"DOF baked default = focus 11.0 @ f/3.2".** The source default is now `[10.0, 2.8]`
>    (focus was pulled from 11 → 10 *onto* the bowl plane after this doc was written).
>    `11.0` is no longer baked anywhere.
> 2. **"focus 11.0 @ f/3.2 (hero sharp)".** Measured on real pixels, `11,3.2` gives the
>    bowl its *softest* reading (LapVar 8) — focus 11 sits *behind* the bowl. It is the
>    worst DOF for the hero, not "hero sharp". See the sweep in `docs/note-to-poppy-dof-fix.md`.
> 3. **"hero.rs default and the golden reference are now one and the same file,
>    reproducible from source with a single flag."** Not true anymore. The locked baseline
>    `hero-look-final.png` was rendered under the `11,3.2` env convention; the source default
>    is `10,2.8`. **A zero-env render from the current source no longer reproduces
>    `hero-look-final.png`** — the identity-lock this doc claimed is broken. That gap is the
>    whole point of the open DOF fix, and it needs a fresh baked render to re-close (see brief).
>
> Everything below is the 07-25 record, preserved but NOT authoritative on DOF.

---

# Golden Beauty-Shot — Default == Golden (record of 2026-07-25 re-baseline)

**Re-baselined 2026-07-25 (Flamingo)** — resolved the reviewer finding that the
out-of-box default did not reproduce the approved beauty shot.
**(DOF portion later found wrong — see the SUPERSEDED banner above.)**

## The recipe (as baked on 07-25 — DOF value since corrected)

The intent on 07-25 was that running the shipped `voxelforge` bin with **only**
`VOXELFORGE_HERO=1` renders the golden beauty shot:

| knob      | 07-25 baked default                     | was (env-only crutch) |
|-----------|-----------------------------------------|-----------------------|
| camera    | `7.6,5.9,-5.2 → 7.6,3.2,6.0`  FOV 52°   | `8.0,4.5,-3.5 → 8.0,2.8,8.0` FOV 50° |
| DOF       | ~~focus 11.0 @ f/3.2 (hero sharp)~~ **← WRONG; source is now `10.0, 2.8`, and 11,3.2 is the softest bowl (LapVar 8), not sharp** | auto-focus @ f/1.8 (hero melted) |
| fog (vol) | **0.032** (medium god-ray haze)         | 0.06 (room hazed over) |
| ambient   | 2900                                    | 2400 |
| exposure  | ev100 9.7                               | 9.7 (unchanged) |
| dist. fog | 0.008                                   | 0.008 (unchanged) |

`cam` / `fog` were "second value of truth" the reviewer caught on 07-25: only `ambient`
had been baked, so the approved look still lived in env vars. Camera/fog/ambient/exposure
were baked then. **DOF was NOT correctly resolved by this doc** — the `11,3.2` value it
records both (a) is no longer the source default and (b) puts focus behind the bowl.

## Proof (07-25 — the DOF line of this proof does not hold)

- `python scripts/grade_gate.py hero-converged-final.png` → G3/G5/G6 all PASS.
  (These gates measure light/window/warmth, not hero sharpness, so they passed despite
  the soft-hero DOF — they do not vouch for the DOF claim.)
- The "fresh zero-env render diffs 0.16/channel vs the golden" claim assumed source==11,3.2.
  Source is now `10,2.8`, so **that identity no longer holds** — see banner point 3.
- Spec (`docs/golden-beauty-shot.md`): hero bowl SHARP, background bokeh ~f/2.8. The
  *spec* is right; the 07-25 DOF value did not meet it.

## Why the golden was re-baselined (still valid — camera/light history)

The previous golden `hero-converged-final.png` was byte-identical to `cv-amb2900.png`,
a **manual** bracket render from 04:16 whose exact env was never captured in any
script, log, or shell history — it is unrecoverable. Its best-fit camera is ~FOV 65°,
which violates the spec's own "FOV ~50°". Rather than reverse-fit to a lost, off-spec
frame, the golden was DEFINED as the baked-default render of the documented recipe. The
old frame is preserved at `hero-converged-final-OLD-cv-amb2900.png`. This reasoning about
camera/light stands; only the DOF value it locked was wrong.
