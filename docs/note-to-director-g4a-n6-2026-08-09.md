# G4a on the N6 shotset — the numbers, and the two premises that did not survive them

2026-08-09, Poppy. Measurement only: python over saved PNGs plus eleven shots off ONE
already-linked exe (no `cargo`, no source edit — `client/src/look.rs` is someone else's
open edit and a build was running at 02:07).

---

## 0. Two corrections before the table, because they change what the table means

**(a) The `--albedo-check` measurer is `scripts/cast_shadow_penumbra.py`, not
`measure_penumbra.py`.** d6418aa added a new file; it did not add a flag to the old one.
The old `measure_penumbra.py` still exists and still reports what d6418aa proved it
reports — the strongest luminance step per floor row, which on these plates is a block-face
seam or a grass/terracotta material boundary. Both columns are in §1 so the gap is visible.

**(b) N6 does NOT contain PCSS 16.0.** Three independent checks, no clocks involved:

1. `_pixel_shotset_N6/manifest.json` stamps its own exe as `"commit": "d6418aaab"` — the
   commit *before* 40cf97e — and `newest client/src: look.rs @ 08-08 15:06`.
2. `_pixel_shotset_N6/vfprobe.exe` (sha `2CB1D1E4…`) differs from N5's (sha `EE7A42D3…`)
   in **30 bytes of 80,680,960** — PE TimeDateStamp, debug directory, PDB GUID. Zero code
   or rodata bytes. It differs from today's `target-flamingo/release/voxelforge.exe`
   (sha `B024BBC2…`, 16:25) in **24 bytes**, same story. N5, N6 and the current release exe
   are one program relinked three times. A `PCSS_WIDTH` 4.0 → 16.0 edit cannot be in a
   binary whose code section is byte-identical to one built before the edit existed.
3. The frame says it too. `grade-vista` is the only Ultra plate — the only one PCSS reaches
   at all — shot 7× off that one exe, only `VOXELFORGE_LOOK_PCSS` moving:

   | pair | mean \|ΔL\| | %px >1 L | %px >4 L |
   |---|---|---|---|
   | default vs default (repeat noise, 3 shots) | 0.16–0.26 | 2.0–3.1 % | 0.6–1.1 % |
   | **default vs `=off`** | **0.10–0.23** | **1.1–2.5 %** | 0.4–0.9 % |
   | default vs `=4` | 0.33–0.43 | 6.8–8.2 % | 1.7–2.3 % |
   | `=4` vs `=off` | 0.32 | 6.6 % | 1.7 % |
   | **default vs `=16`** | **2.50** | **45.4 %** | 24.6 % |

   The shipped default is **inside its own repeat-shot noise floor of PCSS switched off**,
   and 16.0 moves 45 % of the frame. Whatever constant that binary compiled, the N6 plates
   were shot with no measurable penumbra from PCSS. (default ≠ `=4` at ~3× the noise, so
   the compiled width is not exactly 4.0 either; I am not guessing which small value it is
   — the load-bearing fact is default ≡ off ≠ 16.)

So N6 is the light-ratio fix (8303db5) **only**. PCSS 16.0 is still unshot. 40cf97e's
"all 8 canonical plates re-shot from the resulting exe, sha256 2CB1D1E4…" names N6's sha
correctly and the binary behind it is pre-16.0.

---

## 1. The N6 table — all 8 plates

`python scripts/cast_shadow_penumbra.py _pixel_shotset_N6/after/<plate>-nohud2.png
 --albedo-check _pixel_shotset_N6/before/<plate>-nohud2.png`
Full console output: `docs/assets/g4a-n6-albedo-check.txt`.

> **That command no longer runs (2026-08-09, §3.1 shipped).** The last column is the
> confounded one this note is about, and the tool now REFUSES a same-azimuth control
> instead of printing it. The console text is kept as the record of what was measured;
> the edge-width columns beside it are unaffected — they never used that control.

| plate | old `measure_penumbra.py` | control (sky silhouette, 0 penumbra) | all in-scene edges | ratio | albedo-check |
|---|---|---|---|---|---|
| gate3-boot   | 8.36 px | 1.82 px (n=108) | 2.14 px (n=141) | 1.18× | **93.3 %** (base 56.5) — flagged |
| gate3-combat | 5.36 px | n=0 — no sky in frame | 2.17 px (n=117) | — | grass reads ONE hump — untestable |
| gate3-walk   | 10.36 px | 1.90 px (n=85) | 2.79 px (n=151) | 1.47× | 21.1 % (base 36.6) — see §2 |
| grade-vista  | 9.36 px | n=0 — no sky in frame | 2.28 px (n=148) | — | **90.6 %** (base 54.1) — flagged |
| s1-vista     | 6.79 px | 2.24 px (n=94) | 2.41 px (n=152) | 1.08× | **91.5 %** (base 67.5) — flagged |
| hero         | 7.07 px | 1.90 px (n=35) | 2.08 px (n=90) | 1.09× | grass reads ONE hump — untestable |
| s4-raking    | 6.86 px | 2.05 px (n=385) | 2.21 px (n=66) | 1.08× | grass reads ONE hump — untestable |
| s3-clash     | 3.86 px | n=0 — no sky in frame | 2.05 px (n=160) | — | grass reads ONE hump — untestable |

**No PASS is reported on any plate.** Three plates are still flagged by `--albedo-check`
at 90.6–93.3 %, and the widest orientation bucket anywhere (gate3-walk, 15–30° off-axis,
5.66 px) sits on a frame whose own control bucket is 2.94 px = 1.93×, n=9 — one bucket,
nine control edges, on the only plate with a mid-stride actor in it. That is not a gate.

The old tool's 3.86–10.36 px column is what d6418aa said it was: it is measuring block-face
seams and material boundaries against no control at all. Every honest edge in these frames
is 2.0–2.8 px against a 1.8–2.2 px AA/TAA floor.

---

## 2. "The grass has no cast shadow" — the premise is false, and here is what broke

You asked me to chase caster/receiver flags, normal/depth bias, and cascade coverage. None
of them is the answer, and two of them are ruled out on sight:

* `NotShadowCaster` / `NotShadowReceiver` appear **nowhere** in `client/src` (grep). The
  grass ground and the terracotta wall are faces of the same greedy-meshed chunk mesh with
  the same material, so no flag can separate them by construction.
* the bias fields are never touched on the playable light — `apply_look_to_sun` sets
  direction, illuminance, colour, `shadow_maps_enabled`, contact and PCSS, and leaves
  `shadow_depth_bias` / `shadow_normal_bias` at Bevy's defaults (0.02 / 1.8). At 4096 the
  far cascade's texel is ~0.07 blocks, so the normal offset moves a horizontal receiver's
  shadow boundary by `1.8 × texel / tan 22°` ≈ 0.33 blocks. It cannot erase a 25-block
  shadow.

**The grass has a cast shadow. It always did. The control plate `--albedo-check` uses
contains the same shadow.**

`--albedo-check` asks: were these "in shade now" pixels already the dark half *before the
light changed*? Its before-plate is the same camera at **the same azimuth 205°**, 17° instead
of 22°. 8303db5 changed the sunlit-to-shadowed RATIO (2.46 → ~8.5), not the presence of a
shadow — `shadow_maps_enabled` was already true. So the same walls throw the same shadows
onto the same grass in both frames, and a real cast shadow that barely moved scores as
"painted in the albedo".

The control the albedo cannot follow is a **sun azimuth move**: painted texture is bolted to
the blocks; a cast shadow must move. Eleven shots off one binary (sha `B024BBC2…`), only
`VOXELFORGE_LOOK_SUN` moving, `s1-vista`. High-pass the frames (box radius 24 px, which
removes the smooth volumetric in-scatter term that also follows the sun) and correlate the
sharp structure over the shared grass mask —
`python scripts/sun_locked_edges.py A.png B.png --corr`:

| B against A = shipped 22°/205°/22k | grass px | r |
|---|---|---|
| repeat shot, identical env | 310,673 | **0.994** |
| contact shadows OFF (`VOXELFORGE_LOOK_CONTACT=off`) | 312,674 | **0.996** |
| the N6 before-plate — 17°, **same azimuth** (what `--albedo-check` uses) | 125,682 | **0.761** |
| azimuth 205° → 115° | 35,802 | 0.173 |
| azimuth 205° → **25°** (+180°) | 275,286 | **0.010** |

(The 115° row's mask is small because `grass_select` is a hue/sat rule and most of that
frame's grass falls out of the green window once it is in shade — which is itself a symptom,
not a counter-argument. The +180° row keeps 275k px and is the load-bearing one.)

Painted albedo scores r ≈ 1 under any sun. The sharp structure on the grass scores r = 0.01
when the sun swings 180°: it is **completely uncorrelated**, i.e. it is not in the texture.
And it survives `VOXELFORGE_LOOK_CONTACT=off` untouched (r = 0.996, mean |ΔL| 0.149 over
grass, under the 0.23 repeat-noise floor), so it is the **shadow map's cast shadow**, not the
contact-shadow layer. `docs/assets/sun-azimuth-grass-s1-vista.png` is the same 700×260 grass
crop at 205° / 25° / 115°: the long straight shadow bands the ruin walls throw across the
field in the shipped frame are simply gone when the sun moves.

`gate3-walk` reading 21.1 % ("not albedo-locked") was never the outlier — it is the one plate
whose actor moved between the two shots, which decorrelated its dark blobs by accident and
so gave the right answer for the wrong reason.

**Root cause, one line:** `cast_shadow_penumbra.py --albedo-check` uses a control plate that
shares the sun's azimuth with the plate under test, so it cannot distinguish "no cast shadow"
from "the same cast shadow, dimmer" — and 8303db5, which changed only the key/fill ratio, is
exactly the second case. The 89–94 % overlap is the measurement's blind spot, not the
renderer's.

### What is actually true about the grass shadow

Sites picked by the move, not by a human and not by a hue rule
(`sun_locked_edges.py A.png B.png`, s1-vista, grass mask, |ΔL| > 10 L between azimuths):

* **11.43 %** of the frame's pixels are sun-locked grass (149,987 px).
* Their edges: median **2.95 px** (n=72) against the same frame's sky-silhouette control
  **2.22 px** (n=90) = **1.33×**; the 0–15° bucket 3.56 px = 1.63×, 30–46° 3.60 px = 1.58×.

So the grass shadow is **present and hard** — near the AA/TAA floor, which is exactly what
§0(b) predicts for a frame with no working PCSS. The G4a question was never "is there a
shadow"; it is "is its edge soft", and the honest answer on N6 is no, at 1.33×.

---

## 3. Not fixed, as asked. Ranked for your approval

1. ~~**Give `--albedo-check` a control that cannot be confounded**~~ — **DONE 2026-08-09,
   approved.** The flag now takes an azimuth-moved plate and shares one implementation of
   the correlation with `sun_locked_edges.py` (`corr_stats`). Both plates' suns are read
   from the shoot script's own `manifest.json` — provenance, not a promise — and the check
   **refuses, printing no verdict**, when the control is within 90° of the plate's azimuth,
   when nothing on disk establishes either sun, or when the control keeps under half the
   plate's grass. On `s1-vista` the overlap statistic that read 91.5 % against the pre-light
   plate reads **58.5 % against a base rate of 58.5 %** against the +180° plate — chance.
   `scripts/tests/test_albedo_check_control.py` locks the four constants and all four
   verdicts (CAST / PAINTED / REFUSED / UNRELIABLE) through both the API and the CLI,
   including the retracted invocation itself. The "no cast shadow" claim is retracted in
   place in `docs/note-to-director-N6-regrade-2026-08-08.md`.
   The pre-light plate keeps its use as an exposure reference; it just cannot answer
   "cast or painted" while it shares an azimuth.
2. **Shoot PCSS 16.0 for real.** It is a one-line constant that no shipped plate has ever
   carried, and the only plate it reaches is `grade-vista` (Ultra). The forced `=16` frame
   already on disk moves 45 % of pixels vs the shipped default, so it is not a subtle change
   and it needs its own gate pass before it lands.
3. **`grade-vista` at Ultra renders as if PCSS were off** — worth one look at whether
   `pcss_width(tier_on)` is reaching the light at all on that path, since the tier is set and
   the env override at the same nominal width does move the frame.

## Files

* `scripts/sun_locked_edges.py` — new. The azimuth-move control, and `--corr`.
* `docs/assets/g4a-n6-albedo-check.txt` — the full 8-plate G4a sweep.
* `docs/assets/sun-azimuth-grass-s1-vista.png` — the grass crop at 3 azimuths.
* Probe plates live in `_poppy_pcss_probe/` (not committed): `dflt1-3`, `w4`, `w4b`, `w16`,
  `poff`, `az205`, `az025`, `az115`, `az205_nocontact`.
