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
    /// Saturation push. TonyMcMapface (see [`super::insert_stack`]) is a neutral
    /// transform that neither adds nor removes saturation, so this is a small
    /// deliberate lift toward the reference's vivid leaves/sky — NOT the +0.02
    /// repair job AcesFitted's ~80% flattening used to need.
    pub const POST_SATURATION: f32 = 1.05;
    /// Midtone contrast — spreads values off mid-grey, which is the micro-contrast
    /// / voxel-grain axis. Lit wood grain and edge detail live in the midtones.
    ///
    /// 1.30 → 1.12 with the tonemapper change. AcesFitted bakes a strong S-curve
    /// in and then washes the midtones back out, so the old value was buying that
    /// back; Tony leaves the midtones where the lighting put them, and 1.30 on top
    /// of it crushed open shade toward black (the exact G3 failure the ambient
    /// ladder was raised to fix).
    pub const MIDTONE_CONTRAST: f32 = 1.12;
    /// Highlight contrast, matched to the midtones so the curve doesn't kink at
    /// the section boundary.
    pub const HIGHLIGHT_CONTRAST: f32 = 1.12;
    /// Highlight roll-off (gain < 1) — compresses ONLY the brightest surfaces
    /// (sky, sunlit wedges, window panes) back into band while leaving bounce-lit
    /// shade and midtones alone. Lighting can't do this, because "less light"
    /// moves shadows down too.
    ///
    /// 0.64 → 0.86: 0.64 was a *hard* shoulder compensating for ACES already
    /// blowing sunlit voxels out. Under Tony the highlights arrive far less
    /// clipped, and holding 0.64 turned the sky into grey-blue putty instead of
    /// the reference's deep saturated blue.
    pub const HIGHLIGHT_GAIN: f32 = 0.86;
}

/// The hour of the day, as ONE object.
///
/// WHY THESE TRAVEL TOGETHER. Sun angle, sun colour, sky clear colour, bounce
/// fill and exposure are not five independent knobs — they are five faces of one
/// decision ("it is golden hour" / "it is night"). Ship them separately and you
/// get frames that argue with themselves: a low warm sun over a noon-blue sky, or
/// a night scene at daylight exposure where the moon reads as an overcast
/// afternoon. Bundling them means a preset switch can never land half-applied.
///
/// WHY THE LOOK LANE OWNS THEM AT ALL. `main.rs` spawns a white 9000-lux sun 59°
/// up and clears to a pale 0.53/0.72/0.92 sky — a perfectly reasonable *neutral*
/// for the editor/bench lanes, and not a time of day. "Raking golden-hour sun" is
/// the brief, so the hour is a look decision and lands here. These are applied
/// ONLY while the look lane is live (see [`look_enabled`]), so bench, editor and
/// the hero shot still see exactly what `main.rs` set.
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
    /// Sky / `ClearColor`, sRGB.
    pub sky: [f32; 3],
    /// Bounce-fill (`AmbientLight`) colour, sRGB — the axis that moves the
    /// *midtone* numbers, since midtones are open shade and bounce, not the
    /// sunlit wedge.
    pub ambient: [f32; 3],
    /// Bounce-fill brightness, lux.
    pub ambient_lux: f32,
    /// Camera exposure. Bevy's default is `Exposure::BLENDER` (ev100 9.7), which
    /// the gameplay camera was silently running because nothing ever set one.
    pub ev100: f32,
    /// Distance-haze colour, sRGB.
    pub fog: [f32; 3],
}

impl Hour {
    /// The shipped default: late-afternoon raking sun, vivid sky.
    pub const GOLDEN: Self = Self {
        elev_deg: 17.0,
        azim_deg: 205.0,
        illuminance: 11_000.0,
        key: [1.00, 0.84, 0.62],
        sky: [0.36, 0.60, 0.90],
        ambient: [0.96, 0.84, 0.66],
        ambient_lux: 1100.0,
        ev100: 11.0,
        fog: FOG_COLOR_DAY,
    };

    /// `VOXELFORGE_LOOK_NIGHT=1`. A cool moon key kept just strong enough to hold
    /// a shadow direction, so lanterns and the campfire are the only warm sources
    /// in frame — which is the whole point of a night shot for a bloom that is
    /// supposed to fire on emissive surfaces only.
    pub const NIGHT: Self = Self {
        elev_deg: -8.0,
        azim_deg: 205.0,
        illuminance: 260.0,
        key: [0.55, 0.66, 0.95],
        sky: [0.03, 0.05, 0.12],
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
    if let Ok(ev) = std::env::var("VOXELFORGE_LOOK_EXPOSURE") {
        if let Ok(ev) = ev.trim().parse() {
            h.ev100 = ev;
        }
    }
    h
}

/// Haze distances, with a sweep hook. The SHIPPED values are the consts — this
/// exists so a distance can be tried against a real frame without a 20-minute
/// relink, not so the fog can be configured per install.
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
/// [`grade_knobs`]; unset returns the live [`Hour`]'s own key/ambient.
fn light_colors() -> (Color, Color) {
    let h = hour();
    let k = h.key;
    let a = h.ambient;
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
///
/// [`DepthOfField`] is still on this list even though NO tier inserts it any more
/// (see [`insert_stack`]) — it is strip-only. A `remove` of a component the entity
/// doesn't have costs nothing, and keeping it here means a stale DOF from any
/// source is cleared on the next tier apply rather than surviving invisibly on the
/// one camera the player looks through.
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
/// avatar to its campsite; a loaded map does not). It is deliberately an explicit
/// opt-in and not folded into `cfg.shot`: the bench and every other lane's
/// screenshot proof also carry `cfg.shot`, and they must keep rendering the frame
/// they were graded against.
fn look_enabled(cfg: Option<Res<crate::Cfg>>) -> bool {
    if std::env::var_os("VOXELFORGE_LOOK_DISABLE").is_some() {
        return false;
    }
    if std::env::var_os("VOXELFORGE_LOOK_FORCE").is_some() {
        return true;
    }
    cfg.is_some_and(|c| c.play)
}

/// Same predicate, answered from a plain `&Cfg` instead of from a system param.
///
/// `main.rs` needs it at camera-spawn time, where there is no `Res` to borrow.
/// Exported (rather than re-derived over there) so there is exactly one
/// definition of "is the look lane live" in the codebase.
pub fn enabled_for(cfg: &crate::Cfg) -> bool {
    if std::env::var_os("VOXELFORGE_LOOK_DISABLE").is_some() {
        return false;
    }
    std::env::var_os("VOXELFORGE_LOOK_FORCE").is_some() || cfg.play
}

/// The distance haze, built from the public contract constants.
///
/// One constructor for every tier: the haze is not a luxury effect that scales
/// with the GPU, it is the thing that hides the streaming radius (see
/// [`RENDER_RADIUS`]), so a Low-tier machine needs it exactly as much as an Ultra
/// one. `FogFalloff::Linear` rather than `Exponential`: exponential starts biting
/// at the camera (~27% opaque by 40 blocks at the old density 0.008), which
/// desaturated mid-distance geometry that the reference frames keep vivid.
fn distance_fog() -> DistanceFog {
    let (start, end) = fog_range();
    let c = hour().fog;
    let g = FOG_SUN_GLOW;
    DistanceFog {
        color: Color::srgb(c[0], c[1], c[2]),
        falloff: FogFalloff::Linear { start, end },
        // Looking INTO the low sun, the haze picks up its colour — the warm
        // atmospheric glow every shader pack sells golden hour with. It costs
        // nothing (it is a term in the fog shader, not a pass).
        directional_light_color: Color::srgb(g[0], g[1], g[2]),
        directional_light_exponent: FOG_SUN_EXPONENT,
    }
}

/// Build the look stack for `quality` onto the camera entity `e`.
///
/// This is the single tier→effect map — the place Poppy's per-effect ms budget
/// reshapes the ladder. Each tier is a strict superset of the one below it, so
/// the base layer (tonemap + grade + exposure + bloom + haze, the look's
/// identity) is inserted first and the `match` only adds the effects that tier
/// earns.
///
/// NOTHING IN HERE INSERTS `DepthOfField`, AT ANY TIER — deliberately, and
/// permanently. See [`LookStack`] and `docs/look-contract.md` §5: the old High/
/// Ultra arms carried `hero.rs`'s bokeh DOF with its focus plane pinned to the
/// camera boom (~6.5 blocks), which melted everything past the player's own
/// shoulder. The reference frames hold distant pines sharp to the last pixel;
/// near-focus DOF is the grammar of a product still, not of a camera someone
/// looks through while walking.
fn insert_stack(e: &mut EntityCommands, quality: LookQuality) {
    // Base layer — present at every tier, and the SAME function `main.rs` spawns
    // the camera with, so the two paths cannot drift into two different looks.
    e.insert(base_camera_look());
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
            // beyond what TAA gives the SSAO.
            e.insert((
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Low,
                    // Voxel-scale thickness: darkens the contact crease so things
                    // sit ON the ground instead of looking like they hover.
                    constant_object_thickness: 1.45,
                },
            ));
        }
        LookQuality::High => {
            // Default tier. Temporal soft shadows (no PCSS) + medium SSAO +
            // volumetric fog at a reduced step count. The one effect cut vs Ultra
            // is PCSS: per look-tier-spec.md §1 the hero-shot measurement found
            // Bevy clamps `soft_shadow_size` to its 0.5 floor in a room this size,
            // so PCSS buys almost nothing — cut it BEFORE the volumetric ray-march,
            // the visible atmosphere that pins the frame (§5 problem #3 swaps the
            // cut order so High no longer pays for PCSS while dropping the god-ray
            // layer wholesale).
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
            // The full stack: temporal PCSS + Ultra SSAO + volumetric god rays.
            e.insert((
                ShadowFilteringMethod::Temporal,
                TemporalAntiAliasing::default(),
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
                    constant_object_thickness: 1.45,
                },
                VolumetricFog {
                    ambient_intensity: 0.08,
                    step_count: 96,
                    // Dithers the ray-march step boundary so TAA resolves
                    // the shaft with a soft edge instead of a hard cut.
                    jitter: 0.6,
                    ..default()
                },
            ));
        }
    }
}

/// The tier-independent half of the stack, as a `Bundle` a camera can be spawned
/// with.
///
/// `main.rs` uses this at the camera-spawn site so the gameplay camera is dressed
/// from frame ZERO rather than from the first `Update` — which is what the
/// [`LookPlugin`] systems below do, and which meant the very first frames of a
/// session (and every screenshot taken by a lane that never enters `Update` with
/// the look on) rendered through a bare `Camera3d`.
///
/// It is deliberately the tier-INDEPENDENT half only: tiering, runtime F7 swaps
/// and the sun still belong to the plugin. Both paths call [`insert_stack`] /
/// this function out of the same constants, so there is no second copy of the
/// look to drift.
pub fn base_camera_look() -> impl Bundle {
    let (temperature, post_saturation, midtone_contrast, highlight_gain) = grade_knobs();
    let h = hour();
    (
        // MSAA off: voxel edges are 90° and axis-aligned (no jaggies to smooth),
        // and SSAO at every higher tier requires it off anyway.
        Msaa::Off,
        // TonyMcMapface, NOT AcesFitted. Bevy's own docs on `AcesFitted` read
        // "Bright greens and reds turn orange. Bright blues turn magenta." — and
        // the brightest blue in an outdoor frame is the sky, i.e. ACES reproduces
        // the exact magenta cast that was chased out of this lane on 2026-08-01.
        // Tony is "very neutral … color hues are preserved during compression",
        // which keeps sky blue, leaves green and sun amber instead of letting them
        // slide into each other. Bevy also names Tony specifically as the
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
        // implicit `Exposure::BLENDER` (ev100 9.7) while the sun's illuminance was
        // set by a different lane, so "how bright is the world" was an accident
        // nobody owned. It is part of the hour now.
        Exposure { ev100: h.ev100 },
        // Bloom that fires on EMISSIVE ONLY. `Bloom::NATURAL`'s prefilter
        // threshold is 0.0, i.e. every pixel in the frame blooms a little, which
        // is what puts a haze over sunlit wood and softens the voxel grain
        // (measured micro-contrast 3.8 vs the golden's 5.2). A threshold of 1.0
        // in HDR means "only things brighter than white" — lanterns, the
        // campfire, the sun's own disc, a window pane with the sun behind it —
        // and nothing else. The softness feathers the cut so the halo starts
        // gradually instead of switching on at a hard luminance line.
        //
        // `Bloom` carries `#[require(Hdr)]`, so this is also what puts the camera
        // into an HDR render target at all — without it there is no >1.0 signal
        // for a threshold to select on.
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
    mut clear: ResMut<ClearColor>,
    mut q: Query<(Entity, Option<&LookApplied>, Option<&mut AmbientLight>), With<Camera3d>>,
) {
    for (cam, applied, ambient) in &mut q {
        if applied.is_some_and(|a| a.0 == *quality) {
            continue;
        }
        let h = hour();
        // The sky. `main.rs` clears to a pale 0.53/0.72/0.92 — a neutral, not a
        // time of day; the reference frames sit under a deep saturated blue. It
        // is set here, with the sun and the fill, because a sky that disagrees
        // with the key light is the single most obvious way a frame reads fake.
        clear.0 = Color::srgb(h.sky[0], h.sky[1], h.sky[2]);
        // Tint AND power the camera's bounce fill. Colour is the midtone half of
        // the frame's warmth (the half that used to come out of the white-balance
        // matrix — that was the magenta bug); brightness has to move with it,
        // because `main.rs`'s 380 lux was chosen against a 59°-high 9000-lux sun
        // and open shade under a 17° sun is a much larger share of the frame.
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

/// Tier the two look knobs that live on the directional light, not the camera:
/// the PCSS penumbra width (a field on `DirectionalLight`) and `VolumetricLight`
/// (the component that lets the volumetric-fog pass treat this light as an
/// in-scatterer). Same tier-tracking idea as [`apply_look_to_cameras`].
///
/// This mutates the existing `DirectionalLight` rather than inserting a new one:
/// `shadow_maps_enabled` stays as `main.rs` set it, and the light entity keeps
/// whatever else its owner hung on it.
///
/// Angle and illuminance ARE this lane's now (they were `main.rs`'s: a white
/// 9000-lux key 59° up). "Raking golden-hour sun" is the brief itself, not a
/// tint on top of someone else's noon — and elevation, illuminance, hue, sky and
/// exposure only make a coherent frame when they move together, which is what
/// [`Hour`] exists to guarantee.
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
        // Point the sun. Only the rotation matters for a directional light, but
        // the translation is kept high and behind so anything that reasons about
        // the light's position (a lens-flare, a debug gizmo) still gets a sane
        // one.
        let dir = h.sun_dir();
        tf.translation = -dir * 200.0;
        tf.look_to(dir, Vec3::Y);
        dl.illuminance = h.illuminance;
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
