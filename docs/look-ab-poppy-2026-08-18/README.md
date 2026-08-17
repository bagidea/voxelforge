# Look A/B -- 2026-08-18 (poppy)

Before/after plates for the render path, measured on the pinned metric
definitions from `docs/art-gap-vs-reference-2026-08-17.md`. Every frame here
comes from a binary that was already on disk — the build lane was saturated
(3 `rustc`, ~6 GB) when the plates were shot.

> **Read `SKY-FINDINGS.md` before quoting this file on the sky.** The
> "Attribution" conclusion as first written named the wrong subsystem and is
> retracted there, with the one-binary control that replaces it.

**Build status (not "no queue"):** `cargo build --release --bin voxelforge -j 2
--target-dir target-poppy` (PID 4420, confirmed live via
`Get-CimInstance Win32_Process`) has been running since **01:53** and was still
alive at 02:45; `target-poppy\release\voxelforge.exe` is still the Aug-5 binary,
so it has not relinked yet. It is a **world** build
off `b26f970`, so when it lands it carries both `LookGen::V4` *and* the authored
`_n`/`_r` PBR loader — the two things this folder could not photograph. The
blocker is elapsed build time, not an unqueued build.

Re-runnable:

```
powershell -File scripts/_poppy_ab_20260818_shoot.ps1        # the 4 plates
powershell -File scripts/_poppy_ab_20260818_control.ps1      # LOOK_GEN v3/v4 control
powershell -File scripts/_poppy_sky_lever_control.ps1        # the sky control (3 arms)
python      scripts/_poppy_ab_20260818_measure.py REF=_kevin_ref_panel_mid.png ...
python      scripts/_poppy_ab_20260818_board.py              # board_before_after.png
```

## The exe pair is NOT the one the brief named

The brief asked for `target-poppy\release\voxelforge.exe` (Aug 5) as *before* and
`target-poppy\release\voxelforge_shot.exe` (01:53 today) as *after*. Both halves
are unusable for a like-for-like look A/B, and the binaries say so rather than me:

| exe | problem | evidence (string scan) |
|---|---|---|
| `voxelforge_shot.exe` (01:53) | the **isolated hero-shot bin**. `shot_main.rs` declares hero/characters/equipment/enemies/vfx/block_atlas and **not** `voxel.rs` or `look.rs`, so it cannot render the world at all | `VOXELFORGE_CINE` 0, `NOHUD` 0, `LOOK_SUN` 0 |
| `voxelforge.exe` (Aug 5) | predates the in-engine still rig, so the camera cannot be posed and the HUD cannot be removed -- "same angle" is unreachable | `VOXELFORGE_CINE` 0, `VOXELFORGE_NOHUD` 0 |

So the honest pair is the two **world** binaries that honour the same camera
contract (**camera only** — see "Env asymmetry" below, the lighting halves are
*not* symmetric), straddling the look-v4 render path:

| role | exe | mtime | md5 | `VOXELFORGE_LOOK_GEN` |
|---|---|---|---|---|
| BEFORE | `target-flamingo\release\voxelforge.exe` | 2026-08-08 16:25 | `4DED9219A24913A811B307265077B980` | **absent** |
| AFTER | `target\release\voxelforge.exe` | 2026-08-18 00:18 | `B417F9B4A50D826EB97B01C7588882E0` | **present** |

Both were **copied into `_poppy_ab_stage\` before the first frame** -- `target\` is
the shared default dir and a teammate was building into it, so shooting from it
directly risks the binary changing halfway through the pair. The straddle is
proven by scanning each staged copy for the lever the change introduced, not by
mtime or commit order.

### The 01:53 build contains none of the current material work

`VOXELFORGE_MAT_MAPS` -- the kill switch the authored `_n`/`_r` PBR loader
introduced (`voxel.rs:539`, commit `81f5d2a`) -- scans **0 in every binary on
disk**, including both 01:53 exes. That build produced `voxelforge_shot` and
`voxelforge_enemyshot`, and neither includes `voxel.rs`. **The authored-PBR
loader has never been compiled into a runnable world binary.** It needs a
`voxelforge` (world) build before it can be photographed at all — which is
exactly the build in flight since 01:53 (see "Build status" above), not a
missing queue entry.

## Env asymmetry — the two halves were NOT graded alike

`_poppy_ab_20260818_shoot.ps1` sets the same eight variables for both halves.
Only the camera ones mean the same thing on both sides. String scan of the two
staged exes (`[regex]::Matches` over the raw bytes, counts verbatim):

| string | before exe | after exe | so the shared env… |
|---|---|---|---|
| `VOXELFORGE_CINE` | 3 | 3 | **poses both cameras** — the "same angle" claim holds |
| `VOXELFORGE_NOHUD` | 1 | 1 | strips both HUDs |
| `VOXELFORGE_LOOK_QUALITY` / `_SUN` / `_LIGHT` / `_EXPOSURE` | 1 each | 1 each | is *read* by both, but into different stacks |
| `LOOK_IBL` | **0** | 2 | the IBL rig exists only after |
| `LOOK_FILL` | **0** | 2 | the fill rig exists only after |
| `LOOK tier` | **0** | 1 | before never prints a tier |
| `LOOK atmosphere` | **0** | 1 | before has no atmosphere at all |
| `ATLAS mode` / `BLOCK_ART` | **0** / **0** | 2 / 3 | before has no file-atlas path |

`_run_before_*.log` contains **zero** lines matching `LOOK`. So the before plate
is not "the same grade minus v4" — it is a materially older lighting stack that
happens to accept four of the same variable names. **The hue / cool % / luma
deltas in the table below are therefore upper bounds on any single change, and
attribute to nothing on their own.** That is what the one-binary controls
(`control-onebinary/`, `control-sky/`) exist for.

## Asset root: the plates were shot with a broken one

`main.rs:517` pins Bevy's asset root to `<exe dir>/assets`. `_poppy_ab_stage\`
held two `.exe` files and nothing else, so **every** `AssetServer` load 404'd on
all four plates — `grep -c 'Path not found'` gives 5 in each `_run_after_*` and
3 in each `_run_before_*`, all `audio/*.wav`.
What survived: the sky/IBL are built in code, and `block_atlas` resolves
`assets/textures/blocks` relative to the **CWD** (`block_atlas.rs:102`), which
was the repo root. So the visible pixels are unaffected this round — but the
plates were shot on a rig that was quietly failing, and that was not recorded.

Fixed in `scripts/_poppy_sky_lever_control.ps1`: it stages an `assets` junction
next to the exe before the first frame and asserts `Path not found` count == 0
per arm. `control-sky/_control.log` shows `asset 404s: 0` on all three arms.

## Plates

| | outdoor-noon | village-raking |
|---|---|---|
| before | `before_outdoor-noon.png` | `before_village-raking.png` |
| after | `after_outdoor-noon.png` | `after_village-raking.png` |

Board: `board_before_after.png` (captions are read from
`_poppy_ab_20260818_metrics.json`, the same source the pixels are pasted from, so
a caption cannot drift off its plate).

Pair gate (`scripts/_poppy_pair_metrics.py`) -- all four md5s distinct:

| scene | mean abs diff /255 | px changed |
|---|---|---|
| outdoor-noon | 47.66 | 100.00 % |
| village-raking | 66.88 | 99.99 % |

## Numbers

REF = `_kevin_ref_panel_mid.png`, the middle panel of the CEO reference. The
driver reproduces Kevin's published REF figures exactly (edge 60.54 vs his 60.5,
strong-edge 44.8 %, luma 148.6), which is what licenses comparing to it.

| metric | REF | before noon | after noon | before vill | after vill |
|---|---|---|---|---|---|
| **edge density** (Sobel mean) | **60.54** | 37.03 | 37.43 | 21.77 | 20.12 |
| **mean hue** (circular, deg) | **53.5** | 19.4 | 28.2 | 343.2 | 16.9 |
| **cool px %** (of all px) | **16.26** | 3.44 | **0.00** | 22.07 | **0.00** |
| hue concentration 0-1 | 0.560 | 0.725 | 0.963 | 0.465 | 0.982 |
| clipped px % | 24.85 | 45.21 | 13.22 | 37.51 | 14.83 |
| luma mean | 148.6 | 131.8 | 93.0 | 112.3 | 60.7 |

`cool px %` is quoted over **all** pixels. Over saturated pixels only the numbers
are the same to 2 dp here, because our plates measure 100 % saturated.

## What the plates actually show

**The win is real and visible: surface texture — but it is the PROCEDURAL
tiles, not the authored art.** The before plates are flat untextured blocks of
solid colour; the after plates carry per-block texture, relief and roughness
variation. That is the **procedural** tile generator in `voxel.rs` landing.

The authored PNG set is **rejected at runtime, in every single run in this
folder**. `_run_after_outdoor-noon.log:6` and all three sky arms print:

```
BLOCK_ART tile_px=64 != 16 — file set ignored, procedural tiles kept
```

`voxel.rs:82` hard-codes `TILE_PX = 16` and `voxel.rs:107-124` refuses any other
size rather than rescaling somebody's pixel art. The working-tree
`assets/textures/blocks/atlas.json` declares `"tile_px": 64` (it was `16` at
`HEAD`) and every PNG beside it is 64×64. **No pixel of the current authored
block set has ever reached a frame.** See "For the art lane" below.

**The sky is dark in the after plates — and it is the look lane's own grade, not
a dead mesh.** The first version of this section said "the sky dome is dead …
hand it to `scene.rs`". That is **retracted**; `SKY-FINDINGS.md` has the
three-arm one-binary control that replaces it. Corrected reading of the headline
numbers:

* `cool px % -> 0.00` — the blue sky *was* the cool pixel population, and in the
  after binary **every** sky path renders dark: atmosphere `24,14,10`, dome
  `25,14,11`, flat `ClearColor` `13,12,21`, against a before sky measuring
  `132,159,253` / luma **159.8** over the same 110-row band. Turning the
  atmosphere off does **not** bring the blue back.
* `luma mean` falls 131.8 -> 93.0 — **not** "a black upper third". The top 15 %
  band of `after_outdoor-noon.png` means `133, 79, 57` with only 2.5 % near-black
  pixels. At this camera the sky is slivers between blocks, not a band. The
  frame is darker *everywhere*.
* `edge density` barely moves (37.03 -> 37.43) despite the added texture. The
  dark-sky explanation given first is not supported — the sky is a few percent
  of the frame here. Unexplained; do not quote a cause for it.
* `hue concentration` 0.73 -> 0.96 — consistent with a frame that lost its only
  cool population to the warm haze.

So on the two colour numbers the brief asked for, the after plate moves **away**
from REF, and on this evidence that is a **grade/exposure** result owned by this
lane.

## Attribution: this is not the look generation

The exe pair straddles ten days of **every** lane's work, so it cannot attribute
anything by itself. `control-onebinary/` shoots the **same** after-binary twice at
the **same** camera, moving only `VOXELFORGE_LOOK_GEN`:

| metric | ctrl v3 | ctrl v4 |
|---|---|---|
| edge density | 37.62 | 37.78 |
| mean hue | 28.1 | 28.1 |
| cool px % | **0.00** | **0.00** |
| luma mean | 95.4 | 95.7 |

mean abs diff **1.10/255**, 16.2 % of pixels changed.

Two conclusions:

1. **The dark sky is not the v3->v4 step.** It is present in `v3` too, so it is
   inherited from the newer binary rather than introduced by the generation
   switch. That is *all* this lever can say — `LOOK_GEN` never touches the sky
   path, so it cannot name a subsystem. ~~Likely a mesh spawned without
   `Visibility`; hand to `scene.rs`/sky.~~ **Retracted** — see
   `SKY-FINDINGS.md`: the dome is off **by design** when the atmosphere is on
   (`look.rs:3474-3484`, and the run log prints `LOOK atmosphere spawned
   mode=lut`), and the `LOOK_ATMOS`/`LOOK_SKYGRAD` arms show the dome renders
   fine — dark, like everything else in the frame. It is a grade, and it is
   mine.
2. **v3 -> v4 is not measurably a look change at this camera.** The +0.16 edge
   delta sits *below* the run-to-run noise floor: the same nominal config shot in
   the main pass measured 37.43 and in the control 37.78, a 0.35 spread. A claim
   of "v4 improves detail" is not supported by these plates.

## For the art lane — the block set is being thrown away at startup

Not a look finding, but it falls out of these logs and nobody has it:

* every PNG in `assets/textures/blocks/` is **64×64** and the working-tree
  `atlas.json` says `"tile_px": 64`;
* `voxel.rs:82` is `const TILE_PX: usize = 16`, and `voxel.rs:107` only accepts a
  set whose `tile_px` equals it;
* so the loader prints `BLOCK_ART tile_px=64 != 16 — file set ignored,
  procedural tiles kept` and the world runs on generated tiles.

`atlas.json`'s own header comment ("`tile_px` may change too, as long as it is
even") is **wrong** as the code stands — the split-face bake and the LOD atlas
are both built around `TILE_PX`. Two ways out, both someone else's call: author
at 16, or change `TILE_PX` (a rebuild, and the LOD packing has to be re-checked).
Until one of them happens, no authored block pixel reaches a frame.

## Honest gaps

* **Nothing here photographs the authored `_n`/`_r` PBR work** -- it is not in any
  binary on disk (see above). The material lane's headline change is still
  unshot. The `target-poppy` world build in flight since 01:53 is the one that
  will carry it; it had not finished when these plates were measured. Note the
  `tile_px` mismatch above will still gag the *colour* set even after it lands.
* The after binary is `target\release\voxelforge.exe`, which is **not a poppy
  build** -- it is whatever the shared dir held at 00:18. It is used because it is
  the only world exe carrying `LOOK_GEN`.
* Both scenes are the same map and hour family; two cameras is not a look survey.
* REF is 768x455 and our plates are 1280x720. Sobel mean is per-pixel so it is
  comparable, but a px-denominated threshold would not be.
