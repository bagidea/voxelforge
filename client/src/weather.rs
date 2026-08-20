//! Weather & atmosphere (Rose's lane, 2026-08-20).
//!
//! Four CEO-ref features, each driven by an env lever so ONE binary shoots the
//! before/after (same recipe as every other `VOXELFORGE_*` A/B in this repo):
//!
//!   1. WET GROUND — while it rains, the OUTDOOR ground palette's top-face
//!      materials drop roughness, gain reflectance and darken (see
//!      [`wettable`]), so the shipped IBL + lamp specular reads as a wet
//!      surface. voxel.rs's per-(block,face) material table is what makes a
//!      top-only wet layer expressible at all: `Top` is slot `id*3 + 0`, and
//!      the split mesher wears exactly that handle on up-facing quads.
//!   2. RAIN — streak particles + expanding ground ripples, placed and killed
//!      against the real chunk store (`crate::solid_at`), so rain only exists
//!      where the sky is actually visible — never through a roof.
//!   3. GOD RAYS — densifies (or creates) the volumetric medium, opts the sun
//!      into `VolumetricLight` and the camera into `VolumetricFog` if the live
//!      quality tier left them off. The ray-march itself is Bevy's; this lane
//!      composes state at runtime and never edits look.rs.
//!   4. MORNING FOG — two ground-hugging `FogVolume`s (a low dense one and a
//!      higher thin one) follow the camera, plus a one-shot densify of the
//!      camera's `DistanceFog` (previous value printed, never restored
//!      in-process: levers are launch-time).
//!
//! # Composition with the look lane (look.rs is NEVER edited by this file)
//!
//!   * FOG: the shipped `DistanceFog` falloff is Flamingo's fit
//!     (`Linear{HAZE_START, HAZE_FULL}`, refitted 2026-08-20 to 20/240 with
//!     `HAZE_COOL` 0.72 in the main tree). Rain thickens air by SCALING
//!     whatever falloff is live at runtime (x1.5; the morning preset x2.0) —
//!     family and ratios preserved, his constants never written, and the
//!     previous value is printed so a plate can be explained after the fact.
//!   * BLOOM: the camera's `Bloom` belongs to the look stack. Wet sheen
//!     reaches it the honest way — `reflectance` up / roughness down makes
//!     real HDR speculars that the existing bloom then picks up. This lane
//!     inserts no post effects.
//!
//! DEFAULT IS OFF. No env, no weather — every other lane's plates are
//! untouched, and gameplay keeps its approved look until a lever asks.
//!
//! # Levers (read once at launch, cached — env cannot change mid-run)
//!
//!   VOXELFORGE_WEATHER=off|clear|rain|storm|morning|evening|all   (default off)
//!   VOXELFORGE_WEATHER_RAIN=0|1|off|on        force the rain sub-system
//!   VOXELFORGE_WEATHER_WET=0..1               pin wetness (skips the ramp)
//!   VOXELFORGE_WEATHER_GODRAYS=0|1|off|on     force the god-ray state
//!   VOXELFORGE_WEATHER_FOG=0|1|off|on         force the morning-fog state
//!   VOXELFORGE_WEATHER_RAIN_COUNT=<n>         streak budget (default 900)
//!   VOXELFORGE_WEATHER_WIND=x,z               wind vector, m/s (default 1.2,0)
//!                                             — streaks drift AND slant to it
//!
//! The master `off`/`clear` (or unset, or an unknown word) wins over every
//! sub-lever: weather never appears by accident. Sub-levers override the
//! preset's per-feature choice once a preset is active.
//!
//! # B0001 discipline
//!
//! Every system that mutates `Transform` on weather entities reads the camera
//! through a query filtered `Without<that marker>` (the same disjointness
//! proof look.rs's `play_fog_volume` uses), so no two queries in one system
//! can overlap the same `Transform`.

use bevy::asset::RenderAssetUsages;
use bevy::color::LinearRgba;
use bevy::light::{FogVolume, NotShadowCaster, VolumetricFog, VolumetricLight};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::World as VoxelWorld;

/// Streak spawn box: this many metres around the camera (x/z).
const RAIN_RADIUS: f32 = 20.0;
/// Streaks fall from this band above the camera.
const RAIN_SPAWN_ABOVE: (f32, f32) = (7.0, 15.0);
/// A streak this far below the camera is re-rolled wherever it is.
const RAIN_DESPAWN_BELOW: f32 = 6.0;
/// How far up a column we look for a roof before calling a spot "sky visible".
const SKY_SCAN_UP: i32 = 45;
/// Ripple budget: simultaneous live ripples, and spawn attempts per second.
const RIPPLE_MAX: usize = 130;
const RIPPLE_RATE_PER_S: f32 = 90.0;
/// Wetness approach time-constant. Fast enough that the 3.2 s screenshot
/// window catches it fully settled (e^-7 ≈ 0.1%).
const WET_TAU: f32 = 0.45;
/// God-ray medium density when this lane composes it.
const GODRAY_DENSITY: f32 = 0.055;
/// Rain thickens the air: the live `DistanceFog` falloff (the look lane's fit)
/// is scaled by this. A SCALE, never a rewrite — family and ratios preserved.
const RAIN_FOG_K: f32 = 1.5;
/// How much wetness moves a top-face material, tuned against the CEO refs
/// (`1787225660697_1.jpg` / `_2.jpg`: dark asphalt-slick cobbles, mirror
/// lamp streaks).
const WET_ROUGH_DROP: f32 = 0.62;
const WET_REFLECT_GAIN: f32 = 0.30;
const WET_DARKEN: f32 = 0.30;

// ---------------------------------------------------------------------------
// Levers
// ---------------------------------------------------------------------------

/// "0|1|off|on|true|false|yes|no" -> bool. Anything else (or unset) -> None.
fn flag_env(key: &str) -> Option<bool> {
    let v = std::env::var(key).ok()?;
    match v.trim().to_ascii_lowercase().as_str() {
        "0" | "off" | "false" | "no" => Some(false),
        "1" | "on" | "true" | "yes" => Some(true),
        _ => None,
    }
}

/// The launch-time weather configuration. Cheap clone/copy: it is a handful of
/// flags read once.
#[derive(Clone, Copy, Debug)]
pub struct WeatherConfig {
    /// Any preset other than off/clear/unset/unknown is active. Master `off`
    /// wins over every sub-lever.
    pub active: bool,
    pub rain: bool,
    /// The preset asked for rain at all (its "weather state"), independent of
    /// whether the streaks are switched off by `_RAIN=off` — a rainy day is
    /// still wet when the drops are hidden for a capture.
    pub wetty: bool,
    /// Explicit wetness override (`_WET=0..1`), else rain implies 1.0.
    pub wet_pin: Option<f32>,
    pub godrays: bool,
    pub fog: bool,
    pub streaks: usize,
    /// Horizontal wind, m/s. Streaks drift with it and slant to match their
    /// fall+wind velocity, so the field reads as weather, not a sprite sheet.
    pub wind: Vec2,
}

impl WeatherConfig {
    fn from_env() -> Self {
        let preset = std::env::var("VOXELFORGE_WEATHER")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let (p_rain, p_god, p_fog, streaks) = match preset.as_str() {
            "rain" => (true, false, false, 900),
            "storm" => (true, false, true, 1300),
            "morning" | "fog" => (false, false, true, 900),
            "evening" | "godrays" => (false, true, false, 900),
            "all" => (true, true, true, 1300),
            // Unset, "off", "clear" or an unknown word: nothing. Weather must
            // never appear because someone typo'd the preset.
            _ => (false, false, false, 900),
        };
        let active = !matches!(preset.as_str(), "" | "off" | "clear");
        let streaks = std::env::var("VOXELFORGE_WEATHER_RAIN_COUNT")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(streaks)
            .min(4000);
        let wind = std::env::var("VOXELFORGE_WEATHER_WIND")
            .ok()
            .and_then(|v| {
                let parts: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                (parts.len() == 2).then_some(parts)
            })
            .map(|p| Vec2::new(p[0], p[1]))
            .unwrap_or(Vec2::new(1.2, 0.0));
        let cfg = WeatherConfig {
            active,
            rain: flag_env("VOXELFORGE_WEATHER_RAIN").unwrap_or(p_rain),
            wetty: p_rain,
            wet_pin: std::env::var("VOXELFORGE_WEATHER_WET")
                .ok()
                .and_then(|v| v.trim().parse::<f32>().ok())
                .map(|w| w.clamp(0.0, 1.0)),
            godrays: flag_env("VOXELFORGE_WEATHER_GODRAYS").unwrap_or(p_god),
            fog: flag_env("VOXELFORGE_WEATHER_FOG").unwrap_or(p_fog),
            streaks,
            wind,
        };
        println!(
            "WEATHER preset='{preset}' active={} rain={} wet={} godrays={} fog={} streaks={} wind=({:.1},{:.1})",
            cfg.active,
            cfg.rain,
            cfg.wet_pin
                .map(|w| format!("pinned {w:.2}"))
                .unwrap_or_else(|| if cfg.rain { "1.0 (follows rain)".into() } else { "0.0".into() }),
            cfg.godrays,
            cfg.fog,
            cfg.streaks,
            cfg.wind.x,
            cfg.wind.y,
        );
        cfg
    }

    fn wet_target(&self) -> f32 {
        self.wet_pin
            .unwrap_or(if self.wetty || self.rain { 1.0 } else { 0.0 })
    }
}

// ---------------------------------------------------------------------------
// State & markers
// ---------------------------------------------------------------------------

/// One rain streak. `kill_y` is the first solid surface below the spawn point
/// (the splash floor); a streak that is not `alive` is parked and re-rolls
/// after `retry` seconds — that is how "under a roof" columns stop raining.
#[derive(Component)]
struct RainStreak {
    kill_y: f32,
    vy: f32,
    alive: bool,
    retry: f32,
}

/// One expanding ground ripple.
#[derive(Component)]
struct Ripple {
    age: f32,
    life: f32,
}

/// Marks this lane's fog volumes (god-ray medium and the morning layers).
#[derive(Component)]
struct WeatherFogVolume;

#[derive(Resource)]
struct WeatherState {
    cfg: WeatherConfig,
    /// Current wetness 0..1 (ramps toward the target).
    wetness: f32,
    /// Last value actually pushed into the materials.
    wet_applied: f32,
    /// Snapshot of every top-face material's (base_color, roughness,
    /// reflectance) before wetting — wet values are always derived from this,
    /// so the transform is idempotent however many times it re-applies.
    base_top: Vec<(Color, f32, f32)>,
    rng: fastrand::Rng,
    /// Shared rain/ripple assets, filled when the camera first exists.
    streak_mesh: Option<Handle<Mesh>>,
    streak_mat: Option<Handle<StandardMaterial>>,
    ripple_mesh: Option<Handle<Mesh>>,
    ripple_mats: Vec<Handle<StandardMaterial>>,
    ripple_acc: f32,
}

impl WeatherState {
    fn new(cfg: WeatherConfig) -> Self {
        Self {
            cfg,
            wetness: 0.0,
            wet_applied: -1.0,
            base_top: Vec::new(),
            // Fixed seed: a weather A/B re-shoot must place the same drops.
            rng: fastrand::Rng::with_seed(0x5EED_F00D),
            streak_mesh: None,
            streak_mat: None,
            ripple_mesh: None,
            ripple_mats: Vec::new(),
            ripple_acc: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Self-wiring; inert unless `VOXELFORGE_WEATHER` selects a preset.
pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        let cfg = WeatherConfig::from_env();
        app.insert_resource(WeatherState::new(cfg));
        if !cfg.active {
            return;
        }
        app.add_systems(
            Update,
            (
                tick_wetness,
                spawn_rain,
                update_rain,
                update_ripples,
                apply_godrays,
                apply_morning_fog,
                follow_weather_fog,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// 1. Wet ground
// ---------------------------------------------------------------------------

/// Which block TYPES take the wet treatment. True per-block sky masks would
/// need a remesh pass with per-block material handles (the table is shared per
/// (block, face)); the approximation is to wet the OUTDOOR ground/road palette
/// and leave the crafted/interior types (wood, leaves, snow, obsidian, glass,
/// lamp) plus water (own material lane) dry. A stone floor under a roof still
/// sheens — known limit, stated in the lane report; the rain itself (streaks,
/// ripples, splash floors) IS exactly sky-gated per column via `solid_at`.
const fn wettable(id: usize) -> bool {
    matches!(id,
        1 | 2 | 3 | 4      // grass, dirt, stone, sand
        | 8 | 9 | 10 | 11  // red sand, clay, gravel, cobblestone
        | 13 | 14 | 15     // brick, moss, limestone
    )
}

/// Ramp wetness toward the target and, on any real change, push it into every
/// block's TOP-face material. The split mesher wears one material per
/// (block, face); `material_index(id, Face::Top)` is `id*3 + 0`, so the top
/// handles are exactly the first of every 3-slot chunk of the table — sides
/// and bottoms stay dry, which is what a puddle-forming surface does.
fn tick_wetness(
    time: Res<Time>,
    voxels: Option<Res<VoxelWorld>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut st: ResMut<WeatherState>,
) {
    // The (block, face) material table rides the voxel `World` resource.
    let Some(world) = voxels.as_deref() else {
        return;
    };
    let target = st.cfg.wet_target();
    let k = 1.0 - (-time.delta_secs() / WET_TAU).exp();
    st.wetness += (target - st.wetness) * k;
    if (st.wetness - st.wet_applied).abs() < 0.004 {
        return;
    }
    st.wet_applied = st.wetness;
    let w = st.wetness;
    if st.base_top.is_empty() {
        st.base_top = world
            .block_materials
            .chunks_exact(3)
            .filter_map(|c| mats.get(&c[0]))
            .map(|m| (m.base_color, m.perceptual_roughness, m.reflectance))
            .collect();
    }
    let mut changed = 0usize;
    for (i, chunk) in world.block_materials.chunks_exact(3).enumerate() {
        if !wettable(i) {
            continue;
        }
        let Some((bc, rough, refl)) = st.base_top.get(i) else {
            continue;
        };
        let Some(mut m) = mats.get_mut(&chunk[0]) else {
            continue;
        };
        // Darken in linear space (same discipline as look.rs's `scale_lin` —
        // `Color` has no Mul<f32> in this bevy).
        let lin = LinearRgba::from(*bc);
        let f = 1.0 - WET_DARKEN * w;
        m.base_color = Color::from(LinearRgba::rgb(lin.red * f, lin.green * f, lin.blue * f));
        m.perceptual_roughness = (rough * (1.0 - WET_ROUGH_DROP * w)).max(0.04);
        m.reflectance = (refl + WET_REFLECT_GAIN * w).min(0.95);
        changed += 1;
    }
    if changed > 0 {
        println!("WEATHER wetness {w:.2} applied to {changed} top-face materials");
    }
}

// ---------------------------------------------------------------------------
// 2. Rain — streaks, sky occlusion, ripples
// ---------------------------------------------------------------------------

/// Where does the sky start? Scans up from `y` for the first solid cell.
/// Returns `Some(roof_surface_y)` if something covers this column within
/// [`SKY_SCAN_UP`] (then nothing may rain below it), else the highest solid
/// surface BELOW the spawn (the splash floor) or f32::MIN for open sky.
fn column_facts(world: Option<&VoxelWorld>, x: f32, y: f32, z: f32) -> (bool, f32) {
    let Some(world) = world else {
        return (false, f32::MIN);
    };
    let (cx, cz) = (x.floor() as i32, z.floor() as i32);
    let y0 = y.floor() as i32;
    for dy in 1..=SKY_SCAN_UP {
        if crate::solid_at(world, cx, y0 + dy, cz) {
            // A roof sits above the spawn point: this column is covered.
            return (true, f32::MIN);
        }
    }
    // Open sky. Find the splash floor below (first solid going down).
    let mut kill = f32::MIN;
    for dy in 0..=SKY_SCAN_UP {
        let yy = y0 - dy;
        if yy < 0 {
            break;
        }
        if crate::solid_at(world, cx, yy, cz) {
            kill = yy as f32 + 1.0;
            break;
        }
    }
    (false, kill)
}

/// Roll one streak spawn around the camera. Tries several spots so a few
/// covered columns don't empty the sky; if everything is covered (camera
/// indoors) the streak parks and retries later.
fn roll_streak(st: &mut WeatherState, world: Option<&VoxelWorld>, c: Vec3) -> (Vec3, Quat, f32, f32, bool) {
    for _ in 0..10 {
        let x = c.x + (st.rng.f32() * 2.0 - 1.0) * RAIN_RADIUS;
        let z = c.z + (st.rng.f32() * 2.0 - 1.0) * RAIN_RADIUS;
        let y = c.y + RAIN_SPAWN_ABOVE.0 + st.rng.f32() * (RAIN_SPAWN_ABOVE.1 - RAIN_SPAWN_ABOVE.0);
        let (covered, kill) = column_facts(world, x, y, z);
        if !covered {
            let vy = 19.0 + st.rng.f32() * 5.0;
            // Slant the quad along its own fall+wind velocity so the streak
            // reads as moving air, not a static sprite. Zero wind -> identity.
            let vel = Vec3::new(st.cfg.wind.x, -vy, st.cfg.wind.y);
            let rot = Quat::from_rotation_arc(Vec3::NEG_Y, vel.normalize_or_zero());
            return (Vec3::new(x, y, z), rot, kill, vy, true);
        }
    }
    (Vec3::new(0.0, -1000.0, 0.0), Quat::IDENTITY, 0.0, 20.0, false)
}

/// Spawn the streak field once the camera (and the scene behind it) exists.
fn spawn_rain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    world: Option<Res<VoxelWorld>>,
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<RainStreak>)>,
    mut st: ResMut<WeatherState>,
    mut done: Local<bool>,
) {
    if *done || !st.cfg.rain {
        return;
    }
    let Ok(cam_tf) = cam.single() else {
        return;
    };
    *done = true;
    st.streak_mesh = Some(meshes.add(streak_mesh()));
    st.streak_mat = Some(mats.add(StandardMaterial {
        unlit: true,
        base_color: Color::srgba(0.72, 0.80, 0.92, 0.30),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }));
    st.ripple_mesh = Some(meshes.add(ring_mesh()));
    st.ripple_mats = [0.50f32, 0.34, 0.20]
        .iter()
        .map(|a| {
            mats.add(StandardMaterial {
                unlit: true,
                base_color: Color::srgba(0.85, 0.90, 0.98, *a),
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let c = cam_tf.translation;
    let mut alive = 0usize;
    for _ in 0..st.cfg.streaks {
        let (p, rot, kill, vy, ok) = roll_streak(&mut *st, world.as_deref(), c);
        if ok {
            alive += 1;
        }
        let vis = if ok { Visibility::Visible } else { Visibility::Hidden };
        let tf = Transform::from_translation(p).with_rotation(rot);
        let gt = GlobalTransform::from(tf);
        commands.spawn((
            Mesh3d(st.streak_mesh.clone().unwrap()),
            MeshMaterial3d(st.streak_mat.clone().unwrap()),
            tf,
            gt,
            vis,
            NotShadowCaster,
            RainStreak {
                kill_y: kill,
                vy,
                alive: ok,
                retry: 0.5,
            },
        ));
    }
    println!("WEATHER rain: {} streaks spawned ({} visible under open sky)", st.cfg.streaks, alive);
}

/// Fall, splash, re-roll. Parked streaks retry on their cooldown — that is
/// how a camera that walks under a roof re-rains when it comes back out.
fn update_rain(
    time: Res<Time>,
    world: Option<Res<VoxelWorld>>,
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<RainStreak>)>,
    mut q: Query<(&mut Transform, &mut Visibility, &mut RainStreak)>,
    mut st: ResMut<WeatherState>,
) {
    let Ok(cam_tf) = cam.single() else {
        return;
    };
    let dt = time.delta_secs();
    let c = cam_tf.translation;
    for (mut tf, mut vis, mut s) in &mut q {
        if !s.alive {
            s.retry -= dt;
            if s.retry > 0.0 {
                continue;
            }
            let (p, rot, kill, vy, ok) = roll_streak(&mut *st, world.as_deref(), c);
            if ok {
                s.alive = true;
                s.kill_y = kill;
                s.vy = vy;
                tf.translation = p;
                tf.rotation = rot;
                *vis = Visibility::Visible;
            } else {
                s.retry = 0.8;
            }
            continue;
        }
        tf.translation.y -= s.vy * dt;
        tf.translation.x += st.cfg.wind.x * dt;
        tf.translation.z += st.cfg.wind.y * dt;
        let dx = tf.translation.x - c.x;
        let dz = tf.translation.z - c.z;
        let dead = tf.translation.y <= s.kill_y
            || tf.translation.y < c.y - RAIN_DESPAWN_BELOW
            || dx * dx + dz * dz > (RAIN_RADIUS * 1.3).powi(2);
        if dead {
            let (p, rot, kill, vy, ok) = roll_streak(&mut *st, world.as_deref(), c);
            if ok {
                s.kill_y = kill;
                s.vy = vy;
                tf.translation = p;
                tf.rotation = rot;
            } else {
                s.alive = false;
                s.retry = 0.8;
                tf.translation = Vec3::new(0.0, -1000.0, 0.0);
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// Age ripples out and spawn new ones on sky-exposed ground near the camera.
fn update_ripples(
    time: Res<Time>,
    world: Option<Res<VoxelWorld>>,
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<Ripple>)>,
    mut ripples: Query<(Entity, &mut Ripple, &mut Transform, &mut MeshMaterial3d<StandardMaterial>)>,
    mut commands: Commands,
    mut st: ResMut<WeatherState>,
) {
    if st.ripple_mesh.is_none() {
        return; // spawn_rain hasn't run (or rain is off): nothing to ripple
    }
    let dt = time.delta_secs();
    let mut alive = 0usize;
    for (e, mut r, mut tf, mut mat) in &mut ripples {
        r.age += dt;
        if r.age >= r.life {
            commands.entity(e).despawn();
            continue;
        }
        alive += 1;
        let t = r.age / r.life;
        // ease-out growth: a ripple expands fast then settles
        let s = 0.35 + 1.25 * (1.0 - (1.0 - t) * (1.0 - t));
        tf.scale = Vec3::new(s, 1.0, s);
        let third = ((t * 3.0) as usize).min(2);
        if mat.0 != st.ripple_mats[third] {
            mat.0 = st.ripple_mats[third].clone();
        }
    }
    let Ok(cam_tf) = cam.single() else {
        return;
    };
    let Some(vox) = world.as_deref() else {
        return; // no chunk store yet — nothing to ripple on
    };
    let c = cam_tf.translation;
    st.ripple_acc += dt * RIPPLE_RATE_PER_S;
    while st.ripple_acc >= 1.0 && alive < RIPPLE_MAX {
        st.ripple_acc -= 1.0;
        let x = c.x + (st.rng.f32() * 2.0 - 1.0) * 13.0;
        let z = c.z + (st.rng.f32() * 2.0 - 1.0) * 13.0;
        // Ground surface under this column: first solid from just above the
        // camera's height band, walking down, with open sky above it.
        let y0 = (c.y + 8.0).floor() as i32;
        let y_min = (c.y - 4.0).floor() as i32;
        let mut surface = None;
        for yy in (y_min..=y0).rev() {
            if yy < 0 {
                break;
            }
            if crate::solid_at(vox, x.floor() as i32, yy, z.floor() as i32) {
                surface = Some(yy as f32 + 1.0);
                break;
            }
        }
        let Some(sy) = surface else {
            continue;
        };
        let (covered, _) = column_facts(world.as_deref(), x, sy, z);
        if covered {
            continue;
        }
        let p = Vec3::new(x, sy + 0.02, z);
        let tf = Transform::from_translation(p);
        let gt = GlobalTransform::from(tf);
        commands.spawn((
            Mesh3d(st.ripple_mesh.clone().unwrap()),
            MeshMaterial3d(st.ripple_mats[0].clone()),
            tf,
            gt,
            Visibility::Visible,
            NotShadowCaster,
            Ripple {
                age: 0.0,
                life: 0.55 + st.rng.f32() * 0.25,
            },
        ));
        alive += 1;
    }
}

// ---------------------------------------------------------------------------
// 3. God rays
// ---------------------------------------------------------------------------

/// Compose the god-ray state once the sun and camera exist, then re-verify
/// once more at ~1.5 s so a look-stack rebuild that ran after the first pass
/// cannot leave the tier's own opinion in place of this lane's ask.
fn apply_godrays(
    time: Res<Time>,
    mut commands: Commands,
    sun: Query<(Entity, Option<&VolumetricLight>), With<DirectionalLight>>,
    cam: Query<(Entity, Option<&VolumetricFog>, &Transform), (With<crate::OrbitCam>, Without<WeatherFogVolume>)>,
    mut volumes: Query<&mut FogVolume>,
    st: Res<WeatherState>,
    mut pass: Local<u8>,
) {
    if !st.cfg.godrays {
        return;
    }
    match *pass {
        0 => {
            if sun.is_empty() || cam.is_empty() {
                return; // world not up yet
            }
            *pass = 1;
        }
        1 => {
            if time.elapsed_secs() < 1.5 {
                return;
            }
            *pass = 2;
        }
        _ => return,
    }
    for (e, vl) in &sun {
        if vl.is_none() {
            commands.entity(e).insert(VolumetricLight);
        }
    }
    let mut cam_pos = Vec3::ZERO;
    let mut cam_has_medium = false;
    for (e, vf, tf) in &cam {
        cam_pos = tf.translation;
        if vf.is_none() {
            commands
                .entity(e)
                .insert(VolumetricFog {
                    step_count: 48,
                    jitter: 0.6,
                    ambient_intensity: 0.08,
                    ..default()
                });
        } else {
            cam_has_medium = true;
        }
    }
    let mut densified = 0usize;
    for mut v in &mut volumes {
        if v.density_factor < GODRAY_DENSITY {
            v.density_factor = GODRAY_DENSITY;
            densified += 1;
        }
        if v.scattering < 0.55 {
            v.scattering = 0.55;
        }
    }
    if densified == 0 && !cam_has_medium {
        // No medium at all (e.g. VOXELFORGE_LOOK_VFOG=off): provide one.
        commands.spawn((
            WeatherFogVolume,
            FogVolume {
                fog_color: Color::srgb(1.0, 0.90, 0.72),
                density_factor: GODRAY_DENSITY,
                scattering: 0.60,
                ..default()
            },
            Transform::from_xyz(cam_pos.x, 40.0, cam_pos.z).with_scale(Vec3::new(220.0, 80.0, 220.0)),
            GlobalTransform::default(),
            Visibility::default(),
        ));
        println!("WEATHER godrays: weather medium spawned (density {GODRAY_DENSITY})");
    } else {
        println!("WEATHER godrays: pass — sun volumetric, camera medium present, {densified} volume(s) densified to {GODRAY_DENSITY}");
    }
}

// ---------------------------------------------------------------------------
// 4. Morning fog
// ---------------------------------------------------------------------------

/// Densify `DistanceFog` — the falloff's own parameterisation decides what
/// "twice as thick" means.
fn scale_falloff(f: FogFalloff, k: f32) -> FogFalloff {
    match f {
        FogFalloff::Linear { start, end } => FogFalloff::Linear {
            start: start / k,
            end: end / k,
        },
        FogFalloff::Exponential { density } => FogFalloff::Exponential {
            density: density * k,
        },
        FogFalloff::ExponentialSquared { density } => FogFalloff::ExponentialSquared {
            density: density * k,
        },
        // The look lane's aerial-perspective fit — its coefficients are a
        // measured pair, not independent knobs, so rain does NOT touch them.
        f @ FogFalloff::Atmospheric { .. } => f,
    }
}

/// Fog state, one shot: the morning preset gets its two ground-hugging
/// volumes, and ANY rainy preset thickens the live `DistanceFog`. The look
/// lane owns that falloff (`Linear{HAZE_START, HAZE_FULL}` + `HAZE_COOL` in
/// look.rs, refitted 2026-08-20) — this scales it in-place, family preserved,
/// and prints the pre-scale value so any plate can be explained afterwards.
fn apply_morning_fog(
    mut commands: Commands,
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<WeatherFogVolume>)>,
    mut dfog: Query<&mut DistanceFog, With<crate::OrbitCam>>,
    st: Res<WeatherState>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let rainy = st.cfg.wetty || st.cfg.rain;
    if !st.cfg.fog && !rainy {
        return;
    }
    let Ok(cam_tf) = cam.single() else {
        return;
    };
    *done = true;
    let c = cam_tf.translation;
    if st.cfg.fog {
        let (dense, thin) = (
            (2.2, Vec3::new(260.0, 4.4, 260.0), 0.045, "low dense"),
            (6.5, Vec3::new(300.0, 6.0, 300.0), 0.022, "high thin"),
        );
        for (y, scale, density, label) in [dense, thin] {
            commands.spawn((
                WeatherFogVolume,
                FogVolume {
                    fog_color: Color::srgb(0.74, 0.79, 0.88),
                    density_factor: density,
                    scattering: 0.68,
                    ..default()
                },
                Transform::from_xyz(c.x, y, c.z).with_scale(scale),
                GlobalTransform::default(),
                Visibility::default(),
            ));
            println!("WEATHER morning fog: {label} layer at y={y} density={density}");
        }
    }
    // Rainy air is thicker air; the morning preset stacks on top of that.
    let k = if st.cfg.fog { 2.0 } else { RAIN_FOG_K };
    for mut f in &mut dfog {
        let old = f.falloff.clone();
        f.falloff = scale_falloff(old.clone(), k);
        println!("WEATHER fog: live DistanceFog scaled x{k} (was {old:?}) — look-lane fit preserved in family");
    }
}

/// Keep this lane's volumes centred on the camera (x/z only — the layers keep
/// their absolute heights, that is the point of "low-lying").
fn follow_weather_fog(
    cam: Query<&Transform, (With<crate::OrbitCam>, Without<WeatherFogVolume>)>,
    mut fog: Query<&mut Transform, With<WeatherFogVolume>>,
) {
    let Ok(c) = cam.single() else {
        return;
    };
    for mut tf in &mut fog {
        tf.translation.x = c.translation.x;
        tf.translation.z = c.translation.z;
    }
}

// ---------------------------------------------------------------------------
// Meshes
// ---------------------------------------------------------------------------

/// A thin vertical quad hanging from the origin (the streak's head).
fn streak_mesh() -> Mesh {
    let (w, h) = (0.03, 0.62);
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-w / 2.0, 0.0, 0.0],
            [w / 2.0, 0.0, 0.0],
            [w / 2.0, -h, 0.0],
            [-w / 2.0, -h, 0.0],
        ],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0, 1.0]; 4],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

/// A flat ring (annulus) in the XZ plane — the ripple. Unlit + double-sided,
/// so winding order does not matter; scaled per-frame by the ripple system.
fn ring_mesh() -> Mesh {
    const SEG: usize = 28;
    let (r_in, r_out) = (0.30, 0.46);
    let mut positions = Vec::with_capacity((SEG + 1) * 2);
    let mut normals = Vec::with_capacity((SEG + 1) * 2);
    let mut uvs = Vec::with_capacity((SEG + 1) * 2);
    let mut indices = Vec::with_capacity(SEG * 6);
    for i in 0..=SEG {
        let a = i as f32 / SEG as f32 * std::f32::consts::TAU;
        let (s, c) = a.sin_cos();
        positions.push([c * r_in, 0.0, s * r_in]);
        positions.push([c * r_out, 0.0, s * r_out]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 0.0]);
    }
    for i in 0..SEG {
        let (a, b) = (i as u32 * 2, (i as u32 + 1) * 2);
        indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_off_wins_and_presets_map() {
        // Sub-levers cannot conjure weather without a preset.
        std::env::set_var("VOXELFORGE_WEATHER", "off");
        std::env::set_var("VOXELFORGE_WEATHER_RAIN", "1");
        let cfg = WeatherConfig::from_env();
        assert!(!cfg.active && !cfg.rain);
        std::env::set_var("VOXELFORGE_WEATHER", "storm");
        let cfg = WeatherConfig::from_env();
        assert!(cfg.active && cfg.rain && cfg.fog && !cfg.godrays);
        std::env::set_var("VOXELFORGE_WEATHER_GODRAYS", "on");
        let cfg = WeatherConfig::from_env();
        assert!(cfg.godrays, "sub-lever overrides the preset");
        // Unknown preset = off, never a guess.
        std::env::remove_var("VOXELFORGE_WEATHER_GODRAYS");
        std::env::set_var("VOXELFORGE_WEATHER", "tornado");
        assert!(!WeatherConfig::from_env().active);
        std::env::remove_var("VOXELFORGE_WEATHER");
        std::env::remove_var("VOXELFORGE_WEATHER_RAIN");
    }

    #[test]
    fn wetness_targets_follow_rain_and_pin() {
        let mut cfg = WeatherConfig {
            active: true,
            rain: true,
            wetty: true,
            wet_pin: None,
            godrays: false,
            fog: false,
            streaks: 10,
            wind: Vec2::new(1.2, 0.0),
        };
        assert!((cfg.wet_target() - 1.0).abs() < 1e-6);
        // Streaks off does not dry the ground: the preset is still a rainy day.
        cfg.rain = false;
        assert!((cfg.wet_target() - 1.0).abs() < 1e-6);
        // A non-rainy preset with streaks forced on: drops without soaking.
        cfg.wetty = false;
        assert!(cfg.wet_target().abs() < 1e-6);
        cfg.wet_pin = Some(0.37);
        assert!((cfg.wet_target() - 0.37).abs() < 1e-6);
    }

    #[test]
    fn wet_palette_is_outdoor_ground_only() {
        // The outdoor ground/road palette wets; interiors and crafted types
        // stay dry (wood 5, obsidian 12, lamp 16, glass 17, water 18).
        for id in [1usize, 2, 3, 4, 8, 9, 10, 11, 13, 14, 15] {
            assert!(wettable(id), "id {id} should wet");
        }
        for id in [0usize, 5, 6, 7, 12, 16, 17, 18, 19] {
            assert!(!wettable(id), "id {id} must stay dry");
        }
    }

    #[test]
    fn falloff_scaling_keeps_the_family() {
        let f = FogFalloff::Exponential { density: 0.01 };
        match scale_falloff(f, 2.0) {
            FogFalloff::Exponential { density } => assert!((density - 0.02).abs() < 1e-9),
            _ => panic!("family must survive scaling"),
        }
        match scale_falloff(FogFalloff::Linear { start: 20.0, end: 100.0 }, 2.0) {
            FogFalloff::Linear { start, end } => {
                assert!((start - 10.0).abs() < 1e-9 && (end - 50.0).abs() < 1e-9)
            }
            _ => panic!("family must survive scaling"),
        }
    }
}
