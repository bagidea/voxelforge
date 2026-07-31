//! PERF PROBE (Poppy's lane) — what the look post stack actually costs per frame.
//!
//! The post stack (`look.rs`) just landed on the PLAYABLE game. It is the most
//! expensive thing this project has ever drawn, and until now nobody had a number
//! for it. This bin produces that number in **milliseconds**, per effect.
//!
//! WHY A SEPARATE BIN. `look.rs` is Rose's file and `main.rs` is Kevin's
//! (docs/LANES.md), so a probe that edits either is a cross-lane change. This bin
//! `#[path]`-includes `look.rs` **unmodified** — the exact stack the game wears,
//! not a copy that can drift — and supplies the three crate-root items look.rs
//! reaches for (`Cfg.play`, `BOOM_DIST`, `OrbitCam.dist`) as local shims. Same
//! trick `shot_main.rs` uses to render `hero.rs` without touching `main.rs`.
//!
//! WHY IT MEASURES WITH VSYNC OFF. `docs/perf-vsync-cliff-rootcause.md` already
//! settled this: under VSync every reading pins to the 16.7 ms refresh wall and
//! all headroom — and all cost — is invisible. A VSync'd A/B of this stack would
//! read "16.7 vs 16.7 ms, free!". So the probe forces `AutoNoVsync` and reports
//! the true GPU+CPU frame time.
//!
//! WHY LEAVE-ONE-OUT, NOT ADD-ONE-IN. The effects share passes (SSAO and
//! VolumetricFog both read the depth prepass; TAA is what makes PCSS and SSAO Ultra
//! usable at all). Measuring each alone on a bare camera would charge the shared
//! setup to whichever one ran first. `full` minus `full-without-X` is the number
//! that answers the only question a quality tier cares about: *what do I get back
//! if I turn X off, given everything else is on?*
//!
//! WHICH TIER THE LEAVE-ONE-OUT RUNS AT. `look.rs` defaults to `High`, and High
//! has no `VolumetricFog` — so a `no_vfog` run at the default tier would strip
//! nothing and report "volumetrics are free". Ultra is the only tier carrying all
//! six effects, so the per-effect phase pins `VOXELFORGE_LOOK_QUALITY=ultra`
//! (look.rs's own env override, no edit needed). The tier ladder is measured
//! separately — see `scripts/perf_look_probe.sh`.
//!
//! Usage: `VOXELFORGE_PERF=<mode> voxelforge_perf`
//!   off      bare camera, LookPlugin inert  (the baseline)
//!   full     the whole stack as look.rs ships it
//!   no_ssao | no_vfog | no_taa | no_dof | no_bloom | no_pcss
//!            full stack minus that one effect
//! Optional: `VOXELFORGE_PERF_SENSOR=<f32>` overrides `DepthOfField::sensor_height`
//! at runtime (probe-only; look.rs's constant is untouched).
//!
//! WHAT THE NUMBERS ARE NOT. The scene below is a synthetic 64×64 heightfield, not
//! a shipping Voxelforge map. It is built to make the A/B fair — identical geometry,
//! identical camera, every run — so the *deltas* between modes are trustworthy. The
//! absolute ms is the frame price of THIS scene on THIS machine, not of the real
//! world at the real draw-call count. Quote the deltas; don't quote the absolutes as
//! the game's frame budget.

#[path = "look.rs"]
mod look;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::light::ShadowFilteringMethod;
use bevy::light::VolumetricFog;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::PresentMode;

// ---- shims: the three crate-root items `look.rs` reaches for ---------------
// look.rs takes `Option<Res<crate::Cfg>>` and only ever reads `.play`, so a
// one-field stand-in is enough and keeps the include honest.

#[derive(Resource)]
pub(crate) struct Cfg {
    pub(crate) play: bool,
}

/// Same boom length `main.rs` spawns the third-person camera at, so the DoF focus
/// plane in the probe sits where it sits in the game.
pub(crate) const BOOM_DIST: f32 = 6.5;

#[derive(Component)]
pub(crate) struct OrbitCam {
    pub(crate) dist: f32,
}

// ---- probe config ---------------------------------------------------------

/// Which effect (if any) is subtracted from the full stack this run.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Off,
    Full,
    NoSsao,
    NoVfog,
    NoTaa,
    NoDof,
    NoBloom,
    NoPcss,
}

impl Mode {
    fn from_env() -> Self {
        match std::env::var("VOXELFORGE_PERF").as_deref() {
            Ok("off") => Mode::Off,
            Ok("no_ssao") => Mode::NoSsao,
            Ok("no_vfog") => Mode::NoVfog,
            Ok("no_taa") => Mode::NoTaa,
            Ok("no_dof") => Mode::NoDof,
            Ok("no_bloom") => Mode::NoBloom,
            Ok("no_pcss") => Mode::NoPcss,
            _ => Mode::Full,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Full => "full",
            Mode::NoSsao => "no_ssao",
            Mode::NoVfog => "no_vfog",
            Mode::NoTaa => "no_taa",
            Mode::NoDof => "no_dof",
            Mode::NoBloom => "no_bloom",
            Mode::NoPcss => "no_pcss",
        }
    }
}

/// Frames thrown away before sampling starts.
///
/// Not a "let it settle" hand-wave: the first frames pay for pipeline compilation,
/// shadow-atlas allocation and the TAA history filling up. Sampling those charges
/// one-time startup to the steady-state cost of whichever effect is on trial.
const WARMUP_FRAMES: u32 = 240;
/// Frames actually measured (~5 s at 50 fps, ~1.2 s at 200 fps). Enough that the
/// median is stable across repeat runs.
const SAMPLE_FRAMES: u32 = 600;

#[derive(Resource)]
struct Probe {
    frame: u32,
    samples: Vec<f32>,
}

fn main() {
    let mode = Mode::from_env();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: format!("Voxelforge — perf probe [{}]", mode.label()),
            // Same back buffer the game ships at. Every effect in this stack is
            // screen-space, so the resolution IS the workload — measuring at any
            // other size would produce a number Rose can't spend.
            resolution: (1280u32, 720u32).into(),
            // See the module header: VSync would hide the entire result.
            present_mode: PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    // Matches main.rs/shot_main.rs: PCSS needs the 4K atlas or the penumbra
    // stair-steps — and a 2K atlas would also under-report what PCSS costs.
    .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
    .insert_resource(ClearColor(Color::srgb(0.53, 0.72, 0.92)))
    // `GlobalAmbientLight`, not `AmbientLight` — in 0.19 the latter is a per-camera
    // component override, and the scene-wide default is this resource.
    .insert_resource(bevy::light::GlobalAmbientLight {
        brightness: 320.0,
        ..default()
    })
    // `play: false` is how the OFF baseline is produced: look.rs's own
    // `look_enabled` run condition goes false and the whole plugin is inert.
    // That is a truer "LookPlugin off" than stripping components after the fact.
    .insert_resource(Cfg {
        play: mode != Mode::Off,
    })
    .insert_resource(mode)
    .insert_resource(Probe {
        frame: 0,
        samples: Vec::with_capacity(SAMPLE_FRAMES as usize),
    })
    .add_plugins(look::LookPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, (strip_effect, override_sensor_height, sample));

    app.run();
}

/// A fixed scene — same geometry, same camera, every run.
///
/// A heightfield of unit cubes off one shared mesh + material: Bevy batches them,
/// so the draw-call count stays sane while the depth complexity and the shadow
/// caster count stay in the range the real voxel world produces. The camera is a
/// hard-coded transform (no controller, no physics) because an A/B that lets the
/// view drift is not an A/B.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.36, 0.24),
        perceptual_roughness: 0.95,
        ..default()
    });
    let stone_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Deterministic heightfield — an integer hash, not `rand`, so the scene is
    // byte-identical on every run and on every machine.
    const SIDE: i32 = 64;
    let h = |x: i32, z: i32| -> i32 {
        let n = (x.wrapping_mul(374_761_393) ^ z.wrapping_mul(668_265_263)) as u32;
        let n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        (((n >> 16) % 7) as i32) + ((x + z).rem_euclid(5))
    };
    for z in -SIDE / 2..SIDE / 2 {
        for x in -SIDE / 2..SIDE / 2 {
            let top = h(x, z);
            // Two shells of voxels: the surface plus the one below it. Deeper
            // columns are invisible in the real greedy-meshed world too.
            for y in (top - 1).max(0)..=top {
                let mat = if y == top { &ground_mat } else { &stone_mat };
                commands.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5),
                ));
            }
        }
    }

    // A few tall pillars: real occluders, so SSAO and the volumetric shafts have
    // something to bite on instead of grading a flat plain.
    for (px, pz) in [(-10, -6), (6, -12), (12, 8), (-14, 10), (0, 14)] {
        for y in 0..12 {
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(stone_mat.clone()),
                Transform::from_xyz(px as f32 + 0.5, y as f32 + 8.5, pz as f32 + 0.5),
            ));
        }
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(60.0, 120.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Third-person eye height and boom distance, aimed low across the field so the
    // sun rakes toward the camera — the framing volumetrics are most expensive in.
    commands.spawn((
        Camera3d::default(),
        // `Msaa::Off` HERE, not only inside look.rs. `bevy_render` declares
        // `register_required_components::<Camera, Msaa>()` and `Msaa`'s default is
        // `Sample4`, so a camera that nobody configures rasters 4× and pays a
        // resolve. look.rs inserts `Msaa::Off` — but only when the plugin is live,
        // which means the `off` baseline would have been the only mode paying for
        // MSAA and every "full − off" delta would have had MSAA folded into it.
        // Pinning it at spawn makes MSAA a constant across all modes.
        Msaa::Off,
        Transform::from_xyz(0.0, 14.0, 22.0).looking_at(Vec3::new(0.0, 6.0, -6.0), Vec3::Y),
        OrbitCam { dist: BOOM_DIST },
    ));
}

/// Stamped on the entities the `no_pcss` mode has already de-PCSS'd.
///
/// The other modes self-terminate: they query for the component they remove, so the
/// archetype stops matching after one pass. `no_pcss` *inserts* instead of removing,
/// so without this marker it would re-insert `ShadowFilteringMethod` and re-`&mut`
/// the `DirectionalLight` — flagging it Changed — on all 840 measured frames, which
/// is churn billed to the exact mode under test.
#[derive(Component)]
struct PcssStripped;

/// Subtract exactly one effect from the applied stack.
///
/// Every query is filtered on the component it is about to remove, so once the
/// removal has happened the archetype no longer matches and this costs a handful
/// of empty queries per frame — it cannot pollute the measurement it enables.
fn strip_effect(
    mut commands: Commands,
    mode: Res<Mode>,
    ssao: Query<Entity, (With<look::LookApplied>, With<ScreenSpaceAmbientOcclusion>)>,
    vfog: Query<Entity, (With<look::LookApplied>, With<VolumetricFog>)>,
    taa: Query<
        Entity,
        (
            With<look::LookApplied>,
            With<bevy::anti_alias::taa::TemporalAntiAliasing>,
        ),
    >,
    dof: Query<Entity, (With<look::LookApplied>, With<DepthOfField>)>,
    bloom: Query<Entity, (With<look::LookApplied>, With<Bloom>)>,
    cams: Query<Entity, (With<look::LookApplied>, Without<PcssStripped>)>,
    mut suns: Query<
        (Entity, &mut DirectionalLight),
        (With<look::LookLightApplied>, Without<PcssStripped>),
    >,
) {
    match *mode {
        Mode::NoSsao => {
            for e in &ssao {
                // `NormalPrepass` goes too. Bevy attaches it via
                // `#[require(DepthPrepass, NormalPrepass)]` on the SSAO component,
                // and required components are NOT removed with their requirer — so
                // leaving it would keep re-rendering the whole scene into a normal
                // buffer nothing reads, and under-report what SSAO really costs.
                // `DepthPrepass` stays: TAA and DoF need it regardless.
                commands
                    .entity(e)
                    .remove::<ScreenSpaceAmbientOcclusion>()
                    .remove::<bevy::core_pipeline::prepass::NormalPrepass>();
            }
        }
        Mode::NoVfog => {
            // Only the camera-side ray march goes. `DistanceFog` (a cheap per-pixel
            // blend) and `VolumetricLight` on the sun stay, so this reads as the
            // cost of the march alone.
            for e in &vfog {
                commands.entity(e).remove::<VolumetricFog>();
            }
        }
        Mode::NoTaa => {
            for e in &taa {
                commands
                    .entity(e)
                    .remove::<bevy::anti_alias::taa::TemporalAntiAliasing>();
                // MSAA deliberately stays OFF. Swapping MSAA back in would look
                // like the fair "ship-able without TAA" config, but bevy_pbr's
                // `extract_ssao_settings` hard-returns on `Msaa != Off`
                // (ssao/mod.rs:488) — SSAO would silently switch off too and this
                // run would bill TAA for SSAO's savings. One variable at a time.
            }
        }
        Mode::NoDof => {
            for e in &dof {
                commands.entity(e).remove::<DepthOfField>();
            }
        }
        Mode::NoBloom => {
            for e in &bloom {
                commands.entity(e).remove::<Bloom>();
            }
        }
        Mode::NoPcss => {
            // PCSS is two costs on two entities: the wide soft_shadow_size search on
            // the light, and the temporal shadow filter on the camera. Drop both —
            // half of it would be a number nobody can act on.
            for (light, mut dl) in &mut suns {
                #[cfg(feature = "experimental_pbr_pcss")]
                {
                    dl.soft_shadow_size = None;
                }
                #[cfg(not(feature = "experimental_pbr_pcss"))]
                {
                    let _ = &mut dl;
                }
                commands.entity(light).insert(PcssStripped);
            }
            for e in &cams {
                commands
                    .entity(e)
                    .insert((ShadowFilteringMethod::Hardware2x2, PcssStripped));
            }
        }
        Mode::Off | Mode::Full => {}
    }
}

/// `VOXELFORGE_PERF_SENSOR=<f32>` — sweep the DoF sensor height without editing
/// look.rs. Mutating a live component value is not a change to Rose's file; it is
/// how the probe answers "does this constant cost anything?" with a measurement
/// instead of an opinion.
fn override_sensor_height(mut q: Query<&mut DepthOfField>) {
    let Ok(h) = std::env::var("VOXELFORGE_PERF_SENSOR") else {
        return;
    };
    let Ok(h) = h.parse::<f32>() else {
        return;
    };
    for mut dof in &mut q {
        if dof.sensor_height != h {
            dof.sensor_height = h;
        }
    }
}

/// Collect raw frame times, then print the distribution and quit.
///
/// `Time<Real>` on purpose — `Time` is the virtual clock and can be paused,
/// scaled or fixed-stepped by another plugin, which would make this measure the
/// clock rather than the machine.
fn sample(
    time: Res<Time<Real>>,
    mode: Res<Mode>,
    // Printed with every result: the same mode measured at Low and at Ultra are
    // different numbers, and a log line that doesn't say which tier it came from
    // is a number nobody can safely put in a table.
    tier: Res<look::LookQuality>,
    mut p: ResMut<Probe>,
    mut exit: MessageWriter<AppExit>,
) {
    p.frame += 1;
    if p.frame <= WARMUP_FRAMES {
        return;
    }
    p.samples.push(time.delta_secs() * 1000.0);
    if p.samples.len() < SAMPLE_FRAMES as usize {
        return;
    }

    let mut s = p.samples.clone();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = s[s.len() / 2];
    let p95 = s[(s.len() as f32 * 0.95) as usize];
    let mean = s.iter().sum::<f32>() / s.len() as f32;
    println!(
        "PERF mode={} tier={:?} median_ms={median:.3} mean_ms={mean:.3} p95_ms={p95:.3} fps={:.1} frames={}",
        mode.label(),
        *tier,
        1000.0 / median,
        s.len()
    );
    exit.write(AppExit::Success);
}
