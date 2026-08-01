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
//! components to it. Nothing that main.rs authored is overwritten: exposure,
//! ambient brightness and the sun's illuminance are all left exactly as the
//! gameplay lane set them. This module only ever adds rendering components.
//!
//! WHY IT INSERTS IN `Update`, NOT `Startup`. System order against another
//! lane's `Startup` is not a contract. The camera is spawned in main.rs's setup,
//! and a `Startup` system here would race it — sometimes the query is empty and
//! the whole look silently no-ops. Running in `Update` finds the camera the frame
//! after it spawns, and a camera respawned mid-session (map reload) gets the
//! stack too instead of coming back bare.
//!
//! QUALITY TIERS. The stack is built per [`LookQuality`] (a resource, default
//! [`LookQuality::High`]) so Steam isn't an Ultra-or-nothing proposition: a
//! mid-range card holds the High frame, Low is the cheapest frame that still
//! reads "Voxelforge". The tier is live — F7 cycles it and a future settings
//! menu can mutate the resource; [`apply_look_to_cameras`] / [`apply_look_to_sun`]
//! see the change next frame and rebuild the stack on the *existing* camera and
//! sun in place (strip + re-insert), with no restart. Poppy's per-effect ms
//! budget, when it lands, retunes the groupings in [`insert_stack`] / the PCSS
//! match below — that is the one place the tier→effect map lives.

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{DirectionalLightShadowMap, ShadowFilteringMethod, VolumetricFog, VolumetricLight};
use bevy::pbr::{
    DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
use serde::{Deserialize, Serialize};

/// Post-grade constants, carried over from the signed-off hero shot.
///
/// These are the numbers the reviewer approved on the golden beauty shot, not
/// fresh guesses — see `docs/golden-beauty-shot.md` and the grade-vs-golden
/// rounds recorded in `hero.rs`. They are named here so a future tuning pass has
/// one place to edit and so a diff against `hero.rs` stays readable.
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
    /// the authored sky `main.rs` clears to — srgb 135/184/235, ordering
    /// `B > G > R` — comes out of the white balance as 211/169/210, i.e.
    /// **`R > B > G`. That is the magenta.** The same script pins the two
    /// crossings on that sky: red passes green at ≈0.048 (B>G>R → B>R>G — still
    /// blue; green is the leg it passes, not blue) and red passes *blue* — the
    /// real magenta onset, R>B>G — at **≈0.099**. The old "flips ≈0.045"
    /// conflated the two; 0.045 is only where R>G begins, not where the frame
    /// goes magenta. 0.02 sits ~5× under the true onset (0.099 / 0.02).
    ///
    /// `hero.rs` never saw this because its scene has no sky: it clears to
    /// near-black (0.05, 0.03, 0.02) and every lit surface is already amber, so
    /// a red push there only deepens an ordering that was `R > G > B` to begin
    /// with. Byte-identical constants, opposite result — which is exactly why
    /// the warmth the outdoor frame needs is now carried by [`KEY_COLOR`] and
    /// [`AMBIENT_COLOR`] (light, which the flat sky clear does not receive)
    /// instead of by a global matrix (which it does).
    pub const TEMPERATURE: f32 = 0.02;
    /// AcesFitted flattens saturation (~80% vs the golden's 96%); this puts the
    /// punch back across the whole frame.
    pub const POST_SATURATION: f32 = 1.02;
    /// Midtone contrast — spreads values off mid-grey, which is the micro-contrast
    /// / voxel-grain axis. Lit wood grain and edge detail live in the midtones.
    pub const MIDTONE_CONTRAST: f32 = 1.30;
    /// Highlight contrast, matched to the midtones so the curve doesn't kink at
    /// the section boundary.
    pub const HIGHLIGHT_CONTRAST: f32 = 1.30;
    /// Highlight roll-off (gain < 1). The filmic shoulder AcesFitted alone can't
    /// do: it compresses ONLY the brightest surfaces — sky, sunlit wedges, window
    /// panes — back into band, while leaving bounce-lit shade and midtones alone.
    /// Lighting can't do this, because "less light" moves shadows down too.
    pub const HIGHLIGHT_GAIN: f32 = 0.64;

    /// Golden-hour key colour for the sun, in sRGB. `main.rs` spawns the
    /// directional light with Bevy's default WHITE because that is the neutral
    /// the gameplay/editor/bench lanes want; the *look* lane is the thing that
    /// decides the hour of the day, so the tint lands here.
    ///
    /// This is where the frame's warmth is supposed to come from. Warmth from a
    /// light is chromatically honest: it multiplies the surfaces the light
    /// reaches (so sunlit stone reads `R > G > B`, which is gate G6) and it
    /// leaves the `ClearColor` sky alone, because a flat clear is not a lit
    /// surface. Warmth from [`TEMPERATURE`] cannot tell the two apart, which is
    /// the whole magenta bug. Ordering is `R > G > B` with the green leg kept
    /// clearly above blue — Look Bible §4's amber, not the fire-red `hero.rs`
    /// documents collapsing into.
    ///
    /// Illuminance is NOT touched — that is `main.rs`'s 9000 lux and a gameplay
    /// decision. Only the hue changes.
    pub const KEY_COLOR: [f32; 3] = [1.00, 0.86, 0.66];

    /// Warm bounce fill, in sRGB — the same idea as [`KEY_COLOR`] for the
    /// camera's `AmbientLight`, and the axis that actually moves the *midtone*
    /// numbers (`grade_axes.py` warmth R−B / blue B are measured on the midtone
    /// band, which is open shade and bounce, not the sunlit wedge).
    ///
    /// Milder than `hero.rs`'s honey (0.784, 0.541, 0.180): that is an interior
    /// whose every wall is a bounce surface, this is an outdoor scene where the
    /// ambient is standing in for sky light. Brightness is left at `main.rs`'s
    /// 380 lux.
    pub const AMBIENT_COLOR: [f32; 3] = [0.98, 0.78, 0.52];
}

/// Live override for the grade knobs — `VOXELFORGE_LOOK_GRADE=temp,sat,mid,hi_gain`.
///
/// The same no-recompile sweep hook `hero.rs` carries as `VOXELFORGE_GRADE`, and
/// here for a sharper reason: the value of [`grade::TEMPERATURE`] that keeps the
/// authored sky's `B > G > R` ordering alive *through* AcesFitted and the
/// sectional curve is an empirical search, and a release rebuild per candidate is
/// minutes. Unset — the shipped path, and every gate run — returns the constants
/// byte-for-byte. Malformed input falls back to the constants rather than
/// panicking mid-frame: this is a debug hook, not a config file.
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
/// [`grade_knobs`]; unset returns [`grade::KEY_COLOR`] / [`grade::AMBIENT_COLOR`].
fn light_colors() -> (Color, Color) {
    let k = grade::KEY_COLOR;
    let a = grade::AMBIENT_COLOR;
    let d = (
        Color::srgb(k[0], k[1], k[2]),
        Color::srgb(a[0], a[1], a[2]),
    );
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
/// the frame, and the full hero stack (volumetric god rays, Ultra SSAO, PCSS) is
/// what a stronger GPU earns, not the shipped experience. The tier→effect map is
/// in [`insert_stack`]; the only thing that needs touching to retune it from
/// Poppy's per-effect ms numbers is that one `match`. Effects are a strict
/// subset down the ladder (everything Low has, Medium has too, … up to Ultra).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LookQuality {
    /// Tonemapping + ColorGrading + Bloom only. Everything stochastic, temporal
    /// or ray-marched is off — the cheapest frame the look lane can produce that
    /// still reads as Voxelforge (the grade *is* the identity).
    Low,
    /// + TAA, ordinary hardware shadow filter, low-quality SSAO and distance
    /// haze. No depth-of-field, no PCSS penumbra.
    Medium,
    /// + Temporal soft shadows, depth-of-field, medium-quality SSAO. No
    /// volumetric fog — the ray-march is the single most expensive effect in the
    /// stack and the one worth cutting first. The default.
    #[default]
    High,
    /// + Ultra SSAO + PCSS soft shadows + volumetric god rays. Everything the
    /// hero beauty shot was signed off at — the full stack.
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
type LookStack = (
    Msaa,
    Tonemapping,
    ColorGrading,
    ShadowFilteringMethod,
    TemporalAntiAliasing,
    Bloom,
    DepthOfField,
    ScreenSpaceAmbientOcclusion,
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
            // map the gate signed off on. The cost is ~64MB of VRAM, not fps.
            .insert_resource(DirectionalLightShadowMap { size: 4096 })
            .add_systems(
                Update,
                (
                    apply_look_to_cameras,
                    apply_look_to_sun,
                    focus_dof,
                    cycle_look_quality,
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
/// (another lane's file), which has no orbit boom for `focus_dof` to track.
///
/// `Option<Res<_>>` because a plugin shouldn't assume another lane's resource is
/// inserted before it — a missing `Cfg` means "not the play path", not a panic.
///
/// `VOXELFORGE_LOOK_DISABLE` forces this off regardless of `cfg.play` — the
/// Gate 3 before/after pair needs one binary, one scene, with only the look
/// stack toggled, and a quality tier isn't "off": even `LookQuality::Low` still
/// carries the tonemap/grade/bloom identity.
fn look_enabled(cfg: Option<Res<crate::Cfg>>) -> bool {
    if std::env::var_os("VOXELFORGE_LOOK_DISABLE").is_some() {
        return false;
    }
    cfg.is_some_and(|c| c.play)
}

/// Build the look stack for `quality` onto the camera entity `e`.
///
/// This is the single tier→effect map — the place Poppy's per-effect ms budget
/// reshapes the ladder. Each tier is a strict superset of the one below it, so
/// the base layer (grade + tonemap + bloom, the look's identity) is inserted
/// first and the `match` only adds the effects that tier earns.
fn insert_stack(e: &mut EntityCommands, quality: LookQuality) {
    let (temperature, post_saturation, midtone_contrast, highlight_gain) = grade_knobs();
    // Base layer — present at every tier, the parts that make the frame read
    // "Voxelforge" at all: filmic tonemap, the golden grade, warm bloom. MSAA
    // stays off because voxel edges are 90° and axis-aligned (no jaggies to
    // smooth), and every higher tier's SSAO requires it off anyway.
    e.insert((
        Msaa::Off,
        Tonemapping::AcesFitted,
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
        // Trimmed from NATURAL's default: bloom was bleeding highlight energy
        // across voxel edges and softening the grain (micro 3.8 vs the golden's
        // 5.2). NATURAL's high threshold still keeps it off the shadows.
        Bloom {
            intensity: 0.26,
            ..Bloom::NATURAL
        },
    ));
    match quality {
        LookQuality::Low => {
            // The cheapest tier that still clears every gate-identity axis (G1–G6):
            // the grade base plus the two "cut = dies" G4 layers at their floor
            // quality. No TAA, no PCSS, no DOF, no volumetrics — the temporal and
            // ray-marched passes that need accumulation or cost the most stay off
            // (look-tier-spec.md §1). Without these two, Low was failing both
            // halves of G4: hovering blocks (no AO) and a hard shadow edge (no
            // explicit filter).
            e.insert((
                // Gaussian is a fixed multi-tap blur with no history buffer, so it
                // gives a soft (≥3px) shadow edge without TAA — exactly the §1
                // "cut survives" choice for the no-TAA tier. Leaving Low without an
                // explicit filter left the shadow edge at an unintended default
                // rather than the soft edge G4 wants.
                ShadowFilteringMethod::Gaussian,
                // Contact AO is identity, not a luxury: without it blocks hover off
                // the ground. The layer must be present at every tier (§1 ranks it
                // "cut = dies"); only the quality steps down. Low is the floor.
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Low,
                    constant_object_thickness: 1.45,
                },
            ));
        }
        LookQuality::Medium => {
            // TAA cleans up the low-quality SSAO. The shadow filter stays temporal
            // too — it's free here because Medium already runs TAA, and the fixed
            // `Hardware2x2` (2×2 PCF) doesn't reliably reach the ≥3px soft edge G4
            // wants (look-tier-spec.md §5 problem #2). No PCSS, no accumulation
            // beyond what TAA gives the SSAO. Low-quality SSAO + distance haze
            // round it out. No DOF.
            e.insert((
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Low,
                    // Voxel-scale thickness: darkens the contact crease so things
                    // sit ON the ground instead of looking like they hover.
                    constant_object_thickness: 1.45,
                },
                DistanceFog {
                    // Desaturated and thin — keeps material tint readable with
                    // depth instead of flattening distant geometry into the haze.
                    color: Color::srgb(0.50, 0.42, 0.28),
                    falloff: FogFalloff::Exponential { density: 0.008 },
                    ..default()
                },
            ));
        }
        LookQuality::High => {
            // Default tier. Temporal soft shadows (no PCSS) + DOF + medium SSAO +
            // volumetric fog at a reduced step count + distance haze. The one
            // effect cut vs Ultra is PCSS: per look-tier-spec.md §1 the hero-shot
            // measurement found Bevy clamps `soft_shadow_size` to its 0.5 floor in
            // a room this size, so PCSS buys almost nothing — cut it BEFORE the
            // volumetric ray-march, the visible atmosphere that pins the frame (§5
            // problem #3 swaps the cut order so High no longer pays for PCSS while
            // dropping the god-ray layer wholesale).
            e.insert((
                // SSAO is stochastic; one frame is visibly noisy. Temporal
                // filtering + TAA accumulate it into clean contact AO. (PCSS, the
                // other stochastic effect that paired with these at the hero tier,
                // is off here — see the arm note above.)
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                    constant_object_thickness: 1.45,
                },
                DepthOfField {
                    mode: DepthOfFieldMode::Bokeh,
                    // Focus starts at the boom length so the avatar is sharp on
                    // frame one; `focus_dof` then tracks the live boom.
                    focal_distance: crate::BOOM_DIST,
                    // Softer than the hero still's cinematic stop — this is a
                    // camera someone plays behind, intent on depth separation
                    // on distant terrain, not a melted background.
                    aperture_f_stops: 4.0,
                    // Bevy's CoC scales as focal_length²/(sensor·N). With the
                    // physical ~18.6mm default sensor the CoC is near-zero at
                    // this world scale; a large-format sensor makes the effect
                    // exist at all.
                    sensor_height: 0.35,
                    ..default()
                },
                // Volumetric fog at a reduced step count (Ultra ray-marches 96; 32
                // keeps the god-ray shaft at a fraction of the cost, with banding
                // TAA smooths out). `VolumetricLight` on the sun — set in
                // `apply_look_to_sun` — is what lets this pass treat it as an
                // in-scatterer; the two are coupled (spec §3 rule 3).
                (
                    VolumetricFog {
                        ambient_intensity: 0.08,
                        step_count: 32,
                        jitter: 0.6,
                        ..default()
                    },
                    DistanceFog {
                        color: Color::srgb(0.50, 0.42, 0.28),
                        falloff: FogFalloff::Exponential { density: 0.008 },
                        ..default()
                    },
                ),
            ));
        }
        LookQuality::Ultra => {
            // The full signed-off hero stack: temporal PCSS + Ultra SSAO + DOF +
            // volumetric god rays + distance haze.
            e.insert((
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
                    constant_object_thickness: 1.45,
                },
                DepthOfField {
                    mode: DepthOfFieldMode::Bokeh,
                    focal_distance: crate::BOOM_DIST,
                    aperture_f_stops: 4.0,
                    sensor_height: 0.35,
                    ..default()
                },
                (
                    VolumetricFog {
                        ambient_intensity: 0.08,
                        step_count: 96,
                        // Dithers the ray-march step boundary so TAA resolves
                        // the shaft with a soft edge instead of a hard cut.
                        jitter: 0.6,
                        ..default()
                    },
                    DistanceFog {
                        color: Color::srgb(0.50, 0.42, 0.28),
                        falloff: FogFalloff::Exponential { density: 0.008 },
                        ..default()
                    },
                ),
            ));
        }
    }
}

/// Give every 3D camera the post stack for the live [`LookQuality`] tier.
///
/// Compares the tier stamped on the camera (`LookApplied`) to the live resource
/// each frame: equal ⇒ skip (steady state is one enum compare per camera, no
/// insert), different (first apply, or a runtime tier change) ⇒ strip the whole
/// stack and rebuild it for the new tier on the same entity. That is the
/// runtime-swap contract — no respawn, no restart, the existing camera keeps its
/// `OrbitCam` / `AmbientLight` / exposure exactly as the gameplay lane set them.
fn apply_look_to_cameras(
    mut commands: Commands,
    quality: Res<LookQuality>,
    mut q: Query<(Entity, Option<&LookApplied>, Option<&mut AmbientLight>), With<Camera3d>>,
) {
    for (cam, applied, ambient) in &mut q {
        if applied.is_some_and(|a| a.0 == *quality) {
            continue;
        }
        // Tint the camera's bounce fill warm, brightness untouched. This is the
        // midtone half of the warmth that used to come out of the white-balance
        // matrix — see `grade::AMBIENT_COLOR`. `Option<&mut _>` because a camera
        // without its own `AmbientLight` (the editor's) is not this lane's to
        // give one to; it just doesn't get the tint.
        if let Some(mut ambient) = ambient {
            ambient.color = light_colors().1;
        }
        let mut e = commands.entity(cam);
        e.remove::<LookStack>();
        insert_stack(&mut e, *quality);
        e.insert(LookApplied(*quality));
    }
}

/// Tier the two look knobs that live on the directional light, not the camera:
/// the PCSS penumbra width (a field on `DirectionalLight`) and `VolumetricLight`
/// (the component that lets the volumetric-fog pass treat this light as an
/// in-scatterer). Same tier-tracking idea as [`apply_look_to_cameras`].
///
/// This mutates the existing `DirectionalLight` rather than inserting a new one:
/// `main.rs` set `illuminance` and `shadow_maps_enabled`, and those are gameplay
/// lighting decisions that aren't this lane's to overwrite.
fn apply_look_to_sun(
    mut commands: Commands,
    quality: Res<LookQuality>,
    mut q: Query<(Entity, &mut DirectionalLight, Option<&LookLightApplied>)>,
) {
    for (light, mut dl, applied) in &mut q {
        if applied.is_some_and(|a| a.0 == *quality) {
            continue;
        }
        // PCSS penumbra is on only at Ultra: High cuts it first (spec §1 ranks it
        // the cheapest thing to drop, before the volumetric ray-march it now keeps).
        // High and Ultra opt the light into the volumetric pass; Medium/Low don't.
        let pcss = matches!(*quality, LookQuality::Ultra);
        let volumetric = matches!(*quality, LookQuality::High | LookQuality::Ultra);
        // Golden-hour key. `illuminance` and `shadow_maps_enabled` stay exactly
        // as main.rs set them — only the hue is the look lane's call. This is
        // what makes sunlit surfaces order `R > G > B` (G6) without a global
        // matrix that would drag the sky along with them.
        dl.color = light_colors().0;
        // `soft_shadow_size` only exists when Bevy's PCSS flag is compiled in.
        // It's in this crate's default features, but a `--no-default-features`
        // build is a real configuration and shouldn't fail to compile over a
        // look knob.
        #[cfg(feature = "experimental_pbr_pcss")]
        {
            // 3.0 is the width the hero shot's penumbra gate was signed off at.
            dl.soft_shadow_size = if pcss { Some(3.0) } else { None };
        }
        #[cfg(not(feature = "experimental_pbr_pcss"))]
        {
            let _ = (&mut dl, pcss);
        }
        let mut e = commands.entity(light);
        if volumetric {
            e.insert(VolumetricLight);
        } else {
            // Without this the light contributes no in-scattering, so
            // `VolumetricFog` renders nothing — god rays need a light that opts
            // in. Removing it on non-Ultra tiers keeps the sun honest about what
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
/// is the manual test hook; a settings menu would do the same `ResMut` write.
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

/// Keep the depth-of-field focus locked on the avatar as the boom moves.
///
/// A fixed `focal_distance` is fine for a still. It is wrong for a third-person
/// camera whose boom is pulled in by `camera_boom` whenever a wall would clip
/// between camera and avatar — the focus plane would stay out at 6.5 while the
/// camera sat at 2, and the player character, the one thing that must never be
/// soft, would go blurry exactly when the camera hugged a wall.
///
/// Tiers below High carry no `DepthOfField` component, so the query matches
/// nothing and this is a free no-op for them.
fn focus_dof(mut q: Query<(&mut DepthOfField, &crate::OrbitCam)>) {
    for (mut dof, orbit) in &mut q {
        dof.focal_distance = orbit.dist;
    }
}
