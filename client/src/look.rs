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
    CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod, VolumetricFog,
    VolumetricLight,
};
use bevy::pbr::{
    ContactShadows, DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion,
    ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
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
/// [`RENDER_RADIUS`] is visible any more, so that shell — now **170 blocks**,
/// grown from 70 by this change — is pure draw cost. Tightening `RENDER_RADIUS`
/// toward `HAZE_FULL` is available and is Kevin's call, not this lane's, which is
/// why this const does not make it. It is worth more now than it was.
pub const HAZE_FULL: f32 = 150.0;

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

/// PCSS penumbra width for the sun, where the tier turns it on.
///
/// 3.0 → 4.0 (2026-08-06). `measure_penumbra.py` read a 4 px median edge on
/// `wide-hero-final-nohud2.png` and 3 px on the vista frame, against G4a's 5 px
/// floor; the gameplay frames, which are shot at High and take the Gaussian
/// path instead, already read 5–9 px. So the miss is specific to the tier that
/// runs THIS constant, and this is the only knob it has. Sweep it with
/// `VOXELFORGE_LOOK_PCSS=<width>` before moving it again.
pub const PCSS_WIDTH: f32 = 4.0;

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
    pub const POST_SATURATION: f32 = 1.90;

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
    /// Bounce-fill brightness, lux.
    pub ambient_lux: f32,
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
        ambient_lux: 2200.0,
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
        ambient_lux: 90.0,
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
}

/// The live hour, plus the env overrides used to sweep it without a rebuild.
///
/// Unset env ⇒ the constants byte-for-byte, which is what every gate run and the
/// shipped binary get.
fn hour() -> Hour {
    let mut h = if std::env::var_os("VOXELFORGE_LOOK_NIGHT").is_some() {
        Hour::NIGHT
    } else {
        Hour::GOLDEN
    };
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
    if let Some(lux) = std::env::var("VOXELFORGE_LOOK_AMBIENT")
        .ok()
        .and_then(|v| v.trim().parse().ok())
    {
        h.ambient_lux = lux;
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
        grade::TEMPERATURE,
        grade::POST_SATURATION,
        grade::MIDTONE_CONTRAST,
        grade::HIGHLIGHT_GAIN,
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

/// Wears the look lane's post stack over whatever camera and sun the game spawned.
pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        // Default High, not Ultra — see `LookQuality`. `VOXELFORGE_LOOK_QUALITY`
        // overrides the starting tier so Poppy can profile each tier headless
        // (`=low|medium|high|ultra`); anything unrecognised falls back to High.
        let initial = std::env::var("VOXELFORGE_LOOK_QUALITY")
            .ok()
            .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
                "low" => Some(LookQuality::Low),
                "medium" => Some(LookQuality::Medium),
                "high" => Some(LookQuality::High),
                "ultra" => Some(LookQuality::Ultra),
                _ => None,
            })
            .unwrap_or_default();
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
                (apply_look_to_cameras, apply_look_to_sun, cycle_look_quality)
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
            shadows: ColorGradingSection {
                contrast: 1.0,
                ..default()
            },
            midtones: ColorGradingSection {
                contrast: midtone_contrast,
                ..default()
            },
            highlights: ColorGradingSection {
                contrast: grade::HIGHLIGHT_CONTRAST,
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
        Bloom {
            intensity: 0.18,
            prefilter: BloomPrefilter {
                threshold: 1.0,
                threshold_softness: 0.4,
            },
            ..Bloom::NATURAL
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
    let (length, thickness, steps) = match v[..] {
        [l, t, s] => (l, t, s.max(1.0) as u32),
        // Malformed input falls back to the constants rather than panicking
        // mid-frame: this is a sweep hook, not a config file.
        _ => (
            CONTACT_SHADOW_LENGTH,
            CONTACT_SHADOW_THICKNESS,
            CONTACT_SHADOW_STEPS,
        ),
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
    match std::env::var("VOXELFORGE_LOOK_PCSS") {
        Ok(v) if v.trim().eq_ignore_ascii_case("off") => None,
        Ok(v) => v.trim().parse().ok().or(tier_on.then_some(PCSS_WIDTH)),
        Err(_) => tier_on.then_some(PCSS_WIDTH),
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
            // is PCSS: per look-tier-spec.md §1 the hero-shot measurement found
            // Bevy clamps `soft_shadow_size` to its 0.5 floor in a room that size,
            // so PCSS buys almost nothing — cut it BEFORE the volumetric ray-march,
            // the visible atmosphere that pins the frame (§5 problem #3 swaps the
            // cut order so High no longer pays for PCSS while dropping the god-ray
            // layer wholesale).
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
    mut q: Query<(
        Entity,
        &mut DirectionalLight,
        &mut Transform,
        Option<&LookLightApplied>,
    )>,
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
        // PCSS penumbra is on only at Ultra: High cuts it first (spec §1 ranks it
        // the cheapest thing to drop, before the volumetric ray-march it now keeps).
        // High and Ultra opt the light into the volumetric pass; Medium/Low don't.
        let pcss = matches!(*quality, LookQuality::Ultra);
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
                first_cascade_far_bound: 16.0,
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
        e.insert(LookLightApplied(*quality));
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
