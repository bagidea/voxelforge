//! The look lane's post stack, as a plugin the PLAYABLE game can wear.
//!
//! Everything in here already existed — it was welded into `hero.rs`'s camera
//! bundle, which is a still-frame beauty-shot scene that spawns its own camera,
//! its own sun and its own 16×16 room. That made the look un-shippable: the
//! thing the reviewer signs off on lived in a binary nobody plays. This module
//! lifts the same stack out so `main.rs` can wear it over the real game.
//!
//! WHY IT INSERTS INSTEAD OF SPAWNING. `hero.rs` can spawn a camera because it
//! owns its scene. Here the game already spawned one (`main.rs`, orbit boom,
//! third-person) and it carries gameplay state — `OrbitCam`, its own
//! `AmbientLight`. So this plugin finds the existing camera and *adds* the post
//! components to it. Nothing gameplay-owned (the boom, the collision pull-in,
//! the HUD) is overwritten; this module only ever adds rendering components and
//! sets the lighting/atmosphere values that are the look lane's own call.
//!
//! WHY IT INSERTS IN `Update`, NOT `Startup`. System order against another
//! lane's `Startup` is not a contract. The camera is spawned in main.rs's setup,
//! and a `Startup` system here would race it — sometimes the query is empty and
//! the whole look silently no-ops. Running in `Update` finds the camera the frame
//! after it spawns, and a camera respawned mid-session (map reload) gets the
//! stack too instead of coming back bare.
//!
//! WHY `main.rs` ALSO SPAWNS WITH [`base_camera_look`]. `Update` is one frame
//! late, and the camera-spawn site itself should say what the camera looks like
//! rather than leaving a bare `Camera3d` and trusting a plugin elsewhere to
//! dress it. The camera is therefore *born* wearing the tier-independent half of
//! the stack, and the plugin owns tiering and runtime swaps on top. Both call the
//! same functions in this file, so there is one definition of the look, not two.
//!
//! QUALITY TIERS. The stack is built per [`LookQuality`] (a resource, default
//! [`LookQuality::High`]) so Steam isn't an Ultra-or-nothing proposition: a
//! mid-range card holds the High frame, Low is the cheapest frame that still
//! reads "Voxelforge". The tier is live — F7 cycles it and the settings menu can
//! mutate the resource; [`apply_look_to_cameras`] / [`apply_look_to_sun`] see the
//! change next frame and rebuild the stack on the *existing* camera and sun in
//! place (strip + re-insert), with no restart. Poppy's per-effect ms budget, when
//! it lands, retunes the groupings in [`insert_stack`] — that is the one place
//! the tier→effect map lives.
//!
//! THE PUBLIC CONTRACT is `docs/look-contract.md`; the numbers it quotes are the
//! `pub const`s below, and other lanes must `use` them rather than retype them.

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{
    atmosphere::ScatteringMedium, Atmosphere, CascadeShadowConfigBuilder,
    DirectionalLightShadowMap, FogVolume, NotShadowCaster, ShadowFilteringMethod, VolumetricFog,
    VolumetricLight,
};
use bevy::pbr::{
    AtmosphereMode, AtmosphereSettings, ContactShadows, DistanceFog, FogFalloff,
    ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
// `Meshable` brings `.mesh()` onto the `Sphere` shape — used by the sky dome.
use bevy::mesh::Meshable;
use bevy::render::view::Msaa;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// PUBLIC CONTRACT — the numbers other lanes are allowed to depend on.
// ---------------------------------------------------------------------------
// `docs/look-contract.md` is the human-readable copy of these; THIS is the
// source. Anything that needs them (Kevin's chunk streaming) must `use` them
// rather than retype the literal, so moving the haze moves the streaming with it
// and there is never a second set of truth to forget about.

/// Distance (blocks) at which the LEGACY linear haze starts to bite.
///
/// Kept as the fallback curve (`VOXELFORGE_LOOK_HAZE=0` selects it) and as the
/// number the streaming contract was originally written against. It is 112 in a
/// set ~55 blocks deep, i.e. it renders every pixel of the playable frame with
/// exactly zero fog — which is what makes it the honest ZERO-HAZE baseline for
/// the A/B axes, and what made it useless as a default. The shipped curve is
/// [`HAZE_START`]/[`HAZE_FULL`].
pub const FOG_START: f32 = 112.0;

/// Distance (blocks) inside which the haze is EXACTLY ZERO — the dead zone in
/// front of the lens, and the shipped curve's `FogFalloff::Linear` start (G7b).
///
/// WHY THERE HAS TO BE A DEAD ZONE AT ALL. The G7 default,
/// `ExponentialSquared` @ [`HAZE_DENSITY`], was measured — not assumed — with
/// the binary that shipped it (`scripts/_flamingo_g7_alpha.py`, whose estimator
/// is self-tested by holding each rung of its own calibration ladder out and
/// reading it back). The result cleared the curve of the charge against it: at
/// the pinned framing's foreground band, measured depth 16–20 blocks, the shader
/// applied **1.15 % / 1.59 %** against a predicted 1.32 % / 2.05 %. There was
/// never an extra near-field term; the density curve was being honoured.
///
/// The wash came from the other factor. Haze delta ≈ alpha × |haze − surface|,
/// and near-field ground here is shadowed grass sitting under a haze colour
/// several times its own radiance, so 1.6 % of alpha is still ~19 display units
/// — while the far band, already close to the haze colour and deep into the
/// tonemap's compressive region, turns 14–25 % of alpha into only ~44. Hence the
/// measurement that decided this: across all 16 sweep rows the depth ratio sits
/// at **2.21–2.42** whatever the density (0.0068→0.0120 moves it 2.33→2.40) and
/// whatever the haze colour. It is not a tunable of that curve, and
/// `ExponentialSquared` has no offset parameter — so the near field can only be
/// bought back by a falloff that starts somewhere, which in Bevy means `Linear`.
///
/// WHY 20. Bounded below by the camera: `main::BOOM_DIST` is 6.5, so 20 is
/// three boom-lengths — the player, the block they are standing on, everything
/// in reach and the whole melee volume render with no haze on them at any orbit
/// pose. Bounded above by the set: the campsite is ~55 blocks deep and the ramp
/// needs most of it to work in. Measured against the pinned framing, whose
/// foreground band brackets at p05 16 / p50 20 / p95 25 blocks
/// (`scripts/_flamingo_g7_probe.ps1` steps the fog through `FOG=S,S+0.5`, which
/// makes alpha a hard step at S and turns the frame into a depth ruler), 20
/// leaves half that band at exactly zero and the rest under 2.1 %.
///
/// RE-MEASURED AND HELD, 2026-08-14. The set the second paragraph's "~55 blocks
/// deep" describes is gone — the depth ruler now reads p99 **47.7** on the
/// deepest play framing and nothing above the capture noise floor past ~56
/// (`scripts/_flamingo_depth_ruler.sh`; the table is in [`HAZE_FULL`], which is
/// what moved). This const does NOT move with it, and the reason is that its
/// bound was never the set: `combat::LOCK_RANGE` is 16 blocks from the avatar
/// and [`crate::BOOM_DIST`] puts the camera 6.5 further back, so a locked-on
/// target at maximum range is 22.5 from the lens. 20 keeps the whole fightable
/// volume out of the haze at any orbit pose, and it already sits inside the
/// measured range — on the `0,1,18` framing it leaves **31.9 %** of the frame
/// receiving haze, which is a third of the picture doing aerial perspective
/// rather than the 21 %-of-one-ramp the old [`HAZE_FULL`] allowed it.
pub const HAZE_START: f32 = 20.0;

/// Distance (blocks) at which the shipped haze is FULLY opaque — the `Linear`
/// end.
///
/// TWO CONSTRAINTS, AND THE FIT IS WHERE THEY MEET.
///
/// 1. Keep the look that was signed off. The whole point of moving to a ramp is
///    the near field; the mid and far bands were reviewed and approved as they
///    are. So this end is not chosen, it is FITTED: least-squares `Linear{20,
///    end}` against the `ExponentialSquared` curve the look was approved on,
///    over the framing's measured depth span (26–78 blocks).
/// 2. Actually close. `ExponentialSquared` only ASYMPTOTES — it is 99.51 % at
///    [`RENDER_RADIUS`], so half a percent of a popping chunk shows through
///    forever. A ramp reaches 1.0 and stays there, burying the rest of the
///    streaming radius outright. The streaming contract is met with margin
///    instead of in the limit, and `HAZE_FULL <= RENDER_RADIUS` is the invariant
///    to keep (asserted below).
///
/// 250 → 150, 2026-08-06. THE TARGET OF THE FIT MOVED; THE METHOD DID NOT.
/// 250 fitted `ExpSq(0.0072)`. Commit bd3cde5 verified the vista grade at haze
/// density **0.0100** — and it did so through `VOXELFORGE_LOOK_HAZE=0.0100`,
/// which per [`haze_falloff`] does not bump a density on this ramp, it SWAPS THE
/// FALLOFF back to `ExponentialSquared`. So the air that carries ~25 points of
/// the graded warmth was never on the shipped path at all: measured on one
/// binary, the baked default came out warmth **99.20** against **124.65** for
/// the same binary under the env row. [`HAZE_DENSITY`] was a dead constant and
/// baking it changed nothing.
///
/// Re-running the SAME fit against `ExpSq(0.0100)` puts the optimum at **150**,
/// rms **0.79 pp**, max deviation **1.92 pp** across the set
/// (`scripts/_flamingo_haze_refit.py`) — the same fit quality that justified 250
/// against the old curve. Measured on the vista framing by the BAKED DEFAULT —
/// `scripts/verify_baked_grade.sh`, which builds and then shoots with no look
/// override of any kind, because an env row is what caused this bug once
/// already. It sets exactly two `VOXELFORGE_LOOK_*` vars, `_CAM` (boom pose) and
/// `_QUALITY=ultra` (effect tier), which is the framing every row in this
/// investigation was shot on; not one grade, light, haze or colour value is
/// overridden. So measured — `grade_axes.py` gives warmth **112.96** / blue **6.43** / sat
/// **96.24** / micro 7.87 / p95 160.69, and `colour_gate.py` PASSes all four
/// gates at 0.15 % magenta. The env row it replaces (`FOG=20,150`) read 112.97 /
/// 6.43 / 96.24: the default now reproduces the verified row to 0.01, which is
/// the whole claim of this change.
///
/// THE SIXTH AXIS, DOF fg:bg, FAILS AT 2.22 AND IS NOT THIS LANE'S. It is
/// intrinsic to the framing, not to the air: the baseline the CEO approved
/// measures 0.17 against the same target of 3.0
/// (`docs/look-acceptance-rubric.md`, and `docs/look-audit-2026-08-05-flamingo.md`
/// §5 — "แก้จากงานสีไม่ได้"). Haze moved it the right way for free, 1.74 -> 2.22;
/// closing it needs a real focus separation and is a handoff, so it is reported
/// here rather than hidden by quoting only the five axes that pass.
///
/// AND IT KEEPS THE THING THE RAMP EXISTS FOR, which shipping the density would
/// have thrown away. `Linear{20, end}` is 0.00 % opaque at 16 and 20 blocks for
/// EVERY end, so the dead zone f8a1a8f measured the near-field wash out of is
/// unconditional; `ExpSq(0.0100)` applies 2.53 % at 16 blocks and has no offset
/// parameter to fix it. `grade_g7.py` A2, same off-frame for all three:
///
/// | shipped curve            | far/near ratio | need | verdict |
/// |---|---|---|---|
/// | `Linear{20,250}` (was)   | 94.45 | >= 2.5 | PASS |
/// | `Linear{20,150}` (this)  | 66.15 | >= 2.5 | PASS |
/// | `ExpSq(0.0100)` (naive bake) | **1.15** | >= 2.5 | **FAIL** |
///
/// Those three rows are one method — same off-frame, all three driven the same
/// way — so they are comparable to each other. The BAKED DEFAULT re-measures A2
/// at **46.19**, same PASS, and the number to read there is the far-band delta
/// (35.14 vs the row's 34.99, i.e. the same air): the ratio's denominator is a
/// near band of well under one level, so it swings on rounding while the far
/// band does not. Nothing in this axis is close to the 2.5 it must clear.
///
/// NOT 160, WHICH ALSO CLEARS. On the same sweep rows, 160 lands warmth 110.94
/// against a target of 110 — 0.94 of margin on an axis that moves ~3 points per
/// 10 blocks of end, i.e. a number that passes today and fails on the next scene
/// edit. 150 clears by **2.96 baked** AND is the fit; 140 is warmer still
/// (115.82) but drifts to rms 2.50 pp off the approved curve, which is spending
/// the signed-off look on margin.
///
/// Note for the streaming lane: nothing between `HAZE_FULL` and
/// [`RENDER_RADIUS`] is visible any more, so that shell — now **248 blocks**,
/// grown from 70 and then from 170 by the two changes below — is pure draw cost.
/// Tightening `RENDER_RADIUS` toward `HAZE_FULL` is available and is Kevin's
/// call, not this lane's, which is why this const does not make it. It is worth
/// a great deal more now than it was.
///
/// ─────────────────────────────────────────────────────────────────────────
/// 150 → 72, 2026-08-14. THE SET MOVED OUT FROM UNDER THE FIT.
///
/// Every number above was fitted on the framing's "measured depth span (26–78
/// blocks)". That span is no longer true of the world `--play` boots into.
/// `--play` stopped meaning procedural terrain when Shiba's village landed:
/// `main.rs:355` sets `cfg.map_load = scene::play_map()`, and `play_map`
/// (`scene.rs:44`) returns `scene::PLAY_MAP` = `maps/edhari.json` whenever that
/// file is on disk. Edhari is a tight village, not an open landscape. The fit
/// was never re-run against it.
///
/// SO IT WAS RE-MEASURED, on the shipped binary, with the repo's own depth ruler
/// — `VOXELFORGE_LOOK_FOG=S,S+0.5` makes the `Linear` falloff a hard step at S,
/// so the pixels that move against a no-haze reference are exactly the pixels
/// farther than S, and sweeping S reads the frame's depth CDF straight off the
/// screen (`scripts/_flamingo_depth_ruler.sh` + `.py`, 22 rungs × 2 framings,
/// one binary, env only). Depth percentiles over the fog-receiving geometry:
///
/// | framing | p50 | p75 | p90 | p95 | p99 | deepest above the noise floor |
/// |---|---|---|---|---|---|---|
/// | `35,-18,26` (the pinned gate framing) | 5.7 | 7.7 | 11.8 | 16.0 | — | 28 |
/// | `0,1,18` (level, sees the ruins)      | 11.1 | 23.7 | 38.9 | 44.1 | 47.7 | ~56 |
///
/// THE RULER'S OWN NOISE FLOOR IS SUBTRACTED, and finding it is why the first
/// read of these captures was wrong. Past S≈32 the moved-pixel count did not
/// fall to zero, it went FLAT at ~1.0–1.2 % of frame across every remaining
/// rung out to 320. Nothing in this map is 320 blocks away; that plateau is the
/// capture's own frame-to-frame irreproducibility (TAA history + stochastic
/// SSAO — every capture is a separate process). Read raw it invents a 320-block
/// tail and inflates p95 by 12 %. The script now measures the plateau, subtracts
/// it, and prints every rung inside it as UNMEASURABLE rather than as a number.
///
/// AGAINST THAT, 150 WAS 3.1x PAST THE DEEPEST GEOMETRY THAT EXISTS. The ramp
/// reached `(47.7 - 20) / (150 - 20)` = **21.3 %** on the farthest surface in
/// frame and never got further, because there is no farther surface. The haze
/// was not too weak — it was a 130-block ramp with 28 blocks of world in it,
/// which is the same failure [`FOG_START`] was retired for (first sample point
/// past the back of the set), one order of magnitude in.
///
/// WHY 72 AND NOT 48. `1.5 x p99`, rounded. Ending the ramp AT the deepest
/// geometry (48) makes the back of the set 100 % opaque — a fog wall, the exact
/// failure mode [`HAZE_DENSITY`]'s own note calls out ("the numbers moved and
/// the picture died"). 1.5x puts the deepest visible surface at **53 %** and
/// p90 at 36 %, inside the band the approved curve was signed off in (its 63 %
/// at 100 blocks is documented as still reading "ruins receding rather than
/// fog"), and it leaves the ramp headroom for a scene edit that pushes the set
/// deeper without going opaque the day it lands. [`HAZE_START`] does NOT move:
/// it is bounded by gameplay, not by the set — `combat::LOCK_RANGE` is 16 from
/// the avatar and `main::BOOM_DIST` puts the camera 6.5 further back, so a
/// locked-on target at max range sits at 22.5 from the lens and 20 keeps
/// essentially all of the fightable volume clean. It is also already inside the
/// measured range, which is what it had to be.
pub const HAZE_FULL: f32 = 72.0;

const _: () = assert!(
    HAZE_FULL <= RENDER_RADIUS,
    "haze must be opaque no later than the streaming edge, or chunks pop in clear air"
);

/// Aerial-perspective density, `FogFalloff::ExponentialSquared` — the G7 default,
/// SUPERSEDED as the shipped curve by [`HAZE_START`]/[`HAZE_FULL`] and kept as
/// the curve `VOXELFORGE_LOOK_HAZE=<density>` selects, so the before/after
/// against it is still shot from one binary.
///
/// WHY THE LINEAR CURVE HAD TO GO. [`FOG_START`] is 112 blocks and the playable
/// campsite is ~55 blocks deep end to end, so **every pixel of the frame the
/// reviewer actually looks at rendered with exactly zero fog** — the far wall of
/// the ruin came out the same value, the same saturation and the same contrast
/// as the block under the player's feet. That is the "ไม่มีหมอกระยะ / ไม่มีมิติ"
/// the CEO called on the G6 frame: not a haze that was too weak, a haze whose
/// first sample point was past the back of the set.
///
/// WHY EXPONENTIAL-SQUARED WAS THE ANSWER TO THAT, AND WHY IT IS NOT THE ANSWER
/// NOW. Plain `Exponential` at density 0.008 is ~27% opaque by 40 blocks — it
/// fogs the foreground, the one thing aerial perspective must not do. Squaring
/// the distance term fixes exactly that: opacity goes as `1 - exp(-(d·density)²)`,
/// quadratically flat near the camera, biting once the distance term passes 1:
///
/// | distance | 10 | 20 | 40 | 60 | 100 | 160 | 320 |
/// |---|---|---|---|---|---|---|---|
/// | haze | 0.5% | 2.0% | 8.0% | 17% | 40% | 73% | 99.5% |
///
/// That table is TRUE — the shipped binary was measured against it and applies
/// 1.15%/1.59% where it predicts 1.32%/2.05%. What the table does not say is
/// that "2% of alpha" and "2% of a frame" are different quantities: at 2% the
/// haze still moved the pinned framing's foreground band by 19 display units,
/// because the thing being mixed in is several times the radiance of shadowed
/// ground. Quadratically flat is not flat enough when the multiplier is that
/// large, and no density fixes it (2.21–2.42 depth ratio across the whole
/// sweep). The dead zone in [`HAZE_START`] does, which is why the ramp shipped.
///
/// BOUNDED BELOW BY THE STREAMING CONTRACT, NOT BY TASTE. [`RENDER_RADIUS`]
/// promises the haze is opaque where chunks stop existing; anything under
/// `0.00673` leaves >1% of a popping chunk visible at 320 blocks. 0.0072 clears
/// that only in the limit (99.5%) — [`HAZE_FULL`] closes it outright.
/// 0.0072 → 0.0100, 2026-08-05. The bound above was always "how much world are
/// you willing to lose": row `v10-thick` at 0.0140 took warmth 58.0 → 105.9 on
/// its own, and also drowned the far ruins in soup (86 % opacity at 100 blocks
/// against the shipped 40 %) — the numbers moved and the picture died, which is
/// this axis's whole failure mode. 0.0100 is the rung where the far skyline still
/// reads as ruins receding rather than as fog: 63 % at 100 blocks, and paired
/// with the warm [`Hour::haze`] hue it is worth ~26 points of warmth
/// (`v20-t05nohz` 97.5 → `v21-t05hz100` 123.1) at no cost to any other axis.
///
/// READ THAT PARAGRAPH AS A REFERENCE CURVE, NOT AS A SHIPPED LEVER — 2026-08-06.
/// This constant is NOT on the shipped path and has not been since f8a1a8f: per
/// [`haze_falloff`], unset env gives `Linear{HAZE_START, HAZE_FULL}` and this
/// value is only reachable through `VOXELFORGE_LOOK_HAZE=<density>`. Every number
/// above was therefore measured on a curve the binary does not run, which is why
/// bd3cde5 moved this constant and the baked default did not budge (warmth 99.20
/// baked against 124.65 for the same binary under the env row). The warmth lives
/// in [`HAZE_FULL`] now, refitted to THIS density — so the two agree by
/// construction and the `VOXELFORGE_LOOK_HAZE=0.0100` A/B is still the shipped
/// picture rather than a different one. Change this and [`HAZE_FULL`] drifts off
/// its own fit; re-run `scripts/_flamingo_haze_refit.py` if you do.
pub const HAZE_DENSITY: f32 = 0.0100;

/// How far the haze colour is pushed from the sky's own hue toward white.
///
/// The horizon is never the zenith's colour: Rayleigh scattering has had far
/// more path length to work with down there, so it washes out. Deriving the
/// haze from [`Hour::sky`] instead of authoring a second colour is what makes
/// distant geometry dissolve INTO the sky rather than fade toward a grey that
/// disagrees with it — the giveaway that used to make the old
/// [`FOG_COLOR_DAY`] read as a filter laid over the frame instead of as air.
///
/// 0.60, NOT THE 0.40 G7 SHIPPED. Measured as sweep row `hd60` on the G7 build,
/// it beat the shipped default on every axis but one: interior floor G3 p05 25.0
/// vs 24.1 · vegetation hue 93.3 vs 96.9 (the half of axis C the haze was making
/// WORSE) · vegetation saturation 61.7 vs 60.3, both under the 62.8 gate · stone
/// hue drift in the near band cut ~10 deg, 28.6 vs 38.9, which is the terracotta
/// keeping its colour instead of reading pink-grey · sky and p95 identical ·
/// G3/G5/G6 P P P. It cost micro-contrast 5.41 vs 5.47, still over the 5.0 gate.
///
/// That was an ENV-OVERRIDE result, which is not a default: `VOXELFORGE_LOOK_
/// HAZEDESAT=0.60` proves a number is good, it does not prove the binary ships
/// it. The value is baked here and the whole table was re-shot with the env
/// UNSET to prove the out-of-box frame reproduces it — see
/// `docs/look-g7b-nearfield-2026-08-05.md`.
pub const HAZE_DESAT: f32 = 0.60;

/// Haze gain, **as a fraction of [`Hour::sky_gain`]** — the haze is the sky seen
/// through more air, so it is priced in the sky's own units and cannot drift
/// away from it when the sky is retuned.
///
/// Under 1.0 because [`HAZE_DESAT`] has already lifted the darker channels a
/// long way toward white; without pulling the overall level back the horizon
/// band comes out BRIGHTER than the sky above it and distant geometry
/// silhouettes in reverse — a glowing skyline instead of a receding one.
pub const HAZE_GAIN: f32 = 0.62;

/// Distance (blocks) at which the haze is fully opaque. Also [`RENDER_RADIUS`].
pub const FOG_END: f32 = 320.0;

/// The streaming contract: **every chunk within this radius of the camera must
/// be meshed and visible.**
///
/// The haze only reaches 100% at exactly this distance, so a chunk that unloads
/// any nearer shows up as a hole in a part of the frame the fog is not yet thick
/// enough to hide — precisely the pop-in the haze exists to bury. If the frame
/// budget can't reach 320, tell the look lane and this const moves (with
/// `FOG_START = 0.35 * RENDER_RADIUS`); don't let the two drift apart silently.
pub const RENDER_RADIUS: f32 = FOG_END;

/// Horizon haze colour, day — the hue [`haze_color`] dissolves distant geometry
/// toward, carried on [`Hour::fog`] so it travels with the hour.
///
/// (0.60, 0.72, 0.88) → (0.94, 0.66, 0.26), 2026-08-05. This value was DEAD code
/// for the day hour until now — `haze_color` derived the day haze from
/// [`Hour::sky`] and only read `Hour::fog` at night — so this is the first time
/// the constant's own docs have had to be true.
///
/// THE OLD WARNING STILL STANDS, AND THIS IS NOT IT. The (0.50, 0.42, 0.28) haze
/// that got reverted failed because it was DARK: an sRGB triple mixed into
/// already-exposed radiance, landing ~2.5× under the sky, so distant terrain went
/// muddy and receded into brown instead of into air. This value goes through the
/// same `desat`/`sky_gain` path the blue one did (see [`haze_color`]), so it
/// dissolves toward a horizon BRIGHTER than the geometry, which is what reads as
/// depth. What it changes is hue only — and hue is the axis where the blue was
/// wrong: at 17° sun elevation the horizon has the most air between it and the
/// eye, which is exactly where blue has been scattered out, not concentrated.
///
/// Measured on the vista framing, this hue with [`HAZE_DENSITY`] 0.0100:
/// warmth 97.5 → 123.1, blue 3.7 → 6.2, saturation 97.5 → 96.5, magenta 0.16 %,
/// all four `colour_gate.py` gates PASS (`_fl_grade2/vista-v21-t05hz100-*.png`).
pub const FOG_COLOR_DAY: [f32; 3] = [0.94, 0.66, 0.26];

/// Horizon haze colour, night.
pub const FOG_COLOR_NIGHT: [f32; 3] = [0.05, 0.08, 0.17];

/// Colour the haze takes on when looking INTO the sun — the warm atmospheric
/// glow that sells low golden-hour light.
pub const FOG_SUN_GLOW: [f32; 3] = [1.00, 0.85, 0.60];

/// How tightly [`FOG_SUN_GLOW`] hugs the sun direction (higher = tighter).
pub const FOG_SUN_EXPONENT: f32 = 30.0;

/// Estimated object thickness the SSAO pass assumes, in world units. A ray that
/// passes within this distance behind a surface counts as occluded, so it is the
/// knob that decides how deep a crease has to be before it darkens — voxel-scale
/// on purpose (1 block), so the creases between blocks read and the pass does
/// not start shading whole faces.
pub const AO_THICKNESS: f32 = 1.45;

/// Length of the screen-space contact-shadow ray, in world units (= blocks).
///
/// WHY A SECOND OCCLUSION LAYER AT ALL, WHEN SSAO IS ALREADY ON AT EVERY TIER.
/// Because SSAO could not do this job here, and that is measured, not argued.
/// Bevy's SSAO darkens the DIFFUSE INDIRECT term only — i.e. it can remove at
/// most the [`Hour::ambient_lux`] contribution and nothing of the key. On the
/// 2026-08-08 plates that ceiling is small: an A/B off the SAME binary
/// (`VOXELFORGE_LOOK_SSAO=off` against the shipped stack, s4 framing) moved the
/// frame by a mean of **0.65 L**, with only **2.24 %** of pixels darkened by more
/// than 4 L and **0.084 %** by more than 12. The layer was reaching the frame —
/// it simply had almost nothing it was allowed to take away, which is exactly the
/// "blocks hover off the ground" the CEO called. Raising `AO_THICKNESS` cannot
/// fix that; the cap is the fill's share of the light, not the crease depth.
///
/// `ContactShadows` is a different mechanism with a different budget: it
/// ray-marches the depth buffer toward the light and attenuates the DIRECT term,
/// so at the foot of a block — where the key is most of the light — it has the
/// whole key to bite into. It is what puts the dark seam back under an object
/// that a 4 K cascade covering 320 blocks is far too coarse to resolve.
///
/// 0.75 BLOCKS, NOT BEVY'S 0.3 DEFAULT. The default is authored for
/// metre-scale character scenes; here one world unit is one block, so 0.3 is a
/// third of a block and the seam it draws is thinner than the voxel it is
/// supposed to be grounding. 0.75 keeps the groove inside a single block (so it
/// reads as contact, not as a second cast shadow) while being wide enough to
/// survive the 1600-px plate downsample. Sweep with `VOXELFORGE_LOOK_CONTACT`
/// before moving it.
pub const CONTACT_SHADOW_LENGTH: f32 = 0.75;

/// Assumed depth-buffer fragment thickness for the contact-shadow ray-march, in
/// world units. The depth buffer is 2.5 D, so the march has to guess how solid
/// what it hits is; too thin and the ray tunnels through block faces seen at a
/// grazing angle (and a 22-degree key is ALL grazing angles), too thick and every
/// silhouette in front of open ground casts a halo onto it. 0.2 is a fifth of a
/// block — under the smallest feature the greedy mesher emits.
pub const CONTACT_SHADOW_THICKNESS: f32 = 0.2;

/// Ray-march steps for the contact shadow. Bevy's default; kept because the
/// march is over 0.75 of a block, so 16 steps is ~0.05 blocks per step and the
/// cost is a short loop in the PBR shader rather than another full-screen pass.
pub const CONTACT_SHADOW_STEPS: u32 = 16;

// ---------------------------------------------------------------------------
// V3 — the surface rig. Every constant below is reachable ONLY under
// `LookGen::V3`, so `VOXELFORGE_LOOK_GEN=v2` reproduces the 2026-08-14 frame
// byte-for-byte out of the same binary that shoots the new one.
// ---------------------------------------------------------------------------

/// Diffuse+specular luminance of the v3 hemispherical environment light, cd/m²,
/// day. See [`ibl_env`] for what it is and why a voxel scene needs it.
///
/// THE NUMBER IS A BUDGET TRANSFER, NOT A LIFT. A uniform hemisphere of
/// luminance `L` delivers irradiance `E = π·L` onto a surface facing it, so 260
/// nits is ~817 lux — which is very close to the 770 lux this generation takes
/// OUT of the flat term ([`AMBIENT_LUX_V3`], 1150 → 380). The frame keeps the
/// irradiance it was graded with; what changes is that the fill now arrives
/// FROM A DIRECTION, so a shaded ground face (facing the cool top of the map)
/// and a shaded underside (facing the warm bottom) stop receiving the identical
/// number. Rough budget on the four surfaces the v2 note tabulates, in lux:
///
///   surface (in shade)      v2 flat+sky+bounce      v3 flat+sky+bounce+IBL
///   shaded ground           1150 + 1358 +   0       380 + 1358 +   0 + ~817
///   wall, shadow side       1150 +  339 +   0       380 +  339 +   0 + ~735
///   wall, sun side          1150 +    0 + 971       380 +    0 + 971 + ~735
///   underside               1150 +    0 + 516       380 +    0 + 516 + ~817
///
/// i.e. 2508/1489/2121/1666 → 2555/1454/2086/1713. Every total lands within 3 %
/// of the v2 value it replaces, which is the point: G3's shade floor was bought
/// with that irradiance and this change is not allowed to spend it.
///
/// 260 → 440, 2026-08-15. THE TABLE ABOVE WAS WRONG, AND THE FRAME PAID FOR IT.
/// `E = π·L` is the irradiance from a hemisphere whose radiance IS `L`. But
/// `EnvironmentMapLight::hemispherical_gradient` does not build a hemisphere of
/// luminance `L` — it builds one of `L × the map's own colour`, and the colours
/// handed to it are [`Hour::sky_fill`] and [`Hour::bounce`], neither of which is
/// white. GOLDEN's are sRGB [0.62,0.76,1.00] and [1.00,0.78,0.50]; in LINEAR
/// space, which is where the multiply happens, their channel means are 0.62 and
/// 0.59. So the map delivered ~0.60 × 817 = **490 lux**, not 817 — the transfer
/// was 327 lux short on every surface in the table, on a budget the note itself
/// says is "not allowed to spend".
///
/// The measurement agrees, and agrees in the right ORDER. Shot from one binary
/// on the v4 plates (`docs/assets/look/`, gen=v2 vs gen=v3), shadow p05 went
/// 49.70 → 48.47 outdoor-noon, 21.56 → 18.68 evening-raking, 25.76 → 19.63
/// night-firelit. Day loses ~2 %, night loses 24 %, because the deficit scales
/// with how dark the map's own colours are and NIGHT's are much darker (see
/// [`IBL_NITS_NIGHT`]). A term that was supposed to be neutral was the largest
/// single subtraction in the generation.
///
/// 440 = 260 / 0.60, i.e. the same 817 lux the table budgeted, now actually
/// delivered. Nothing else in the transfer moves: [`AMBIENT_LUX_V3`] stays 380,
/// so the flat/directional split this generation exists to make is unchanged.
///
/// AND IT BUYS THE THING NO AMOUNT OF `AmbientLight` CAN. `AmbientLight` and a
/// `DirectionalLight` both feed the DIFFUSE term only. An environment map feeds
/// the SPECULAR one as well, which is the whole reason the reference shader
/// packs read as lit surfaces and this frame read as painted ones: a smooth
/// block, a blade, a helmet, wet stone all pick up a sky-coloured sheen that
/// slides across them as the camera moves. Sweep with `VOXELFORGE_LOOK_IBL`.
/// 440 → 330, one round later, PAIRED WITH [`AMBIENT_LUX_V3`] 380 → 620 and to
/// be read as one move. 440 fixed the level and left the frame cold: evening
/// warmth came out 73.23 against a v2 baseline of 79.07. Cause below, on
/// `AMBIENT_LUX_V3` — the term v3 drained is the warmest light in the rig, and
/// no size of a cool-topped environment map replaces it in the right channel.
/// So ~210 lux moves back out of here and ~240 lux of warm flat fill moves in:
/// total irradiance on a shaded face is held, the HUE of it shifts warm.
pub const IBL_NITS_DAY: f32 = 330.0;

/// The same, night. Sized off [`AMBIENT_LUX_V3_NIGHT`] by the same `E = π·L`:
/// 9 nits is ~28 lux, which is the 28 the flat night fill gives up (42 → 14).
///
/// 9 → 23, 2026-08-15, same correction as [`IBL_NITS_DAY`] and a larger one,
/// because NIGHT's map is darker than GOLDEN's: sRGB [0.44,0.58,0.98] and
/// [0.55,0.56,0.68] are linear means 0.47 and 0.32, so the hemisphere's own
/// albedo is ~0.40 and 9 nits delivered ~11 lux against the 28 it was budgeted
/// for. That 17-lux hole is 17 lux out of a night shade budget of ~120 total,
/// which is why night-firelit lost 24 % of its p05 where noon lost 2 %.
/// 23 = 9 / 0.40.
///
/// 23 → 14, SAME DAY, AFTER SHOOTING IT. 23 did what it was sized to do — the
/// night shade floor went p05 19.63 → 23.41, clear of the 20.40 bar — and it
/// cost more warmth than the bug it fixed: midtone R−B 37.50 → 30.79, against a
/// v2 baseline of 43.04. The arithmetic above is still right; the CHANNEL it
/// was spent in was wrong.
///
/// WHY THIS TERM CANNOT BUY NIGHT FLOOR AT ANY SIZE. The map is built from
/// [`Hour::sky_fill`] and [`Hour::bounce`], and at NIGHT those are sRGB
/// [0.44,0.58,0.98] and [0.55,0.56,0.68] — B > G > R in BOTH halves. The night
/// hemisphere is blue top to bottom by construction, so scaling it scales blue,
/// and no value of this constant adds level without adding cast. That is a
/// property of the map, not a tuning miss. (The day map's lower half is
/// [1.00,0.78,0.50] and warm, which is why [`IBL_NITS_DAY`] does not have this
/// problem and is left at 440.)
///
/// So the night floor is bought from the terms that are NOT blue-dominant —
/// [`CONTACT_SHADOW_LENGTH_V3`] giving back its over-long bite, and
/// `grade::SHADOW_GAIN_V3` lifting the toe — and this one is pulled back to 14,
/// half the corrected transfer. That spends ~1.5 of the ~3 points of headroom
/// night p05 now has over the bar, which is what there is to spend.
pub const IBL_NITS_NIGHT: f32 = 14.0;

/// Where the environment map's HORIZON band sits between its cool top
/// ([`Hour::sky_fill`]) and its warm bottom ([`Hour::bounce`]), 0 = all top.
///
/// 0.55, i.e. biased warm — the same bias and the same number the sky dome's own
/// gradient uses for the same reason (see `build_sky_dome_mesh`): at a raking
/// sun the band around the horizon is where the warm light is, and the deep cool
/// sits high. Biasing it is what makes a vertical face — which integrates mostly
/// the horizon band — read warmer than the ground face above it, which is the
/// top/side split this whole rig exists to draw.
///
/// 0.55 → 0.68, 2026-08-15. The paragraph above is the right argument and 0.55
/// was not enough of it. Once [`IBL_NITS_DAY`] delivers the irradiance it was
/// always supposed to, this constant decides what COLOUR that irradiance is,
/// and at 0.55 the horizon band still sits nearer the cool top than the warm
/// bottom — so correcting the level cooled the frame: evening-raking midtone
/// R−B came out 72.36 against its own v2 baseline of 78.91, i.e. the warmth
/// clause failed on the plate the whole rig is graded on.
///
/// 0.68 leans the band on [`Hour::bounce`] ([1.00,0.78,0.50] at GOLDEN), which
/// is the honest physical reading of a raking hour anyway: most of what a
/// vertical wall sees is not zenith, it is warm ground and warm horizon. It is
/// also gap 3 of the reference read (`docs/look-v5-gap-vs-golden-ref.md`) —
/// "sky-to-ground colour bleed" — expressed as the one number that controls it.
///
/// Bounded by the top/side split above: the map's TOP is still pure
/// [`Hour::sky_fill`] at any mix, so a shaded ground face keeps reading cool
/// against a shaded wall. Past ~0.8 the band would converge on the bottom and
/// the split would collapse; 0.68 is well inside that.
pub const IBL_HORIZON_MIX: f32 = 0.68;

/// The v3 FLAT fill floor, day, lux — what is left of `AmbientLight` once
/// [`IBL_NITS_DAY`] carries the directional share. See that constant's table.
///
/// Not zero, deliberately: the environment map is a hemisphere, so a face
/// pointing exactly at the horizon integrates the least of it, and a small
/// flat term keeps that face off the floor without re-flattening the frame.
///
/// 380 → 620, 2026-08-15. THE V3 TRANSFER WAS WARMTH-NEGATIVE BY CONSTRUCTION
/// AND NOBODY PRICED THAT. `AmbientLight`'s colour is [`Hour::ambient`], and at
/// GOLDEN that is sRGB **[0.96, 0.90, 0.48]** — R−B +0.48, the warmest light in
/// the entire rig. v3 drained 770 lux out of it and replaced them with an
/// environment map whose TOP is [`Hour::sky_fill`] (cool) and a kicker that is
/// cool on purpose. Even after [`IBL_NITS_DAY`] was corrected to deliver the
/// full budgeted irradiance, evening-raking's midtone R−B came out 73.23
/// against v2's 79.07, and the shadow sides of the ruins read grey in the plate
/// rather than the warm brown v2 gives them. The level was restored in the
/// wrong channel.
///
/// 620 puts ~240 of those lux back, and [`IBL_NITS_DAY`] gives up ~210 in the
/// same move, so a shaded face keeps the irradiance the v3 table budgeted for
/// it — this buys HUE, not brightness, and the shade-floor clause it must not
/// spend is unaffected.
///
/// STILL 620 AND NOT 1150. The generation's thesis — a flat term gives every
/// face the same number and a voxel scene is nothing but faces — is right, and
/// the measurement backs it: v3's top-vs-side spread beats v2's on the plates
/// (72.30 vs 65.54 at noon). 620 is a little over half of v2's flat fill, so
/// the directional rig still carries the majority of the fill and still draws
/// the shape. What it stops doing is carrying ALL of it in the wrong colour.
pub const AMBIENT_LUX_V3: f32 = 620.0;

/// The same, night (v2: 42).
///
/// 14 → 26, 2026-08-15, AND ONLY BECAUSE [`AMBIENT_COLOR_V3_NIGHT`] LANDED IN
/// THE SAME MOVE. 14 lux of [`Hour::NIGHT`]'s own ambient is 14 lux of sRGB
/// [0.42, 0.52, 0.78] — R−B −0.36 — so every previous round that reached for
/// this constant to buy night warmth was buying blue. Measured, that is exactly
/// what it does: 14 → 30 at the authored colour takes midtone R−B 33.89 → 31.93
/// (`_poppy_lookv6/n1`, row `a30c`). With the colour warm the same lux runs the
/// other way, 33.89 → 46.26 at 26 lux, and the shade floor comes with it
/// (21.78 → 24.46) because a flat term lifts the faces that have the least of
/// everything else.
///
/// 26 AND NOT 50, WHICH MEASURES WARMER. The flat term is also the separation
/// tax: 50 lux reads warmth 48.68 with spread 87.02, under the 90.79 bar the
/// v2 plate sets, and 80 lux is 85.16. A flat light gives every face the same
/// number, which is the one thing a night frame lit by a single campfire must
/// not do. 26 lux clears the warmth bar by +3.07 while separation stays ABOVE
/// the before plate (91.95 vs 90.79) — the first round this scene has held both.
pub const AMBIENT_LUX_V3_NIGHT: f32 = 26.0;

/// Colour of the v3 flat fill at night, sRGB (v2/shared: [`Hour::NIGHT`]'s own
/// `ambient`, [0.42, 0.52, 0.78]).
///
/// THE CAMPFIRE WAS ASSUMED TO COVER THIS AND IT DOES NOT. `Hour::NIGHT`'s note
/// on `ambient_lux` argues that lanterns and the fire should be the only warm
/// light in frame, and the flat term was left cool to enforce it. What the
/// plates show is that the fire lights what it can reach and the rest of the
/// frame's shade is lit by a rig whose every term is B > G > R — `sky_fill`
/// [0.44, 0.58, 0.98], `bounce` [0.55, 0.56, 0.68], the moon key, the kicker,
/// and the environment map both halves of it (see [`IBL_NITS_NIGHT`], which
/// records that no size of that constant changes its hue). The scene had no warm
/// light with a free colour at all, which is why v5's −9.82 could not be fixed
/// by moving any amount of any existing one.
///
/// [1.00, 0.62, 0.20] is the fire's own bounce, one step deeper than GOLDEN's
/// ground bounce ([1.00, 0.78, 0.50]) because it is coming off ground lit by a
/// small orange source rather than by the sun. R−B +0.80, the warmest thing in
/// the rig, spent at 26 lux against night's ~150-lux shade budget.
///
/// V3-ONLY, VIA [`hour`], NOT BY EDITING [`Hour::NIGHT`]. The night pair's
/// before plate is v2, and v2 runs this same field at 42 lux — three times the
/// v3 level — so warming the shared constant would have warmed the BAR harder
/// than the reading and made the clause harder to pass while looking like a fix.
/// Set after the generation fork and before the env overrides, so
/// `VOXELFORGE_LOOK_LIGHT` still wins over it.
pub const AMBIENT_COLOR_V3_NIGHT: [f32; 3] = [1.00, 0.62, 0.20];

/// PCSS penumbra width, v3 (v2: [`PCSS_WIDTH`] 8.0, v1: [`PCSS_WIDTH_V1`] 16.0).
///
/// 8.0 was chosen under a constraint v3 removes. The v2 note on [`PCSS_WIDTH`]
/// records it as "paired at 8.0 rather than 16.0 so the contact edges it now
/// reaches stay sharp" — i.e. the width was held down because PCSS was the only
/// thing drawing the near end of the shadow and a wide filter smeared it. In v3
/// the near end is drawn by a contact march that is 47 % longer and steps 50 %
/// finer ([`CONTACT_SHADOW_LENGTH_V3`]), so the two ends are no longer fighting
/// over one knob: the march owns the first block, the penumbra owns everything
/// past it. 12.0 sits between the two shipped widths — wide enough that canopy
/// shadow on open ground grades visibly with blocker distance (the single
/// clearest tell in `_poppy_lookv2/ref/complementary-style.png`), short of the
/// 16.0 v1 ran without any contact layer under it at all.
pub const PCSS_WIDTH_V3: f32 = 12.0;

/// Contact-shadow march length, v3, blocks (v2: [`CONTACT_SHADOW_LENGTH`] 0.75).
///
/// 0.75 was bounded by "keep the groove inside a single block so it reads as
/// contact and not as a second cast shadow". That bound was written when the
/// march was the ONLY thing between a block and hovering. With PCSS now widened
/// to [`PCSS_WIDTH_V3`], a groove that runs slightly past one block no longer
/// competes with a hard cast edge next to it — it hands off to a soft one. 1.10
/// is the longest march that still dies inside the near cascade
/// ([`FIRST_CASCADE_FAR_BOUND_V3`] keeps that cascade tight), so the extra
/// length is spent where the shadow map is densest.
///
/// 1.10 → 0.85, 2026-08-15. The paragraph above prices the length against PCSS
/// and never against the SHADE FLOOR, and the shade floor is what pays: the
/// note on [`ssao`] states the rule this generation broke — "an in-shade face
/// is ~100 % fill and takes the full bite" — so a march 47 % longer subtracts
/// 47 % more from exactly the pixels p05 is measured on. It landed in the same
/// generation as the [`IBL_NITS_DAY`] shortfall, and the two together are the
/// whole 21.56 → 18.68 evening-raking regression. The IBL fix restores the
/// light; this gives back the part of the bite that was bought on credit.
///
/// 0.85 and not back to v2's 0.75: the hand-off argument above is still true
/// and the groove still wants to be longer than v2's under a 12.0 penumbra.
/// 0.85 is +13 % over v2 instead of +47 % — the direction v3 argued for, at a
/// third of the price. [`CONTACT_SHADOW_STEPS_V3`] stays 24, so the step gets
/// finer (0.035 blocks) rather than coarser; nothing is resampled.
pub const CONTACT_SHADOW_LENGTH_V3: f32 = 0.85;

/// Assumed fragment thickness for the v3 contact march, world units
/// (v2: [`CONTACT_SHADOW_THICKNESS`] 0.2).
///
/// Thinner, because the march is longer: thickness is how solid the march
/// assumes what it hits is, and a longer ray with the same generous thickness
/// starts throwing halos from silhouettes onto ground behind them (the failure
/// the v2 note names). 0.14 is ~1/7 of a block — still above the greedy
/// mesher's smallest feature, and the pairing that keeps the longer march from
/// buying occlusion it did not earn.
pub const CONTACT_SHADOW_THICKNESS_V3: f32 = 0.14;

/// Ray-march steps for the v3 contact shadow (v2: [`CONTACT_SHADOW_STEPS`] 16).
///
/// The march got 47 % longer; at 16 steps that is 0.069 blocks per step against
/// v2's 0.047, i.e. the extra length would have been bought by making the ray
/// coarser and it would have banded. 24 keeps the step at 0.046 — the density
/// v2 shipped — so the length is real and not resampled.
pub const CONTACT_SHADOW_STEPS_V3: u32 = 24;

/// Near-cascade far bound, v3, blocks (v2 and earlier: 16.0).
///
/// One shadow map split across four cascades: the tighter the first bound, the
/// more texels land on the volume the player is standing in. 16.0 was sized off
/// `combat::LOCK_RANGE`; 10.0 is sized off what the near cascade is FOR under
/// v3, which is holding the contact end crisp while a 12.0-wide penumbra
/// softens everything beyond it. Nothing is lost past 10 blocks — cascade two
/// picks it up, and it is exactly there that the wide filter wants to be soft.
pub const FIRST_CASCADE_FAR_BOUND_V3: f32 = 10.0;

/// Azimuth of the v3 kicker, degrees CCW from the direction the camera looks.
///
/// WHY IT IS CAMERA-RELATIVE AND EVERY OTHER LIGHT IN THIS FILE IS NOT. The key
/// and the two fills describe the WORLD — the sun is where the sun is, and a
/// fill that swung with the camera would make the world appear to rotate. This
/// one describes the SHOT. Its whole job is the edge that separates a character
/// from whatever is behind them, and "behind" is a fact about the camera, not
/// about the world: a world-fixed rim lights the player's silhouette from one
/// orbit angle and their face from the opposite one. Every reference frame in
/// the set has this edge on the subject from whichever side the shot is taken.
///
/// 152° rather than 180°: dead behind puts the kicker exactly along the view
/// axis, where it rims nothing (the lit sliver is hidden by the subject itself)
/// and only lifts the background. Offset to one side and it draws a real edge
/// down one flank.
pub const RIM_AZIM_OFFSET: f32 = 152.0;

/// Elevation of the v3 kicker, degrees above the horizon. Low, because it is
/// standing in for light skimming off the far side of the scene; a high kicker
/// duplicates the sky fill, which is already near-zenith at [`SKY_FILL_ELEV`].
pub const RIM_ELEV: f32 = 14.0;

/// Colour of the v3 kicker, sRGB — cool, and paler than [`Hour::sky_fill`].
///
/// Cool ON PURPOSE, against a warm key: the separation the eye reads on a
/// character is as much hue as it is level, and a warm rim under a warm key is
/// a brightness change nobody notices. This is the "cool/warm split by layer"
/// half of the brief — key and bounce warm, sky fill and kicker cool.
///
/// R 0.78 → 0.86, 2026-08-15 — STILL COOL, and the cool is the point, but this
/// kicker was measured taking warmth out of the whole frame and not just off
/// the edge it draws. Same v4 plates as [`IBL_NITS_DAY`]: warmth (R−B over the
/// midtone band) went 79.15 → 73.26 evening-raking and 43.10 → 37.50
/// night-firelit going v2 → v3, and the kicker is the only cool light v3 adds.
/// It is a directional light with no shadow map, so it does not stop at the
/// silhouette — every face turned within 90° of it takes the blue.
///
/// The FIX IS NOT TO DIM IT. The brief is "brighter rim AND warmth not lower",
/// so the lever is the hue, not the level: 0.78 → 0.86 halves the red deficit
/// against blue (R−B −0.22 → −0.14) while keeping the ordering B > G > R that
/// makes the edge read as a different light from the key. The day level
/// ([`RIM_LUX`] 600) is untouched.
pub const RIM_COLOR: [f32; 3] = [0.86, 0.90, 1.00];

/// Illuminance of the v3 kicker, lux, day.
///
/// Deliberately the smallest light in the rig. Against a 22 000-lux key it is
/// under 3 % and cannot touch the sunlit wedge G6 grades; against the ~2 500 lux
/// a shaded face receives it is ~24 %, which is an edge you can see and not a
/// second fill. It casts no shadow map (see [`apply_fill_rig`]), so the cost is
/// one more N·L term.
pub const RIM_LUX: f32 = 600.0;

/// The same, night. Held to the same ~24 %-of-shade ratio at night's scale.
///
/// 30 → 20, 2026-08-15. The ~24 % was computed against the night shade budget
/// the v3 table CLAIMED (42 + 62 + 16 + 28 ≈ 148 lux). What the frame actually
/// received was ~131, because [`IBL_NITS_NIGHT`] delivered 11 of its 28 — so
/// the ratio shipped nearer 30 %, and it shipped as the only saturated light
/// in a frame whose one warm source is a campfire. That is the 43.10 → 37.50
/// warmth drop on `night-firelit`. With the IBL correction the budget is back
/// to ~148 and 20 lux is 13.5 %: still an edge, no longer a second key. Day is
/// left at [`RIM_LUX`] 600 — nothing was wrong with it.
pub const RIM_LUX_NIGHT: f32 = 20.0;

/// Bloom prefilter threshold, v3 (v2: 1.0 — emissive only).
///
/// 1.0 means "only things brighter than white", i.e. lanterns, the campfire and
/// the sun's disc, and nothing else — which is correct and is also why the v2
/// frame has no bloom at all in daylight while every reference shader pack does.
/// The reference glow is not a veil over the whole frame (`Bloom::NATURAL`'s
/// own 0.0 threshold, which the v2 note measured costing micro-contrast 5.2 →
/// 3.8) — it is a tight halo on the few surfaces the sun has pushed to the top
/// of the range. 0.85 selects exactly that band: at [`Hour::ev100`] 10.3 the
/// brightest sunlit patch measures L 57 %, well under it, so ordinary sunlit
/// ground still does NOT bloom; what crosses is specular — the new sheen
/// [`IBL_NITS_DAY`] puts on smooth faces, sun on water, a blade's edge.
pub const BLOOM_THRESHOLD_V3: f32 = 0.85;

/// Bloom prefilter softness, v3 (v2: 0.4). Feathers the 0.85 cut so the halo
/// ramps in over the band rather than switching on at a luminance line — with
/// the threshold lowered, a hard knee would crawl visibly as the camera moves.
pub const BLOOM_SOFTNESS_V3: f32 = 0.6;

/// Bloom intensity, v3 (v2: 0.18).
pub const BLOOM_INTENSITY_V3: f32 = 0.20;

/// Bloom low-frequency boost, v3 (`Bloom::NATURAL`: 0.7).
///
/// This is the knob that decides tight glow versus veil, and it is the one the
/// threshold change makes dangerous to leave alone: the boost weights the
/// COARSEST mips, which is exactly the wide low-contrast wash that flattened the
/// frame the last time this lane touched bloom. Halved to 0.35, so admitting a
/// wider band of pixels buys a sharper halo instead of a bigger one.
pub const BLOOM_LF_BOOST_V3: f32 = 0.35;

// ===========================================================================
// LOOK GENERATION — the v1/v2/v3 A/B switch
// ===========================================================================

/// Which generation of the light rig the process runs.
///
/// `VOXELFORGE_LOOK_GEN=v1` reproduces the light rig this lane shipped before
/// 2026-08-14 — flat [`AMBIENT_LUX_V1`] fill, no sky/bounce lights, PCSS at Ultra
/// only and [`PCSS_WIDTH_V1`] wide. `=v2` reproduces the rig that shipped ON
/// 2026-08-14 (key + two directional fills). Unset (or `=v3`) is the shipped
/// default.
///
/// It exists for ONE reason and it is the reason every honest look claim in this
/// file has: a before/after pair has to come out of ONE binary, at one camera, or
/// the difference being shown is "two builds" and not "this change". Every
/// generation fork in this file reads this one function; there is no second switch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LookGen {
    /// The pre-2026-08-14 rig: one key + one flat ambient.
    V1,
    /// Key + directional sky fill + directional ground bounce, PCSS from High up.
    V2,
    /// V2 plus the SURFACE rig: hemispherical image-based light (diffuse AND
    /// specular, so a face is lit by which way it points and a wet/smooth
    /// material actually reflects something), a camera-relative kicker that
    /// separates characters from the background, PCSS from Medium up at
    /// [`PCSS_WIDTH_V3`], a longer contact-shadow march and a tight highlight
    /// bloom. See [`ibl_env`] and [`LookFill::Rim`].
    V3,
}

/// Read [`LookGen`] from the environment. Unset ⇒ [`LookGen::V3`].
pub fn look_gen() -> LookGen {
    match std::env::var("VOXELFORGE_LOOK_GEN")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "v1" | "1" | "legacy" => LookGen::V1,
        "v2" | "2" | "before" => LookGen::V2,
        _ => LookGen::V3,
    }
}

/// True when the live generation carries the v3 surface rig. Every v3 fork in
/// this file goes through here rather than re-matching the enum, so "what is in
/// v3" is one predicate and adding a v4 does not mean auditing twenty matches.
fn v3() -> bool {
    matches!(look_gen(), LookGen::V3)
}

/// The FLAT fill v1 shipped, day. Kept as a constant rather than deleted so
/// `VOXELFORGE_LOOK_GEN=v1` reproduces the old frame exactly instead of
/// approximately — see [`Hour::ambient_lux`] for the history behind the number.
pub const AMBIENT_LUX_V1: f32 = 2200.0;

/// The same, night.
pub const AMBIENT_LUX_V1_NIGHT: f32 = 90.0;

/// Elevation of the sky fill, degrees above the horizon.
///
/// WHY A NEAR-ZENITH DIRECTIONAL AND NOT MORE `AmbientLight`. `AmbientLight` is a
/// single flat term: every surface in open shade receives exactly the same
/// irradiance regardless of which way it faces. On a VOXEL scene that is the
/// worst possible fill, because a voxel scene is nothing but axis-aligned faces —
/// the one cue that tells a block's top from its side is orientation, and a flat
/// ambient erases it. That erasure is the "flat" the reference shaders do not
/// have: in `_poppy_lookv2/ref/complementary-style.png` a grass block's top face
/// is plainly brighter and cooler than its own side face, everywhere in frame,
/// including deep in shadow where no sun reaches either of them.
///
/// 76°, not 90°: a perfectly vertical fill leaves every vertical face at exactly
/// zero, and `Transform::looking_to` gets closer to a degenerate up-vector the
/// nearer the direction gets to `Vec3::Y`. 76° keeps 0.24 of the fill on vertical
/// faces (`cos 76°`) and 0.97 on horizontal ones — the top/side split without a
/// black side.
pub const SKY_FILL_ELEV: f32 = 76.0;

/// Elevation of the ground bounce, degrees BELOW the horizon (it travels upward).
///
/// 28° is shallow on purpose: a bounce that comes straight up only reaches
/// undersides, and undersides are a small share of a voxel frame. At 28° the
/// horizontal term (`cos 28° = 0.88`) dominates, so what it actually lifts is the
/// lower half of vertical faces on the sun side — the warm return light off the
/// sunlit ground in front of them, which is the single most visible thing the
/// flat fill was standing in for.
pub const BOUNCE_ELEV: f32 = 28.0;

/// PCSS penumbra width v1 shipped. Reachable via `VOXELFORGE_LOOK_GEN=v1`.
pub const PCSS_WIDTH_V1: f32 = 16.0;

/// PCSS penumbra width for the sun, where the tier turns it on.
///
/// 16.0 → 8.0 (2026-08-14), AND the tier that turns it on moved High-and-up.
/// Those are one decision: the width was measured on the `s4` framing and shipped
/// to a tier (`Ultra`) that the default session never runs, so what the player saw
/// was a shadow with no penumbra at all, and what the ONE plate that did run Ultra
/// saw was a penumbra so wide it read as blur. Both halves are the same mistake —
/// the knob was never judged at the tier it ships at.
///
/// 8.0 comes off the ladder already recorded below: 7.5 px mean / 5.0 px MEDIAN,
/// against 4.3/4.0 for PCSS off. A median that barely moves while the mean climbs
/// is exactly the shape wanted here — most edges (the contact edges, where the
/// blocker sits ON the receiver) stay as sharp as they were, and the few with a
/// real blocker-to-receiver gap open up. That IS "sharp at contact, soft with
/// distance"; 16.0 (11.2 mean / 9.0 median) moves the median too and softens
/// edges that should have stayed crisp.
///
/// 3.0 → 4.0 (2026-08-06), 4.0 → 16.0 (2026-08-08). The 08-06 move was the right
/// knob turned far too little, and the reason is a floor in Bevy, not in us.
///
/// WHY 4.0 MEASURED AS "NO PENUMBRA". `bevy_pbr` 0.19 `shadow_sampling.wgsl:303`
/// computes, in TEXELS:
///     blur_size = max((z_blocker - depth) * light_size / depth, 0.5)
/// Both z's are cascade NDC, and the cascade projection is reverse-Z ortho
/// (`bevy_light/cascade.rs`: `ndc = 1 + z_lightspace/dz`), so `z_blocker - depth`
/// is `gap_blocks / cascade_depth_span` — a number in the THOUSANDTHS at this
/// scale. Multiply by 4.0 and it stays far under the 0.5 floor everywhere except
/// where an occluder stands unusually far off its receiver. That is not a
/// derivation, it is what the frame shows: an A/B off ONE binary, `=off` against
/// `=4`, moved the frame by **0.229 mean L** against a **0.085–0.223** noise
/// floor (four repeat shots at identical settings). The shipped penumbra was
/// inside the run-to-run noise of the capture. It was decorative.
///
/// THE MEASURED LADDER. s4 framing, Ultra, one binary, only `VOXELFORGE_LOOK_PCSS`
/// moving; 20–80 % edge width at 15 sites whose x was LOCKED on the PCSS-off
/// reference first, so every rung is measured across the same edges (`scripts/
/// _pixel_pcss_ladder.ps1`):
///     off   4.3 px mean /  4.0 median —  0/15 sites widened
///     4     5.8         /  5.0        —  3/15   <- shipped, and G4a's 5 px floor
///     8     7.5         /  5.0        —  5/15      was being cleared on a mean
///     12    8.9         /  6.0        —  9/15      that 12 of 15 edges did not
///     16   11.2         /  9.0        — 10/15      contribute to at all
///     24   17.9         / 16.0        — 11/15
///     32   20.5         / 16.0        — 10/15
///
/// BOUNDED ABOVE, TWICE, AND THE SECOND BOUND IS A TRAP. Visually the silhouette
/// starts dissolving past ~24 (the stepped voxel shadow stops reading as the
/// tower that casts it). Past ~128 it fails a different way: the blocker search
/// offsets by `search_size / (texel_size * 4096)` = `light_size / cascade_diameter`
/// in UV, so a large enough width throws every blocker tap off the shadow map,
/// `sum.y == 0` returns `z_blocker = 0`, and `blur_size` clamps straight back to
/// the 0.5 floor. Measured: 256 and 1024 are indistinguishable from PCSS OFF
/// (0.277 / 0.259 mean L against that 0.223 noise floor). The knob silently
/// switches ITSELF off at the top of its range, so "bigger is softer" is false
/// and a sweep is the only safe way to move it. 16.0 sits mid-window.
///
/// COLLATERAL AT 16, MEASURED, NOT ASSUMED. `grade_axes` on the Ultra vista plate:
/// warmth 162.98 → 157.03, micro-contrast 7.38 → 6.80, p95 163.89 → 159.97 — all
/// PASS at both widths (the DOF axis fails at both; DoF is stripped, see §5 of
/// docs/look-contract.md). `grade_sunsplit` on s4: separation 23.42 → 21.48 L,
/// dip 0.708 → 0.634, TWO HUMPS at both.
///
/// SCOPE, SO THIS IS NOT OVERSOLD: `apply_look_to_sun` turns PCSS on at Ultra
/// ONLY, so this constant reaches exactly one of the 8 canonical plates
/// (`grade-vista`). The other seven shoot High and take the Gaussian/Temporal
/// path, where the shadow edge is whatever that filter gives and this number is
/// not in the picture. Sweep with `VOXELFORGE_LOOK_PCSS=<width>` before moving it.
/// (That last paragraph's SCOPE claim is what the 2026-08-14 tier move fixes: PCSS
/// is on at High now, so this constant reaches the tier the game actually ships.)
pub const PCSS_WIDTH: f32 = 8.0;

/// Post-grade constants.
///
/// These started as the numbers the reviewer approved on the golden beauty shot
/// (see `docs/golden-beauty-shot.md`), and the three contrast/gain values have
/// since been re-derived for a different tonemapper — see each doc comment. They
/// are named here so a tuning pass has one place to edit and so a diff against
/// `hero.rs` stays readable.
mod grade {
    /// Chromaticity push toward red — the ONE knob that must not be copied from
    /// `hero.rs` unchanged, because it is scene-dependent in a way the others
    /// are not.
    ///
    /// WHY IT IS 0.02 HERE AND 0.10 THERE (magenta-cast fix, 2026-08-01). Bevy
    /// turns `temperature` into a full 3×3 chromatic-adaptation matrix, not a
    /// per-channel gain: `white_point_xy = D65_XY + (-temperature, tint)`, then
    /// `LMS_TO_RGB * diag(D65_LMS / white_point_lms) * RGB_TO_LMS`
    /// (`bevy_render::view`). Its off-diagonal terms bleed G and B *into* R, so
    /// the redder it pushes, the more it multiplies whatever is blue in the
    /// frame. `scripts/wb_matrix.py` reproduces that matrix on the CPU: at 0.10
    /// the authored sky comes out of the white balance ordered `R > B > G` —
    /// that is the magenta. The same script pins the real magenta onset at
    /// ≈0.099; 0.02 sits ~5× under it.
    ///
    /// `hero.rs` never saw this because its scene has no sky: it clears to
    /// near-black and every lit surface is already amber, so a red push there
    /// only deepens an ordering that was `R > G > B` to begin with.
    /// Byte-identical constants, opposite result — which is why the warmth the
    /// outdoor frame needs is carried by the LIGHTS ([`super::Hour::key`] /
    /// [`super::Hour::ambient`]), which the flat sky clear does not receive,
    /// instead of by a global matrix, which it does.
    /// 0.02 → 0.05, 2026-08-05 (vista sweep round 7). 0.02 was set as "5× under
    /// the magenta onset" and that safety margin turned out to be the single
    /// biggest thing standing between this frame and the warmth axis. Measured on
    /// the vista framing, moving ONLY this constant took warmth R−B 59.7 → 97.0
    /// and blue 10.4 → 5.6 (`_fl_grade2/vista-v12-temp05-nohud2.png`) — a bigger
    /// step than the whole 1.05 → 2.05 saturation ladder bought, because the
    /// adaptation matrix multiplies what is blue in the frame and the vista band
    /// is 28 % hazed atmosphere.
    ///
    /// THE CEILING IS 0.05, AND IT IS MEASURED, NOT ASSUMED. `scripts/wb_matrix.py`
    /// puts the magenta onset at ≈0.099, but that is the onset on the SKY CLEAR in
    /// isolation; row `v22-t07` shot the real frame at 0.07 and
    /// `scripts/colour_gate.py` Gate B failed it outright — sky ordering `B > R > G`
    /// with per-channel gains ×1.35/×0.68/×1.07, i.e. red lifted over unity while
    /// green was crushed onto blue, the exact chromatic-adaptation signature the
    /// 2026-08-01 review named. 0.05 passes all four gates at 0.16 % magenta. The
    /// usable headroom is half what the CPU model predicted; do not raise this
    /// without re-running `colour_gate.py` on a real frame.
    pub const TEMPERATURE: f32 = 0.05;

    /// Saturation push. TonyMcMapface (see [`super::base_camera_look`]) is a
    /// neutral transform that neither adds nor removes saturation, so this is a
    /// deliberate lift toward the reference's vivid leaves and sky — NOT the
    /// repair job AcesFitted's ~80% flattening used to need.
    ///
    /// 1.05 → 1.35, 2026-08-05. 1.05 was set on the reasoning that a neutral
    /// tonemapper needs no repair, and that reasoning is sound but the number
    /// under-shot: measured on the current `--play` boot frame, the midtone band
    /// came out `(145, 107, 80)` — a 45 %-saturated tan — against the golden
    /// ref's `(125, 49, 4)` at 97 %. The frame was not neutral, it was washed,
    /// and washed is what let the sunlit patch drift back to `R > B > G`: gate C
    /// in `scripts/colour_gate.py` FAILED on the shipped default (192.4, 176.7,
    /// 181.3 → G−B = −4.6 against a `SUN_GB_MIN` of 20). Saturation is the knob
    /// that pulls G back off B, so this fixes the gate and the axes together.
    ///
    /// BOUNDED ABOVE BY THE PICTURE, NOT BY THE AXES — and that bound is the
    /// whole finding. `scripts/_flamingo_grade_sweep.sh` walked 1.05 → 3.10 on
    /// this scene. The P0 chromatic axes do not clear until ≈1.75 (warmth 123.1,
    /// blue 12.5, sat 90.6) and only fully at the 2026-08-01 prescription's
    /// B-drained lights (1.90 → warmth 151.8, blue 6.6, sat 96.0) — but from
    /// ≈1.45 up the Edhari ruin stops reading as sunlit limestone and starts
    /// reading as a mustard poster, and the one block material whose albedo is
    /// already `R > B > G` turns from a soft warm pink into a violet slab. 1.35
    /// is the last rung where the frame still reads as stone at golden hour
    /// (warmth 95.7, blue 53.0, sat 64.7 — see
    /// `docs/look-audit-2026-08-05-flamingo.md` for the sweep table and frames).
    ///
    /// The remaining gap to warmth ≥110 / blue ≤10 / sat ≥90 is NOT reachable
    /// from this constant — and 1.35 → 1.90, 2026-08-05, does not contradict that,
    /// it depends on it. The vista sweep (`scripts/vista_grade_sweep.sh`,
    /// 21 rendered rows) closed the gap with [`TEMPERATURE`] and the haze hue;
    /// saturation alone still tops out at warmth ≈58 no matter how far it is
    /// pushed (row `v06-hz2s205`, sat 2.05 → warmth 58.0). What changed is that
    /// once the other two levers carry the warmth, 1.90 is no longer buying
    /// mustard: rows `v21`/`v23` sit at 123.1 and 119.9 warmth with the ruin still
    /// reading as lit stone, because the chroma is coming from the light and the
    /// air rather than from a global multiplier on an already-green frame.
    ///
    /// WHY SATURATION CANNOT SUPPLY WARMTH HERE, measured rather than argued:
    /// `scripts/band_map.py` shows 60 % of the vista midtone band is
    /// green-dominant grass, and on those pixels saturation pushes RED DOWN —
    /// green-px mean R fell 65.1 → 41.3 across sat 1.05 → 1.35. It is the right
    /// knob for the `sat` axis and the wrong one for `warmth`; treating them as
    /// one knob is what stalled this at 1.35 for a day.
    ///
    /// 1.90 → 1.02, 2026-08-09 (gamut-clip retraction). Every "warmth"/"sat" win
    /// logged above was measuring damage, not colour: Bevy's `post_saturation` runs
    /// AFTER the tonemap with no clamp (`saturation()` in
    /// `bevy_core_pipeline::tonemapping::tonemapping_shared.wgsl` —
    /// `color = luma + s * (color - luma)`), and on a green-dominant pixel `luma`
    /// sits near `G` while raw `B` is already small, so `s` this far above 1.0 drives
    /// `B` negative and the swapchain hard-clips it to 0. `scripts/_rose_gamut_clip_probe.py`
    /// on the shipped 1.90 found the midtone band's blue channel railed at 0 on
    /// 99.9 % of `grade-vista`, 99.7 % of `hero`, 55.5 % of `s4-raking`, 39.5 % of
    /// `s1-vista` — against 18.8 % on the accepted golden ref — which is exactly
    /// what turns the Edhari ruin into the "mustard poster" ¶520 already warned
    /// about; the HSV-based `saturation(mid)` axis scored that damage as a WIN
    /// because a channel clipped to 0 reads as `(max−0)/max = 100 %` saturated.
    ///
    /// NOT A CUSTOM SHADER FIX, ON PURPOSE. A per-pixel hue-preserving desaturate-
    /// to-fit (scale the `luma + s·(color−luma)` excursion down until the minimum
    /// channel lands at exactly 0 instead of punching through it) is the textbook
    /// correct answer, but `post_saturation` is baked into the `bevy_core_pipeline`
    /// crate (crates.io, not vendored) — doing it properly means a brand-new
    /// full-screen post-process pass after the tonemap, and this codebase has zero
    /// prior art for a custom render-graph node. That is real work with its own
    /// regression surface, not an urgent-lane edit; the scalar pulled back here is
    /// the fix that is provably safe today.
    ///
    /// THE NUMBER, SWEPT LIVE VIA `VOXELFORGE_LOOK_GRADE`, NO REBUILD PER RUNG:
    /// mid-`B==0%` on grade-vista/hero/s1-vista/s4-raking —
    /// 1.90 → (99.9, 99.7, 39.5, 55.5), 1.30 → (68.4, 70.5, 12.2, 26.3),
    /// 1.05 → (34.1, 51.2, 0.3, 6.7), 1.02 → (2.7, 23.5, 0.0, 2.1), 1.00 (neutral)
    /// → (0, 0, 0, 0). `hero` is the resistant plate: its clip jumps from 0 % at
    /// 1.00 to 51 % at 1.05, so any value between them is on a knife-edge — 1.02
    /// is the highest rung that keeps every plate clear of the 35 % target, all
    /// four with room (worst case `hero` 23.5 %, better than the golden ref's own
    /// 18.8 % once rounding is accounted for). Below 1.35 was flagged as
    /// "reading as a mustard poster" in the note above, so the frames were
    /// re-inspected at 1.02, not just re-measured: `s1-vista` and `grade-vista`
    /// read as sunlit limestone with real tonal range, not the flat orange wash
    /// 1.90 produced — the "washed" verdict this constant has carried since 1.05's
    /// first outing predates [`TEMPERATURE`] 0.02→0.05, the ambient/sky_gain
    /// lifts and the `ev100` moves, which now carry the warmth identity that used
    /// to be asked of this one knob alone.
    pub const POST_SATURATION: f32 = 1.02;

    /// Midtone contrast — spreads values off mid-grey, which is the
    /// micro-contrast / voxel-grain axis. Lit wood grain and edge detail live in
    /// the midtones.
    ///
    /// 1.30 → 1.12 with the tonemapper change. AcesFitted bakes a strong S-curve
    /// in and then washes the midtones back out, so the old value was buying that
    /// back; Tony leaves the midtones where the lighting put them, and 1.30 on
    /// top of it crushes open shade toward black — the exact G3 failure the
    /// ambient ladder was raised to fix.
    pub const MIDTONE_CONTRAST: f32 = 1.12;

    /// Highlight contrast, matched to the midtones so the curve doesn't kink at
    /// the section boundary.
    pub const HIGHLIGHT_CONTRAST: f32 = 1.12;

    /// Highlight roll-off (gain < 1) — compresses ONLY the brightest surfaces
    /// (sky, sunlit wedges, window panes) back into band while leaving
    /// bounce-lit shade and midtones alone. Lighting can't do this, because "less
    /// light" moves shadows down too.
    ///
    /// 0.64 → 0.86: 0.64 was a *hard* shoulder compensating for ACES already
    /// blowing sunlit voxels out. Under Tony the highlights arrive far less
    /// clipped, and holding 0.64 turns the sky into grey-blue putty instead of
    /// the reference's deep saturated blue.
    pub const HIGHLIGHT_GAIN: f32 = 0.86;

    // ── v3-only tonal ramp — 2026-08-15 ────────────────────────────────────
    // The four constants above are shared by BOTH generations, which is why
    // they have never been the lever for a v2/v3 A/B: moving one moves the
    // "before" plate too and the pair stops being a pair. These three are
    // reached only through `v3()` in [`base_camera_look`], so `gen=v2` still
    // reproduces the 2026-08-14 frame byte-for-byte.

    /// Shadow-section GAIN, v3 (shared default: 1.0, i.e. untouched).
    ///
    /// THE SECTION THAT WAS NEVER ALLOWED TO MOVE. The note on the
    /// `ColorGrading` literal records shadows being held neutral because
    /// *contrast* on this section crushed open shade to black (13.7 % → 3.3 %
    /// on the hero plate). That measurement is about CONTRAST, which pivots
    /// about the section midpoint and therefore drags the bottom down. `gain`
    /// is a multiply: it can only move the toe UP.
    ///
    /// Which is what the frame needs. Against `docs/assets/golden-beauty-shot-ref.png`
    /// the single loudest difference is not that this lane's shadows are too
    /// soft or too hard — it is that they bottom out. The reference's darkest
    /// cabinet recess still carries hue and readable edges; `night-firelit_after`
    /// has a whole wall at effectively zero and both day plates have unlit
    /// faces reading as holes in the frame. 1.08 lifts the toe 8 % after the
    /// tonemap, on top of the ~330 lux [`super::IBL_NITS_DAY`] puts back into
    /// the same pixels before it — light first, curve second.
    pub const SHADOW_GAIN_V3: f32 = 1.08;

    // THE MEASUREMENT THE NEXT THREE CONSTANTS ARE SIZED OFF.
    //
    // Luminance percentiles, `docs/assets/golden-beauty-shot-ref.png` against
    // the three v4 after-plates, same grader, same `Rec.709` luma:
    //
    //   plate                  p05     p50     p95    p95/p05   L<8    ch>=250
    //   golden-beauty-ref     21.86   55.51  165.83     7.59    0.00%   4.89%
    //   outdoor-noon_after    48.47   93.01  141.75     2.92    0.27%   0.28%
    //   evening-raking_after  18.68   71.96  120.60     6.45    1.89%   0.36%
    //   night-firelit_after   19.63   39.27  155.49     7.92    0.08%   3.48%
    //
    // TWO THINGS IN THAT TABLE ARE NOT WHAT LOOKING AT THE FRAMES SUGGESTED.
    //
    // First, NOTHING IN THIS LANE CLIPS. The scattered flat-white blocks on
    // `outdoor-noon_after` and the campfire core on `night-firelit_after` read
    // as blown, and they are not: 0.28 % and 3.48 % of pixels have any channel
    // at ≥250, against the reference's own 4.89 %. The reference clips MORE.
    // A shoulder to "recover" highlights would have been a fix for a defect
    // that does not exist, and would have made the real one worse.
    //
    // Second, THE REAL DEFECT IS RANGE, AND IT IS WORST WHERE THE FRAME LOOKS
    // CLEANEST. `outdoor-noon` has a p95/p05 of 2.92 against the reference's
    // 7.59 — everything in it is packed into one milky mid band, which is what
    // "flat, not AAA" actually looks like in numbers. The two evening/night
    // plates already sit near the reference's ratio, but they bought it from
    // the WRONG END: p05 18.68 / 19.63 and 1.89 % of the evening frame below
    // L=8, against a reference that has literally zero pixels there. The ratio
    // is right and the floor is wrong.
    //
    // So the ramp gets widened at the top and floored at the bottom, and the
    // floor is bought with LIGHT ([`super::IBL_NITS_DAY`]) before the curve is
    // asked to do anything.

    /// Midtone contrast, v3 (shared: [`MIDTONE_CONTRAST`] 1.12).
    ///
    /// The p50 row is the whole argument: the reference sits at 55.51 with a
    /// p95 of 165.83, `outdoor-noon_after` at 93.01 with a p95 of 141.75. The
    /// midtones are not just narrow, they are riding high enough to leave no
    /// room above them. 1.18 spreads that band. Not 1.30 — the shared note
    /// above records 1.30 crushing open shade toward black, and while
    /// [`SHADOW_GAIN_V3`] now puts a floor under exactly that failure, a floor
    /// is a reason to step toward the cliff, not to jump off it.
    ///
    /// 1.18 → 1.10, 2026-08-15. THIS KNOB WAS THE SEPARATION REGRESSION, AND IT
    /// MOVES THE OPPOSITE WAY FROM WHAT THE 1.18 NOTE ABOVE ASSUMES. v5 shipped
    /// `evening-raking` separating WORSE after than before (71.66 → 70.46) and
    /// the write-up guessed [`super::CONTACT_SHADOW_LENGTH_V3`] paid for it.
    /// It did not. Swept on one binary through `VOXELFORGE_LOOK_GRADE`
    /// (`scripts/_poppy_lookv6_sweep.sh`, dirs `_poppy_lookv6/n4`,`n5`,`e1`), the
    /// spread estimator against this constant is monotone DOWNWARD:
    ///
    ///   midtone contrast   1.50   1.42   1.34   1.26   1.18   1.10   1.00
    ///   night spread      70.53  75.99  80.92  85.16  89.32  93.07  99.08
    ///
    /// (one base for every rung — `_poppy_lookv6/n3..n5`, night-firelit, warm
    /// ambient held, only this knob moving. The same ladder shot on the SHIPPED
    /// v5 rig instead reads 82.75 / 91.13 / 94.99 / 100.83 at 1.34 / 1.18 / 1.10
    /// / 1.00, i.e. the slope is the constant's, not the base's.)
    ///
    /// Bevy grades in sections, and on these plates the p25 band the estimator
    /// takes its dark median from sits INSIDE the midtone section while the p75
    /// band has already crossed into the highlights — so pushing midtone
    /// contrast up lifts the dark end of the measurement and leaves the bright
    /// end where it is, closing the gap the knob was raised to open. 1.10 buys
    /// back +4.02 of evening separation (70.34 → 74.36 on one binary, everything
    /// else held) and +3.75 at night, and it is the ONLY lever measured this
    /// round that adds separation without spending the shade floor: p05 moved
    /// 21.69 → 21.57 evening and 21.78 → 21.77 night across that same step.
    ///
    /// NOT 1.00, which measures better still (+8.29 night spread). 1.00 is no
    /// midtone contrast at all — the section becomes a pass-through and the p50
    /// argument at the top of this note, which is still true, goes unanswered.
    /// 1.10 is the last rung that keeps a midtone push while giving separation
    /// back, and it is a retreat from 1.18, not from 1.12: it lands under the
    /// shared [`MIDTONE_CONTRAST`] because the v3 stack has [`SHADOW_GAIN_V3`]
    /// and [`HIGHLIGHT_CONTRAST_V3`] widening the ends that v2 asked this one
    /// knob to widen alone.
    pub const MIDTONE_CONTRAST_V3: f32 = 1.10;

    /// Highlight contrast, v3 (shared: [`HIGHLIGHT_CONTRAST`] 1.12).
    ///
    /// 1.12 was picked to match the midtones "so the curve doesn't kink at the
    /// section boundary" — a smoothness argument with no highlight measurement
    /// under it. Now there is one, and it says the top end is SHORT: the
    /// reference runs p95 → p99 165.8 → 227.0 (a 61-point span), this lane's
    /// noon plate 141.8 → 179.2 (37 points). 1.20 lengthens that span, and it
    /// is safe to do because the clip column above says there is 20 % of the
    /// range sitting unused above the brightest pixel in frame.
    pub const HIGHLIGHT_CONTRAST_V3: f32 = 1.20;

    /// Highlight gain, v3 (shared: [`HIGHLIGHT_GAIN`] 0.86).
    ///
    /// 0.86 is a shoulder, and the shared note is explicit that it exists to
    /// compress "the brightest surfaces (sky, sunlit wedges, window panes) back
    /// into band". The table above is what "into band" cost: the brightest
    /// channel anywhere on `outdoor-noon_after` is 251 and only 0.28 % of the
    /// frame is within 5 of the top, so the compression is being applied to
    /// headroom nothing was using. 0.98 gives it back.
    ///
    /// NOT 1.00. The shared note's other half is a real bound — 0.64 "turns the
    /// sky into grey-blue putty", i.e. this knob is also what keeps the sky's
    /// saturation off the clip, and G5 grades the window gradient's spread on a
    /// plate that can go achromatic if all three channels rail. 0.98 stays a
    /// shoulder, just a shoulder sized to a measured top end instead of an
    /// assumed one.
    ///
    /// 0.98 → 1.06, 2026-08-15. The note above says 0.98 is a shoulder sized to
    /// a measured top end; the same measurement re-run a round later says the
    /// top end is still not being reached, and this is the cheapest warmth on
    /// the sheet. One binary, only this knob moving (`_poppy_lookv6/e3`,`o2`,
    /// `n7`): evening-raking midtone R−B 80.73 → 81.92 and its shade floor
    /// 21.12 → 21.55 — warmth AND floor, up together, which no other lever this
    /// round managed. It goes past 1.00 because the shoulder is what was costing
    /// the warmth: the highlight section is where the warm key's own band lands,
    /// and compressing it was pulling R down toward B on exactly the pixels that
    /// carry the hour's colour.
    ///
    /// THE BOUND IS NOW ON THE OTHER SIDE, AND IT IS NOT THE SKY. The 0.98 note
    /// keeps 1.00 as a ceiling for fear of railing all three channels achromatic;
    /// measured, the sky is not what moves — noon's separation is what pays,
    /// 74.32 → 73.39 across this step, because a lifted highlight section pulls
    /// the bright end of the frame together as it clears the shoulder. Noon has
    /// +7.57 of separation margin to spend and evening's warmth clause has failed
    /// two rounds running, so the trade is taken deliberately and in that
    /// direction. Past ~1.10 it would be spending margin that is not there.
    pub const HIGHLIGHT_GAIN_V3: f32 = 1.06;

    /// White balance, v3 (shared: [`TEMPERATURE`] 0.05).
    ///
    /// THE ONLY LEVER THAT MOVES EVENING WARMTH, AND v5's PLAN HAD IT LAST.
    /// `docs/look-gap-v5-2026-08-15.md` reasoned that evening's −4.20 warmth was
    /// the day kicker ([`super::RIM_LUX`] 600, unshadowed, cool) and that a white
    /// balance was the backstop. Swept on one binary, that is backwards. Dimming
    /// the kicker does not warm the frame — it cools it:
    ///
    ///   evening-raking, one exe, only `VOXELFORGE_LOOK_RIM` moving
    ///   rim lux      600    420    300
    ///   warmth     74.06  73.93  74.45      (bar: 79.01)
    ///
    /// Every light-side lever measured the same way came back inside a point:
    /// ambient 620 → 1150 bought +1.40, the sky-fill/bounce split 1400/1100 →
    /// 800/1700 bought +1.34. The kicker was never the payer, so trimming it
    /// spends separation for nothing. This constant moves warmth 74.06 → 80.68 in
    /// one step, and it is the only thing on the sheet that clears the bar.
    ///
    /// 0.07 IS ONE RUNG UNDER A CLIFF AND THE CLIFF IS MEASURED, NOT INHERITED.
    /// The shared [`TEMPERATURE`] note pins the ceiling at 0.05 off a 2026-08-01
    /// frame that ran `POST_SATURATION` 1.90; that note also says do not raise
    /// this without re-running `scripts/colour_gate.py` on a real frame, so it was
    /// re-run on four, shot through a sky-facing pose on this binary
    /// (`_poppy_lookv6/sky`, `VOXELFORGE_CINE` look-at raised to y=30):
    ///
    ///   temperature      0.05    0.07    0.08    0.09
    ///   magenta frac    0.01%   0.05%  23.62%  69.38%
    ///   colour gate      PASS    PASS    FAIL    FAIL
    ///
    /// 0.05 → 0.07 is 40× under the 2 % bar; 0.08 is over it by 12×. The onset
    /// really is that sharp, which is why this is 0.07 and not 0.075 and why the
    /// next round does not get to nudge it "a little more" without shooting that
    /// sweep again. Gate B (sky ordering / sky gain) SKIPPED on all four frames —
    /// the flat `ClearColor` it reads has been replaced by the sky dome, so the
    /// check that originally caught the magenta cast is structurally unmeasurable
    /// now and Gate A is carrying it alone. Disclosed, not glossed.
    ///
    /// V3-ONLY, and that is the whole reason it is a separate constant: the
    /// before plate of every pair is v2, [`TEMPERATURE`] is shared, and a white
    /// balance that moved both sides would move the bar and the reading together
    /// and measure nothing. Same shape as [`MIDTONE_CONTRAST_V3`], forked in
    /// [`super::grade_knobs`] rather than at the `ColorGrading` literal so
    /// `VOXELFORGE_LOOK_GRADE` still overrides it under both generations.
    pub const TEMPERATURE_V3: f32 = 0.07;
}

/// The hour of the day, as ONE object.
///
/// WHY THESE TRAVEL TOGETHER. Sun angle, sun colour, sky clear colour, bounce
/// fill and exposure are not five independent knobs — they are five faces of one
/// decision ("it is golden hour" / "it is night"). Ship them separately and you
/// get frames that argue with themselves: a low warm key over a noon-blue sky, or
/// a night scene at daylight exposure where the moon reads as an overcast
/// afternoon. Bundling them means a preset switch can never land half-applied.
///
/// WHY THE LOOK LANE OWNS THEM AT ALL. `main.rs` spawns a white 9000-lux sun 59°
/// up and clears to a pale 0.53/0.72/0.92 sky — a perfectly reasonable *neutral*
/// for the editor and bench lanes, and not a time of day. "Raking golden-hour
/// sun" is the brief itself, so the hour is a look decision and lands here. All
/// of it is applied ONLY while the look lane is live (see [`look_enabled`]), so
/// bench, editor and the hero shot still see exactly what `main.rs` set.
#[derive(Clone, Copy, Debug)]
pub struct Hour {
    /// Sun elevation above the horizon, degrees.
    pub elev_deg: f32,
    /// Sun azimuth, degrees.
    pub azim_deg: f32,
    /// Sun illuminance, lux.
    pub illuminance: f32,
    /// Sun colour, sRGB. This is where the frame's warmth comes from: light
    /// multiplies the surfaces it reaches (so sunlit stone reads `R > G > B`,
    /// gate G6) and leaves the flat sky clear alone. A white-balance matrix
    /// cannot tell those two apart — that was the 2026-08-01 magenta bug.
    pub key: [f32; 3],
    /// Sky / `ClearColor` HUE, sRGB. Scaled by [`Self::sky_gain`] into the
    /// scene-referred radiance actually written to the target.
    pub sky: [f32; 3],
    /// Scene-referred gain on [`Self::sky`], applied in LINEAR space, in units
    /// where 1.0 is the tonemapper's white.
    ///
    /// WHY THIS EXISTS. `ClearColor` is written straight into the camera's HDR
    /// target and — unlike every lit surface — never passes through `Exposure`.
    /// Authored at sRGB `[0.36, 0.60, 0.90]` the sky's brightest channel lands
    /// at linear 0.79, i.e. UNDER the `Bloom` prefilter threshold of 1.0 (see
    /// [`base_camera_look`]), so the sky could never bloom and the tonemapper
    /// only ever had a sub-white value to roll off — a flat LDR plate pasted
    /// behind an HDR scene. Anything above `1/0.79 = 1.27` gives bloom and the
    /// tonemap actual range to work on.
    ///
    /// Bounded ABOVE by gate G5, not by taste: `grade_gate.py` finds the window
    /// as the brightest NEAR-NEUTRAL pixel (`|R-B| <= 45`). Push the sky until
    /// all three channels clip and it goes achromatic white, captures that
    /// locator, and G5's 3-point gradient then samples a flat clear colour —
    /// spread 0, FAIL. 2.4 puts blue at linear 1.89 (blooms) while red and
    /// green stay under 1.0, so the halo keeps the sky's own hue.
    ///
    /// The VALUE is set by the P0 highlight-p95 axis, because this — not
    /// `ev100` — is the lever that moves it: p95 is a GLOBAL percentile and the
    /// sky is the largest bright region in a vista frame, so an un-exposed sky
    /// pins it. Measured on the vista frame, holding the sky and sweeping
    /// exposure 11.0 -> 10.7 left p95 at 117.8 UNCHANGED; holding exposure at
    /// 10.8 and lifting the gain moved it 117.8 -> 162.8 (2.4) -> 174.5 (3.0)
    /// -> 215.9. 2.4 lands on the golden ref's own 165.8; 3.0 clears the 185
    /// ceiling by only 10.5, thin for a framing showing more sky than this one.
    pub sky_gain: f32,
    /// Bounce-fill (`AmbientLight`) colour, sRGB — the axis that moves the
    /// *midtone* numbers, since midtones are open shade and bounce, not the
    /// sunlit wedge.
    pub ambient: [f32; 3],
    /// Bounce-fill brightness, lux. In v2 this is the FLOOR only — the part of the
    /// fill that has no direction and therefore no business being large. The rest
    /// moved into [`Self::sky_fill_lux`] / [`Self::bounce_lux`], which do have one.
    pub ambient_lux: f32,
    /// Sky-fill colour, sRGB — the cool half of the fill, coming down from near
    /// zenith (see [`SKY_FILL_ELEV`]). This is what makes a block's top face read
    /// as a different surface from its own side face while both are in shade.
    pub sky_fill: [f32; 3],
    /// Sky-fill illuminance, lux. Zeroed under [`LookGen::V1`].
    pub sky_fill_lux: f32,
    /// Ground-bounce colour, sRGB — the warm half, travelling UP off the sunlit
    /// ground onto undersides and the lower half of sun-facing walls.
    pub bounce: [f32; 3],
    /// Ground-bounce illuminance, lux. Zeroed under [`LookGen::V1`].
    pub bounce_lux: f32,
    /// Camera exposure. Bevy's implicit default is `Exposure::BLENDER`
    /// (ev100 9.7), which the gameplay camera was silently running because
    /// nothing ever gave it one.
    pub ev100: f32,
    /// Distance-haze colour, sRGB.
    pub fog: [f32; 3],
}

impl Hour {
    /// The shipped default: late-afternoon raking sun under a vivid sky.
    pub const GOLDEN: Self = Self {
        // 17 -> 22 deg and 11_000 -> 22_000 lux, 2026-08-08. ONE decision, two
        // numbers, because they are two terms of the same product.
        //
        // THE FAULT. Every one of the 8 canonical plates rendered with a key
        // light that cast a real shadow map and a frame in which no shadow was
        // visible. It was not a broken shadow pass — `VOXELFORGE_LOOK_DISABLE=1`
        // on the SAME binary (main.rs's own 57-deg key over its 380-lux fill)
        // shoots crisp cast shadows on the same meshes. It was the RATIO. Ground
        // is horizontal, so it receives `illuminance * sin(elev)`, and open shade
        // receives [`Self::ambient_lux`], which no shadow map can attenuate
        // because `AmbientLight` is a flat term. At 17 deg / 11 000 lux that is
        // 11000*0.292 = 3212 against a 2200-lux fill: a sunlit-to-shadowed ratio
        // of **2.46**, half a stop and change. A shadow was being drawn and there
        // was almost nothing for it to subtract.
        //
        // WHY NOT CUT THE FILL, WHICH IS THE SHORTER LEVER. Measured: it works
        // and it is unshippable. `VOXELFORGE_LOOK_AMBIENT=200` on the shipped sun
        // opens the split outright (grass dip 0.000 -> 0.752). But the fill IS
        // G3's floor — the note on `ambient_lux` below records p05-L 2.2-3.8 %
        // at 1100 lux against an 8 % gate — so buying G2 that way spends G3.
        //
        // SO THE OTHER SIDE OF THE SAME RATIO WAS MOVED, AND THAT IS THE WHOLE
        // TRICK: shadowed ground is lit by ambient ALONE, so raising the key does
        // not move it at all. Measured across the illuminance ladder
        // (`scripts/_pixel_light_ladder.ps1`, s4 framing, one binary, only
        // `VOXELFORGE_LOOK_SUN` moving), the shade mode sat at L = 33.9 / 34.3 /
        // 34.7 / 34.4 / 34.7 across 20 k -> 45 k while the sun mode climbed
        // 52.2 -> 64.4. G3's floor is untouched by construction.
        //
        // BOUNDED ABOVE, AND THE BOUND IS MEASURED. Past ~24 k the sunlit ground
        // climbs THROUGH the sky plateau (L = 160.9, which `sky_gain`'s note
        // already pins as byte-constant), and `grade_axes`' midtone band [p35,p75]
        // starts sampling sky instead of ground: sky share of that band goes
        // 4.9 % (shipped) -> 11.5 % (this) -> 43.0 % (30 k) -> 59.6 % (45 k), and
        // warmth R-B "collapses" 135.9 -> 44.6 -> -20.0 as it does. That collapse
        // is the band migrating, not the ground going cold — but a frame whose
        // ground is as bright as its sky is wrong on its own terms, so 24 k is a
        // ceiling either way and 22 k sits under it.
        //
        // AND WHY THE ANGLE MOVED TOO, RATHER THAN ILLUMINANCE ALONE. Holding
        // 17 deg and buying the same ground irradiance from lux only needs 26 k,
        // which is over that ceiling: 17/26k measured warmth **101.8** on the
        // vista framing (FAIL) against **138.0** for 22/22k (PASS), because
        // elevation raises what the GROUND receives without raising the peak on
        // sunlit vertical faces. 22 deg is still a raking key — long shadows, the
        // thing the hour is named for — and every colour constant in this struct
        // is untouched.
        //
        // MEASURED RESULT, `scripts/grade_sunsplit.py`, s4 framing:
        // grass separation 5.8 -> 19.8 L, dip 0.048 -> 0.347 (bar 0.20, repeat
        // spread +-0.02), one hump -> two. Collateral on the same frame: warmth
        // 135.9 -> 133.5 (PASS both), p95 160.9 -> 161.6 (in the 150-185 band),
        // micro-contrast 6.2 -> 9.4, G3/G5/G6 all PASS.
        elev_deg: 22.0,
        azim_deg: 205.0,
        illuminance: 22_000.0,
        // G LIFTED, B HELD — 2026-08-05. `docs/gate3-colour-review-2026-08-01.md`
        // §5.4 rule 1 is the safety envelope that keeps a warm frame out of
        // magenta: `G − B >= 0.30` AND `G >= 0.85 × R`, on BOTH key and ambient.
        // The shipped hues violated the first clause — key G−B was 0.22, ambient
        // 0.18 — which is exactly why raising [`grade::POST_SATURATION`] on them
        // GREW the magenta fraction instead of shrinking it (measured: 4.8 % →
        // 7.5 % across sat 1.05 → 2.70 on the shipped hues).
        //
        // The 2026-08-01 prescription satisfied the rule by DRAINING B (key
        // 0.62 → 0.52, ambient 0.66 → 0.38). That was measured on the campsite,
        // and §5.5 flagged the deeper shade as the one open taste call. On the
        // Edhari ruin the answer to that call is no: the scene is
        // ambient-dominated pale limestone, so pulling blue out of the fill
        // turns the whole frame mustard (`_fl_grade_sweep/boot-r06-*`,
        // `boot-r15-*`). Lifting G instead satisfies the same rule from the
        // other side — key G−B 0.30, G/R 0.92; ambient G−B 0.30, G/R 0.94 — and
        // keeps the blue in open shade that makes the stone read as stone.
        key: [1.00, 0.92, 0.62],
        sky: [0.36, 0.60, 0.90],
        sky_gain: 2.4,
        // B DRAINED 0.60 → 0.48 and LUX DOUBLED 1100 → 2200 — 2026-08-06, the
        // one knob the AAA scorecard (`docs/aaa-gap-scorecard-2026-08-06.md`)
        // ranked 1st AND 2nd. Ambient is the only light in open shade, so it
        // alone decides what the darkest 5 % of a frame looks like, and at 1100
        // lux that band measured p05-L 2.2–3.8 % against G3's 8 % floor: the
        // shade was not dark, it was CRUSHED — no detail left to grade. Lux is
        // the fill knob (`ev100` above moves the SUNLIT patch and is spoken
        // for by G6's floor), so it is this number's job alone.
        //
        // The same lift is the warmth lever the grade could not supply: with
        // `TEMPERATURE` at its 0.05 magenta ceiling the midtone band still read
        // R−B +80 to +99 against +110, and `POST_SATURATION` pushes R DOWN on
        // the 60 % of that band which is green-dominant grass (see the note on
        // that constant). Raising the fill raises R in shade directly, and
        // draining its B stops the open shade reading as sky-blue wash — the
        // vista frame's darkest shade measured R−B −4, i.e. COLD, the only
        // frame to fail G3's hue clause as well as its level clause.
        //
        // 0.48 keeps the §5.4 magenta envelope with room to spare: ambient
        // G−B 0.42 (floor 0.30), G/R 0.94 (floor 0.85).
        //
        // Reverted here from the 0.45 that 968f7f7 drained it to (2026-08-07).
        // That change was reasoned, not measured: "every 0.01 shaved off B buys
        // the warmth gap 1:1" is true only for a pixel lit by ambient ALONE, and
        // on these plates the warmth deficit lives in the SUNLIT midtone band,
        // where the key dominates and a 0.03 shift in the fill's blue is lost in
        // the rounding. The warmth gap closes on exposure instead — see the
        // ev100 note below, where hero's midtone R−B goes 109.2 -> 131.3 on the
        // exposure knob alone. Restored to the value with the longer history so
        // this release moves exactly ONE knob; claim no warmth credit for it.
        // Sweep with `VOXELFORGE_LOOK_LIGHT` / `VOXELFORGE_LOOK_AMBIENT` before
        // moving again — the hero shot's own fill sits at 2800 lux, so 2200 is
        // deliberately short of that.
        ambient: [0.96, 0.90, 0.48],
        // 2200 → 1150, and the missing 1050 did not go away — it went DIRECTIONAL
        // (see `sky_fill_lux`/`bounce_lux` below and [`SKY_FILL_ELEV`] for why).
        // The irradiance budget is roughly conserved on the surface that had the
        // most of it, and deliberately NOT conserved on the ones that should never
        // have had as much:
        //
        //   surface (in shade)      v1 flat    v2 flat + sky + bounce
        //   shaded ground           2200       1150 + 1358 +    0  = 2508
        //   wall, shadow side       2200       1150 +  339 +    0  = 1489
        //   wall, sun side          2200       1150 +    0 +  971  = 2121
        //   underside               2200       1150 +    0 +  516  = 1666
        //
        // Every one of those four used to be the SAME number, which is the whole
        // reason a voxel scene under this rig read flat: orientation is the only
        // shape cue a cube has, and a flat fill spends it.
        ambient_lux: 1150.0,
        // Cool, because it stands in for the sky and the sky is blue even at 22°
        // sun — `Hour::sky` is [0.36, 0.60, 0.90]. Held a little paler than the sky
        // itself so the shaded top faces read as "lit by sky", not "painted blue".
        sky_fill: [0.62, 0.76, 1.00],
        sky_fill_lux: 1400.0,
        // Warm, because it is the sun's own light returning off ground the sun has
        // already tinted — bounce is always the colour of what it bounced off, and
        // at this hour that is amber grass and dirt.
        bounce: [1.00, 0.78, 0.50],
        bounce_lux: 1100.0,
        // 11.0 was this lane's own value and it cost 1.3 stops against Bevy's
        // implicit `Exposure::BLENDER` (9.7): measured on the vista frame it
        // held the brightest sunlit patch at RGB(209,124,55), L=53.9, under
        // G6's `L >= 55` sunlit floor. Exposure is the ONLY axis that moves
        // that patch — `sky_gain` above barely touches it (L 53.9 -> 54.5
        // across the whole usable gain range), so the sunlit floor is this
        // number's job alone.
        //
        // 10.8, not 9.7: 9.7 overshoots to L=69.9 but bleaches the frame's
        // identity getting there — the same patch goes (241,167,108), R-B 133
        // against the shipped 155, and the window's G5 gradient spread collapses
        // 56.7 -> 35.8. 10.8 clears the floor at L=57.1 with R-B 151 and spread
        // 53.6: the gate is passed without spending the golden-hour warmth that
        // G6's own hue clause exists to protect.
        //
        // 10.3 (2026-08-07, measured): ev100 is an EXPOSURE VALUE, so it runs
        // BACKWARDS — a HIGHER number is LESS light. The 10.8 -> 10.9 change that
        // sat here called itself "a lift ... for the AAA p95 axis", but raising
        // ev100 DARKENS the frame and pushes p95 DOWN, away from the 150 floor it
        // was trying to reach. It moved the axis the wrong way. (The rest of this
        // comment block always had the sign right: 9.7 is described as brighter
        // than 10.8, which is why L=69.9 there and 57.1 here.)
        //
        // Measured on the frozen 05:38 build with only VOXELFORGE_LOOK_EXPOSURE
        // moving, three plates x three rungs (10.9 / 10.6 / 10.3):
        //
        //             p95 @10.9 -> @10.3      warmth R-B @10.9 -> @10.3
        //   gate3-walk   115.2 -> 137.8          112.6 -> 130.0
        //   hero          90.3 -> 100.7          109.2 -> 131.3  (crosses >=110)
        //   s1-vista     160.9 -> 160.9          157.7 -> 169.7
        //
        // Every axis that responds improves monotonically toward 10.3 and nothing
        // regresses, so 10.3 is the floor of the swept range, not a compromise
        // inside it. s1-vista's p95 does not move because it is not measuring the
        // scene: 99% of that frame's 95th-percentile population is sky, and the
        // sky plateau (132,160,255, L=160.9062) is byte-identical at all three
        // rungs. p95 is only an exposure axis on plates whose highlights are lit
        // geometry — which is why the number is quoted per-plate here.
        //
        // Blow-out was measured directly rather than left to G5, which is broken
        // on all three plates (it grades the campfire on hero, sky haze on
        // s1-vista, and the y=0 crop seam on gate3-walk — see grade_hero.py's
        // deprecation note). At 10.3: ZERO pixels with all of R,G,B >= 250, and
        // single-channel R clip <= 1.22%, which G5's own rubric row explicitly
        // allows in golden hour. Nothing is spent to buy the exposure.
        ev100: 10.3,
        fog: FOG_COLOR_DAY,
    };

    /// `VOXELFORGE_LOOK_NIGHT=1`. A cool moon key kept just strong enough to hold
    /// a shadow direction, so lanterns and the campfire are the only warm sources
    /// in frame — which is the whole point of a night frame for a bloom that is
    /// supposed to fire on emissive surfaces and nothing else.
    pub const NIGHT: Self = Self {
        elev_deg: -8.0,
        azim_deg: 205.0,
        illuminance: 260.0,
        key: [0.55, 0.66, 0.95],
        sky: [0.03, 0.05, 0.12],
        // Deliberately 1.0, i.e. stays LDR. The whole point of a night frame is
        // that lanterns and the campfire are the ONLY things above the bloom
        // threshold; giving the night sky HDR headroom would light a halo
        // around the skyline and undo it.
        sky_gain: 1.0,
        ambient: [0.42, 0.52, 0.78],
        // Same split as GOLDEN, at night's scale. The BSL night reference
        // (`_poppy_lookv2/ref/bsl-01.jpeg`) is the clearest case for it in the whole
        // set: the sandstone rooftops there catch a cold sky while the wall faces
        // below them fall away into near-black, and the ONLY warm light in frame is
        // the torches. A flat 90-lux fill cannot draw that — it lifts the roof and
        // the wall by the same amount.
        ambient_lux: 42.0,
        sky_fill: [0.44, 0.58, 0.98],
        sky_fill_lux: 62.0,
        // Barely there: at night there is no sunlit ground to bounce off. It exists
        // to keep undersides from going to pure fill-black, nothing more.
        bounce: [0.55, 0.56, 0.68],
        bounce_lux: 16.0,
        // STAYS 7.5. An 8.6 was tried and MEASURED AND REJECTED; the numbers are
        // kept here so the next person does not spend the same six boots on it.
        //
        // The A/B was single-binary: `VOXELFORGE_LOOK_EXPOSURE` is applied after
        // the `Hour` constant is picked, so one exe shoots both legs and only
        // this float moves. The no-lever leg printed `ev100=8.60` against the
        // lever leg's `ev100=7.50` at runtime, which is what proves the lever and
        // this constant are the same knob (log pinned at
        // `_poppy_ev100night_pinned/_shoot.log`).
        //
        // 8.6 fails the v7 measurability bars outright, on night-firelit:
        //
        //                p05      warmth   separation   band
        //   7.5        24.46       46.41        91.99  32.75
        //   8.6        19.26       38.51        51.36  13.07
        //   bar     >= 20.40   >= before    >= before
        //
        // Raising ev100 darkens, so the obvious correction is to go the other
        // way -- and a 5-rung ladder (8.6/7.5/7.0/6.5/6.0, all on one binary,
        // `scripts/_poppy_ev100_night_sweep.*`) does clear every bar all the way
        // down to 6.0, with p05/warmth/separation rising monotonically. THAT
        // LADDER MUST NOT BE READ AS "6.0 WINS". `_poppy_lookv7_gate.py` says in
        // its own docstring that its clauses ask "does the ESTIMATOR still have a
        // signal to work with", not "does it look nice" -- they are monotone in
        // exposure, so they nominate the bottom rung of whatever range is swept.
        // They can reject a value; they cannot pick one.
        //
        // What picks one is the reference this comment already named below: the
        // BSL night plate, whose property is tonal, not per-pixel -- most of the
        // mass dark, a small isolated bright tail. Scored as EMD between luma
        // histograms (`scripts/_poppy_ev100_night_anchor.py`), distance to the
        // reference gets WORSE the further the exposure is ridden down:
        //
        //   rung        6.0     6.5     7.0     7.5    8.6   | day control
        //   meanEMD  11.277   8.981   6.856   4.927  2.452   |     12.253
        //   dark%      2.64    6.75   12.05   17.01  27.30   |      4.76
        //
        // At 6.0 the frame is nearly as far from the night reference as a NOON
        // frame is (11.277 vs 12.253) and the near-black wall faces this comment
        // asks for are gone (dark% 17.01 -> 2.64). The two metrics are opposed,
        // so the value has to maximise night-ness SUBJECT TO the bars holding --
        // and because warmth/separation are graded as "no worse than 7.5",
        // monotonicity excludes every darker rung, while every brighter rung buys
        // measurability by spending the night. The constrained optimum inside the
        // swept range is the value that was already here.
        ev100: 7.5,
        fog: FOG_COLOR_NIGHT,
    };

    /// Unit vector pointing FROM the sky TO the scene — what a `DirectionalLight`
    /// transform has to look along.
    fn sun_dir(&self) -> Vec3 {
        let e = self.elev_deg.to_radians();
        let a = self.azim_deg.to_radians();
        Vec3::new(a.sin() * e.cos(), -e.sin(), a.cos() * e.cos()).normalize()
    }

    /// Travel direction of the sky fill — near-zenith, leaning in from the side of
    /// the sky OPPOSITE the sun (`azim + 180`).
    ///
    /// Opposite, not with: the faces that need a fill are the ones the key cannot
    /// reach, and those face away from the sun. Putting the fill on the sun's own
    /// side would pile a second light onto the faces that are already the brightest
    /// in frame and leave the dark ones exactly as dark.
    fn sky_fill_dir(&self) -> Vec3 {
        let e = SKY_FILL_ELEV.to_radians();
        let a = (self.azim_deg + 180.0).to_radians();
        Vec3::new(a.sin() * e.cos(), -e.sin(), a.cos() * e.cos()).normalize()
    }

    /// Travel direction of the ground bounce — UPWARD (`+Y`), horizontally along
    /// the sun's own heading.
    ///
    /// Sharing the sun's azimuth is what puts it on the opposite side to
    /// [`Self::sky_fill_dir`]: the two fills then split the frame's shaded faces
    /// between them by orientation, warm on the sun side and cool on the shadow
    /// side, instead of stacking on one side and leaving the other on the flat
    /// floor alone.
    fn bounce_dir(&self) -> Vec3 {
        let e = BOUNCE_ELEV.to_radians();
        let a = self.azim_deg.to_radians();
        // `+e.sin()` on Y, where `sun_dir` has `-e.sin()`: this one goes up.
        Vec3::new(a.sin() * e.cos(), e.sin(), a.cos() * e.cos()).normalize()
    }
}

/// The live hour, plus the env overrides used to sweep it without a rebuild.
///
/// Unset env ⇒ the constants byte-for-byte, which is what every gate run and the
/// shipped binary get.
/// `VOXELFORGE_LOOK_NIGHT` — which of the two [`Hour`] constants is live.
///
/// Hoisted out of [`hour`] because the v3 rig sizes two of its own numbers off
/// the same switch ([`IBL_NITS_DAY`], [`RIM_LUX`]) and reading the variable in
/// three places is three places for the spelling to drift.
fn is_night() -> bool {
    std::env::var_os("VOXELFORGE_LOOK_NIGHT").is_some()
}

fn hour() -> Hour {
    let night = is_night();
    let mut h = if night { Hour::NIGHT } else { Hour::GOLDEN };
    // The generation fork, applied BEFORE the env overrides below so
    // `_LOOK_AMBIENT` still wins over it — a sweep hook that a generation switch
    // could silently override would be worse than no hook.
    match look_gen() {
        LookGen::V1 => {
            h.ambient_lux = if night {
                AMBIENT_LUX_V1_NIGHT
            } else {
                AMBIENT_LUX_V1
            };
            h.sky_fill_lux = 0.0;
            h.bounce_lux = 0.0;
        }
        LookGen::V2 => {}
        // v3 moves most of what is left of the FLAT term into the hemispherical
        // environment light — same irradiance, now arriving from a direction.
        // The two directional fills are untouched: they are already directional,
        // and the budget table on [`IBL_NITS_DAY`] is written against them.
        LookGen::V3 => {
            h.ambient_lux = if night {
                AMBIENT_LUX_V3_NIGHT
            } else {
                AMBIENT_LUX_V3
            };
            // Night only: the flat term's COLOUR is forked too, because at night
            // it is the rig's one warm-capable light and the shared `Hour::NIGHT`
            // value is cool. Day keeps `Hour::GOLDEN`'s own ambient — it is
            // already the warmest thing in the rig, which is the whole reason
            // [`AMBIENT_LUX_V3`] is spent on it. See [`AMBIENT_COLOR_V3_NIGHT`].
            if night {
                h.ambient = AMBIENT_COLOR_V3_NIGHT;
            }
        }
    }
    if let Some([elev, azim, illum]) = env_floats::<3>("VOXELFORGE_LOOK_SUN") {
        h.elev_deg = elev;
        h.azim_deg = azim;
        h.illuminance = illum;
    }
    if let Some(ev) = std::env::var("VOXELFORGE_LOOK_EXPOSURE")
        .ok()
        .and_then(|v| v.trim().parse().ok())
    {
        h.ev100 = ev;
    }
    if let Some([r, g, b]) = env_floats::<3>("VOXELFORGE_LOOK_SKY") {
        h.sky = [r, g, b];
    }
    // `VOXELFORGE_LOOK_SKYGAIN=<gain>` — the ONE lever the sky's brightness (and
    // therefore the p95/bloom axes it pins) had no env handle for. `_LOOK_SKY`
    // above only swaps the HUE; `sky_gain: 2.4` was unreachable without a rebuild,
    // so an A/B of the sky's brightness was impossible from one binary. One float,
    // same parse family as `_LOOK_EXPOSURE`.
    if let Some(g) = std::env::var("VOXELFORGE_LOOK_SKYGAIN")
        .ok()
        .and_then(|v| v.trim().parse().ok())
    {
        h.sky_gain = g;
    }
    if let Some(lux) = std::env::var("VOXELFORGE_LOOK_AMBIENT")
        .ok()
        .and_then(|v| v.trim().parse().ok())
    {
        h.ambient_lux = lux;
    }
    // `VOXELFORGE_LOOK_FILL=<sky_lux>,<bounce_lux>` — the same sweep-without-relink
    // hook `_LOOK_AMBIENT` is for the flat term, for the two directional ones. The
    // fill rig's whole claim is a RATIO between three numbers, and a ratio that can
    // only be re-tried by rebuilding is a ratio nobody re-tries.
    if let Some([sky, bounce]) = env_floats::<2>("VOXELFORGE_LOOK_FILL") {
        h.sky_fill_lux = sky;
        h.bounce_lux = bounce;
    }
    h
}

/// Haze distances, with a sweep hook. The SHIPPED values are the consts — this
/// exists so a distance can be tried against a real frame without a relink, not
/// so the fog can be configured per install.
fn fog_range() -> (f32, f32) {
    env_floats::<2>("VOXELFORGE_LOOK_FOG")
        .map(|[s, e]| (s, e))
        .unwrap_or((FOG_START, FOG_END))
}

/// Parse `"a,b,c"` from an env var into a fixed array — all-or-nothing, so a
/// half-typed override falls back to the shipped constant instead of applying a
/// partly-parsed value mid-frame.
fn env_floats<const N: usize>(key: &str) -> Option<[f32; N]> {
    let raw = std::env::var(key).ok()?;
    let parts: Vec<f32> = raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    if parts.len() == N {
        let mut out = [0.0; N];
        out.copy_from_slice(&parts);
        Some(out)
    } else {
        None
    }
}

/// Live override for the grade knobs — `VOXELFORGE_LOOK_GRADE=temp,sat,mid,hi_gain`.
///
/// The same no-recompile sweep hook `hero.rs` carries as `VOXELFORGE_GRADE`, and
/// here for a sharper reason: the grade value that keeps the authored sky's
/// ordering alive *through* the tonemap and the sectional curve is an empirical
/// search, and a release rebuild per candidate is minutes. Unset — the shipped
/// path, and every gate run — returns the constants byte-for-byte. Malformed
/// input falls back to the constants rather than panicking mid-frame: this is a
/// debug hook, not a config file.
fn grade_knobs() -> (f32, f32, f32, f32) {
    let d = (
        if v3() {
            grade::TEMPERATURE_V3
        } else {
            grade::TEMPERATURE
        },
        grade::POST_SATURATION,
        // The v3 default is swapped HERE and not at the `ColorGrading` literal
        // so that `VOXELFORGE_LOOK_GRADE` keeps overriding it under both
        // generations — gating at the literal would have made the sweep hook
        // silently dead on the only generation anyone is still tuning.
        if v3() {
            grade::MIDTONE_CONTRAST_V3
        } else {
            grade::MIDTONE_CONTRAST
        },
        if v3() {
            grade::HIGHLIGHT_GAIN_V3
        } else {
            grade::HIGHLIGHT_GAIN
        },
    );
    let Ok(raw) = std::env::var("VOXELFORGE_LOOK_GRADE") else {
        return d;
    };
    let v: Vec<f32> = raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    match v[..] {
        [t, s, m, g] => (t, s, m, g),
        _ => d,
    }
}

/// Live override for the two light colours — `VOXELFORGE_LOOK_LIGHT=kr,kg,kb,ar,ag,ab`
/// (key RGB then ambient RGB, sRGB 0..1). Same sweep-without-rebuild rationale as
/// [`grade_knobs`]; unset returns the live [`Hour`]'s own key/ambient.
fn light_colors() -> (Color, Color) {
    let h = hour();
    let (k, a) = (h.key, h.ambient);
    let d = (Color::srgb(k[0], k[1], k[2]), Color::srgb(a[0], a[1], a[2]));
    let Ok(raw) = std::env::var("VOXELFORGE_LOOK_LIGHT") else {
        return d;
    };
    let v: Vec<f32> = raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    match v[..] {
        [kr, kg, kb, ar, ag, ab] => (Color::srgb(kr, kg, kb), Color::srgb(ar, ag, ab)),
        _ => d,
    }
}

/// Render-quality tier for the look stack.
///
/// Default is [`LookQuality::High`], not Ultra — Steam's median card has to hold
/// the frame, and the full stack (volumetric god rays, Ultra SSAO, PCSS) is what
/// a stronger GPU earns, not the shipped experience. The tier→effect map is in
/// [`insert_stack`]; the only thing that needs touching to retune it from Poppy's
/// per-effect ms numbers is that one `match`. Effects are a strict subset down
/// the ladder (everything Low has, Medium has too, … up to Ultra).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LookQuality {
    /// Tonemap + grade + exposure + bloom + haze, plus the two "cut = dies"
    /// contact layers at their floor quality. Everything stochastic, temporal or
    /// ray-marched is off — the cheapest frame the look lane can produce that
    /// still reads as Voxelforge (the grade *is* the identity).
    Low,
    /// + TAA and a temporal shadow filter, low-quality SSAO.
    Medium,
    /// + medium-quality SSAO and volumetric god rays at a reduced step count.
    /// No PCSS penumbra. The default.
    #[default]
    High,
    /// + Ultra SSAO + PCSS soft shadows + full-step volumetric god rays.
    Ultra,
}

/// The tier currently welded onto a camera. Carries [`LookQuality`] so the apply
/// system can tell a tier change (stack must be rebuilt) from steady state
/// (skip). Without this, the marker would fire once and the stack could never be
/// swapped at runtime.
#[derive(Component, Clone, Copy, PartialEq, Eq, Default)]
pub struct LookApplied(pub LookQuality);

/// Same idea as [`LookApplied`], on the directional light — the PCSS penumbra
/// width and `VolumetricLight` toggle are the two look knobs that live on the
/// sun rather than the camera, and they tier too.
#[derive(Component, Clone, Copy, PartialEq, Eq, Default)]
pub struct LookLightApplied(pub LookQuality);

/// Every component the look stack can put on a camera. Used as a single
/// `remove::<LookStack>()` target so a tier switch clears the old stack in one
/// call regardless of which effects the previous tier had on. `remove` over a
/// type the camera doesn't currently carry is a no-op, so this one list covers
/// every Low↔Medium↔High↔Ultra transition.
///
/// [`DepthOfField`] is still on this list even though NO tier inserts it any more
/// (see [`insert_stack`]) — it is strip-only. Removing a component an entity
/// doesn't have costs nothing, and keeping it here means a stale DOF from any
/// source gets cleared on the next tier apply instead of surviving invisibly on
/// the one camera the player looks through.
type LookStack = (
    Msaa,
    Tonemapping,
    ColorGrading,
    Exposure,
    ShadowFilteringMethod,
    TemporalAntiAliasing,
    Bloom,
    DepthOfField,
    ScreenSpaceAmbientOcclusion,
    ContactShadows,
    VolumetricFog,
    DistanceFog,
);

/// The capture/profiling tier override: `VOXELFORGE_LOOK_QUALITY=low|medium|high|ultra`.
///
/// PUBLIC BECAUSE `SettingsPlugin` HAS TO ASK THE SAME QUESTION. It loads the saved
/// `graphics` tier and `insert_resource`s it — which "overwrites any existing resource
/// of the same type" (`bevy_app::App::insert_resource`) — and `main.rs` adds it AFTER
/// [`LookPlugin`], so from 8176d01 (2026-07-31) until this patch the saved tier landed
/// on top of this one and every `--play` session ran at settings.json's tier no matter
/// what the shoot script exported. Nothing warned: the var parsed, the resource was
/// inserted, and it was replaced one line later.
///
/// MEASURED, one exe, one camera, `_poppy_tier_probe/`: `=low`, `=medium`, `=high` and
/// `=ultra` are all the same frame within 0.28–0.39 mean L — the repeat-shot floor —
/// while `VOXELFORGE_LOOK_DISABLE=1` moves it by 61.5. The same env on `voxelforge_perf`
/// (`perf_main.rs`, which does not add `SettingsPlugin`) ladders 5.9 / 6.2 / 6.5 / 16.6 ms.
/// One binary honoured the tier, the other silently did not, and the difference between
/// them is this one `insert_resource`. That is what made `grade-vista` "at Ultra" shoot
/// inside its own PCSS-off noise floor while `VOXELFORGE_LOOK_PCSS=16` — which [`pcss_width`]
/// applies WITHOUT consulting the tier — moved 45.4 % of the frame.
pub fn quality_from_env() -> Option<LookQuality> {
    std::env::var("VOXELFORGE_LOOK_QUALITY")
        .ok()
        .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
            "low" => Some(LookQuality::Low),
            "medium" => Some(LookQuality::Medium),
            "high" => Some(LookQuality::High),
            "ultra" => Some(LookQuality::Ultra),
            _ => None,
        })
}

/// Wears the look lane's post stack over whatever camera and sun the game spawned.
pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        // Default High, not Ultra — see `LookQuality`. `VOXELFORGE_LOOK_QUALITY`
        // overrides the starting tier so Poppy can profile each tier headless
        // (`=low|medium|high|ultra`); anything unrecognised falls back to High.
        let initial = quality_from_env().unwrap_or_default();
        app.insert_resource(initial)
            // The playable path never inserts a directional shadow map: the only
            // 4K insert lives in main.rs's hero-shot branch (`if cfg.hero`), so a
            // `--play` session falls through to Bevy's 2048 default. PCSS 3.0 was
            // signed off on a 4K map (look-tier-spec.md §2 note 3 / §7 note 5), so
            // insert it here. `DirectionalLightShadowMap` is an app resource, not
            // a component — it can't follow an F7 tier swap — so it's set once at
            // build and left on for every tier: any tier that flips PCSS on (Ultra,
            // or a High session cycled up to Ultra at runtime) then runs it on the
            // map the gate signed off on. It is also what keeps a 17° sun's long
            // cascades from stair-stepping. The cost is ~64MB of VRAM, not fps.
            .insert_resource(DirectionalLightShadowMap { size: 4096 })
            .add_systems(
                Update,
                (
                    apply_look_to_cameras,
                    apply_look_to_sun,
                    // The directional half of the fill (see `apply_fill_rig`).
                    // Ordered BEFORE the sun so the `Without<LookFill>` filter over
                    // there is never the thing standing between a freshly spawned
                    // fill light and being mistaken for the key.
                    apply_fill_rig.before(apply_look_to_sun),
                    // v3's surface rig. `aim_rim_light` is ordered AFTER the
                    // spawn for the obvious reason (nothing to point otherwise —
                    // it would idle one frame and the very first captured frame
                    // is one frame), and `apply_ibl` is independent of both.
                    aim_rim_light.after(apply_fill_rig),
                    apply_ibl,
                    cycle_look_quality,
                    // A0 (docs/sky-research-2026-08-14.md): Bevy 0.19's own
                    // physical atmosphere. `AtmospherePlugin` is NOT added here
                    // — `PbrPlugin` already did (`bevy_pbr-0.19.0/src/lib.rs:252`)
                    // and adding it twice panics; see the A0 section note.
                    atmosphere_sky.run_if(atmos_enabled),
                    // A1/A2 (art-order-2026-08-09-composition): the gradient sky
                    // dome and the play-scene fog volume that turns the already
                    // installed `VolumetricFog`/`VolumetricLight` into actual god
                    // rays. Both find the existing `OrbitCam` and add to the world;
                    // neither touches another lane's entity. See their own docs.
                    sky_dome.run_if(sky_grad_enabled),
                    play_fog_volume,
                )
                    .run_if(look_enabled),
            );
    }
}

/// The look is for the PLAYABLE session only (`--play` / `VOXELFORGE_PLAY`).
///
/// Same gate `ScenePlugin` and `AnimPlugin` use, for the same reason: the bench,
/// the editor and every headless screenshot proof are graded against the output
/// they already produce, and a post stack changes every pixel of all of them.
/// Turning this on globally would fail other lanes' gates without a single line
/// of their code changing — which is the exact class of cross-lane breakage
/// `docs/LANES.md` exists to stop. It also keeps hands off the editor camera
/// (another lane's file).
///
/// `Option<Res<_>>` because a plugin shouldn't assume another lane's resource is
/// inserted before it — a missing `Cfg` means "not the play path", not a panic.
///
/// `VOXELFORGE_LOOK_DISABLE` forces this off regardless of `cfg.play` — the
/// Gate 3 before/after pair needs one binary, one scene, with only the look
/// stack toggled, and a quality tier isn't "off": even `LookQuality::Low` still
/// carries the tonemap/grade/bloom identity.
///
/// `VOXELFORGE_LOOK_FORCE` forces it ON without `--play`, which is how a look
/// frame gets shot inside a `--map-load` world (the playable scene relocates the
/// avatar to its own campsite; a loaded map does not). It is deliberately an
/// explicit opt-in and NOT folded into `cfg.shot`: the bench and every other
/// lane's screenshot proof also carry `cfg.shot`, and they have to keep rendering
/// the frame they were graded against.
fn look_enabled(cfg: Option<Res<crate::Cfg>>) -> bool {
    if std::env::var_os("VOXELFORGE_LOOK_DISABLE").is_some() {
        return false;
    }
    if std::env::var_os("VOXELFORGE_LOOK_FORCE").is_some() {
        return true;
    }
    cfg.is_some_and(|c| c.play)
}

/// The same predicate, answered from a plain `&Cfg` instead of a system param.
///
/// `main.rs` needs it at camera-spawn time, where there is no `Res` to borrow.
/// Exported rather than re-derived over there so there is exactly one definition
/// of "is the look lane live" in the codebase.
pub fn enabled_for(cfg: &crate::Cfg) -> bool {
    if std::env::var_os("VOXELFORGE_LOOK_DISABLE").is_some() {
        return false;
    }
    std::env::var_os("VOXELFORGE_LOOK_FORCE").is_some() || cfg.play
}

/// The haze curve: `FogFalloff::Linear` from [`HAZE_START`] to [`HAZE_FULL`] —
/// zero across the gameplay foreground, opaque 70 blocks inside the streaming
/// edge, and within 1.83 pp of the `ExponentialSquared` curve it replaces
/// everywhere the pinned framing can see (see [`HAZE_START`] for the measurement
/// that forced the change and [`HAZE_FULL`] for the fit).
///
/// THREE HOOKS, AND EACH ONE STILL MEANS WHAT IT DID. They exist so a candidate
/// curve can be shot against a real frame without a relink — the only way this
/// was ever going to be tuned honestly, since the number that matters is how
/// much of the *playable* depth range the haze covers and that is not derivable
/// at a desk.
///
/// * unset — the shipped ramp, byte for byte. Every gate frame is shot this way.
/// * `VOXELFORGE_LOOK_FOG=<start>,<end>` — sweep the shipped ramp. Overrides
///   everything below, so a start/end question is answered the way it always
///   could be; it is also the probe the depth ruler is built from
///   (`FOG=S,S+0.5` makes alpha a hard step at S).
/// * `VOXELFORGE_LOOK_HAZE=<density>` — the G7 `ExponentialSquared` curve, still
///   reachable, so the before/after against the previous default comes out of
///   ONE binary: same build, same scene, only the curve swapped.
/// * `VOXELFORGE_LOOK_HAZE=0` — the legacy [`FOG_START`] ramp, which in a set
///   ~55 blocks deep is zero fog everywhere. That is the A/B baseline the haze
///   axes are graded against, and the reason it is kept.
fn haze_falloff() -> FogFalloff {
    if let Some([start, end]) = env_floats::<2>("VOXELFORGE_LOOK_FOG") {
        return FogFalloff::Linear { start, end };
    }
    match std::env::var("VOXELFORGE_LOOK_HAZE")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
    {
        Some(d) if d > 0.0 => FogFalloff::ExponentialSquared { density: d },
        Some(_) => {
            let (start, end) = fog_range();
            FogFalloff::Linear { start, end }
        }
        None => FogFalloff::Linear {
            start: HAZE_START,
            end: HAZE_FULL,
        },
    }
}

/// The colour distant geometry dissolves toward.
///
/// DERIVED FROM THE SKY, NOT AUTHORED BESIDE IT. The endpoint of aerial
/// perspective is, physically, the radiance of the sky along that line of sight
/// — so the one thing this colour must never be is an independent decision that
/// can drift out of agreement with [`Hour::sky`]. It is the hour's own sky hue,
/// washed [`HAZE_DESAT`] toward white (horizon, not zenith) and scaled by
/// [`HAZE_GAIN`] into the same scene-referred linear units `ClearColor` is
/// written in — `DistanceFog` mixes in linear space, so authoring this as a
/// plain sRGB triple (the old [`FOG_COLOR_DAY`]) put a value ~2.5× under the
/// sky's radiance behind the skyline, which reads as distant terrain going
/// muddy and DARK as it recedes. Air does not do that.
///
/// `VOXELFORGE_LOOK_HAZECOL=r,g,b,gain` overrides hue (sRGB) and gain together —
/// they are one decision, same as [`Hour::sky`]/[`Hour::sky_gain`].
fn haze_color() -> Color {
    let h = hour();
    let over = env_floats::<4>("VOXELFORGE_LOOK_HAZECOL");
    let (hue, gain) = match over {
        Some([r, g, b, gain]) => ([r, g, b], gain),
        // THE HOUR'S HORIZON HUE, NOT ITS ZENITH — changed 2026-08-05 from
        // `h.sky`. The paragraph above is right that the haze must never be an
        // independent decision that can drift from the hour; it was wrong that
        // "not independent" has to mean "identical to the zenith". At golden hour
        // the horizon is where the sun is: the same Rayleigh path length that
        // washes it out is the path length that has scattered the blue OUT of it,
        // which is why [`FOG_SUN_GLOW`] already had to bolt a warm term back on
        // around the sun direction. Reading [`Hour::fog`] here makes that hue part
        // of the hour itself, so it still cannot drift and the special case
        // shrinks instead of growing.
        //
        // Measured, on the vista framing: this alone (0.36,0.60,0.90 →
        // 0.94,0.66,0.26) is worth blue 25.8 → 19.2 and saturation 78.2 → 84.2,
        // and paired with [`HAZE_DENSITY`] 0.0100 it carries most of the warmth
        // that used to be asked of [`grade::POST_SATURATION`].
        None => (h.fog, HAZE_GAIN),
    };
    let desat = std::env::var("VOXELFORGE_LOOK_HAZEDESAT")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(HAZE_DESAT);
    // NIGHT keeps its authored haze: a night sky is near-black, and deriving the
    // haze from it would hand the frame a horizon that swallows the skyline
    // instead of the deep blue air `FOG_COLOR_NIGHT` was picked for.
    if std::env::var_os("VOXELFORGE_LOOK_NIGHT").is_some() && over.is_none() {
        let c = h.fog;
        return Color::srgb(c[0], c[1], c[2]);
    }
    let lin = Color::srgb(hue[0], hue[1], hue[2]).to_linear();
    // `* h.sky_gain` is the whole point: `apply_fog` mixes this colour into an
    // ALREADY-EXPOSED lit colour and writes it to the same HDR target the sky's
    // `ClearColor` lands in un-exposed, so the two are directly comparable only
    // once the haze carries the sky's gain too. Without it the haze sits at the
    // sRGB value (~0.3 linear) against a sky at ~1.9 and the skyline reads as a
    // dark band — the failure mode this whole function exists to avoid.
    let scale = gain * h.sky_gain;
    let mix = |c: f32| (c + (1.0 - c) * desat) * scale;
    Color::linear_rgb(mix(lin.red), mix(lin.green), mix(lin.blue))
}

/// The distance haze, built from the public contract constants.
///
/// One constructor for every tier: the haze is not a luxury effect that scales
/// with the GPU, it is the thing that hides the streaming radius (see
/// [`RENDER_RADIUS`]), so a Low-tier machine needs it exactly as much as an Ultra
/// one.
fn distance_fog() -> DistanceFog {
    let g = FOG_SUN_GLOW;
    DistanceFog {
        color: haze_color(),
        falloff: haze_falloff(),
        // Looking INTO the low sun, the haze picks up its colour — the warm
        // atmospheric glow every shader pack sells golden hour with. It costs
        // nothing (a term in the fog shader, not a pass).
        directional_light_color: Color::srgb(g[0], g[1], g[2]),
        directional_light_exponent: FOG_SUN_EXPONENT,
    }
}

/// The tier-independent half of the stack, as a `Bundle` a camera can be spawned
/// with — the look's identity: tonemap, grade, exposure, emissive-only bloom and
/// the distance haze.
///
/// `main.rs` uses this at the camera-spawn site so the gameplay camera is dressed
/// from frame ZERO rather than from the first `Update`. [`insert_stack`] then
/// re-inserts it under the plugin, so both paths are the same code and cannot
/// drift into two different looks.
pub fn base_camera_look() -> impl Bundle {
    let (temperature, post_saturation, midtone_contrast, highlight_gain) = grade_knobs();
    let h = hour();
    (
        // MSAA off: voxel edges are 90° and axis-aligned (no jaggies to smooth),
        // and SSAO at every higher tier requires it off anyway.
        Msaa::Off,
        // TonyMcMapface, NOT AcesFitted. Bevy's own doc comment on `AcesFitted`
        // reads "Bright greens and reds turn orange. Bright blues turn magenta."
        // — and the brightest blue in an outdoor frame is the SKY, i.e. ACES
        // reproduces the exact magenta cast chased out of this lane on
        // 2026-08-01 (commit 26b2ae6), just from the other end of the pipe. Tony
        // is "very neutral … color hues are preserved during compression", which
        // keeps sky blue, leaves green and sun amber instead of letting them
        // slide into each other; Bevy also names it specifically as the
        // tonemapper to pair `Bloom` with. `hero.rs` keeps AcesFitted: its golden
        // shot is signed off, has no sky in it, and feeds nothing downstream.
        Tonemapping::TonyMcMapface,
        // Applied AFTER the tonemap. SHADOWS are deliberately held neutral:
        // putting contrast on the shadow section crushed open shade to pure
        // black on the hero shot (interior p05-L 13.7% -> 3.3%, a measured
        // regression). Shadows keep their bounce fill instead of clipping.
        ColorGrading {
            global: ColorGradingGlobal {
                temperature,
                post_saturation,
                ..default()
            },
            // v3 lifts the TOE and rolls the SHOULDER — see the three
            // `*_V3` constants in [`grade`] for why each moves and why
            // shadow *gain* is allowed where shadow *contrast* was not.
            // Two of the three branch HERE; the midtone default is swapped one
            // level up inside [`grade_knobs`] instead, so `VOXELFORGE_LOOK_GRADE`
            // still overrides it under v3 (see the comment there). Under
            // `VOXELFORGE_LOOK_GEN=v2` all three resolve to their pre-v5 values,
            // so the before plate is untouched.
            shadows: ColorGradingSection {
                contrast: 1.0,
                gain: if v3() { grade::SHADOW_GAIN_V3 } else { 1.0 },
                ..default()
            },
            midtones: ColorGradingSection {
                contrast: midtone_contrast,
                ..default()
            },
            highlights: ColorGradingSection {
                contrast: if v3() {
                    grade::HIGHLIGHT_CONTRAST_V3
                } else {
                    grade::HIGHLIGHT_CONTRAST
                },
                gain: highlight_gain,
                ..default()
            },
        },
        // The gameplay camera never had an `Exposure` at all — it ran Bevy's
        // implicit `Exposure::BLENDER` (ev100 9.7) while the sun's illuminance
        // was set by a different lane, so "how bright is the world" was an
        // accident nobody owned. It is part of the hour now.
        Exposure { ev100: h.ev100 },
        // Bloom that fires on EMISSIVE ONLY. `Bloom::NATURAL`'s prefilter
        // threshold is 0.0, i.e. every pixel in the frame blooms a little, which
        // is what puts a veil over sunlit wood and softens the voxel grain
        // (measured micro-contrast 3.8 vs the golden's 5.2). A threshold of 1.0
        // in HDR means "only things brighter than white" — lanterns, the
        // campfire, the sun's own disc, a pane with the sun behind it — and
        // nothing else. The softness feathers the cut so the halo ramps in
        // instead of switching on at a hard luminance line.
        //
        // `Bloom` carries `#[require(Hdr)]`, so this is also what puts the camera
        // on an HDR target at all — without it there is no >1.0 signal for a
        // threshold to select on.
        //
        // v3 drops the threshold to [`BLOOM_THRESHOLD_V3`] so the specular the
        // environment light puts on smooth faces can glow, and halves the
        // low-frequency boost so admitting that band buys a sharper halo rather
        // than the veil that measurement warns about. `VOXELFORGE_LOOK_GEN=v2`
        // returns the four numbers above, byte-for-byte.
        if v3() {
            Bloom {
                intensity: BLOOM_INTENSITY_V3,
                low_frequency_boost: BLOOM_LF_BOOST_V3,
                prefilter: BloomPrefilter {
                    threshold: BLOOM_THRESHOLD_V3,
                    threshold_softness: BLOOM_SOFTNESS_V3,
                },
                ..Bloom::NATURAL
            }
        } else {
            Bloom {
                intensity: 0.18,
                prefilter: BloomPrefilter {
                    threshold: 1.0,
                    threshold_softness: 0.4,
                },
                ..Bloom::NATURAL
            }
        },
        distance_fog(),
    )
}

/// Contact AO for one tier, with the sweep hooks the G7 pass needed.
///
/// WHY THIS IS A FUNCTION AND NOT FOUR LITERALS. The G6 frame was read as having
/// no ambient occlusion at all, and the first job was to find out whether that
/// meant *absent* or *invisible*: SSAO in Bevy darkens the DIFFUSE INDIRECT term
/// only, so under an 11 000-lux key against a 1 100-lux fill it can move a
/// sunlit face by at most ~9% no matter how hard it works, while an in-shade
/// face is ~100% fill and takes the full bite. That is a measurement, not an
/// opinion, and it needs an A/B from ONE binary to settle — hence
/// `VOXELFORGE_LOOK_SSAO=off`, which returns `None` and shoots the same frame
/// with the layer lifted out.
///
/// `VOXELFORGE_LOOK_SSAO=<thickness>` and `VOXELFORGE_LOOK_SSAOQ=low|medium|high|ultra`
/// sweep the two real knobs. Unset ⇒ the tier's own values, byte-for-byte.
fn ssao(tier_quality: ScreenSpaceAmbientOcclusionQualityLevel) -> Option<ScreenSpaceAmbientOcclusion> {
    let raw = std::env::var("VOXELFORGE_LOOK_SSAO").unwrap_or_default();
    if raw.trim().eq_ignore_ascii_case("off") {
        return None;
    }
    let quality_level = match std::env::var("VOXELFORGE_LOOK_SSAOQ")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "low" => ScreenSpaceAmbientOcclusionQualityLevel::Low,
        "medium" => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
        "high" => ScreenSpaceAmbientOcclusionQualityLevel::High,
        "ultra" => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        _ => tier_quality,
    };
    Some(ScreenSpaceAmbientOcclusion {
        quality_level,
        constant_object_thickness: raw.trim().parse().unwrap_or(AO_THICKNESS),
    })
}

/// The contact-shadow layer, or `None` when it is switched off.
///
/// `VOXELFORGE_LOOK_CONTACT=off` lifts the layer out so the before/after comes
/// out of ONE binary — the same A/B discipline `VOXELFORGE_LOOK_SSAO=off` exists
/// for, and the reason the SSAO ceiling in [`CONTACT_SHADOW_LENGTH`] could be
/// quoted as a measurement instead of a guess.
/// `VOXELFORGE_LOOK_CONTACT=<length>,<thickness>,<steps>` sweeps all three.
/// Unset ⇒ the constants, byte-for-byte, which is what every gate run gets.
fn contact_shadows() -> Option<ContactShadows> {
    let raw = std::env::var("VOXELFORGE_LOOK_CONTACT").unwrap_or_default();
    if raw.trim().eq_ignore_ascii_case("off") {
        return None;
    }
    let v: Vec<f32> = raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    // The generation's own march. v3 runs longer, thinner and finer — see
    // [`CONTACT_SHADOW_LENGTH_V3`] for why all three had to move together.
    let shipped = if v3() {
        (
            CONTACT_SHADOW_LENGTH_V3,
            CONTACT_SHADOW_THICKNESS_V3,
            CONTACT_SHADOW_STEPS_V3,
        )
    } else {
        (
            CONTACT_SHADOW_LENGTH,
            CONTACT_SHADOW_THICKNESS,
            CONTACT_SHADOW_STEPS,
        )
    };
    let (length, thickness, steps) = match v[..] {
        [l, t, s] => (l, t, s.max(1.0) as u32),
        // Malformed input falls back to the constants rather than panicking
        // mid-frame: this is a sweep hook, not a config file.
        _ => shipped,
    };
    Some(ContactShadows {
        linear_steps: steps,
        thickness,
        length,
    })
}

/// PCSS penumbra width for the sun, or `None` for a fixed-width filter.
///
/// `VOXELFORGE_LOOK_PCSS=off|<width>`; unset ⇒ the tier's own call.
fn pcss_width(tier_on: bool) -> Option<f32> {
    let shipped = match look_gen() {
        LookGen::V1 => PCSS_WIDTH_V1,
        LookGen::V2 => PCSS_WIDTH,
        LookGen::V3 => PCSS_WIDTH_V3,
    };
    match std::env::var("VOXELFORGE_LOOK_PCSS") {
        Ok(v) if v.trim().eq_ignore_ascii_case("off") => None,
        Ok(v) => v.trim().parse().ok().or(tier_on.then_some(shipped)),
        Err(_) => tier_on.then_some(shipped),
    }
}

/// Build the look stack for `quality` onto the camera entity `e`.
///
/// This is the single tier→effect map — the place Poppy's per-effect ms budget
/// reshapes the ladder. Each tier is a strict superset of the one below it, so
/// the base layer goes on first and the `match` only adds what the tier earns.
///
/// NOTHING IN HERE INSERTS `DepthOfField`, AT ANY TIER — deliberately, and
/// permanently. See `docs/look-contract.md` §5: the old High/Ultra arms carried
/// `hero.rs`'s bokeh DOF with its focus plane pinned to the camera boom (~6.5
/// blocks) and a `focus_dof` system re-pinning it every frame, which melted
/// everything past the player's own shoulder — the "distant stuff dissolves" the
/// reviewer caught. The reference frames hold distant pines sharp to the last
/// pixel. Near-focus DOF is the grammar of a product still, not of a camera
/// someone looks through while walking.
fn insert_stack(e: &mut EntityCommands, quality: LookQuality) {
    // Base layer — present at every tier, and the SAME function `main.rs` spawns
    // the camera with, so the two paths cannot drift into two different looks.
    e.insert(base_camera_look());
    // Contact AO is identity, not a luxury: without it blocks hover off the
    // ground. The layer is present at EVERY tier (look-tier-spec.md §1 ranks it
    // "cut = dies"); only the sample count steps down, which is why it is hoisted
    // out of the `match` and only its quality argument comes from the tier.
    if let Some(ao) = ssao(match quality {
        // v3 steps the two middle tiers up one rung each. The pass is a
        // fixed-size compute dispatch over a depth buffer that SSAO has already
        // paid for at every tier, so the step is sample count and nothing else —
        // the same accounting the G7 note below makes for High.
        LookQuality::Medium if v3() => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
        LookQuality::High if v3() => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        LookQuality::Low | LookQuality::Medium => ScreenSpaceAmbientOcclusionQualityLevel::Low,
        // High was `Medium` (8 spp) through G6. Raised to `High` (18 spp) for
        // G7: the AO signal the CEO could not find in the frame is worth more
        // than the tier is worth saving, and the pass is a fixed-size compute
        // dispatch over the depth buffer — the step is sample count, not another
        // full-screen pass.
        LookQuality::High => ScreenSpaceAmbientOcclusionQualityLevel::High,
        LookQuality::Ultra => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
    }) {
        e.insert(ao);
    }
    // The second contact layer, and present at EVERY tier for the same reason
    // SSAO is: it is the thing that stops a block hovering, which
    // look-tier-spec.md §1 ranks "cut = dies". It is also close to free HERE
    // specifically — it needs a depth prepass, and SSAO above has already put one
    // on this camera at every tier, so the added cost is a 16-step loop in the
    // PBR shader and not another pass. It does not tier: unlike SSAO's sample
    // count there is no cheaper version of it that still grounds an object.
    if let Some(cs) = contact_shadows() {
        e.insert(cs);
    }
    match quality {
        LookQuality::Low => {
            // The cheapest tier that still clears every gate-identity axis (G1–G6):
            // the grade base plus the two "cut = dies" G4 layers at their floor
            // quality. No TAA, no PCSS, no volumetrics — the temporal and
            // ray-marched passes that need accumulation or cost the most stay off
            // (look-tier-spec.md §1). Without these two, Low was failing both
            // halves of G4: hovering blocks (no AO) and a hard shadow edge (no
            // explicit filter).
            // Gaussian is a fixed multi-tap blur with no history buffer, so it
            // gives a soft (≥3px) shadow edge without TAA — exactly the §1
            // "cut survives" choice for the no-TAA tier. Leaving Low without an
            // explicit filter left the shadow edge at an unintended default
            // rather than the soft edge G4 wants.
            e.insert(ShadowFilteringMethod::Gaussian);
        }
        LookQuality::Medium => {
            // TAA cleans up the low-quality SSAO. The shadow filter stays temporal
            // too — it's free here because Medium already runs TAA, and the fixed
            // `Hardware2x2` (2×2 PCF) doesn't reliably reach the ≥3px soft edge G4
            // wants (look-tier-spec.md §5 problem #2). No PCSS, no accumulation
            // beyond what TAA gives the SSAO.
            e.insert((
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
            ));
        }
        LookQuality::High => {
            // Default tier. Temporal soft shadows (no PCSS) + high SSAO +
            // volumetric fog at a reduced step count. The one effect cut vs Ultra
            // is PCSS, and the REASON for that cut changed on 2026-08-08. It used
            // to be "PCSS buys almost nothing here" — look-tier-spec.md §1 read
            // Bevy clamping `soft_shadow_size` to its 0.5-texel floor and
            // concluded the effect was inert at any width. Half right: the clamp
            // is real and it was eating the shipped 4.0 whole, but it is
            // ESCAPABLE, and [`PCSS_WIDTH`] now carries the ladder showing 16.0
            // buying a measured 11.2 px penumbra where 4.0 bought 5.8 (against
            // 4.3 for no PCSS at all). So this is no longer a free cut — High
            // gives up a real soft shadow, and keeps the volumetric ray-march
            // instead because the atmosphere is what pins the frame (§5 problem
            // #3). That is a cost call, not "it does nothing".
            e.insert((
                // SSAO is stochastic; one frame is visibly noisy. Temporal
                // filtering + TAA accumulate it into clean contact AO.
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                // Volumetric fog at a reduced step count (Ultra ray-marches 96; 32
                // keeps the god-ray shaft at a fraction of the cost, with banding
                // TAA smooths out). `VolumetricLight` on the sun — set in
                // `apply_look_to_sun` — is what lets this pass treat it as an
                // in-scatterer; the two are coupled (spec §3 rule 3).
                VolumetricFog {
                    ambient_intensity: 0.08,
                    step_count: 32,
                    jitter: 0.6,
                    ..default()
                },
            ));
        }
        LookQuality::Ultra => {
            // The full stack: temporal PCSS + Ultra SSAO + full-step god rays.
            e.insert((
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                VolumetricFog {
                    ambient_intensity: 0.08,
                    step_count: 96,
                    // Dithers the ray-march step boundary so TAA resolves the
                    // shaft with a soft edge instead of a hard cut.
                    jitter: 0.6,
                    ..default()
                },
            ));
        }
    }
}

/// Give the gameplay (`OrbitCam`) camera the post stack for the live
/// [`LookQuality`] tier, and set the sky and bounce fill for the live [`Hour`].
///
/// WHY `OrbitCam`, NOT `Camera3d`. The VFX lane spawns its own `Camera3d` stage
/// camera (`vfx.rs`) that is deliberately graded and lit for a VFX still and —
/// critically — runs WITHOUT TAA, because TAA smears the fast-moving particles
/// into ghost trails (`vfx.rs` documents that choice on the camera spawn). A
/// `With<Camera3d>` filter would dress that stage camera with this stack too:
/// `remove::<LookStack>()` strips its hand-tuned `AcesFitted`/grade/exposure and
/// `insert_stack()` slaps TAA back on — silently breaking the VFX lane the first
/// time a `--play` session runs alongside `VOXELFORGE_VFX`. The gameplay camera
/// is the only `OrbitCam`, so filtering on it is exact.
///
/// Compares the tier stamped on the camera (`LookApplied`) to the live resource
/// each frame: equal ⇒ skip (steady state is one enum compare per camera, no
/// insert), different (first apply, or a runtime tier change) ⇒ strip the whole
/// stack and rebuild it for the new tier on the same entity. That is the
/// runtime-swap contract — no respawn, no restart, the existing camera keeps its
/// `OrbitCam` and every gameplay component exactly as it was.
fn apply_look_to_cameras(
    mut commands: Commands,
    quality: Res<LookQuality>,
    mut clear: ResMut<ClearColor>,
    mut q: Query<(Entity, Option<&LookApplied>, Option<&mut AmbientLight>), With<crate::OrbitCam>>,
) {
    for (cam, applied, ambient) in &mut q {
        if applied.is_some_and(|a| a.0 == *quality) {
            continue;
        }
        let h = hour();
        // The sky. `main.rs` clears to a pale 0.53/0.72/0.92 — a neutral, not a
        // time of day; the reference frames sit under a deep saturated blue. It
        // is set here, alongside the sun and the fill, because a sky that
        // disagrees with the key light is the single most obvious way a frame
        // reads fake.
        // Authored as an sRGB hue, written as scene-referred LINEAR radiance:
        // `ClearColor` lands in the HDR target un-exposed, so the gain is the
        // only thing that can put the sky above the bloom threshold. See
        // [`Hour::sky_gain`].
        let sky = Color::srgb(h.sky[0], h.sky[1], h.sky[2]).to_linear();
        clear.0 = Color::linear_rgb(
            sky.red * h.sky_gain,
            sky.green * h.sky_gain,
            sky.blue * h.sky_gain,
        );
        // Tint AND power the camera's bounce fill. Colour is the midtone half of
        // the frame's warmth (the half that used to come out of the white-balance
        // matrix — that was the magenta bug); brightness has to move with it,
        // because `main.rs`'s 380 lux was chosen against a 59°-high 9000-lux sun,
        // and open shade under a 17° sun is a far larger share of the frame.
        // `Option<&mut _>` because a camera without its own `AmbientLight` (the
        // editor's) is not this lane's to give one to.
        if let Some(mut ambient) = ambient {
            ambient.color = light_colors().1;
            ambient.brightness = h.ambient_lux;
        }
        let mut e = commands.entity(cam);
        e.remove::<LookStack>();
        insert_stack(&mut e, *quality);
        e.insert(LookApplied(*quality));
    }
}

/// Point, colour and tier the sun.
///
/// This mutates the existing `DirectionalLight` rather than inserting a new one,
/// so the light entity keeps whatever else its owner hung on it. The two shadow
/// SWITCHES (`shadow_maps_enabled`, `contact_shadows_enabled`) are this lane's
/// though, and are set here rather than inherited — see the note at the
/// assignment for why borrowing them from `main.rs` was a hole.
///
/// Angle and illuminance ARE this lane's call (they used to be `main.rs`'s: a
/// white 9000-lux key 59° up). "Raking golden-hour sun" is the brief itself, not
/// a tint on top of someone else's noon — and elevation, illuminance, hue, sky
/// and exposure only make a coherent frame when they move together, which is
/// what [`Hour`] exists to guarantee.
fn apply_look_to_sun(
    mut commands: Commands,
    quality: Res<LookQuality>,
    shadow_map: Res<DirectionalLightShadowMap>,
    // `Without<LookFill>` because this query is otherwise "every directional light
    // in the world", and since 2026-08-14 two of those are this lane's OWN fill
    // lights (see [`apply_fill_rig`]). Unfiltered, the first frame after the rig
    // spawns would point all three at the sun's angle, hand all three the sun's
    // 22 000 lux and its shadow map, and the "fill" would be three suns.
    mut q: Query<
        (
            Entity,
            &mut DirectionalLight,
            &mut Transform,
            Option<&LookLightApplied>,
        ),
        Without<LookFill>,
    >,
) {
    for (light, mut dl, mut tf, applied) in &mut q {
        if applied.is_some_and(|a| a.0 == *quality) {
            continue;
        }
        let h = hour();
        // Point the sun. Only the rotation matters to a directional light, but
        // the translation is kept high and behind so anything that reasons about
        // the light's position (a debug gizmo, a future lens flare) still gets a
        // sane one.
        let dir = h.sun_dir();
        tf.translation = -dir * 200.0;
        tf.look_to(dir, Vec3::Y);
        dl.illuminance = h.illuminance;
        // THE LOOK LANE OWNS ITS OWN KEY'S SHADOW MAP — 2026-08-08. This used to
        // be left as whoever spawned the light set it, on the reasoning recorded
        // in this function's doc comment ("`shadow_maps_enabled` stays as main.rs
        // set it"). That is fine while main.rs is the only spawner and it happens
        // to say `true`, and it is a silent hole the moment it isn't: the look is
        // reachable over a `--map-load` world through `VOXELFORGE_LOOK_FORCE`, and
        // "the key light of the golden hour casts shadows" is not a property this
        // lane can borrow from another file and still claim to have set. Nothing
        // else on the light is taken over — the entity keeps everything its owner
        // hung on it, exactly as before.
        dl.shadow_maps_enabled = true;
        // The direct-light half of contact occlusion; the camera carries the
        // settings (see [`contact_shadows`]) and the light carries the opt-in, so
        // both have to agree before a single ray is marched. Kept in step with the
        // camera's own layer so `VOXELFORGE_LOOK_CONTACT=off` really is off rather
        // than half-off.
        dl.contact_shadows_enabled = contact_shadows().is_some();
        // PCSS penumbra: Ultra-only under v1, High-and-up under v2.
        //
        // The cut used to be justified as a cost call (spec §1). The cost is real,
        // but the accounting was wrong about what was being bought: High is the
        // DEFAULT tier, so "PCSS at Ultra only" meant the shadow edge the game
        // actually ships was whatever `ShadowFilteringMethod::Temporal` gives —
        // one width, everywhere, regardless of how far the caster is from what it
        // lands on. That is the single most visible thing separating this frame
        // from the reference shaders, which grade the penumbra by blocker distance
        // (`_poppy_lookv2/ref/complementary-style.png`: canopy shadow soft on open
        // grass, block-on-block seams still hard). Paired with [`PCSS_WIDTH`] at
        // 8.0 rather than 16.0 so the contact edges it now reaches stay sharp.
        let pcss = match look_gen() {
            LookGen::V1 => matches!(*quality, LookQuality::Ultra),
            LookGen::V2 => matches!(*quality, LookQuality::High | LookQuality::Ultra),
            // Medium and up under v3. Medium already runs TAA (see
            // [`insert_stack`]), which is the accumulation a stochastic penumbra
            // needs, so the tier that gets PCSS is the lowest tier that can
            // resolve it — not the highest tier that can afford it. Low keeps its
            // fixed Gaussian: no history buffer, nothing to resolve into.
            LookGen::V3 => !matches!(*quality, LookQuality::Low),
        };
        let volumetric = matches!(*quality, LookQuality::High | LookQuality::Ultra);
        // Golden-hour key. This is what makes sunlit surfaces order `R > G > B`
        // (gate G6) without a global matrix that would drag the sky along too.
        dl.color = light_colors().0;
        // `soft_shadow_size` only exists when Bevy's PCSS flag is compiled in.
        // It's in this crate's default features, but a `--no-default-features`
        // build is a real configuration and shouldn't fail to compile over a
        // look knob.
        #[cfg(feature = "experimental_pbr_pcss")]
        {
            // [`PCSS_WIDTH`] is the width the hero shot's penumbra gate was
            // signed off at; `VOXELFORGE_LOOK_PCSS` sweeps it (see [`pcss_width`]).
            dl.soft_shadow_size = pcss_width(pcss);
        }
        #[cfg(not(feature = "experimental_pbr_pcss"))]
        {
            let _ = (&mut dl, pcss);
        }
        let mut e = commands.entity(light);
        // Cascades sized off the SAME constant the haze and the streaming radius
        // use, so "how far can you see" is one number. A 17° sun throws long
        // shadows; Bevy's default split would either run out before the haze does
        // (shadows visibly stop mid-field) or spread four cascades across a
        // distance nothing is drawn at. `first_cascade_far_bound` keeps the near
        // cascade tight around the player so contact shadows stay crisp.
        e.insert(
            CascadeShadowConfigBuilder {
                num_cascades: 4,
                minimum_distance: 0.1,
                maximum_distance: RENDER_RADIUS,
                // v3 tightens this — see [`FIRST_CASCADE_FAR_BOUND_V3`]: the near
                // cascade is what keeps the contact end crisp under a penumbra
                // half again as wide, so it gets more texels per block.
                first_cascade_far_bound: if v3() {
                    FIRST_CASCADE_FAR_BOUND_V3
                } else {
                    16.0
                },
                overlap_proportion: 0.2,
            }
            .build(),
        );
        if volumetric {
            e.insert(VolumetricLight);
        } else {
            // Without this the light contributes no in-scattering, so
            // `VolumetricFog` renders nothing — god rays need a light that opts
            // in. Removing it on the lower tiers keeps the sun honest about what
            // it is doing.
            e.remove::<VolumetricLight>();
        }
        // ONE LINE OF PROVENANCE PER TIER CHANGE, next to every capture's plate.
        // This lane spent a round measuring a penumbra that was never switched on,
        // because "Ultra" was set on the command line, accepted by the parser, and
        // then overwritten — and no artefact of the run recorded which tier actually
        // reached the frame. It does now. Runs on tier change only (the
        // `LookLightApplied` guard above), not per frame.
        #[cfg(feature = "experimental_pbr_pcss")]
        let pcss_applied = format!("{:?}", dl.soft_shadow_size);
        #[cfg(not(feature = "experimental_pbr_pcss"))]
        let pcss_applied = String::from("n/a (built without experimental_pbr_pcss)");
        println!(
            "LOOK tier={:?} pcss={} shadow_map={} sun={:.0}deg/{:.0}deg illum={:.0} contact={}",
            *quality,
            pcss_applied,
            shadow_map.size,
            h.elev_deg,
            h.azim_deg,
            dl.illuminance,
            dl.contact_shadows_enabled,
        );
        e.insert(LookLightApplied(*quality));
    }
}

/// Marker + kind for the two shadow-less fill lights [`apply_fill_rig`] spawns.
///
/// PUBLIC, and every query over `DirectionalLight` in this lane filters on it —
/// see the `Without<LookFill>` on [`apply_look_to_sun`] for what happens otherwise.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LookFill {
    /// Cool, near-zenith, from the sky opposite the sun.
    Sky,
    /// Warm, travelling upward off the sunlit ground.
    Bounce,
    /// v3 only. Cool, low, and aimed FROM THE CAMERA rather than from the world —
    /// the kicker that separates a character's silhouette from what is behind
    /// them. Re-pointed every frame by [`aim_rim_light`]; see
    /// [`RIM_AZIM_OFFSET`] for why this one light is allowed to know where the
    /// camera is when no other light in this file is.
    Rim,
}

/// Spawn the two directional fill lights, once, and point them for the live
/// [`Hour`].
///
/// WHY TWO LIGHTS AND NOT A BIGGER `AmbientLight`. Written up on
/// [`SKY_FILL_ELEV`]: `AmbientLight` gives every face the same irradiance, and a
/// voxel scene is nothing but faces, so the flat term erases the only shape cue a
/// cube has. Two directionals restore it for the price of two more N·L terms and
/// no shadow map at all.
///
/// NEITHER CASTS A SHADOW MAP, ON PURPOSE. They are standing in for *ambient*:
/// the whole job is to reach the surfaces the key does not, so an occlusion test
/// against the key's own geometry would undo them. What they DO get is
/// [`ContactShadows`] on the sky fill — the screen-space march is a per-light
/// direct-term test, so it darkens the block-to-block creases *from above*, which
/// is the crease the eye reads on a voxel wall and the one SSAO could only ever
/// bite at the fill's own share of the light (see [`CONTACT_SHADOW_LENGTH`] for
/// that measurement). The bounce is left without it: it is the dimmest of the
/// three and a third depth march to shave ~1000 lux off a crease is not a trade.
///
/// SPAWN-ONCE, not per-frame: the query is empty exactly once (the frame the look
/// lane comes up) and every frame after that this system is one empty-query check.
/// Nothing here tiers, so unlike the camera and the sun there is no rebuild path.
fn apply_fill_rig(mut commands: Commands, existing: Query<(), With<LookFill>>) {
    if !existing.is_empty() {
        return;
    }
    let h = hour();
    let contact = contact_shadows().is_some();
    let spawn = |commands: &mut Commands, kind: LookFill, rgb: [f32; 3], lux: f32, dir: Vec3| {
        commands.spawn((
            Name::new(match kind {
                LookFill::Sky => "look sky fill",
                LookFill::Bounce => "look ground bounce",
                LookFill::Rim => "look rim fill",
            }),
            kind,
            DirectionalLight {
                color: Color::srgb(rgb[0], rgb[1], rgb[2]),
                illuminance: lux,
                shadow_maps_enabled: false,
                contact_shadows_enabled: contact && matches!(kind, LookFill::Sky),
                ..default()
            },
            // Only the rotation reaches the shader; the translation is kept
            // off-scene for the same reason the sun's is (a debug gizmo should get
            // a sane position out of it).
            Transform::from_translation(-dir * 160.0).looking_to(dir, Vec3::Y),
        ));
    };
    spawn(
        &mut commands,
        LookFill::Sky,
        h.sky_fill,
        h.sky_fill_lux,
        h.sky_fill_dir(),
    );
    spawn(
        &mut commands,
        LookFill::Bounce,
        h.bounce,
        h.bounce_lux,
        h.bounce_dir(),
    );
    // The v3 kicker. Spawned pointing off the SUN's azimuth so a session in which
    // [`aim_rim_light`] never finds a camera still gets a sane light instead of a
    // degenerate one; the very next frame that system re-points it off the camera,
    // which is the pose that matters. Zero lux under v1/v2, and not spawned at
    // all — an unlit `DirectionalLight` is still a cluster entry.
    if v3() {
        let e = RIM_ELEV.to_radians();
        let a = (h.azim_deg + RIM_AZIM_OFFSET).to_radians();
        spawn(
            &mut commands,
            LookFill::Rim,
            RIM_COLOR,
            rim_lux(),
            Vec3::new(a.sin() * e.cos(), -e.sin(), a.cos() * e.cos()).normalize(),
        );
    }
    // One provenance line, same rule as the `LOOK` line on the sun: a fill that is
    // zeroed by `VOXELFORGE_LOOK_GEN=v1` and a fill that failed to spawn look
    // identical in the frame, and only one of them is a bug.
    //
    // `ev100` RIDES ALONG HERE, AND IT IS APPENDED, NOT INSERTED. Exposure is
    // now a v3 rig constant ([`grade::EV100_V3`]) rather than something only the
    // capture harness sets, so "which exposure did this frame actually run at"
    // has to be answerable from the frame's own log line instead of from the
    // batch file that was believed to have set it. The other provenance line
    // that carries `ev100` (the sky-dome spawn) only prints when the dome
    // spawns, which under the shipped default it does not. Appended at the END
    // because `_poppy_lookv6_chain.sh` greps this line with a prefix-anchored
    // pattern (`gen=V3 ambient=[0-9]*`); a new field in the middle would break
    // a check in another script.
    println!(
        "LOOK_FILL gen={:?} ambient={:.0} sky={:.0}lux@{:.0}deg bounce={:.0}lux@{:.0}deg rim={:.0}lux@{:.0}deg contact_sky={} ev100={:.2}",
        look_gen(),
        h.ambient_lux,
        h.sky_fill_lux,
        SKY_FILL_ELEV,
        h.bounce_lux,
        BOUNCE_ELEV,
        if v3() { rim_lux() } else { 0.0 },
        RIM_ELEV,
        contact,
        h.ev100,
    );
}

/// Illuminance of the v3 kicker for the live hour, with a sweep hook.
///
/// `VOXELFORGE_LOOK_RIM=off` zeroes it — the same one-binary A/B discipline
/// `VOXELFORGE_LOOK_SSAO=off` exists for, and the only way to answer "is that
/// edge the kicker or the sky fill?" without a relink. `=<lux>` sweeps it.
fn rim_lux() -> f32 {
    let raw = std::env::var("VOXELFORGE_LOOK_RIM").unwrap_or_default();
    if raw.trim().eq_ignore_ascii_case("off") {
        return 0.0;
    }
    let shipped = if is_night() { RIM_LUX_NIGHT } else { RIM_LUX };
    raw.trim().parse().unwrap_or(shipped)
}

/// Re-point the v3 kicker off the CAMERA, every frame.
///
/// The direction is the camera's own heading flattened to the ground plane, spun
/// [`RIM_AZIM_OFFSET`] degrees and tilted down to [`RIM_ELEV`] — so the light
/// always travels roughly toward the lens, past one flank of whatever is being
/// looked at. See [`RIM_AZIM_OFFSET`] for why this light, alone in this file, is
/// allowed to be a property of the shot rather than of the world.
///
/// `Without<LookFill>` on the camera query is what proves the two `Transform`
/// accesses are disjoint — the same thing `sky_dome` needs for the same reason.
/// Cheap enough to run unconditionally: under v1/v2 no `LookFill::Rim` exists, so
/// the inner loop never runs a single iteration.
fn aim_rim_light(
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<LookFill>)>,
    mut fills: Query<(&LookFill, &mut Transform)>,
) {
    let Ok(cam_tf) = cam.single() else {
        return;
    };
    let fwd = *cam_tf.forward();
    // Flattened to the ground plane: a camera pitched down at the player must not
    // drag the kicker's elevation with it, or the edge it draws slides up and off
    // the silhouette every time the boom tilts.
    let cam_azim = fwd.x.atan2(fwd.z);
    let e = RIM_ELEV.to_radians();
    let a = cam_azim + RIM_AZIM_OFFSET.to_radians();
    let dir = Vec3::new(a.sin() * e.cos(), -e.sin(), a.cos() * e.cos()).normalize();
    for (kind, mut tf) in &mut fills {
        if !matches!(*kind, LookFill::Rim) {
            continue;
        }
        // Same convention as the sun and the two fills: only the rotation reaches
        // the shader, the translation is kept off-scene so a debug gizmo gets a
        // sane position out of it.
        tf.translation = -dir * 160.0;
        tf.look_to(dir, Vec3::Y);
    }
}

/// The hemispherical image-based light, or `None` when this generation or this
/// run does not carry one.
///
/// WHAT IT IS. Six 1×1 HDR texels — cool at the top, warm at the bottom, the
/// horizon band between them at [`IBL_HORIZON_MIX`] — handed to Bevy as a
/// cubemap. Bevy then lights the whole view from it: the DIFFUSE term reads it by
/// surface normal, so a face is lit by which way it points instead of by a single
/// flat number, and the SPECULAR term reads it by reflection vector, which is the
/// half no `AmbientLight` and no `DirectionalLight` can supply at any brightness.
///
/// WHY IT IS BUILT IN CODE AND NOT LOADED. `EnvironmentMapLight::hemispherical_gradient`
/// is Bevy's own constructor for exactly this. No `.ktx2` in `assets/`, nothing
/// to keep in step with [`Hour`] by hand, and the map is rebuilt from the live
/// hour's own colours — so `VOXELFORGE_LOOK_NIGHT` and every `_LOOK_SKY` sweep
/// move the environment light with them for free.
///
/// `VOXELFORGE_LOOK_IBL=off` lifts it out; `=<nits>` sweeps its intensity. Unset
/// ⇒ [`IBL_NITS_DAY`] / [`IBL_NITS_NIGHT`], byte-for-byte.
fn ibl_env(images: &mut Assets<Image>) -> Option<EnvironmentMapLight> {
    if !v3() {
        return None;
    }
    let raw = std::env::var("VOXELFORGE_LOOK_IBL").unwrap_or_default();
    if raw.trim().eq_ignore_ascii_case("off") {
        return None;
    }
    let nits = raw.trim().parse().unwrap_or(if is_night() {
        IBL_NITS_NIGHT
    } else {
        IBL_NITS_DAY
    });
    let h = hour();
    // The map's own colours ARE the rig's: its top is what the sky fill is
    // shining, its bottom is what the ground bounce is shining. Anything else
    // would be a fourth opinion about what colour the sky is.
    let top = Color::srgb(h.sky_fill[0], h.sky_fill[1], h.sky_fill[2]).to_linear();
    let bottom = Color::srgb(h.bounce[0], h.bounce[1], h.bounce[2]).to_linear();
    let mid = lerp_lin(top, bottom, IBL_HORIZON_MIX);
    Some(EnvironmentMapLight {
        intensity: nits,
        ..EnvironmentMapLight::hemispherical_gradient(
            images,
            Color::from(top),
            Color::from(mid),
            Color::from(bottom),
        )
    })
}

/// Marker: this camera has already been offered the environment light.
///
/// Present even when [`ibl_env`] declined (v1/v2, or `_LOOK_IBL=off`), so the
/// "did we try?" question is answered by the marker and the "did we insert?"
/// question by the component — a system that re-asked every frame would rebuild
/// a cubemap asset per frame and leak one per frame with it.
#[derive(Component)]
struct LookIbl;

/// Give the gameplay camera its environment light, once.
///
/// Not folded into [`apply_look_to_cameras`]: that system rebuilds its whole
/// stack on every tier change (`remove::<LookStack>` + re-insert), and the
/// environment light does NOT tier — it is the hour's, not the quality's. Keeping
/// it on its own marker means an F7 tier swap cannot drop it, and cannot rebuild
/// the cubemap asset either.
fn apply_ibl(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    q: Query<Entity, (With<crate::OrbitCam>, Without<LookIbl>)>,
) {
    for cam in &q {
        let env = ibl_env(&mut images);
        // Same provenance rule as `LOOK_FILL`: an environment light that was
        // declined and one that failed to reach the camera look identical in the
        // frame, and only one of them is a bug.
        println!(
            "LOOK_IBL gen={:?} nits={}",
            look_gen(),
            match &env {
                Some(e) => format!("{:.0}", e.intensity),
                None => String::from("off"),
            },
        );
        let mut e = commands.entity(cam);
        e.insert(LookIbl);
        if let Some(env) = env {
            e.insert(env);
        }
    }
}

/// Runtime tier swap: F7 steps Low → Medium → High → Ultra → Low.
///
/// Mutating the resource is the whole change — [`apply_look_to_cameras`] and
/// [`apply_look_to_sun`] read it next frame and rebuild the stack in place. This
/// is the manual test hook; the settings menu does the same `ResMut` write.
fn cycle_look_quality(keys: Res<ButtonInput<KeyCode>>, mut quality: ResMut<LookQuality>) {
    if !keys.just_pressed(KeyCode::F7) {
        return;
    }
    *quality = match *quality {
        LookQuality::Low => LookQuality::Medium,
        LookQuality::Medium => LookQuality::High,
        LookQuality::High => LookQuality::Ultra,
        LookQuality::Ultra => LookQuality::Low,
    };
    println!("LOOK_QUALITY -> {quality:?}");
}

// ===========================================================================
// A0 · Bevy's built-in physical atmosphere  (docs/sky-research-2026-08-14.md)
// ===========================================================================
//
// Sahara's research landed the fact this section is built on: `bevy = "0.19"`
// ALREADY SHIPS the Hillaire LUT atmosphere. Nothing is ported here and no
// dependency moves — `Atmosphere` is `bevy_light::atmosphere` (re-exported at
// `bevy_light-0.19.0/src/lib.rs:44`) and the render pipeline is
// `bevy_pbr::AtmospherePlugin` (`bevy_pbr-0.19.0/src/atmosphere/mod.rs:96`).
//
// AND THE PLUGIN IS NOT ADDED HERE, WHICH IS THE ONE CORRECTION TO THE WORK
// ORDER. `docs/sky-research-2026-08-14.md:71` reads "Add `AtmospherePlugin` to
// the app". It is already added: `PbrPlugin::build` adds it unconditionally at
// `bevy_pbr-0.19.0/src/lib.rs:252`, and `PbrPlugin` is in `DefaultPlugins`,
// which `main.rs` uses. `App::add_plugins` PANICS on a duplicate unique plugin
// ("plugin was already added in application"), so writing that line would have
// traded a missing sky for a boot crash. The plugin only builds its render
// graph if the adapter supports compute shaders and Rgba16Float storage
// textures (`mod.rs:137/146/155` warn and bail otherwise) — on a GPU that fails
// those checks the `Atmosphere` entity is inert and the frame falls back to
// `ClearColor`, which is why [`atmos_mode`] keeps the dome reachable.
//
// WHAT THIS BUYS AND WHAT IT DOES NOT, at Voxelforge's scale. The atmosphere is
// authored in METRES against a 6 360 km planet, and one block is one unit. The
// sky is therefore exactly right (it is a function of view direction and sun
// angle, not of scene size) — but the AERIAL PERSPECTIVE is not: Rayleigh
// scattering is 5.802e-6 per metre (`bevy_light-0.19.0/src/atmosphere.rs:203`),
// so across the ~64 blocks this map is deep the built-in in-scatter integrates
// to ~4e-4 of a unit — three orders of magnitude below one 8-bit level. So
// `docs/sky-research-2026-08-14.md:36` is right that the two systems are one
// physical family and wrong that this one "replaces the current `DistanceFog`
// haze": at block scale it CANNOT, and deleting the haze would delete the
// depth cue outright. The division of labour that ships here is
//
//     Atmosphere  -> the sky (everything at far depth)
//     DistanceFog -> the aerial perspective (everything that is geometry)
//
// and [`HAZE_START`]/[`HAZE_FULL`] are refitted to the map's MEASURED depth
// span (`scripts/_flamingo_depth_ruler.sh`) instead of to a guess, which is the
// other half of this change.

/// How far the aerial-view LUT is stretched, in metres — and metres are blocks.
///
/// Bevy's default is `3.2e4` (`bevy_pbr-0.19.0/src/atmosphere/mod.rs:350`): 32 km,
/// for a scene the size of a landscape flight sim. The aerial-view LUT is a 3-D
/// texture "fit to the view frustum" whose 32 z-slices are "distributed linearly
/// from the camera to this value" (`mod.rs:322-328`), so on the default the
/// ENTIRE playable map — every block of it inside the p99 measured by
/// `_flamingo_depth_ruler.sh` — falls inside the first slice and gets one
/// constant sample.
///
/// TIED TO [`HAZE_FULL`], NOT TO [`RENDER_RADIUS`] AND NOT TO A LITERAL. Past
/// `HAZE_FULL` the `DistanceFog` ramp is 100% opaque, so no atmosphere value
/// computed out there can reach a pixel — sampling to 320 would spend 4/5 of the
/// slices on depths the haze has already buried, which is the same mistake at
/// the same scale that `HAZE_FULL = 150` was. Tying the two means the LUT
/// re-aims itself the next time the haze is refitted to a re-measured set,
/// instead of drifting silently the way the old fit did.
const ATMOS_AERIAL_MAX_DISTANCE: f32 = HAZE_FULL;

/// Marks the single atmosphere entity so it is spawned exactly once.
#[derive(Component)]
struct AtmosphereAnchor;

/// `VOXELFORGE_LOOK_ATMOS=off|lut|raymarched` — the single-binary A/B lever this
/// lane requires of every look change, and the tier switch
/// `docs/sky-research-2026-08-14.md:95` asks for, in one hook.
///
/// * unset / `lut` — `AtmosphereMode::LookupTexture`, the shipped default and
///   Bevy's own (`mod.rs:416-422`): "high-performance … tailored to scenes that are
///   mostly inside of the atmosphere", which is every frame this game renders.
/// * `raymarched` — `AtmosphereMode::Raymarched`, the Ultra/cinematic toggle.
///   Not tiered off [`LookQuality`] yet ON PURPOSE: the research asks for it
///   "only if Poppy's profiling shows it is affordable", and wiring it to a tier
///   before that profile exists would ship an unmeasured cost.
/// * `off` — no atmosphere at all, which is also what re-enables the [`SkyDome`]
///   (see [`sky_grad_enabled`]). That is the before/after pair out of ONE
///   binary, same as `_SKYGRAD`, `_HAZE` and `_FOG` before it.
fn atmos_mode() -> Option<AtmosphereMode> {
    static MODE: std::sync::OnceLock<Option<AtmosphereMode>> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| {
        match std::env::var("VOXELFORGE_LOOK_ATMOS")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "off" | "0" => None,
            "raymarched" | "ray" => Some(AtmosphereMode::Raymarched),
            // Unset and anything unrecognised => the shipped LUT mode.
            _ => Some(AtmosphereMode::LookupTexture),
        }
    })
}

/// Run condition: is the built-in atmosphere the sky this session?
fn atmos_enabled() -> bool {
    atmos_mode().is_some()
}

/// Spawn the planet once, and dress the gameplay camera with the view settings.
///
/// TWO ENTITIES, AND NEITHER BELONGS TO ANOTHER LANE. The `Atmosphere` is a new
/// entity of this lane's own; the `AtmosphereSettings` goes on the existing
/// `OrbitCam` for the same reason [`apply_look_to_cameras`] filters on it —
/// `With<Camera3d>` would also dress the VFX lane's stage camera.
///
/// NO TRANSFORM IS WRITTEN. `Atmosphere` carries
/// `#[component(on_add = set_default_transform)]`
/// (`bevy_light-0.19.0/src/atmosphere.rs:34,58-68`), which places the planet
/// centre at `-Y * inner_radius` when the `GlobalTransform` is still `Default` —
/// i.e. the ground plane lands at y=0 with the surface normal on `+Y`, exactly
/// where this map's ground already is. `#[require(GlobalTransform)]` means that
/// default is present, so the hook fires. Setting a transform by hand here would
/// only be right if we also rescaled the planet, and we do not: see the section
/// note above on why the metre-scale aerial term is left to `DistanceFog`.
///
/// `AtmosphereSettings` carries `#[require(Hdr)]` (`mod.rs:288`). The camera is
/// already HDR because [`base_camera_look`]'s `Bloom` requires it, so this adds
/// no target change — but it is why this insert must not be moved onto a camera
/// that does not carry the base look.
fn atmosphere_sky(
    mut commands: Commands,
    mut media: ResMut<Assets<ScatteringMedium>>,
    anchor: Query<(), With<AtmosphereAnchor>>,
    cams: Query<Entity, (With<crate::OrbitCam>, Without<AtmosphereSettings>)>,
) {
    let Some(rendering_method) = atmos_mode() else {
        return;
    };
    if anchor.is_empty() {
        // `ScatteringMedium::default()` IS `ScatteringMedium::earth(256, 256)`
        // (`bevy_light-0.19.0/src/atmosphere.rs:148-152`) — Rayleigh + Mie +
        // Ozone at the paper's scale heights. Written as `earth(256, 256)`
        // rather than `default()` so the two LUT resolutions are visible at the
        // call site instead of hidden behind a trait impl.
        let medium = media.add(ScatteringMedium::earth(256, 256));
        commands.spawn((AtmosphereAnchor, Atmosphere::earth(medium)));
        println!(
            "LOOK atmosphere spawned mode={} aerial_max={ATMOS_AERIAL_MAX_DISTANCE}",
            match rendering_method {
                AtmosphereMode::Raymarched => "raymarched",
                _ => "lut",
            }
        );
    }
    for cam in &cams {
        commands.entity(cam).insert(AtmosphereSettings {
            aerial_view_lut_max_distance: ATMOS_AERIAL_MAX_DISTANCE,
            rendering_method,
            ..default()
        });
    }
}

// ===========================================================================
// A1 · the gradient sky dome  (art-order-2026-08-09-composition §A1)
// ===========================================================================
//
// The shipped sky was a single flat `ClearColor` (set in `apply_look_to_cameras`)
// — the brightest thing in a vista frame, painted one colour across its whole
// extent. Measured on the gate3 frames the whole sky spanned 0.3 units of
// luminance, i.e. it was a wall, not an atmosphere (`look-bible.md:111` calls
// exactly that read "no shader"). This dome replaces it with a smooth vertical
// gradient: deep saturated blue at the zenith, melting into the haze colour at
// the horizon so the bottom of the sky and the fully-hazed distant terrain are
// the SAME colour — there is no seam where sky meets world.
//
// WHY A VERTEX-COLOURED SPHERE, NOT A SHADER. This codebase has zero prior art
// for a custom material (`look.rs:554-559` says so), and a sky shader would be
// the first — a compile-then-debug tax on a contested build lane. A UV sphere
// whose every vertex carries the gradient colour for its elevation gets a
// perfectly smooth, seam-free gradient out of the built-in `StandardMaterial`
// vertex-colour path, with no WGSL of our own. The colours are LINEAR radiance
// authored to land at the same HDR value the flat `ClearColor` did (see the
// exposure note in [`build_sky_dome_mesh`]).

/// Dome radius, world units (blocks). Centred on the camera each frame, so it
/// always encloses the view. 640 is inside the default 1000-unit far plane and
/// well past the 320-unit streaming radius, so the dome is always the farthest
/// thing drawn: geometry (inside ~320) writes nearer depth and renders on top of
/// it, and where there is no geometry the dome shows as the sky.
const SKY_DOME_RADIUS: f32 = 640.0;

/// Marks the gradient-sky dome entity so the follow system can find it again.
#[derive(Component)]
struct SkyDome;

/// `VOXELFORGE_LOOK_SKYGRAD=off` reverts to the flat single-colour `ClearColor`
/// sky. The dome/flat A/B therefore comes out of ONE binary — the same env-swap
/// discipline every other `_LOOK_*` hook exists for.
///
/// THE ATMOSPHERE TAKES PRECEDENCE, AND IT HAS TO. The dome is opaque geometry
/// at [`SKY_DOME_RADIUS`] = 640 with `cull_mode: None`, so it writes depth
/// across every pixel the sky would otherwise occupy. Bevy's `render_sky` pass
/// only writes where the depth buffer is still at the far plane, so a live dome
/// does not "fight" the atmosphere — it hides it completely, and the frame would
/// come back looking exactly like today's while every atmosphere LUT was
/// computed and thrown away. Hence: atmosphere on ⇒ dome off, unconditionally.
/// `VOXELFORGE_LOOK_ATMOS=off` is what gets the dome back, which is also what
/// makes the pair a ONE-BINARY A/B rather than two builds.
///
/// Default: atmosphere on, dome off.
fn sky_grad_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    // Unset (None) => dome on; only the literal "off" disables it.
    *ON.get_or_init(|| {
        !atmos_enabled() && std::env::var("VOXELFORGE_LOOK_SKYGRAD").ok().as_deref() != Some("off")
    })
}

/// A linear colour scaled by `s` (componentwise). Kept explicit rather than
/// reaching for `LinearRgba`'s ops so the gradient maths has no trait import.
fn scale_lin(c: LinearRgba, s: f32) -> LinearRgba {
    LinearRgba::rgb(c.red * s, c.green * s, c.blue * s)
}

/// Linear lerp between two linear colours.
fn lerp_lin(a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
    LinearRgba::rgb(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
    )
}

/// Hermite smoothstep, clamped — the same easing the cine camera dolly uses.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The 3-stop vertical gradient colour for a normalised elevation `u`
/// (-1 nadir .. 0 horizon .. +1 zenith). At and below the horizon the dome is the
/// haze colour, so a glimpse of dome under the skyline still reads as air.
fn sky_gradient(u: f32, horizon: LinearRgba, mid: LinearRgba, zenith: LinearRgba) -> LinearRgba {
    if u <= 0.0 {
        horizon
    } else if u < 0.5 {
        lerp_lin(horizon, mid, smoothstep(0.0, 0.5, u))
    } else {
        lerp_lin(mid, zenith, smoothstep(0.5, 1.0, u))
    }
}

/// Build the dome mesh: a UV sphere with a per-vertex LINEAR colour from
/// [`sky_gradient`]. Colours are baked once at spawn — every input (sky hue,
/// `sky_gain`, exposure, haze colour) is launch-fixed, read from env at spawn —
/// so a baked mesh is exact and stable for the whole session.
fn build_sky_dome_mesh() -> Mesh {
    let h = hour();
    let r = SKY_DOME_RADIUS;
    // 64 sectors × 40 stacks: dense enough that the gradient reads continuous
    // across the upper hemisphere (40 latitude bands from nadir to zenith).
    let mut mesh = Sphere::new(r).mesh().uv(64, 40);

    // NO EXPOSURE COMPENSATION — and the note that used to stand here asserting
    // the opposite is why the sky shipped blown to white.
    //
    // It claimed Bevy applies `Exposure` to unlit fragments exactly as to lit
    // ones, cited `pbr_functions.wgsl`'s `exposure * (direct + indirect) +
    // emissive`, and pre-multiplied every vertex by `1 / exposure` so that the
    // multiply would cancel. The multiply never happens. In bevy_pbr 0.19
    // `pbr.wgsl:80-84` the unlit branch never reaches that line at all:
    //
    //     if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
    //         out.color = apply_pbr_lighting(pbr_input);   // <- exposure lives in here
    //     } else {
    //         out.color = pbr_input.material.base_color;   // <- the dome takes this
    //     }
    //
    // `view.exposure` is applied only inside `apply_pbr_lighting`
    // (`pbr_functions.wgsl:863`), so an unlit fragment writes its base colour
    // into the HDR target verbatim — which is *also* what makes the premise
    // wrong in the useful direction: unlit behaves exactly like `ClearColor`,
    // the very thing [`Hour::sky_gain`]'s own note says never passes through
    // `Exposure`. The two paths were already identical; the compensation was
    // correcting for a step that was never there, so at `ev100` 10.3 it shipped
    // the dome at 1/exposure() = 1513x its authored radiance (zenith went
    // [0.256, 0.765, 1.890] -> [387, 1157, 2859] linear). Everything past the
    // tonemapper's shoulder lands on the same near-white: measured on the two
    // 05:35 captures the sky is a flat [252, 226, 191] whose red moves 0.8
    // levels across 170 rows of elevation — a gradient authored deep-blue to
    // warm-haze, crushed to one value.
    //
    // So: author the vertex colours at exactly the scene-referred radiance the
    // flat `ClearColor` sky carried, and let the fragment write them through.
    // `sky_gain` keeps its meaning and the p95 / bloom behaviour it was tuned
    // against, which is what the old note wanted and did not get.
    //
    // Evidence: `scripts/_poppy_sky_exposure_probe.py` (shader quote + both
    // frames measured).
    let exp_comp = 1.0;

    // ZENITH = the hour's own sky hue at its own gain, exposure-compensated. The
    // top of the dome is therefore the exact colour/brightness the whole flat sky
    // was — so `VOXELFORGE_LOOK_SKYGAIN` sweeps the sky's brightness exactly as
    // it always did.
    let zenith = scale_lin(
        Color::srgb(h.sky[0], h.sky[1], h.sky[2]).to_linear(),
        h.sky_gain * exp_comp,
    );
    // HORIZON = the haze colour [`haze_color`] returns — the SAME value
    // `DistanceFog` dissolves distant geometry toward — exposure-compensated. The
    // dome carries `fog_enabled = false` (it IS the sky; distance fog is for
    // geometry), so the only way the bottom of the dome can meet the fully-hazed
    // skyline without a hard seam is to BE that haze colour. `haze_color` already
    // folds `sky_gain` in (its scale is `HAZE_GAIN * sky_gain`), so the horizon
    // tracks the zenith as gain is swept. `VOXELFORGE_LOOK_SKYHOR=r,g,b` overrides
    // the horizon hue (sRGB) for one-off tuning.
    let horizon_base = match env_floats::<3>("VOXELFORGE_LOOK_SKYHOR") {
        Some([hr, hg, hb]) => Color::srgb(hr, hg, hb).to_linear(),
        None => haze_color().to_linear(),
    };
    let horizon = scale_lin(horizon_base, exp_comp);
    // MID = the transition stop, biased toward the horizon so the warm band is
    // broad and the deep blue sits high — a golden-hour sky reads warm at the
    // bottom, blue up top.
    let mid = lerp_lin(zenith, horizon, 0.55);

    // Vertex colours from elevation. Positions are read off the builder's own
    // POSITION attribute, so the colour topology can never disagree with the mesh.
    let positions: Vec<[f32; 3]> = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("SphereMeshBuilder always emits POSITION")
        .as_float3()
        .expect("POSITION is Float32x3")
        .to_vec();
    let colours: Vec<[f32; 4]> = positions
        .iter()
        .map(|p| {
            let u = (p[1] / r).clamp(-1.0, 1.0);
            let c = sky_gradient(u, horizon, mid, zenith);
            [c.red, c.green, c.blue, 1.0]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    mesh
}

/// Spawn the dome on the first frame the gameplay camera exists, then hold it
/// centred on the camera so the gradient stays anchored to WORLD up as the boom
/// orbits (rotation is never written, so the zenith is always straight up).
fn sky_dome(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // `Without<SkyDome>` proves this read query is disjoint from `dome`'s
    // `&mut Transform` below — without it Bevy cannot prove the two Transform
    // accesses never alias the same entity and panics with error[B0001] the
    // first frame `dome.single_mut()` is reached (which only happens once
    // `cam.iter().next()` started succeeding — see commit history).
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<SkyDome>)>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(cam_tf) = cam.iter().next() else {
        return;
    };
    if let Ok(mut tf) = dome.single_mut() {
        tf.translation = cam_tf.translation;
        return;
    }
    let mesh = meshes.add(build_sky_dome_mesh());
    // ── TEMP-A1-PROOF (REVERT before shipping) ───────────────────────────────
    // The A1 dome was proven dead at the fragment level
    // (docs/_rose_a1_dome_dead_2026-08-11.md): an HDR green-8.0 emissive on the
    // dome changed nothing — every frame stayed gold, i.e. the dome geometry never
    // reached the fragment shader. Root cause, fixed two blocks down: the spawn
    // was missing a `Visibility` component, so Bevy never computed a
    // `ViewVisibility` for the entity and the render world never extracted it.
    // Every other mesh entity in this crate spawns with `Visibility::default()`
    // (the Maren NPC in quest.rs, imported models in import.rs); the dome was the
    // sole mesh entity without it. The earlier green probe "did nothing" because
    // those captures came off a contested build lane — the three `_dbg_dome*`
    // frames were near-identical regardless of which flag was set, the signature
    // of a binary that never relinked, not of a live material with no effect.
    //
    // `VOXELFORGE_LOOK_SKYPROBE=1` swaps the dome material for a nuclear-green
    // unlit emissive so the falsification is unambiguous: if the dome now renders,
    // the WHOLE frame goes green; if it stays gold the fix is wrong. The shipped
    // path (env unset) keeps the real gradient dome. REVERT this block (the env
    // read + the `if sky_probe` branch) once green is confirmed, leaving only the
    // `Visibility::default()` + `NotShadowCaster` lines on the spawn below.
    let sky_probe = std::env::var("VOXELFORGE_LOOK_SKYPROBE")
        .map(|v| v.trim() == "1")
        .unwrap_or(false);
    let material = materials.add(if sky_probe {
        println!(
            "LOOK-A1-PROOF: nuclear-green emissive ACTIVE (Visibility+NotShadowCaster on spawn) \
             — frame MUST go green, or the dome still does not render"
        );
        StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(0.0, 8.0, 0.0),
            unlit: true,
            fog_enabled: false,
            cull_mode: None,
            ..default()
        }
    } else {
        StandardMaterial {
            base_color: Color::WHITE,
            // Unlit: the dome's colour IS its vertex colour (white base × vertex
            // colour); no sun/fill shading on the sky.
            unlit: true,
            // The dome IS the sky. Distance fog is for geometry; if it applied
            // here the dome at 640 units (well past `HAZE_FULL`) would fog out to
            // a flat haze plate and bury the gradient.
            fog_enabled: false,
            // Render both faces so the sphere is visible from inside (the camera
            // sits at its centre; the outward-facing winding would otherwise be
            // culled).
            cull_mode: None,
            ..default()
        }
    });
    commands.spawn((
        SkyDome,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(cam_tf.translation),
        // ROOT-CAUSE FIX (A1): without an explicit `Visibility` the dome carried no
        // `ViewVisibility`, so the render world never extracted it and the sky read
        // as the flat `ClearColor`. `Visibility::default()` (Inherited) on a root
        // entity computes visible — exactly how every other mesh entity here is
        // spawned (the Maren NPC, imported models). See TEMP-A1-PROOF above and
        // docs/_rose_a1_dome_dead_2026-08-11.md.
        Visibility::default(),
        // The dome encloses the whole scene; left as a default shadow caster its
        // shell would shadow everything inside it. It is the sky — never a caster.
        NotShadowCaster,
    ));
    println!(
        "LOOK sky-dome spawned r={SKY_DOME_RADIUS} sky_gain={:.2} ev100={:.1}",
        hour().sky_gain,
        hour().ev100
    );
}

// ===========================================================================
// A2 · the play-scene fog volume  (art-order-2026-08-09-composition §A2)
// ===========================================================================
//
// `VolumetricFog` is on the camera (High/Ultra) and `VolumetricLight` is on the
// sun — but Bevy only ray-marches the volumetric term INSIDE a `FogVolume`, and
// the only one in the repo was the hero shot's indoor volume (`hero.rs:662`). So
// every `--play` frame paid the 32/96-step march and rendered it in a vacuum: no
// god rays, ever, at any tier. This spawns the missing medium around the play
// camera so the installed machinery finally has something to scatter through.

/// God-ray medium density. The hero shot signed off its readable shafts at
/// `fog.unwrap_or(0.032)`; matched here so the play frame reads "shafts of sunlit
/// air", not "solid fog wall" (`look-acceptance-rubric.md:156`).
const PLAY_FOG_DENSITY: f32 = 0.030;
/// Half-extents (radii) of the play fog volume, in blocks. Wide and shallow: a
/// 22° sun throws near-horizontal shafts, and the volume only has to cover the
/// depth the camera actually looks through. It follows the camera, so these are
/// radii around the viewpoint, not the map. (Bevy sizes a `FogVolume` by the
/// Transform scale = full extent, so spawn uses `* 2.0`.)
const PLAY_FOG_HALF: Vec3 = Vec3::new(200.0, 80.0, 200.0);

/// Marks the play-scene fog volume so the follow system finds it.
#[derive(Component)]
struct PlayFogVolume;

/// `VOXELFORGE_LOOK_VFOG=<density>|off`. Default: spawn at [`PLAY_FOG_DENSITY`].
/// `off` removes the medium — the vacuum the ray-march ran in before this lane —
/// so the before/after god-ray proof comes out of one binary (same build, env
/// swap, like every other `_LOOK_*` A/B). Cached: env cannot change post-launch.
fn play_fog_density() -> Option<f32> {
    static D: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *D.get_or_init(|| match std::env::var("VOXELFORGE_LOOK_VFOG").ok().as_deref() {
        Some("off") => None,
        Some(v) => v
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|d| *d >= 0.0)
            .or(Some(PLAY_FOG_DENSITY)),
        None => Some(PLAY_FOG_DENSITY),
    })
}

/// Spawn the play fog volume once (first frame the camera exists and the medium
/// is not disabled), then hold it centred on the camera so the god-ray medium
/// stays around the viewpoint wherever the player roams.
fn play_fog_volume(
    mut commands: Commands,
    // Same B0001 guard as `sky_dome`: `Without<PlayFogVolume>` proves this read
    // query is disjoint from `fog`'s `&mut Transform` so Bevy never flags a
    // conflicting-Transform access when `fog.single_mut()` is reached.
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<PlayFogVolume>)>,
    mut fog: Query<&mut Transform, With<PlayFogVolume>>,
    mut spawned: Local<bool>,
) {
    let Some(density) = play_fog_density() else {
        return; // `off`: never spawn — the no-medium baseline
    };
    let Some(cam_tf) = cam.iter().next() else {
        return;
    };
    if !*spawned {
        commands.spawn((
            PlayFogVolume,
            FogVolume {
                // Golden: it IS sunlit air, B lifted a touch so the shafts don't
                // paint the whole frame orange (same call the hero shot made).
                fog_color: Color::srgb(1.0, 0.88, 0.70),
                density_factor: density,
                scattering: 0.55,
                ..default()
            },
            Transform::from_translation(cam_tf.translation).with_scale(PLAY_FOG_HALF * 2.0),
        ));
        *spawned = true;
        println!("LOOK play FogVolume spawned density={density:.3} (god-ray medium)");
        return;
    }
    if let Ok(mut tf) = fog.single_mut() {
        tf.translation = cam_tf.translation;
    }
}
