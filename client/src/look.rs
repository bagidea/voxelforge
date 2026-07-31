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
//! It runs in `Update` against `Without<LookApplied>` rather than in `Startup`
//! for a boring reason that has already cost this repo a session: system order
//! against another lane's `Startup` is not a contract. The camera is spawned in
//! main.rs's setup, and a `Startup` system here would race it — sometimes the
//! query is empty and the whole look silently no-ops. Filtered on a marker, the
//! archetype is empty on every frame after the first, so the steady-state cost
//! is a no-op query, and a camera respawned mid-session (map reload) gets the
//! stack too instead of coming back bare.

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{ShadowFilteringMethod, VolumetricFog, VolumetricLight};
use bevy::pbr::{
    DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};

/// Post-grade constants, carried over from the signed-off hero shot.
///
/// These are the numbers the reviewer approved on the golden beauty shot, not
/// fresh guesses — see `docs/golden-beauty-shot.md` and the grade-vs-golden
/// rounds recorded in `hero.rs`. They are named here so a future tuning pass has
/// one place to edit and so a diff against `hero.rs` stays readable.
mod grade {
    /// Chromaticity push toward red. The voxel materials tonemap slightly cool
    /// through AcesFitted; this is what pulled measured midtone-B down to the
    /// golden's range.
    pub const TEMPERATURE: f32 = 0.10;
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
}

/// Marks a camera that has already been given the post stack.
#[derive(Component)]
pub struct LookApplied;

/// Marks a directional light that has already been given soft shadows + volumetrics.
#[derive(Component)]
pub struct LookLightApplied;

/// Wears the look lane's post stack over whatever camera and sun the game spawned.
pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (apply_look_to_cameras, apply_look_to_sun, focus_dof).run_if(look_enabled),
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
fn look_enabled(cfg: Option<Res<crate::Cfg>>) -> bool {
    cfg.is_some_and(|c| c.play)
}

/// Give every 3D camera that doesn't have it yet the full post stack.
fn apply_look_to_cameras(
    mut commands: Commands,
    q: Query<Entity, (With<Camera3d>, Without<LookApplied>)>,
) {
    for cam in &q {
        commands.entity(cam).insert((
            // SSAO requires MSAA off. Voxel edges stay crisp regardless — they're
            // 90° and axis-aligned, and TAA carries the sub-pixel work.
            Msaa::Off,
            Tonemapping::AcesFitted,
            // Applied AFTER the tonemap. SHADOWS are deliberately held neutral:
            // putting contrast on the shadow section crushed open shade to pure
            // black on the hero shot (interior p05-L 13.7% -> 3.3%, a measured
            // regression). Shadows keep their bounce fill instead of clipping.
            ColorGrading {
                global: ColorGradingGlobal {
                    temperature: grade::TEMPERATURE,
                    post_saturation: grade::POST_SATURATION,
                    ..default()
                },
                shadows: ColorGradingSection {
                    contrast: 1.0,
                    ..default()
                },
                midtones: ColorGradingSection {
                    contrast: grade::MIDTONE_CONTRAST,
                    ..default()
                },
                highlights: ColorGradingSection {
                    contrast: grade::HIGHLIGHT_CONTRAST,
                    gain: grade::HIGHLIGHT_GAIN,
                    ..default()
                },
            },
            // PCSS penumbras and Ultra SSAO are both stochastic — one frame of
            // either is visibly noisy. Temporal filtering + TAA accumulate them
            // into clean soft shadows and contact AO. These two belong together;
            // enabling PCSS without a temporal filter just ships the noise.
            ShadowFilteringMethod::Temporal,
            TemporalAntiAliasing::default(),
            Bloom {
                // Trimmed from NATURAL's default: bloom was bleeding highlight
                // energy across voxel edges and softening the grain (micro 3.8 vs
                // the golden's 5.2). NATURAL's high threshold still keeps it off
                // the shadows, so the shadow gates don't move.
                intensity: 0.26,
                ..Bloom::NATURAL
            },
            DepthOfField {
                mode: DepthOfFieldMode::Bokeh,
                // Focus starts at the boom length so the avatar is sharp on frame
                // one; `focus_dof` then tracks the live boom (see there).
                focal_distance: crate::BOOM_DIST,
                // Softer than the hero still's cinematic stop. This is a camera
                // someone plays behind, not a frame they look at — the intent is
                // depth separation on distant terrain, not a melted background.
                aperture_f_stops: 4.0,
                // Bevy's circle-of-confusion scales as focal_length²/(sensor·N).
                // With the physical ~18.6mm default sensor the CoC is near-zero at
                // this world scale and even f/0.1 does nothing visible. A
                // large-format sensor is what makes the effect exist at all.
                sensor_height: 0.35,
                ..default()
            },
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
                // Voxel-scale thickness: the default is too faint to read against
                // a warm bounce fill, which left objects looking like they hover.
                // This darkens the contact crease so things sit ON the ground.
                constant_object_thickness: 1.45,
            },
            // Grouped in a nested tuple on purpose: the insert would otherwise
            // pass 15 top-level items and Bevy's Bundle tuple impls stop there.
            // A nested tuple is itself a Bundle, so this costs nothing.
            (
                VolumetricFog {
                    ambient_intensity: 0.08,
                    step_count: 96,
                    // Dithers the ray-march step boundary so TAA resolves the shaft
                    // with a soft edge instead of a hard triangular cut.
                    jitter: 0.6,
                    ..default()
                },
                DistanceFog {
                    // Desaturated and thin. A denser orange haze flattened distant
                    // geometry into the foreground's hue; this keeps material tint
                    // readable with depth.
                    color: Color::srgb(0.50, 0.42, 0.28),
                    falloff: FogFalloff::Exponential { density: 0.008 },
                    ..default()
                },
            ),
            LookApplied,
        ));
    }
}

/// Give the sun PCSS soft shadows and make it visible to the volumetric pass.
///
/// This mutates the existing `DirectionalLight` rather than inserting a new one:
/// `main.rs` set `illuminance` and `shadow_maps_enabled`, and those are gameplay
/// lighting decisions that aren't this lane's to overwrite. Only the two fields
/// the look brief names are touched.
fn apply_look_to_sun(
    mut commands: Commands,
    mut q: Query<(Entity, &mut DirectionalLight), Without<LookLightApplied>>,
) {
    for (light, mut dl) in &mut q {
        // `soft_shadow_size` only exists when Bevy's PCSS flag is compiled in. It's
        // in this crate's default features, but a `--no-default-features` build is
        // a real configuration and shouldn't fail to compile over a look knob.
        #[cfg(feature = "experimental_pbr_pcss")]
        {
            // Hard ~1px PCF reads as a CG cutout at voxel scale. 3.0 is the width
            // the hero shot's penumbra gate was signed off at.
            dl.soft_shadow_size = Some(3.0);
        }
        #[cfg(not(feature = "experimental_pbr_pcss"))]
        {
            let _ = &mut dl;
        }
        commands.entity(light).insert((
            // Without this the light contributes no in-scattering, so `VolumetricFog`
            // on the camera renders nothing at all — god rays need a light that opts
            // in, not just fog.
            VolumetricLight,
            LookLightApplied,
        ));
    }
}

/// Keep the depth-of-field focus locked on the avatar as the boom moves.
///
/// A fixed `focal_distance` is fine for a still. It is wrong for a third-person
/// camera whose boom is pulled in by `camera_boom` whenever a wall would clip
/// between camera and avatar — the focus plane would stay out at 6.5 while the
/// camera sat at 2, and the player character, the one thing that must never be
/// soft, would go blurry exactly when the camera hugged a wall.
fn focus_dof(mut q: Query<(&mut DepthOfField, &crate::OrbitCam)>) {
    for (mut dof, orbit) in &mut q {
        dof.focal_distance = orbit.dist;
    }
}
