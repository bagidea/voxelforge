# VERDICT — PCSS on grade-vista: prettier, or just blurrier?

**Flamingo · 2026-08-09 · eye call, zero build, `client/src/` untouched**

Asked: re-shoot `grade-vista` off the existing N7 exe with `VOXELFORGE_LOOK_PCSS=16`,
put it next to the shipped `_yama_shotset_N7/after/grade-vista-nohud2.png` (which is
High-with-PCSS-off, per b8436ce), and answer one question by eye.

**Answer: at 16, on this plate, it is BLURRIER, not prettier. Net loss.**
One axis genuinely improves; three regress, and the three are the ones that carry the
frame. But the damage has a specific shape and Ultra can plausibly repair it — see
"what this verdict does NOT cover" at the bottom, which is the reason to still wait
for Yamamoto's full set.

---

## What was shot

One binary, `_yama_shotset_N7/vfprobe.exe`, sha256 `5d01d945…`, the exact exe that
shot N7. argv/env lifted verbatim from `_yama_N7_shoot.sh:32`
(`VOXELFORGE_LOOK_CAM=35,-18,26 VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_PLAY=1`),
through the same `scripts/_flamingo_dehud2.py`. Only `VOXELFORGE_LOOK_PCSS` moved.
Four rungs: **no env at all** (= the shipped plate's exact env), 6, 16, 48.

`VOXELFORGE_LOOK_PCSS` does reach the frame even though the tier does not —
`client/src/look.rs:1417` (working tree; `:1355` at HEAD) is
`Ok(v) => v.trim().parse().ok().or(tier_on.then_some(PCSS_WIDTH))`: the env parse
lands in the `Ok` arm and only the `or(...)` fallback ever consults `tier_on`, so an
explicit env value wins regardless of tier.

### The A/B was proven readable before anything was judged

Rung 1 passes **no** PCSS env, so it is a straight re-shoot of the shipped plate. It
exists to separate run-to-run drift from the PCSS delta:

| pair | mean abs L | px moved >2 LSB |
|---|---|---|
| shipped N7 plate vs today's no-PCSS re-shoot | 0.290 | **1.40 %** ← noise floor |
| no-PCSS re-shoot vs PCSS 6 | 1.199 | 10.32 % |
| no-PCSS re-shoot vs PCSS 16 | 5.860 | **45.22 %** |
| no-PCSS re-shoot vs PCSS 48 | 11.937 | 59.85 % |

45.22 % independently reproduces Poppy's 45.4 % off the same binary, and it is 32×
the noise floor. The comparison is real; the eye call below is not reading noise.

Crops were then picked **from the diff** (top block-mean hotspots, min 500 px apart)
so they could not be cherry-picked to flatter either side — and a second sheet was
hand-picked on long-throw shadow edges, which is where a penumbra filter is
*supposed* to win, so PCSS got its best case too.

---

## The one thing PCSS 16 wins

**The long-throw diagonal on the left wall.** PCSS-off gives a hard, stair-stepped
edge — the aliasing reads as a jaggy line, not as sunlight. At 16 that edge becomes a
real gradient and the wall finally reads like a wall in afternoon sun. This is a
genuine, visible improvement and it is the strongest argument for the feature.

It is also not buyable cheaply: at 6 the same edge is only marginally softer (10 % of
the frame moved and the edge barely changed). The win arrives with the cost attached.

## The three things it costs

1. **Ground-shadow density and the depth cue go with it.** The mid-right dirt slope
   loses its cast shadow almost entirely; foreground grass loses its shadowed band.
   Mid-ground and foreground flatten into the same tonal register — the frame stops
   reading as deep.
2. **Contact shadows die.** Where a wall meets the floor, and under the far block
   stack, the dark seam that grounds the geometry is gone. Blocks stop reading as
   stacked masses and start reading as pasted-on flats. For a voxel look, where the
   entire silhouette language is "boxes on boxes", this is the expensive one.
3. **Blocker silhouette melts, plus blotching.** At 16 you can no longer read *what*
   is casting the upper diagonal — the notched geometry that made the shape legible
   is a smear. And flat lit faces pick up mottled patches (blocker-search
   undersampling), i.e. a *new* artifact, not a softer old one.

At 48 all three are worse and the shadow shapes have dissolved outright — which
matches, by eye, the `PCSS_WIDTH` doc's own written ceiling ("silhouette starts
dissolving past ~24"). Where my eye disagrees with that doc: **on grade-vista the
dissolve has already started at 16.** That ceiling was derived on the **s4** framing
(`look.rs:407`, 15 sites), and s4 is not this camera.

### The numbers already said this; they were read as a pass

`look.rs:430` records the collateral at 16 on this plate: warmth 162.98 → 157.03,
micro-contrast 7.38 → 6.80, p95 163.89 → 159.97 — logged as "all PASS at both widths".
All three are also **all down**. "Flatter, cooler, less micro-contrast" is the numeric
shadow of exactly what the eye sees. The gate passing hid the direction of travel.

---

## What this verdict does NOT cover — and why the Ultra set still matters

The tier is still stomped to High in these runs (b8436ce). So what was judged is
**High + PCSS 16**, not Ultra. Ultra additionally brings Ultra SSAO and full-step
volumetric god rays.

That matters, because **cost #2 — the dead contact shadows — is precisely
SSAO-shaped damage.** Ultra SSAO could put back the contact darkening PCSS just took
away, and if it does, the balance flips: the wall-edge win survives while the worst
cost is paid for by another part of the stack.

So: **on its own, 16 is a net loss on this plate — but do not cut PCSS on this
verdict.** When Yamamoto's real Ultra set lands, judge it on these three, in order:

1. Does the wall/floor seam get its dark contact line back?
2. Do the far block stacks separate from each other again?
3. Is the upper diagonal's blocker still legible as geometry?

If 1 and 2 come back and 3 does not, the answer is a **width below 16 at Ultra**, not
PCSS off — the wall edge is worth keeping.

---

## Artefacts

Output — under `_flamingo_pcss_ab/`, ignored by `.gitignore:166` (`_*/`):

    VERDICT-card.png    the call, 2x crops, 3 columns:
                        shipped N7 plate | no-PCSS re-shoot control | PCSS 16
    sheet-full.png      whole frame, the same three, stacked half-scale
    sheet-crops.png     diff-picked hotspots, control | 6 | 16 | 48
    sheet-bestcase.png  hand-picked long-throw edges, same ladder
    sheet-heat.png      where the change lives
    raw/*.png,*.log     the four captures + their stdout

Scripts — at the **repo root**, not inside that dir, and ignored by two other
rules: `.gitignore:169` (`_[!_]*.py`) and `.gitignore:170` (`_*.sh`):

    _flamingo_pcss_ab.sh                  the shoot (env lifted from _yama_N7_shoot.sh:32)
    _flamingo_pcss_drift.py               noise floor vs signal
    _flamingo_pcss_sheet.py               diff-picked crops + heat map
    _flamingo_pcss_sheet2.py              best-case crops
    _flamingo_pcss_card.py                the verdict card

So committing this lane needs **three** rules un-ignored, not one. Only this
document is committed; the probe scripts and plates stay local, same as the other
`_*` review lanes. (This repo has been bitten twice by blanket `_*` rules
swallowing lane tooling — 472aa8f, 50e1e09 — so the rule numbers are written down
rather than guessed.)

### On the plate the card shows

The middle column is **today's no-PCSS re-shoot**, not the shipped file, because the
ladder has to be measured against a same-session baseline or run drift contaminates
the delta. The shipped `_yama_shotset_N7/after/grade-vista-nohud2.png` is now the
**left** column on both the card and `sheet-full.png`, so the plate that actually
shipped is visible, not merely cited in the drift table. Left vs middle is the
1.40 % noise floor: if you can see a difference there, distrust the middle-vs-right
call.

No rebuild. No `client/src/` edit. No process left running.
