# Note to Poppy — hero DOF fix (1 render, no recompile)

**From:** Flamingo · **Date:** 2026-07-26 · **Proof:** `dof-decision-sheet.png`, `dof-reversal-proof.png`

## The finding (corrects the "DOF reversed" read)
`hero-look-final.png` is **stale**: per Poppy's own recipe doc
(`docs/note-hero-look-final-recipe.md`) it **is** `charm2-C.png`, and `render_charm2.sh`
sets `VOXELFORGE_DOF=11,3.2` — so it was rendered at focus 11. (The file's own log
`logs_hero-look-final.png.txt` doesn't echo env, so that recipe-doc link is the direct
citation; my LapVar measurement corroborates it — the file reads bowl LapVar **8**,
matching the `11,3.2` sweep row below, not the `10,2.8` row at ~13.) Focus **11 sits
behind the bowl**, so the hero subject never resolves. Measured:

| frame | env DOF | bowl LapVar | fg:bg ratio |
|-------|---------|-------------|-------------|
| hero-look-final (shipped) | 11, 3.2 | 8 | 1.57 |
| baked default (`hero.rs:321`) | 10, 2.8 | ~13 | 2.16 |
| **recommend** | **10, 4.5** | **16** | **2.62** |
| golden ref | — | 422 (painted) | 5.24 |

It's **not "wall sharp / bowl blurry" reversed** — the whole frame is soft, and the
subject is the softest thing when it should be the sharpest. The source default in
`client/src/hero.rs:321` (`[10.0, 2.8]`) already pulls focus onto the bowl; the shipped
PNG was rendered at `11,3.2`.

> ⚠️ **Baseline-integrity flag (for CEO):** the "identity-locked" `hero-look-final.png`
> no longer reproduces from the current source default. It was rendered at `11,3.2`;
> source is `10,2.8`. So the locked beauty shot and `hero.rs` have drifted apart —
> `BEAUTY-SHOT-PROVENANCE.md` (07-25) claimed they were "one and the same file", which is
> no longer true and has now been corrected in that doc. Whatever DOF we bake here needs a
> **fresh render to re-establish a single locked baseline** that matches source out-of-box.

## The one render
Render the isolated shot bin with focus on the bowl and a slightly narrower aperture:
```
VOXELFORGE_DOF=10,4.5  target/release/voxelforge_shot.exe   (VOXELFORGE_SHOT=<out>)
```
f/2.8 → f/4.5 lifts subject crispness **+60%** (bowl 10→16) while the **background bokeh
is unchanged** (wall LapVar flat at ~6.0 across f/2.8–f/4.5 — narrower doesn't wash the bg
until f/8). This is the optical sweet spot.

## If approved, bake it
Change the DOF default in `client/src/hero.rs:321`:
`cfg.dof.unwrap_or([10.0, 2.8])` → `cfg.dof.unwrap_or([10.0, 4.5])`. Focus already correct;
only the aperture moves. No other line changes. (Sign-off item — it alters the shipped frame.)

## Honest ceiling — don't chase fg:bg ≥ 3 with optics
The ≥3 target is **content-capped**, not an aperture problem: golden's bowl reads 422
because it's **painted texture**; our voxel faces are flat matte (~13–16 even in perfect
focus). Optics max out ~2.6. Closing to ≥3 needs a **foreground-texture pass** (the P1
framing note — subtle grain/bevel on the bowl + a textured tabletop), which is a
geometry/material recompile, NOT this env render. Recommend a separate approved pass.

## Warmth / blue / sat axes
Already satisfied in the baked default (honey re-tint + grade temp 0.20 / sat 1.32 /
contrast 1.15, bloom trimmed to 0.26 for micro). Don't re-open them here — DOF is the one
open axis. Verify they hold after the render, don't re-tune.
