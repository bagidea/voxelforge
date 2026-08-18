# MAT_MAPS, finished: every scene's own floor, and which map costs what

Poppy · 2026-08-18 · companion to `docs/flamingo-matmaps-judge-2026-08-18.md`

That verdict was written with one null pair, on `day`. Two of its six pairs
could not be judged at all (exit 2, no floor of their own) and the one PASS it
did report leaned on day's floor across a scene boundary — which the judge's own
BORROWED FLOOR banner says is not a result. This closes both holes and then
splits the finding in two, because "the authored maps" was never one lever.

Nothing in the judge changed. `scripts/_fl_matmaps_judge.py` is called here, not
edited; every number below is its `measure()` on plates named in the tables.

---

## 1. The binary is the same one, on purpose

    target/release/voxelforge.exe   2026-08-18 08:01:34   82093056 B
    sha256 c46a701597b869fafa617e41d40c0a32...

A noise floor is a property of one build. `scripts/_poppy_matmaps_shoot2.sh`
checks that sha256 and refuses on a mismatch, instead of the usual "exe newer
than source" gate — `client/src/voxel.rs` was rewritten after these plates were
shot (commit `55e08e3`, the TILE_PX work, which builds into `target-poppy/`),
and a newer binary would make the floor measure the rebuild.

Each null pair is two runs shot back to back under the plate's own env,
`<scene>_onNULLA` then `<scene>_onNULLB`. Neither is a graded plate, so the
floor is never the thing it is flooring.

## 2. What a floor of its own does to the verdict

| atlas16 arm | old (day's floor, borrowed) | new (own floor) |
|---|---|---|
| day | *refused — that WAS the floor* | **FAIL** M1, M4 |
| evening-raking | exit 2, refused | **FAIL** M1, M4 |
| night-firelit | FAIL M4 only; **M1 PASS**, M2 PASS | **FAIL** M1, M2, M4 |

night-firelit's M1 was the one thing on the board that looked like the maps
delivering relief. It does not survive its own floor:

    borrowed (day)      paired median +0.0007  vs 3x floor 0.0006   PASS
    own (night null)    paired median +0.0007  vs 3x floor 0.0012   FAIL

Same signal, doubled floor. night is a darker frame with a live fire in it, so
it breathes harder between two runs than a noon frame does — which is exactly
why a floor does not travel between scenes. Flamingo called this one as too
close to call; it is now called.

M2 also flips on night (chroma_std +0.00045 against a bar of 0.00012). That is
not a broken pair — see §4.

Full detail, all three scenes: `_poppy_matmaps/verdict2/atlas16_*/matmaps-verdict.json`.

## 3. Which map ate the value span

M4 (`lum_spread`, the P6 value-span axis) failed everywhere with both maps live.
So each was run alone, out of an atlas dir carrying only that suffix
(`scripts/_poppy_matmaps_split.py` — identical albedo, identical manifest, the
only difference is which file the loader can find), against its own `off` plate
and its own null pair:

| night-firelit | M4 lum_spread | ×own floor | share of the both-on loss | M3 micro_rms | A6 spec_cov | M2 chroma_std |
|---|---|---|---|---|---|---|
| `_n` only (authored normals) | **−0.996** | 111.9× | 38.8% | **+3.655** | **+2.281** | +0.00035 |
| `_r` only (authored roughness) | **−2.276** | 10.1× | **88.6%** | −0.123 | −0.188 | +0.00000 |
| both (the shipped drop) | −2.570 | 5.9× | 100% | +3.607 | +2.229 | +0.00045 |

Read across, not down:

* **The roughness maps cost 88.6% of the value span and buy nothing measurable
  in this frame.** `_r` alone is the only arm that FAILS M3 — it *softens* the
  picture (micro_rms −0.123 against a 0.025 floor) — and it moves highlight
  coverage the wrong way (−0.188).
* **The normal maps are where the picture comes from.** `_n` alone carries the
  entire local-contrast gain (+3.655 of the +3.607 both-on delta) and the entire
  specular-coverage gain (+2.281 of +2.229). It costs value span too, and 112×
  its own (very quiet) floor, but 0.996 against roughness's 2.276.
* The two are **sub-additive**: −0.996 + −2.276 = −3.272, but together they
  measure −2.570. They are competing for the same shadowed texels, not stacking.

Sheet, real pixels, captions read out of the same `measure()`:
`docs/assets/look/matmaps-channels-night-firelit.png`.

## 4. Two things the numbers say that the metrics cannot

Both channels trip a different documented assumption, and neither is a bug in
the plates:

* **`_r` alone PASSES M1** (paired median +0.0006, 85.6% of tiles rose).
  Roughness is not relief. This is control C5's blind spot arriving in the wild:
  `shade_resid` is a variance statistic and an *achromatic* change — which a
  specular response is — is indistinguishable from shading in one frame. M1 on
  the `_r` arm should be read as "something achromatic moved", not as relief.
* **`_n` alone FAILS M2** (chroma_std +0.00035 against a 0.00012 bar). The
  albedo tiles are byte-identical across that pair, so nothing about the albedo
  moved. A normal map tilts faces, and this scene is lit by *coloured* fire, so
  a tilted face catches a different mix — the chroma moves for a physical
  reason. M2 is a good guard against an albedo swap smuggled in beside the
  lever; on a coloured-light scene with normals it is over-tight, and its FAIL
  here means "shade_resid is not cleanly attributable", not "the pair is
  contaminated".

One honest caveat on the row: the three `off` plates are not the same file.
MAT_MAPS=off reads no authored map, so the atlas dir should not matter, and it
nearly does not — but they drift lum_spread +0.146 (`_n` arm) and −0.366 (`_r`
arm) against atlas16's off. That is 6–15× smaller than the −2.276 it is being
asked to resolve, and every delta in §3 is measured against that arm's OWN off,
so the drift does not enter it. It does mean the three baselines are not
interchangeable below ~0.4.

## 5. The standing answer

`VOXELFORGE_MAT_MAPS=on` is still not ready to be a default, and now the fix has
an address: **the roughness maps, not the normals.** The `_n` set earns its
place (all of the micro-contrast, all of the highlight coverage); the `_r` set
pays 88.6% of the cost, softens the frame and returns nothing this judge can
find. The next move is a roughness re-author or a per-material roughness ceiling,
tested with exactly this rig — not a decision about "the authored maps".

## 6. Reproduce

    python scripts/_poppy_matmaps_prep.py                     # atlas16
    python scripts/_poppy_matmaps_split.py                    # atlas16_{nonly,ronly}
    scripts/_poppy_matmaps_shoot2.sh target/release/voxelforge.exe   # passes A, B, C

    P=docs/assets/look
    for s in day evening-raking night-firelit; do
      python scripts/_fl_matmaps_judge.py judge $P/matmaps_${s}_off.png $P/matmaps_${s}_on.png \
        --null $P/matmaps_${s}_onNULLA.png $P/matmaps_${s}_onNULLB.png \
        --out _poppy_matmaps/verdict2/atlas16_$s
    done
    for v in nonly ronly; do
      python scripts/_fl_matmaps_judge.py judge \
        $P/matmaps_night-firelit_${v}_off.png $P/matmaps_night-firelit_${v}_on.png \
        --null $P/matmaps_night-firelit_${v}_onNULLA.png $P/matmaps_night-firelit_${v}_onNULLB.png \
        --out _poppy_matmaps/verdict2/$v
    done

    python scripts/_poppy_matmaps_channel_sheet.py

`$?` from the judge, not a `tail` of it: 0 = PASS, 1 = measured and FAILED,
2 = refused. A pipe would hand back the pager's exit code instead.

## 7. Still open, and not mine to close

The TILE_PX gate is gone (`55e08e3`), so a 64 px manifest now reaches the
loader — verified on the default dir, no ATLAS_DIR override:

    BLOCK_ART file-backed dir=assets/textures/blocks tiles=19 kinds=19 tile_px=64
    BLOCK_ART LOD atlas 3456x64 (864 KiB) cell=64px
    BLOCK_PBR authored ...brick_n.png 64x64   (and 71 more)

But **the 64 px art itself is still untracked**: `assets/textures/blocks/*.png`
is modified and every `*_n.png` / `*_r.png` is `??` in Monanisa's lane. HEAD is
self-consistent at 16 px (manifest 16, tiles 16²), so the gate change is a safe
no-op on a clean clone — it just means HEAD is one commit *in that lane* away
from actually shipping the drop. The `atlas16` arm above exists precisely
because that art could not be assumed.
