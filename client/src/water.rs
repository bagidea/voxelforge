//! Water surface material — the Rust half of `assets/shaders/water.wgsl`.
//!
//! ## Why an `ExtendedMaterial` and not more `StandardMaterial` tuning
//!
//! Every knob that could make the shipped water look alive is one Bevy's PBR
//! material does not have: a normal that moves, a reflection weighted by view
//! angle, and a colour that depends on how much water the eye is looking
//! *through*. `voxel::block_material` had already been pushed as far as it goes
//! (see its `WATER` arm) — the remaining gap is not a parameter, it is a
//! fragment shader. `ExtendedMaterial` keeps every existing `StandardMaterial`
//! property of the water tile (its texture, its alpha mode, its shadow rules)
//! and only replaces the fragment stage, so nothing the material lane tuned is
//! thrown away.
//!
//! ## How it reaches the water and nothing else
//!
//! `remesh_chunk_entity` marks the per-block child it spawns for `BlockId::WATER`
//! with [`WaterSurface`]. [`swap_water_material`] then trades that child's
//! `MeshMaterial3d<StandardMaterial>` for the extended one, once, the frame it
//! appears. Chunk streaming respawns those children constantly, so the swap has
//! to be a system and not a one-shot at startup.
//!
//! ## Env levers (same discipline as every `VOXELFORGE_LOOK_*` knob)
//!
//! * `VOXELFORGE_WATER=off` — never swap. The shipped `StandardMaterial` water
//!   renders instead: the A/B baseline for "did the shader do anything", out of
//!   ONE binary, which is the only kind of before/after this project accepts.
//! * `VOXELFORGE_WATER_WAVE=<amp>` — wave amplitude (default
//!   [`WAVE_AMPLITUDE`]). `0` gives a mirror-flat surface that still takes the
//!   Fresnel sky, which isolates term 2 from term 1.
//! * `VOXELFORGE_WATER_DEPTH=<metres>` — Beer's-law falloff distance (default
//!   [`DEPTH_SCALE`]).

use bevy::asset::{AssetId, Handle};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, MeshMaterial3d};
use bevy::prelude::*;
// `ShaderRef` moved out of `bevy_render` into its own `bevy_shader` crate (0.17+)
// and is re-exported as `bevy::shader`; `AsBindGroup` stayed in `bevy_render`.
// Importing either from the other path is a build error, so both are spelled out.
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Path of the fragment shader, relative to `assets/`.
const WATER_SHADER: &str = "shaders/water.wgsl";

/// Default wave amplitude — the strength of the normal perturbation, not a
/// vertex displacement (the mesh stays flat; only the lighting ripples).
///
/// Tuned to read as "a river under a low sun": high enough that the sun glint
/// breaks into a moving path down the water, low enough that the surface still
/// reflects the sky as a coherent sheet rather than sparkling noise. Chosen
/// deliberately on the low side — high-frequency normal noise is the exact
/// failure mode that scores well on a contrast metric and looks *worse*.
const WAVE_AMPLITUDE: f32 = 0.055;
/// Spatial frequency of the base swell, in radians per world block. ~0.9 puts
/// one full swell every ~7 blocks, which is a wave a voxel river can carry.
const WAVE_FREQUENCY: f32 = 0.9;
/// Travel speed of the base swell, radians/second. Slow: fast water reads as
/// boiling. The finer octaves run 1.4x and 2.1x this.
const WAVE_SPEED: f32 = 0.85;
/// Beer's-law falloff distance in metres: the water is ~63% of the way from the
/// shallow colour to the deep one after this much thickness.
const DEPTH_SCALE: f32 = 3.5;

/// Water's Fresnel F0. Real water is 0.02 at normal incidence — using the true
/// number is what makes the grazing ramp read as water instead of as varnish.
const FRESNEL_F0: f32 = 0.02;
/// Schlick exponent. 5.0 is the physical value; kept honest.
const FRESNEL_POWER: f32 = 5.0;
/// Blinn-Phong exponent for the sun glint. High = a tight, hot specular that
/// the wave normals then chop into a broken path.
const GLINT_SHARPNESS: f32 = 220.0;
/// Glint gain. Multiplies the sun tint; this is an HDR add, so it is allowed to
/// exceed 1.0 and blow out into bloom the way a real sun path does.
const GLINT_GAIN: f32 = 2.4;
/// Opacity at full depth. Not 1.0 — deep water that is perfectly opaque loses
/// the faint bed shape that sells it as a volume. Lives in `deep_color.a`.
const DEEP_OPACITY: f32 = 0.92;
/// Opacity at zero depth (the shoreline). Lives in `shallow_color.a`.
///
/// Clearer than the block lane's flat `alpha: 0.62` — a film of water over sand
/// SHOULD nearly disappear, and that contrast against [`DEEP_OPACITY`] is the
/// whole point of the term. Deliberately not near-zero: at 0.10 the shoreline
/// read as a hole in the world rather than as clear water.
const SHORE_OPACITY: f32 = 0.30;

/// Marks the per-chunk child mesh that carries `BlockId::WATER` faces, so the
/// swap system can find it without guessing from a material handle.
#[derive(Component)]
pub struct WaterSurface;

/// The uniform block. Mirrors `struct WaterMaterial` in `water.wgsl` field for
/// field, in order. Every field is a `Vec4`: std140 pads a `vec3` to 16 bytes
/// anyway, and a mixed scalar/vector layout is the classic way to get silent
/// garbage in a hand-written uniform.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct WaterExtension {
    /// rgb: colour of a 0 m film of water. a: opacity at zero depth.
    #[uniform(100)]
    pub shallow_color: Vec4,
    /// rgb: colour water converges on at `wave.w` metres. a: opacity at full
    /// depth.
    #[uniform(100)]
    pub deep_color: Vec4,
    /// rgb: sky colour at the horizon (reflection, low `r.y`).
    #[uniform(100)]
    pub horizon_color: Vec4,
    /// rgb: sky colour at the zenith (reflection, high `r.y`).
    #[uniform(100)]
    pub zenith_color: Vec4,
    /// rgb: sun tint for the glint. a: glint gain.
    #[uniform(100)]
    pub sun_color: Vec4,
    /// xyz: unit vector pointing AT the sun.
    #[uniform(100)]
    pub sun_dir: Vec4,
    /// x: amplitude, y: frequency, z: speed, w: depth scale (metres).
    #[uniform(100)]
    pub wave: Vec4,
    /// x: Fresnel F0, y: Fresnel power, z: glint sharpness, w: RESERVED (0).
    #[uniform(100)]
    pub optics: Vec4,
}

impl Default for WaterExtension {
    fn default() -> Self {
        Self {
            // A sunset-valley river: clear green over its sand bed at the
            // shore, deep teal-blue down the channel. Both are the *transmission*
            // tint multiplied onto the artist's `water.png`, not a replacement
            // for it — Monanisa's tile still sets the base.
            shallow_color: Vec4::new(0.62, 0.92, 0.85, SHORE_OPACITY),
            deep_color: Vec4::new(0.05, 0.20, 0.34, DEEP_OPACITY),
            // Overwritten every frame by `drive_water_from_sun` from the live
            // sky palette; these are only what the first frame renders with.
            horizon_color: Vec4::new(1.00, 0.72, 0.45, 1.0),
            zenith_color: Vec4::new(0.30, 0.48, 0.80, 1.0),
            sun_color: Vec4::new(1.0, 0.85, 0.65, GLINT_GAIN),
            sun_dir: Vec4::new(0.0, 1.0, 0.0, 0.0),
            wave: Vec4::new(wave_amplitude(), WAVE_FREQUENCY, WAVE_SPEED, depth_scale()),
            optics: Vec4::new(FRESNEL_F0, FRESNEL_POWER, GLINT_SHARPNESS, 0.0),
        }
    }
}

impl MaterialExtension for WaterExtension {
    fn fragment_shader() -> ShaderRef {
        WATER_SHADER.into()
    }
}

/// The concrete material type: everything `StandardMaterial` already gave the
/// water tile, with our fragment stage on top.
pub type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterExtension>;

/// `VOXELFORGE_WATER=off` disables the swap. Cached — env cannot change after
/// launch, and this is read once per water child spawned.
fn water_enabled() -> bool {
    static E: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *E.get_or_init(|| !matches!(std::env::var("VOXELFORGE_WATER").ok().as_deref(), Some("off")))
}

/// `VOXELFORGE_WATER_WAVE=<amp>`; falls back to [`WAVE_AMPLITUDE`]. A negative
/// value is rejected rather than clamped — it would invert every wave normal.
fn wave_amplitude() -> f32 {
    static A: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *A.get_or_init(|| {
        std::env::var("VOXELFORGE_WATER_WAVE")
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|a| *a >= 0.0)
            .unwrap_or(WAVE_AMPLITUDE)
    })
}

/// `VOXELFORGE_WATER_DEPTH=<metres>`; falls back to [`DEPTH_SCALE`]. Must be
/// positive: the shader divides by it.
fn depth_scale() -> f32 {
    static D: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *D.get_or_init(|| {
        std::env::var("VOXELFORGE_WATER_DEPTH")
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|d| *d > 0.0)
            .unwrap_or(DEPTH_SCALE)
    })
}

/// Extended water materials, keyed by the `StandardMaterial` they were cloned
/// from — NOT one global handle.
///
/// `voxel::material_index` gives every block `N_FACES = 3` material slots, so
/// `BlockId::WATER` owns three distinct `StandardMaterial`s (top / side /
/// bottom). Caching a single extended material off whichever water child the
/// query happened to reach first would push that face's texture onto all three,
/// and chunk children spawn in no guaranteed order, so WHICH face won would vary
/// run to run. It is invisible today only because `water.png` is one tile for
/// all three faces — it would become a visible, non-deterministic bug the moment
/// the art or a map splits them, which is exactly the kind of latent swap this
/// lane has been bitten by before. Keying by source asset id keeps each face's
/// own `StandardMaterial` the single source of truth, as documented.
///
/// Bounded by the number of water face slots (3), not by the world size: every
/// chunk's water child of a given face shares one source handle and therefore
/// one extended material.
#[derive(Resource, Default)]
pub struct WaterMats(pub std::collections::HashMap<AssetId<StandardMaterial>, Handle<WaterMaterial>>);

/// Trade the water child's `StandardMaterial` for the extended one, the frame it
/// spawns. Runs every frame because chunk streaming respawns these children.
fn swap_water_material(
    mut commands: Commands,
    mut mats: ResMut<Assets<WaterMaterial>>,
    mut cache: ResMut<WaterMats>,
    std_mats: Res<Assets<StandardMaterial>>,
    // `Without<MeshMaterial3d<WaterMaterial>>` is what makes this idempotent:
    // once swapped, the child no longer matches.
    pending: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (With<WaterSurface>, Without<MeshMaterial3d<WaterMaterial>>),
    >,
) {
    if !water_enabled() {
        return;
    }
    for (e, base) in &pending {
        let src = base.0.id();
        let handle = match cache.0.get(&src) {
            Some(h) => h.clone(),
            None => {
                // Clone the block lane's tuned water `StandardMaterial` rather
                // than authoring a second one, so its texture / alpha mode /
                // roughness stay the single source of truth. If it has not
                // loaded yet, skip this frame and retry — the query still
                // matches next tick.
                let Some(base_mat) = std_mats.get(&base.0) else {
                    continue;
                };
                let h = mats.add(WaterMaterial {
                    base: base_mat.clone(),
                    extension: WaterExtension::default(),
                });
                cache.0.insert(src, h.clone());
                println!(
                    "WATER extended material built ({} face slot(s) so far) wave={:.3} \
                     depth={:.1}m f0={FRESNEL_F0} glint={GLINT_GAIN} shore_a={SHORE_OPACITY} \
                     deep_a={DEEP_OPACITY}",
                    cache.0.len(),
                    wave_amplitude(),
                    depth_scale()
                );
                h
            }
        };
        commands
            .entity(e)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}

/// Keep the reflection and the glint pointed at the ACTUAL sun.
///
/// A Fresnel reflection whose sky colour is a constant is a lie the moment the
/// hour changes, and a glint that does not track the light is worse than none.
/// Both are read off the live `DirectionalLight` each frame — one uniform write
/// on one material, so the cost does not scale with the world.
fn drive_water_from_sun(
    cache: Res<WaterMats>,
    mut mats: ResMut<Assets<WaterMaterial>>,
    sun: Query<(&GlobalTransform, &DirectionalLight)>,
    ambient: Query<&AmbientLight>,
) {
    if cache.0.is_empty() {
        return;
    }
    let Some((tf, light)) = sun.iter().next() else {
        return;
    };
    // A `DirectionalLight` shines along its own -Z, so the vector pointing AT
    // the sun is +Z of its transform. Same convention `look.rs` uses.
    let to_sun = tf.back().as_vec3().normalize_or_zero();
    let sun_dir = Vec4::new(to_sun.x, to_sun.y, to_sun.z, 0.0);

    let c = light.color.to_linear();
    let sun_color = Vec4::new(c.red, c.green, c.blue, GLINT_GAIN);

    // Sky stops. The horizon takes the sun's own tint (that is what a low sun
    // does to the band it sits in) and the zenith takes the ambient, which is
    // the closest thing this renderer has to a sky irradiance term. Scaled to
    // sane reflectance levels — these are multiplied by Fresnel, not added.
    let warm = 0.35 + 0.65 * (1.0 - to_sun.y.clamp(0.0, 1.0));
    let horizon = Vec4::new(c.red * warm, c.green * warm * 0.82, c.blue * warm * 0.62, 1.0);
    let zenith = ambient.iter().next().map(|a| {
        let ac = a.color.to_linear();
        let g = (a.brightness / 200.0).clamp(0.05, 1.5);
        Vec4::new(ac.red * g, ac.green * g, ac.blue * g * 1.25, 1.0)
    });

    // Every face slot, not just one: they are separate assets and a side quad
    // lit by a stale sun vector would glint in the wrong direction from the top
    // quad it touches. Three writes at most (see [`WaterMats`]).
    for handle in cache.0.values() {
        let Some(mat) = mats.get_mut(handle) else {
            continue;
        };
        mat.extension.sun_dir = sun_dir;
        mat.extension.sun_color = sun_color;
        mat.extension.horizon_color = horizon;
        if let Some(z) = zenith {
            mat.extension.zenith_color = z;
        }
    }
}

/// Registers the material and both systems. Cheap to add unconditionally: with
/// `VOXELFORGE_WATER=off` the swap returns immediately and no `WaterMaterial`
/// asset is ever created, so the pipeline is never specialised.
pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .init_resource::<WaterMats>()
            .add_systems(Update, (swap_water_material, drive_water_from_sun).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shader source, pulled in at COMPILE time. `include_str!` and not a
    /// runtime read: the path is then checked by the compiler, the test cannot
    /// pass by silently failing to find the file, and it works from any cwd.
    const WGSL: &str = include_str!("../../assets/shaders/water.wgsl");

    /// Field names of `struct WaterMaterial` in `water.wgsl`, in declaration
    /// order, with their declared types.
    ///
    /// A real parse of the real file — the earlier version of this test built an
    /// array literal from the Rust struct and asserted its length was 8, which
    /// is true no matter what either side says and never opened the shader at
    /// all. WGSL is not compiled anywhere in CI, so if this test does not read
    /// it, nothing does.
    fn wgsl_uniform_fields() -> Vec<(String, String)> {
        let body = WGSL
            .split_once("struct WaterMaterial {")
            .expect("water.wgsl must declare `struct WaterMaterial {`")
            .1
            .split_once('}')
            .expect("unterminated struct WaterMaterial")
            .0;
        body.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .map(|l| {
                let (name, ty) = l
                    .trim_end_matches(',')
                    .split_once(':')
                    .unwrap_or_else(|| panic!("unparsable field line in water.wgsl: {l:?}"));
                (name.trim().to_string(), ty.trim().to_string())
            })
            .collect()
    }

    /// THE ABI GATE. `WaterExtension`'s reflected field list (declaration order,
    /// which is the order the `AsBindGroup` derive packs a combined `#[uniform]`
    /// block in) must match `struct WaterMaterial` in the shader name for name.
    ///
    /// Reorder either side and this goes red — reflection supplies the Rust
    /// order at runtime and the parser above supplies the WGSL order from the
    /// file, so neither is a literal anyone can forget to update. That is the
    /// failure this file's doc comments claim to be protected from, and until
    /// now nothing enforced it.
    #[test]
    fn the_wgsl_uniform_mirrors_the_rust_struct() {
        use bevy::reflect::Struct;

        let w = WaterExtension::default();
        let rust: Vec<&str> = (0..w.field_len()).map(|i| w.name_at(i).unwrap()).collect();
        let wgsl = wgsl_uniform_fields();
        let wgsl_names: Vec<&str> = wgsl.iter().map(|(n, _)| n.as_str()).collect();

        assert_eq!(
            rust, wgsl_names,
            "WaterExtension and water.wgsl's `struct WaterMaterial` disagree on field \
             names or ORDER — every knob past the first mismatch reads a neighbour's value"
        );
        for (name, ty) in &wgsl {
            assert_eq!(
                ty, "vec4<f32>",
                "field `{name}` is `{ty}`; the Rust mirror is Vec4 for every field, and a \
                 mixed scalar/vector layout is the classic std140 corruption bug"
            );
        }
    }

    /// A negative control for the parser above: prove it can actually SEE a
    /// mismatch, rather than returning something that trivially compares equal.
    /// Without this, a parser that silently produced an empty list would make
    /// the ABI gate green forever.
    #[test]
    fn the_abi_gate_can_fail() {
        use bevy::reflect::Struct;

        let w = WaterExtension::default();
        let rust: Vec<&str> = (0..w.field_len()).map(|i| w.name_at(i).unwrap()).collect();
        let wgsl = wgsl_uniform_fields();

        assert!(!wgsl.is_empty(), "the parser found no fields — it is not reading the shader");
        assert_eq!(wgsl.len(), rust.len());

        // Plant the exact bug the gate exists to catch — two adjacent fields
        // swapped — and require the comparison to reject it.
        let mut planted: Vec<&str> = wgsl.iter().map(|(n, _)| n.as_str()).collect();
        planted.swap(0, 1);
        assert_ne!(
            rust, planted,
            "a swapped pair still compared equal — the gate is not comparing order"
        );
    }

    /// `optics.w` was the depth opacity until the no-prepass fallback was fixed
    /// and both opacity stops moved into the colour alphas. It is now a reserved
    /// hole. Pin it at 0 so a later edit has to change this test on purpose
    /// instead of quietly giving one slot two meanings.
    #[test]
    fn the_reserved_optics_slot_is_zero() {
        assert_eq!(
            WaterExtension::default().optics.w,
            0.0,
            "optics.w is RESERVED; the opacity stops live in shallow_color.a / deep_color.a"
        );
    }

    /// The no-prepass path must be a TRUE no-op.
    ///
    /// The bug this pins: with `DEPTH_PREPASS` undefined the shader used to run
    /// the grade anyway at `thickness = 0`, which forced alpha to the shore stop
    /// and tinted everything the shallow colour — the water nearly vanished
    /// against the block lane's shipped `alpha: 0.62`. Reachable in practice on
    /// Low tier with `VOXELFORGE_LOOK_SSAO=off`, which pulls the last
    /// `DepthPrepass` off the camera. The fix is structural (the whole term sits
    /// inside `#ifdef DEPTH_PREPASS`), so this checks the structure.
    #[test]
    fn without_a_depth_prepass_the_base_color_is_untouched() {
        let guard = WGSL
            .split_once("#ifdef DEPTH_PREPASS")
            .expect("the depth term must be guarded")
            .1
            .split_once("#endif")
            .expect("unterminated #ifdef DEPTH_PREPASS")
            .0;
        assert!(
            guard.contains("pbr_input.material.base_color = vec4<f32>("),
            "the base_color write escaped the DEPTH_PREPASS guard — with no prepass it \
             would overwrite the block lane's tuned water colour and alpha"
        );
        // ...and nothing outside the guard may write it before alpha_discard.
        let before_guard = WGSL.split_once("#ifdef DEPTH_PREPASS").unwrap().0;
        assert!(
            !before_guard.contains("pbr_input.material.base_color ="),
            "an unguarded base_color write exists ahead of the depth term"
        );
    }

    /// The physical constants are the reason the Fresnel ramp reads as water.
    /// A future "let's make it shinier" edit that pushes F0 toward a metal
    /// should have to delete this test on purpose.
    #[test]
    fn fresnel_stays_physical() {
        assert!(
            (FRESNEL_F0 - 0.02).abs() < 1e-6,
            "water's normal-incidence reflectance is 0.02; anything higher is varnish"
        );
        assert!((FRESNEL_POWER - 5.0).abs() < 1e-6, "Schlick's exponent is 5");
    }

    /// Amplitude is the noise-vs-look knob. Guard the default against the
    /// failure mode this project has already been bitten by: high-frequency
    /// detail that lifts a contrast score without improving the picture.
    #[test]
    fn the_default_wave_is_a_swell_not_noise() {
        assert!(
            WAVE_AMPLITUDE <= 0.12,
            "amplitude {WAVE_AMPLITUDE} is into sparkle-noise territory"
        );
        assert!(WAVE_AMPLITUDE > 0.0, "a zero default would ship a flat mirror");
    }
}
