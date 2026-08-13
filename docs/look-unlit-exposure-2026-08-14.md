# Unlit fragments do not pass through Exposure (2026-08-14, poppy)

A lesson record, not a feature note. It cost the sky dome a full round: the mesh was
correct, the gradient was authored correctly, the material was correct — and it shipped
at **1513x** its intended radiance because of one plausible, wrong sentence in a comment.

Fix: `client/src/look.rs`, commit `5eba1e8`. Evidence: `scripts/_poppy_sky_exposure_probe.py`
(shader quote + maths + measurement), plate `docs/assets/_poppy_sky_overexposed_2026-08-14.png`
(before).

The after-side took two attempts, and the second one is the one to read:

| plate | pair | angle guard | verdict |
|---|---|---|---|
| `_poppy_sky_ab_2026-08-14.png` | ONE binary, dome put back to pre-fix radiance via `VOXELFORGE_LOOK_SKYGAIN=3632.16` (`scripts/_poppy_sky_gain_ab.sh`) | **2.69** levels | isolates the dome — read this one |
| `_poppy_sky_fixed_2026-08-14.png` | pre-fix exe 05:35 vs fixed exe 06:15, `prove_playable.sh` frames | **28.60 / 13.93** levels | guard fires; see "the pair that wasn't a pair" |

Both are rendered by `scripts/_poppy_sky_after_plate.py` (`POPPY_SKY_SET=ab|proof`).
The fixed binary itself is green end to end: `cargo build --release` exit 0 with no
`^error` line, then `BIN=./target/release/voxelforge.exe bash scripts/prove_playable.sh`
→ `PROVE_PLAYABLE: PASS — 4/4 shots clean`, with `PLAY_WALK … moved=9.91 grounded=true
=> PASS`.

## The rule

> In `bevy_pbr`, `Exposure` is applied **inside `apply_pbr_lighting`**. An unlit fragment
> never calls it. `unlit: true` writes `base_color` into the HDR target verbatim — exactly
> like `ClearColor`.

So for anything unlit — sky domes, billboards, UI-ish world quads, debug gizmo meshes,
emissive-only decals drawn as unlit — **the camera's `ev100` is not in the chain**. Do not
compensate for it. Author the colour at the scene-referred radiance you actually want in
the HDR target.

## The receipts

`bevy_pbr-0.19.0/src/render/pbr.wgsl:80-84`:

```wgsl
if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
    out.color = apply_pbr_lighting(pbr_input);   // exposure lives in here
} else {
    out.color = pbr_input.material.base_color;   // the dome takes this
}
```

`view.exposure` appears in `pbr_functions.wgsl` at lines 840 / 863 / 915 — all of them
inside `apply_pbr_lighting`, i.e. all of them on the branch the dome does not take. The
old comment quoted `exposure * (direct + indirect) + emissive` and concluded the multiply
applies to every fragment. The line is real; the branch above it was not read.

## The maths that turned a comment into 1513x

`Exposure::exposure()` in `bevy_camera` is `exp2(-ev100) / 1.2`. At `Hour::GOLDEN`'s
`ev100 = 10.3`:

```
exposure()   = 2^-10.3 / 1.2 = 6.607e-4
1 / exposure = 2^10.3 * 1.2  = 1513.4
```

`build_sky_dome_mesh` multiplied every vertex colour by that `1 / exposure` so the
(non-existent) division would cancel. Zenith `[0.256, 0.765, 1.890]` linear therefore went
to `[387, 1157, 2859]` — three to four orders of magnitude past the tonemapper's shoulder,
where every distinct input maps to the same near-white output.

## What it looks like when this happens to you

Not white. That is the trap, and it is why "the sky is blown" got argued about instead of
measured. Measured over the 49,486 detected sky pixels of `playable-walk-after.png`:

| | before |
|---|---|
| mean sky RGB | `[252.6, 227.2, 192.1]` |
| R range across every one of those pixels | 250 – 255 |
| literally `255,255,255` | **0 pixels** (the probe's hand-picked windows found 0.2 %) |
| mean R travel across 170 rows of elevation | **0.8 levels** |
| G / B travel, same rows | 6.0 / 11.0 levels |

A warm cream that looks like a plausible golden-hour haze in a thumbnail. The tell is not
the colour, it is the **flatness**: a dome authored deep-blue-at-zenith to warm-haze-at-horizon
that moves 0.8 levels of red from top to bottom is not a gradient, it is one value with
noise on it. Past the shoulder, the whole authored zenith-to-horizon range compresses into
under a level of output — which is also why the mean tells you nothing here and the span
tells you everything.

**Diagnostic that works:** take the per-row mean over the sky pixels and look at the
top-to-bottom span. Cheap, unambiguous, and it fails loudly whether the clip is to white,
to cream, or to any other single colour. A range-stretch of the crop is the visual version
of the same test — a live gradient survives it, a crushed one turns to noise.

## The pair that wasn't a pair

The first after-plate compared the `prove_playable.sh` frames from the pre-fix release
binary (05:35) against the same frames from the fixed one (06:15). Its own angle guard
rejected it: **28.60** and **13.93** levels of mean |Δ| over the non-sky pixels, against a
3.0 threshold. `scripts/_poppy_sky_drift_probe.py` says where that lives — the bottom
fifth of the frame, pure ground the dome cannot reach, moved **10.75 / 12.76** levels, and
every row band got brighter rather than shifting sideways. `git log -- client/` names the
cause: `72408ad` *one source of truth per block colour + pin the designer palette* (05:14)
and `3262c6e` (05:30) landed between the two builds, so every voxel was repainted in the
same step. Two binaries taken an hour apart do not straddle one commit.

So the sky was re-measured out of a **single** binary, with the dome pushed back to its
pre-fix radiance instead: `exp_comp` scaled both dome stops by 1/exposure() = 1513.4, and
`sky_gain` scales both stops too (zenith `h.sky * h.sky_gain` at `look.rs:1905`; horizon
`haze_color()`, whose scale is `HAZE_GAIN * h.sky_gain` at `look.rs:1264`) — so
`VOXELFORGE_LOOK_SKYGAIN = 2.4 × 1513.4 = 3632.16` reconstructs the shipped-broken dome
exactly, out of the binary that contains the fix. Distance haze is pushed past the world in
**both** shots (`VOXELFORGE_LOOK_FOG=100000,200000`) because `haze_color()` is also the
geometry fog colour and would otherwise carry the 1513x into the terrain. Guard: **2.69**
levels. Over the 21,633 sky pixels frozen from the pre-fix frame:

| | pre-fix dome | fixed dome |
|---|---|---|
| mean sky RGB | `[253.6, 235.3, 208.7]` | `[250.0, 226.2, 196.9]` |
| pinned at R ≥ 250 | 100 % *(by construction — it is the mask)* | **88.1 %** |
| literally `255,255,255` | **3.818 %** | **0.000 %** |
| R travel across 63 rows of elevation | **0.3** levels | **16.0** levels |
| G / B travel, same rows | 8.0 / 20.4 | **28.4** / 23.6 |

Red moves 53x further across the same pixels, and the pure-white core is gone. That is the
fix, isolated.

**Still open, and honest about it:** 88 % of those pixels are *still* at R ≥ 250 and the sky
still means `[250, 226, 197]`. The 1513x was one bug, not the whole story — at `sky_gain`
2.4 the dome still sits at the top of the range in this framing, and the `edhari-load`
framing measured through the contaminated pair never moved at all (span 1.0 → 0.9, pinned
100 % → 100 %). Whether that is a second over-brightness or just a framing with almost no
visible sky is unmeasured; it needs a sky-dominant shot, not this one.

## Rules this leaves behind

1. **A comment that asserts engine behaviour is a claim, not a fact.** Read the branch, not
   just the line. This one had been sitting in the file since the dome was written, phrased
   confidently enough that nobody re-checked it.
2. **Compensation code is a smell that has to be paid for with a shader quote.** Any
   `1 / something_the_engine_does` needs the file:line where the engine does it, on the
   path your fragment actually takes.
3. **Unlit == ClearColor.** `Hour::sky_gain`'s own doc-comment already said `ClearColor`
   never passes through `Exposure`. The two facts were in the same file, contradicting each
   other, for weeks. When two comments in one module disagree, one of them is a bug.
4. **Measure the gradient, not the brightness.** "Is it blown out?" is answered by span,
   not by mean or by a max of 255.
5. **Freeze the pixel set across a before/after.** The mask that finds the blown sky
   (`R>=250 & B>=170`) selects nothing once the sky is fixed. Compute the mask on the BEFORE
   frame and apply it verbatim to the AFTER frame, plus an angle guard (mean |Δ| over the
   non-sky pixels) to prove the capture did not move. `_poppy_sky_after_plate.py` does both.
6. **A before/after binary pair has to straddle the fix and nothing else.** Two exes an hour
   apart carry every other commit that landed in between — here, a full block-palette
   repaint. Prefer reconstructing the old behaviour from the NEW binary through an env
   lever when the maths lets you (`SKYGAIN = old_gain × 1/exposure()`); it holds the map,
   the palette and the controller fixed by construction. And keep the guard: it is what
   caught this, one plate before it would have been published as proof.
7. **A dry-run must not be able to leave a wrong plate on disk.** The first pass rendered a
   sheet headed "SKY AFTER THE FIX" whose after column *was* the before frame — it read as
   "the fix moved nothing". `_poppy_sky_after_plate.py` now refuses and writes nothing when
   the two frames are byte-identical.

## Where else to check this — and one live flag

The sky dome (`look.rs:2009`, and Rose's A1-proof material at `1999`) was the only
*exposure-compensated* unlit material, so the
1513x bug is fixed and contained. But reading the branch turned up a second consequence of
the same shader fact, and it is not the sky's:

> **`unlit: true` also drops `emissive` entirely.**

Receipts, same version: `pbr.wgsl` contains the string `emissive` **zero** times. Every use
of it lives in `pbr_functions.wgsl` lines 341 / 830 / 837 / 840 — all inside
`apply_pbr_lighting` (which opens at line 336), i.e. all on the branch an unlit fragment
does not take. The only thing that runs afterwards is
`main_pass_post_lighting_processing` (line 994), and that is fog → tonemap → deband →
premultiply-alpha. Nothing adds emissive back.

**This was already half-known in the same file**, which is what makes it worth chasing
rather than dismissing. `vfx.rs:687-688`, on the hit-flash shell:

```rust
// NOT unlit: `unlit` short-circuits to base colour on some paths and
// the emissive tint (the part that blooms) can be dropped with it.
```

and again at `723-724` for the parry ring. Two materials were deliberately kept lit for
exactly this reason — hedged ("on some paths", "can be"), because nobody had read the
branch. The shader says it is not *some* paths: an unlit fragment never reaches a single
line that touches emissive.

Meanwhile `client/src/vfx.rs` has six materials that set `emissive` **and** `unlit: true`
(sparks ×3 at 458/459, 464/465, 470/471; ember 477/478; parry 501/502; trail 511/513 —
`coal` and `ash` are lit, so they are fine): e.g. `base_color: srgb(1.0, 0.97, 0.90)` with
`emissive: rgb(17.0, 15.5, 12.5)` and `unlit: true`. On this code path those fragments ship
at their base colour — an ordinary sub-1.0 near-white — and the 17x emissive that was
supposed to punch them through the bloom threshold never reaches the HDR target. The two
commented materials sit right beside them with the opposite choice.
**Not verified against a render, and not my lane** — flagged here for whoever owns vfx.
Cheap test: fire a burst with `VOXELFORGE_VFX_EV` swept; if sparks are genuinely emissive
they stay hot as exposure drops, and if they are just base colour they dim with everything
else. Fix, if confirmed, is to drop `unlit: true` on the emissive materials (emissive works
on the lit branch) rather than to raise the numbers.

Note also the nuance one line up, at 840: `emissive_light * mix(1.0, view.exposure,
emissive.a)` — on the **lit** branch, emissive alpha is what selects whether emissive is
exposure-scaled or written scene-referred. That is the knob the dome's author was reaching
for and could not reach from an unlit material.

The general rule for the rest of the repo: anything that sets `unlit: true` and then reasons
about brightness — a HUD-space world quad, a flat-shaded water plane, a debug gizmo mesh —
gets neither `ev100` nor `emissive`. Do not pre-brighten for an exposure that is not applied,
and do not expect an emissive that is not read.
