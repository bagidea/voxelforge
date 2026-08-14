//! `vfx.rs` — the combat + world VFX layer (lane: **pixel / Flamingo**).
//!
//! ## Why this file depends on nothing but Bevy
//!
//! `combat.rs` (kevin) and `scene.rs` (poppy) are other people's lanes, and
//! `main.rs` is kevin's too (`docs/LANES.md`). So this module does NOT reach into
//! `crate::combat` for `Health` / `Enemy` / `PlayerCombat`: if it did, it could only
//! ever compile inside the `voxelforge` binary — which this lane may not wire — and
//! it would tie every VFX tweak to a cross-lane rebuild.
//!
//! Instead it is a **self-contained plugin with a message + component contract**.
//! Everything here is driven by things any lane can hand it:
//!
//! | You want | You do | Owner |
//! |---|---|---|
//! | impact burst + hit flash + hitstop + cam kick | `MessageWriter<vfx::Impact>::write(..)` | combat lane |
//! | enemy death that dissolves | `MessageWriter<vfx::Unravel>::write(..)` | combat lane |
//! | weapon trail while a heavy swings | put [`SwingTrail`] on the weapon/arm entity, set `hot` | combat lane |
//! | campfire coals + embers + flicker | put [`CampfireVfx`] on the campfire root | scene lane |
//! | camera receives the impact kick | put [`VfxCamera`] on the play camera | app lane |
//!
//! Nothing above is required for the plugin to be safe to add: with no messages and
//! no marker components it costs a handful of empty queries per frame and draws
//! nothing. That is deliberate — it can land in `main.rs` before any lane wires it.
//!
//! ## Art direction this implements (not invented here)
//!
//! * `docs/story-bible.md` — **the Unravelling**: "blocks losing cohesion and
//!   returning to raw chaos", "crumbling block edges, ash particles in air". That
//!   is why a husk death is a *staggered voxel dissolve rising into ash*, and why
//!   husk hits throw pale ash motes rather than red blood — a husk is a villager
//!   the Hollow is still draining, not a body.
//! * `docs/combat-design.md` §5.2 — hit-stop **80 ms** light / **100 ms** enemy-hit /
//!   **120 ms** parry / **150 ms** stagger, and the explicit rule *"do NOT use global
//!   time dilation"*. So [`Hitstop`] freezes only this module's own animation, and is
//!   published as a resource for other lanes to read.
//! * `docs/combat-design.md` §5.3 — shake amplitudes/durations; `combat.rs` already
//!   owns the shake itself, so this file adds the **directional camera kick** (recoil
//!   along the hit vector + a little roll) that shake alone cannot express.
//! * `docs/combat-design.md` §5.1 — white flash on a normal hit, red on the player
//!   being hit, and a bright ring on a parry.
//! * `docs/look-bible.md` §2 — key light 15–25° above the horizon; the showcase
//!   stage at the bottom of this file honours that (18°) so VFX are graded under the
//!   lighting the look bible actually asks for.
//!
//! ## No RNG dependency
//!
//! Particle spread uses a small xorshift in [`VfxRng`] rather than the `rand` crate:
//! one less dependency in the graph, and a
//! fixed seed means two renders of the same shot are byte-comparable, which the
//! grading scripts in `scripts/` rely on.

#![allow(dead_code)] // the contract is public API; each binary uses a subset of it.

use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;

// ===========================================================================
// Tuning — every number here traces to docs/combat-design.md or docs/look-bible.md
// ===========================================================================

/// Hit-stop windows (§5.2). Values in seconds, mirroring `combat.rs`'s constants
/// so the visual freeze and the gameplay freeze are the same length.
pub const HITSTOP_LIGHT: f32 = 0.080;
pub const HITSTOP_ENEMY: f32 = 0.100;
pub const HITSTOP_PARRY: f32 = 0.120;
pub const HITSTOP_STAGGER: f32 = 0.150;

/// Camera kick (this file's addition on top of `combat::Shake`). Amplitude is in
/// world units of recoil along the hit vector; duration in seconds.
const KICK_AMP: f32 = 0.085;
const KICK_DUR: f32 = 0.14;
const KICK_ROLL: f32 = 0.021; // radians

/// Hit flash (§5.1) — a shell that wraps the struck body for a few frames.
const FLASH_TTL: f32 = 0.11;

/// Weapon trail (§5.1 "weapon glint"): how long one ghost segment lives.
const TRAIL_TTL: f32 = 0.20;
/// Trail segments per second while a swing is `hot`. Emission is interpolated across
/// the frame (see `emit_trail`), so this is a real density knob rather than a cap of
/// one-per-frame: 180/s puts ~3 overlapping segments in every 60 fps frame.
const TRAIL_RATE: f32 = 180.0;

/// Campfire embers: seconds between released embers, and how long one lives.
const EMBER_EVERY: f32 = 0.085;
const EMBER_TTL: (f32, f32) = (1.1, 2.0);
/// Chance (per ember beat) of a smoke puff / a spark pop. Tuned so smoke drifts
/// in a lazy trickle (~3/s) and sparks crackle only occasionally (~1/s) — the fire
/// reads alive without a constant rain of particles.
const SMOKE_CHANCE: f32 = 0.30;
const SPARK_CHANCE: f32 = 0.06;

// ===========================================================================
// Public contract — messages
// ===========================================================================

/// What kind of thing just got hit. Drives palette + flash colour (§5.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HitFlavor {
    /// A husk (or anything the Hollow is draining): pale ash + warm sparks, no blood.
    Husk,
    /// The player: red vital motes + a red flash, so being hit reads differently
    /// from landing a hit even at a glance.
    Player,
    /// A successful parry: a bright white ring, no debris (nothing broke).
    Parry,
}

impl HitFlavor {
    fn hitstop(self) -> f32 {
        match self {
            HitFlavor::Husk => HITSTOP_LIGHT,
            HitFlavor::Player => HITSTOP_ENEMY,
            HitFlavor::Parry => HITSTOP_PARRY,
        }
    }
}

/// "Something just got struck here." The one message the combat lane needs to send
/// for impact sparks, voxel debris, the hit flash, the hit-stop and the camera kick.
///
/// ```ignore
/// impacts.write(vfx::Impact {
///     pos: hit_point,
///     dir: (target_pos - attacker_pos).normalize_or_zero(),
///     power: 1.0,                       // 1.0 = light, ~1.8 = heavy/charged
///     flavor: vfx::HitFlavor::Husk,
///     body_half: Some(Vec3::new(0.4, 1.2, 0.4)),  // None = no wrap flash
/// });
/// ```
#[derive(Clone, Debug)]
pub struct Impact {
    /// World-space point of contact (roughly chest height reads best).
    pub pos: Vec3,
    /// Direction the blow travelled, attacker → target. Debris flies along it and
    /// the camera recoils along it.
    pub dir: Vec3,
    /// Scale on count/speed/shake. 1.0 = light hit, 1.8 = heavy, 2.4 = stagger break.
    pub power: f32,
    pub flavor: HitFlavor,
    /// Half-extents of the struck body. `Some` spawns the wrap-around hit flash
    /// (§5.1) centred on `pos`; `None` skips it (e.g. a hit on a wall).
    pub body_half: Option<Vec3>,
}

impl Message for Impact {}

/// "This actor's form has lost cohesion" — the death dissolve (story-bible: the
/// Unravelling). Fire it *instead of* letting the body vanish; the VFX layer keeps
/// its own copy of the silhouette, so the real entity can despawn the same frame.
///
/// ```ignore
/// unravels.write(vfx::Unravel {
///     pos: husk_tf.translation,
///     half: Vec3::new(0.42, 1.25, 0.42),
///     tint: Color::srgb(0.32, 0.34, 0.40),   // the husk's armour colour
/// });
/// ```
#[derive(Clone, Debug)]
pub struct Unravel {
    /// Centre of the body that is coming apart.
    pub pos: Vec3,
    /// Half-extents of that body — the dissolve fills this volume with voxels.
    pub half: Vec3,
    /// Base colour of the blocks that are losing cohesion.
    pub tint: Color,
}

impl Message for Unravel {}

/// "A foot just struck the ground here" — a footfall or a dodge kick-off.
/// Deliberately cheap: this can fire twice a second per moving actor, so it's a
/// handful of small ground-dust motes, not the full [`Impact`] treatment (no
/// flash, no hit-stop, no light burst).
///
/// ```ignore
/// dust.write(vfx::FootDust { pos: heel_pos, power: 1.0 }); // ~2.2 for a dodge kick-off
/// ```
#[derive(Clone, Debug)]
pub struct FootDust {
    /// World-space point the foot struck.
    pub pos: Vec3,
    /// Scale on count/speed. 1.0 = an ordinary footstep, ~2.2 = a dodge roll's
    /// kick-off (more mass, more speed, more dust).
    pub power: f32,
}

impl Message for FootDust {}

// ===========================================================================
// Public contract — components
// ===========================================================================

/// Put this on the weapon / arm entity that swings. While `hot` is true the VFX
/// layer lays down a fading ribbon along wherever that entity actually is, so the
/// trail follows the real animation instead of a guessed arc.
///
/// The combat lane owns exactly one line: `hot = matches!(state, Heavy | Charged)`.
#[derive(Component)]
pub struct SwingTrail {
    /// Emit while true.
    pub hot: bool,
    /// Half-extents of one ribbon segment — match the blade's cross-section.
    pub half: Vec3,
    /// Internal emitter accumulator; leave at 0.0.
    pub accum: f32,
}

impl Default for SwingTrail {
    fn default() -> Self {
        Self { hot: false, half: Vec3::new(0.05, 0.30, 0.05), accum: 0.0 }
    }
}

/// Put this on a campfire root transform and the VFX layer gives it a glowing coal
/// bed, rising embers and a flickering warm light — without touching whatever the
/// scene lane already spawned there (it only *adds* children).
#[derive(Component)]
pub struct CampfireVfx {
    /// Radius of the pit, in world units. Coals and embers are placed inside it.
    pub radius: f32,
    /// Peak intensity of the flicker light (lumens).
    pub light: f32,
}

impl Default for CampfireVfx {
    fn default() -> Self {
        Self { radius: 0.55, light: 240_000.0 }
    }
}

/// Mark the camera that should receive the directional impact kick. Opt-in so the
/// editor / bench / beauty-shot cameras are never nudged.
///
/// Schedule note for the app lane: run `vfx` after whatever positions the camera
/// (`fly_camera`, `combat::apply_shake`) — the kick is applied as a delta on top.
#[derive(Component)]
pub struct VfxCamera;

// ===========================================================================
// Public contract — resources
// ===========================================================================

/// Time left in the current hit-stop. This module freezes **its own** particle
/// integration while it is > 0; per `combat-design.md` §5.2 it deliberately does
/// *not* touch `Time`, so gameplay, camera and audio keep running. Other lanes may
/// read it (e.g. to hold an animation frame).
#[derive(Resource, Default)]
pub struct Hitstop {
    pub left: f32,
}

impl Hitstop {
    pub fn frozen(&self) -> bool {
        self.left > 0.0
    }
    fn punch(&mut self, secs: f32) {
        self.left = self.left.max(secs);
    }
}

/// Deterministic xorshift32 — see the module header for why this isn't `rand`.
#[derive(Resource)]
pub struct VfxRng(u32);

impl Default for VfxRng {
    fn default() -> Self {
        Self(0x5EED_1234)
    }
}

impl VfxRng {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in `0.0..1.0`.
    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }
    /// Uniform in `-1.0..1.0`.
    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// A unit-ish vector in a cone around `axis`, `spread` = 0 (tight) .. 1 (hemisphere).
    fn cone(&mut self, axis: Vec3, spread: f32) -> Vec3 {
        let jitter = Vec3::new(self.signed(), self.signed(), self.signed());
        (axis.normalize_or_zero() + jitter * spread).normalize_or_zero()
    }
}

// ===========================================================================
// Internal state
// ===========================================================================

/// One piece of flying voxel debris / spark / ember / ash mote.
#[derive(Component)]
struct Particle {
    vel: Vec3,
    /// Constant downward pull. Sparks use ~2, debris ~14, embers *negative* (they rise).
    gravity: f32,
    /// Per-second velocity damping factor; embers need heavy drag to hang in the air.
    drag: f32,
    /// Radians/sec about each axis — voxel debris tumbling sells the "block" read.
    spin: Vec3,
    age: f32,
    ttl: f32,
    /// Scale at birth. Particles shrink toward 0 so they *lose cohesion* rather than
    /// pop out of existence (story-bible), and no per-piece alpha material is needed.
    /// Stored as the real `Vec3` because trail segments are deliberately non-uniform.
    born: Vec3,
    /// Fraction of `ttl` to hold full size before shrinking. Debris holds, ash doesn't.
    hold: f32,
    /// Delay before this piece starts moving at all — staggers a dissolve so the body
    /// comes apart top-down instead of exploding uniformly.
    delay: f32,
}

/// The short-lived shell that wraps a struck body (§5.1 hit flash) and the light
/// burst that goes with an impact. Both just fade and die.
#[derive(Component)]
struct Ephemeral {
    age: f32,
    ttl: f32,
    /// Scale multiplier reached at end of life (flash shells bloom outward).
    grow_to: f32,
    born_scale: Vec3,
    /// Peak light intensity, if this entity carries a `PointLight`.
    light0: f32,
    /// Peak base colour as raw sRGB + alpha, and peak emissive, for entities that
    /// carry a private material. Kept as plain floats rather than a `Color` so the
    /// fade needs no colour-space trait in scope and can't drift between spaces.
    base0: [f32; 4],
    emissive0: [f32; 3],
}

/// Directional recoil accumulated from impacts, applied to [`VfxCamera`].
#[derive(Resource, Default)]
struct CamKick {
    dir: Vec3,
    amp: f32,
    roll: f32,
    age: f32,
    dur: f32,
    /// What `apply_cam_kick` added to the camera LAST frame. It has to be undone
    /// before this frame's offset goes on, or the kick is cumulative — see the note
    /// in `apply_cam_kick`.
    applied_pos: Vec3,
    applied_roll: f32,
}

/// A campfire that has already had its coals built.
#[derive(Component)]
struct CampfireBuilt {
    accum: f32,
}

/// The flicker light this module owns (never the scene lane's own light).
#[derive(Component)]
struct FireFlicker {
    base: f32,
}

/// Where a [`SwingTrail`] emitter was last frame, so this frame's segments can be
/// laid down along the path between the two instead of stacked on one spot. Added
/// and removed by `emit_trail`; the public `SwingTrail` contract is untouched.
#[derive(Component)]
struct TrailPrev {
    pos: Vec3,
    rot: Quat,
}

/// One glowing coal — its emissive breathes on its own phase.
#[derive(Component)]
struct Coal {
    phase: f32,
}

/// Shared meshes + materials. One handle each, so a burst of 40 particles is 40
/// entities against 1 mesh and ~7 materials — cheap enough to fire every swing.
#[derive(Resource)]
struct VfxAssets {
    cube: Handle<Mesh>,
    /// Three shared tones, not one — see `load_vfx_assets` for why sparks are the
    /// one particle in this file that used to be flat.
    spark: [Handle<StandardMaterial>; 3],
    ember: Handle<StandardMaterial>,
    ash: Handle<StandardMaterial>,
    stone_dust: Handle<StandardMaterial>,
    vital: Handle<StandardMaterial>,
    parry: Handle<StandardMaterial>,
    trail: Handle<StandardMaterial>,
    coal: Handle<StandardMaterial>,
    /// Deep wet red — the "damage landed" read on every non-parry hit and the
    /// death burst (the CEO's pass: blood must read clearly on target).
    blood: Handle<StandardMaterial>,
    /// Flat dark red — the pool a corpse leaves on the ground; lingers longer.
    blood_pool: Handle<StandardMaterial>,
    /// Cool translucent grey — campfire smoke puffs and the soft impact dust.
    smoke: Handle<StandardMaterial>,
    /// Warm neutral translucent mote — the ambient dust that keeps a scene alive.
    air_dust: Handle<StandardMaterial>,
}

// ===========================================================================
// Plugin
// ===========================================================================

/// Add with a single line: `.add_plugins(vfx::VfxPlugin)`.
///
/// Registers the two messages, the [`Hitstop`] resource and every system below.
/// Inert until something writes a message or carries one of the marker components.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Impact>()
            .add_message::<Unravel>()
            .add_message::<FootDust>()
            .init_resource::<Hitstop>()
            .init_resource::<VfxRng>()
            .init_resource::<CamKick>()
            .add_systems(Startup, load_vfx_assets)
            .add_systems(
                Update,
                (
                    // tick_hitstop runs FIRST so an impact fired this frame freezes
                    // starting this frame, not one frame late.
                    tick_hitstop,
                    (on_impact, on_unravel, on_footdust, emit_trail, campfire_build, campfire_pulse, ambient_dust),
                    (tick_particles, tick_ephemeral, tick_coals),
                    // The kick is a delta on the final camera transform, so it must
                    // land after any system that *sets* that transform.
                    apply_cam_kick,
                )
                    .chain(),
            );
    }
}

fn load_vfx_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A unit cube, scaled per particle — the whole VFX vocabulary is voxels, because
    // a round sprite in this world would read as a different game (look-bible §1).
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Emissive values are deliberately > 1.0: they are the bloom seeds. The camera's
    // Bloom threshold (hero.rs uses Bloom::NATURAL) only catches HDR pixels, so a
    // spark at 1.0 would look like grey plastic.
    // Golden-hour spark palette: real hot metal doesn't throw one flat hue — the
    // freshest chips read near-white, most sit at the honeyed gold the story-bible
    // key light already uses, and a few have cooled to ember-orange. `on_impact`
    // picks between these three per spark (deterministically, via `VfxRng`) so a
    // single burst shows the same colour spread real sparks do, instead of one
    // shared material looking pasted-on flat. Kept to three shared materials
    // (not one per particle) so the "cheap enough to fire every swing" budget
    // this module's header promises still holds.
    let spark = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.97, 0.90),
            emissive: LinearRgba::rgb(17.0, 15.5, 12.5),
            unlit: true,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.86, 0.55),
            emissive: LinearRgba::rgb(14.0, 7.4, 2.2),
            unlit: true,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.62, 0.24),
            emissive: LinearRgba::rgb(11.0, 4.0, 0.9),
            unlit: true,
            ..default()
        }),
    ];
    let ember = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.52, 0.16),
        emissive: LinearRgba::rgb(7.5, 2.6, 0.5),
        unlit: true,
        ..default()
    });
    // Ash is lit, not emissive — it has to read as *matter* drifting through the key
    // light, otherwise the dissolve looks like a glow effect instead of a body
    // coming apart.
    let ash = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.60, 0.58),
        perceptual_roughness: 0.95,
        ..default()
    });
    let stone_dust = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.44, 0.42),
        perceptual_roughness: 0.95,
        ..default()
    });
    let vital = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.06, 0.07),
        perceptual_roughness: 0.6,
        ..default()
    });
    let parry = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(18.0, 17.0, 15.0),
        unlit: true,
        ..default()
    });
    // Translucent, NOT solid. A swept blade covers a big wedge of screen; at full
    // opacity that wedge is a sheet of paper hiding the enemy you are hitting. The
    // ribbon has to say "the blade was here a moment ago", so you must be able to
    // see through it. Alpha is low because segments overlap ~3 deep by design.
    let trail = materials.add(StandardMaterial {
        base_color: Color::srgba(0.98, 0.92, 0.78, 0.20),
        emissive: LinearRgba::rgb(2.0, 1.5, 0.8),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let coal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.09, 0.05),
        emissive: LinearRgba::rgb(4.2, 1.1, 0.15),
        perceptual_roughness: 1.0,
        ..default()
    });
    // Blood — deep red with a small vital emissive so droplets bloom against the
    // husk's grey armour, wet (low roughness) so it reads as fluid, not cloth.
    let blood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.04, 0.05),
        emissive: LinearRgba::rgb(0.9, 0.05, 0.05),
        perceptual_roughness: 0.35,
        metallic: 0.1,
        ..default()
    });
    // The pool a body leaves behind: darker, flatter, no bloom — a stain, not a light.
    let blood_pool = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.02, 0.03),
        perceptual_roughness: 0.9,
        ..default()
    });
    // Smoke — translucent grey, unlit so it reads as air catching the key light
    // rather than as a solid cube drifting up out of a fire.
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgba(0.45, 0.43, 0.42, 0.18),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    // Ambient dust — lit, faint warm grey, translucent, so motes drift through the
    // key light without ever stealing the frame from the sparks and embers.
    let air_dust = materials.add(StandardMaterial {
        base_color: Color::srgba(0.74, 0.71, 0.65, 0.16),
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(VfxAssets {
        cube,
        spark,
        ember,
        ash,
        stone_dust,
        vital,
        parry,
        trail,
        coal,
        blood,
        blood_pool,
        smoke,
        air_dust,
    });
}

// ===========================================================================
// ① Impact — sparks + voxel debris + flavour motes   ⑥ hit flash   ③ hitstop/kick
// ===========================================================================

fn on_impact(
    mut commands: Commands,
    mut hits: MessageReader<Impact>,
    mut rng: ResMut<VfxRng>,
    mut hitstop: ResMut<Hitstop>,
    mut kick: ResMut<CamKick>,
    assets: Option<Res<VfxAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(assets) = assets else { return };

    for hit in hits.read() {
        let power = hit.power.max(0.2);
        let dir = hit.dir.normalize_or_zero();
        // Debris flies back along the blow *and* upward: straight-back-only reads flat
        // because the camera is usually behind the attacker.
        let out = (dir + Vec3::Y * 0.55).normalize_or_zero();

        // ---- ③ hit-stop + camera kick -----------------------------------
        // §5.2: freeze the affected actors, never global time. Here that means this
        // module's particles hold still for a beat — the frame the hit lands is the
        // frame the player reads, so nothing in it should be smeared by motion.
        hitstop.punch(hit.flavor.hitstop() * power.min(1.9));
        kick.dir = dir;
        kick.amp = KICK_AMP * power;
        kick.roll = KICK_ROLL * power * if dir.x >= 0.0 { 1.0 } else { -1.0 };
        kick.age = 0.0;
        kick.dur = KICK_DUR;

        // ---- sparks: fast, hot, short-lived -----------------------------
        // Count scales with power so a charged hit is visibly bigger than a jab
        // without needing a second effect authored.
        let sparks = (9.0 * power) as usize + 4;
        for _ in 0..sparks {
            let v = rng.cone(out, 0.85) * rng.range(3.4, 8.2) * power;
            let born = Vec3::splat(rng.range(0.035, 0.075));
            let tone = assets.spark[(rng.unit() * 3.0) as usize % 3].clone();
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(tone),
                Visibility::default(),
                Transform::from_translation(hit.pos + rng.cone(Vec3::Y, 1.0) * 0.09)
                    .with_scale(born),
                Particle {
                    vel: v,
                    gravity: 2.4, // sparks are light — they arc lazily, debris doesn't
                    drag: 2.6,
                    spin: Vec3::ZERO,
                    age: 0.0,
                    ttl: rng.range(0.18, 0.36),
                    born,
                    hold: 0.15,
                    delay: 0.0,
                },
            ));
        }

        // ---- voxel debris: the block-world signature --------------------
        let chunks = (5.0 * power) as usize + 3;
        for _ in 0..chunks {
            let v = rng.cone(out, 0.6) * rng.range(1.8, 4.6) * power;
            let mat = match hit.flavor {
                HitFlavor::Husk => assets.stone_dust.clone(),
                HitFlavor::Player => assets.stone_dust.clone(),
                HitFlavor::Parry => assets.spark[(rng.unit() * 3.0) as usize % 3].clone(),
            };
            let born = Vec3::splat(rng.range(0.07, 0.17));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(mat),
                Visibility::default(),
                Transform::from_translation(hit.pos + rng.cone(Vec3::Y, 1.0) * 0.14)
                    .with_scale(born),
                Particle {
                    vel: v,
                    gravity: 14.0, // real weight: chips fall, they don't float
                    drag: 0.4,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 9.0,
                    age: 0.0,
                    ttl: rng.range(0.65, 1.15),
                    born,
                    hold: 0.72, // hold size, then crumble — a chip doesn't shrink as it falls
                    delay: 0.0,
                },
            ));
        }

        // ---- flavour motes: ash for a husk, vital red for the player ----
        // story-bible: a husk is being *drained*, not bleeding. Pale motes drift UP
        // out of it (negative gravity) — the same visual language as the Unravelling,
        // so a player learns to read "this thing is being unmade" from the first hit.
        let motes = (7.0 * power) as usize + 3;
        for _ in 0..motes {
            let (mat, grav, drag, ttl, size) = match hit.flavor {
                HitFlavor::Husk => (assets.ash.clone(), -0.85, 2.2, (0.55, 1.10), (0.03, 0.07)),
                HitFlavor::Player => (assets.vital.clone(), 11.0, 0.8, (0.35, 0.70), (0.04, 0.09)),
                HitFlavor::Parry => (assets.parry.clone(), 0.0, 3.4, (0.16, 0.30), (0.03, 0.06)),
            };
            // A parry throws a flat ring (nothing broke — it's a deflection), the
            // others follow the blow.
            let v = if hit.flavor == HitFlavor::Parry {
                let a = rng.unit() * std::f32::consts::TAU;
                Vec3::new(a.cos(), rng.signed() * 0.18, a.sin()) * rng.range(3.0, 5.2)
            } else {
                rng.cone(out, 1.0) * rng.range(0.9, 2.6)
            };
            let born = Vec3::splat(rng.range(size.0, size.1));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(mat),
                Visibility::default(),
                Transform::from_translation(hit.pos + rng.cone(Vec3::Y, 1.0) * 0.12)
                    .with_scale(born),
                Particle {
                    vel: v,
                    gravity: grav,
                    drag,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 4.0,
                    age: 0.0,
                    ttl: rng.range(ttl.0, ttl.1),
                    born,
                    hold: 0.1,
                    delay: 0.0,
                },
            ));
        }

        // ---- blood: the "damage landed" read (CEO's pass) ----------------
        // A non-parry hit throws wet red droplets that arc and splatter DOWN,
        // so a landed blow is unmistakable even against the husk's grey armour.
        // The husk still sheds ash (it is being unmade), but the blood is the
        // damage telegraph a player needs to read in the middle of a trade.
        if hit.flavor != HitFlavor::Parry {
            let drops = (6.0 * power) as usize + 4;
            for _ in 0..drops {
                let v = rng.cone(out, 0.9) * rng.range(2.0, 5.6) * power
                    + Vec3::Y * rng.range(0.6, 1.8);
                let born = Vec3::splat(rng.range(0.03, 0.075));
                commands.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.blood.clone()),
                    Visibility::default(),
                    Transform::from_translation(hit.pos + rng.cone(Vec3::Y, 1.0) * 0.10)
                        .with_scale(born),
                    Particle {
                        vel: v,
                        gravity: 18.0, // blood is heavy fluid — it falls, it does not drift
                        drag: 0.6,
                        spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 7.0,
                        age: 0.0,
                        ttl: rng.range(0.45, 0.85),
                        born,
                        hold: 0.35,
                        delay: 0.0,
                    },
                ));
            }
        }

        // ---- soft dust cloud: the weight of the blow ----------------------
        // Fast chips read as "broke something"; a slow, large dust puff reads as
        // "hit something heavy". Together they make an impact feel like mass
        // rather than a firework.
        {
            let puffs = (4.0 * power) as usize + 3;
            for _ in 0..puffs {
                let v = rng.cone(out, 0.8) * rng.range(0.7, 2.0) * power
                    + Vec3::Y * rng.range(0.4, 1.2);
                let born = Vec3::splat(rng.range(0.10, 0.22));
                commands.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.smoke.clone()),
                    Visibility::default(),
                    Transform::from_translation(hit.pos + rng.cone(Vec3::Y, 1.0) * 0.16)
                        .with_scale(born),
                    Particle {
                        vel: v,
                        gravity: -0.3, // dust hangs, then disperses
                        drag: 2.6,
                        spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 1.5,
                        age: 0.0,
                        ttl: rng.range(0.5, 0.9),
                        born,
                        hold: 0.12,
                        delay: 0.0,
                    },
                ));
            }
        }

        // ---- ⑥ hit flash: a shell around the struck body ----------------
        // Per-flash material (not shared) because this one really does need to fade
        // its alpha — a shrinking shell would look like the body itself shrank.
        if let Some(half) = hit.body_half {
            // §5.1: white when you LAND a hit, red when you TAKE one. That colour is
            // the fastest read on the screen, so it carries in both the emissive
            // (which blooms) and the base colour (which survives if bloom is off).
            // Alpha is deliberately LOW (was 0.80 on the first pass): at 0.8 the shell
            // is an opaque white box and the thing you just hit disappears behind the
            // feedback that it was hit. The flash has to sit ON the silhouette, not
            // replace it — you still need to read the husk's pose on the frame the
            // blow lands.
            let (base, em) = match hit.flavor {
                HitFlavor::Husk => ([1.0, 0.97, 0.92, 0.38], [1.8, 1.7, 1.5]),
                HitFlavor::Player => ([1.0, 0.30, 0.26, 0.46], [2.4, 0.24, 0.20]),
                HitFlavor::Parry => ([1.0, 0.99, 0.94, 0.50], [3.0, 2.8, 2.4]),
            };
            let flash_mat = materials.add(StandardMaterial {
                base_color: Color::srgba(base[0], base[1], base[2], base[3]),
                emissive: LinearRgba::rgb(em[0], em[1], em[2]),
                alpha_mode: AlphaMode::Blend,
                // NOT unlit: `unlit` short-circuits to base colour on some paths and
                // the emissive tint (the part that blooms) can be dropped with it.
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            let born = half * 2.06;
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(flash_mat),
                Visibility::default(),
                Transform::from_translation(hit.pos).with_scale(born),
                Ephemeral {
                    age: 0.0,
                    ttl: FLASH_TTL,
                    grow_to: 1.22,
                    born_scale: born,
                    light0: 0.0,
                    base0: base,
                    emissive0: em,
                },
            ));
        }

        // ---- ⑥ parry ring: an instant, compact burst at the point of contact --
        // A parry never carries `body_half` (nothing broke, so no wrap shell — see
        // the flash block above), but the deflection still needs its own
        // unmistakable "clang": the receive window it has to be read against is a
        // dozen frames wide, so this has to peak on the very frame it spawns, not
        // ease in. `Ephemeral`'s colour/light fade is already front-loaded
        // ((1-t)², see `tick_ephemeral`) — starting the SCALE big too is what
        // makes it read as a burst rather than a blob growing into one.
        if hit.flavor == HitFlavor::Parry {
            let ring_mat = materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.99, 0.94, 0.85),
                emissive: LinearRgba::rgb(22.0, 20.0, 17.0),
                alpha_mode: AlphaMode::Blend,
                // NOT unlit — same reason as the hit-flash shell above: unlit can
                // drop the emissive tint that actually blooms.
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            let born = Vec3::splat(0.16 + 0.05 * power);
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(ring_mat),
                Visibility::default(),
                Transform::from_translation(hit.pos).with_scale(born),
                Ephemeral {
                    age: 0.0,
                    ttl: 0.10,
                    grow_to: 3.4,
                    born_scale: born,
                    light0: 0.0,
                    base0: [1.0, 0.99, 0.94, 0.85],
                    emissive0: [22.0, 20.0, 17.0],
                },
            ));
        }

        // ---- light burst: the impact lights its own surroundings --------
        // Without this the sparks glow but nothing around them reacts, which is the
        // single biggest "particles pasted on top" tell.
        commands.spawn((
            PointLight {
                color: match hit.flavor {
                    HitFlavor::Player => Color::srgb(1.0, 0.42, 0.34),
                    _ => Color::srgb(1.0, 0.84, 0.62),
                },
                intensity: 400_000.0 * power,
                range: 7.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(hit.pos),
            Visibility::default(),
            Ephemeral {
                age: 0.0,
                ttl: 0.16,
                grow_to: 1.0,
                born_scale: Vec3::ONE,
                light0: 400_000.0 * power,
                base0: [0.0; 4],
                emissive0: [0.0; 3],
            },
        ));
    }
}

// ===========================================================================
// ⑦ Foot dust — a puff of ground dust on every footfall / dodge kick-off
// ===========================================================================
//
// Both `anim.rs`'s `AnimFootstep` and `AnimDodge` message streams already
// existed with zero subscribers anywhere in the crate — the rig was reporting
// exactly which frame each foot struck and nothing ever used it. This is the
// generic sink: cheap, reusable for both (a dodge kick-off is just a bigger
// `power`), see `vfx_bridge.rs`'s `footstep_dust` / `dodge_dust`.

fn on_footdust(
    mut commands: Commands,
    mut steps: MessageReader<FootDust>,
    mut rng: ResMut<VfxRng>,
    assets: Option<Res<VfxAssets>>,
) {
    let Some(assets) = assets else { return };

    for step in steps.read() {
        let power = step.power.max(0.2);
        let motes = (3.0 * power) as usize + 2;
        for _ in 0..motes {
            let v = rng.cone(Vec3::Y, 0.9) * rng.range(0.5, 1.5) * power
                + Vec3::new(rng.signed(), 0.0, rng.signed()) * 0.4 * power;
            let born = Vec3::splat(rng.range(0.03, 0.07) * power.min(1.6));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.stone_dust.clone()),
                Visibility::default(),
                Transform::from_translation(step.pos + rng.cone(Vec3::Y, 1.0) * 0.06)
                    .with_scale(born),
                Particle {
                    vel: v,
                    gravity: 1.4, // a mote of dust, not a chip — it drifts, it doesn't fall
                    drag: 3.2,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 3.0,
                    age: 0.0,
                    ttl: rng.range(0.25, 0.45),
                    born,
                    hold: 0.05,
                    delay: 0.0,
                },
            ));
        }
    }
}

// ===========================================================================
// ⑤ Unravel — the death dissolve
// ===========================================================================

fn on_unravel(
    mut commands: Commands,
    mut deaths: MessageReader<Unravel>,
    mut rng: ResMut<VfxRng>,
    assets: Option<Res<VfxAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(assets) = assets else { return };

    for d in deaths.read() {
        // The body's own colour, so the dissolve is recognisably *that* enemy coming
        // apart rather than a generic puff.
        let body_mat = materials.add(StandardMaterial {
            base_color: d.tint,
            perceptual_roughness: 0.7,
            metallic: 0.15,
            ..default()
        });

        // Fill the silhouette with a voxel grid. 4×7×4 is enough to read as "that
        // shape" for one beat before it comes apart, and cheap enough to be free.
        let (nx, ny, nz) = (4usize, 7usize, 4usize);
        let step = Vec3::new(
            d.half.x * 2.0 / nx as f32,
            d.half.y * 2.0 / ny as f32,
            d.half.z * 2.0 / nz as f32,
        );
        let block = step.min_element() * 0.92;

        for iy in 0..ny {
            // Top-down stagger: the head loses cohesion first and the feet last, so
            // the body *sags* apart. A uniform start reads as an explosion, which is
            // the opposite of what the Unravelling is.
            let t_from_top = 1.0 - (iy as f32 / (ny - 1).max(1) as f32);
            let delay = t_from_top * 0.28;
            for ix in 0..nx {
                for iz in 0..nz {
                    let local = Vec3::new(
                        (ix as f32 + 0.5) * step.x - d.half.x,
                        (iy as f32 + 0.5) * step.y - d.half.y,
                        (iz as f32 + 0.5) * step.z - d.half.z,
                    );
                    // Outward drift + a rise: blocks "return to raw chaos" upward,
                    // they don't collapse into a pile (that would read as ragdoll).
                    let outward = Vec3::new(local.x, 0.0, local.z).normalize_or_zero();
                    let vel = outward * rng.range(0.25, 0.95)
                        + Vec3::Y * rng.range(0.55, 1.5)
                        + Vec3::new(rng.signed(), 0.0, rng.signed()) * 0.25;
                    commands.spawn((
                        Mesh3d(assets.cube.clone()),
                        MeshMaterial3d(body_mat.clone()),
                        Visibility::default(),
                        Transform::from_translation(d.pos + local)
                            .with_scale(Vec3::splat(block)),
                        Particle {
                            vel,
                            gravity: -0.55, // buoyant: form is leaving the world upward
                            drag: 1.4,
                            spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 2.6,
                            age: 0.0,
                            ttl: rng.range(0.85, 1.35),
                            born: Vec3::splat(block),
                            hold: 0.18, // barely holds — it is shrinking almost at once
                            delay,
                        },
                    ));
                }
            }
        }

        // Ash the body leaves behind, rising longer than the blocks do.
        for _ in 0..34 {
            let p = d.pos
                + Vec3::new(
                    rng.signed() * d.half.x,
                    rng.signed() * d.half.y,
                    rng.signed() * d.half.z,
                );
            let born = Vec3::splat(rng.range(0.025, 0.06));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.ash.clone()),
                Visibility::default(),
                Transform::from_translation(p).with_scale(born),
                Particle {
                    vel: Vec3::new(rng.signed() * 0.35, rng.range(0.5, 1.3), rng.signed() * 0.35),
                    gravity: -0.5,
                    drag: 1.1,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 2.0,
                    age: 0.0,
                    ttl: rng.range(1.2, 2.1),
                    born,
                    hold: 0.4,
                    delay: rng.range(0.0, 0.35),
                },
            ));
        }

        // ---- blood burst + pool: the death made visible --------------------
        // The dissolve carries the form away; the blood is what stays. A burst of
        // droplets at the moment of death plus a pool that lingers on the ground
        // where the body stood — the two together are the "it is dead"
        // punctuation the dissolve alone never quite delivered.
        for _ in 0..26 {
            let p = d.pos
                + Vec3::new(
                    rng.signed() * d.half.x * 0.6,
                    rng.signed() * d.half.y * 0.4,
                    rng.signed() * d.half.z * 0.6,
                );
            let born = Vec3::splat(rng.range(0.03, 0.08));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.blood.clone()),
                Visibility::default(),
                Transform::from_translation(p).with_scale(born),
                Particle {
                    vel: Vec3::new(
                        rng.signed() * 1.6,
                        rng.range(1.2, 3.4),
                        rng.signed() * 1.6,
                    ),
                    gravity: 16.0,
                    drag: 0.7,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 8.0,
                    age: 0.0,
                    ttl: rng.range(0.5, 0.9),
                    born,
                    hold: 0.3,
                    delay: rng.range(0.0, 0.12),
                },
            ));
        }
        // The pool: a flat disc of blood at the feet, holding long then drying up
        // (shrinking) so the ground remembers the kill after the body has gone.
        {
            let pool_scale = Vec3::new(d.half.x * 1.7, 0.03, d.half.z * 1.7);
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.blood_pool.clone()),
                Visibility::default(),
                Transform::from_translation(d.pos - Vec3::Y * d.half.y)
                    .with_scale(pool_scale),
                Particle {
                    vel: Vec3::ZERO,
                    gravity: 0.0,
                    drag: 0.0,
                    spin: Vec3::ZERO,
                    age: 0.0,
                    ttl: 5.0,
                    born: pool_scale,
                    hold: 0.7,
                    delay: 0.0,
                },
            ));
        }

        // A dim, cool pulse as the form lets go — reads as energy leaving, and it
        // separates the death beat from the hit beats that preceded it.
        commands.spawn((
            PointLight {
                color: Color::srgb(0.72, 0.80, 0.92),
                intensity: 260_000.0,
                range: 9.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(d.pos),
            Visibility::default(),
            Ephemeral {
                age: 0.0,
                ttl: 0.55,
                grow_to: 1.0,
                born_scale: Vec3::ONE,
                light0: 260_000.0,
                base0: [0.0; 4],
                emissive0: [0.0; 3],
            },
        ));
    }
}

// ===========================================================================
// ② Weapon trail
// ===========================================================================

fn emit_trail(
    mut commands: Commands,
    time: Res<Time>,
    hitstop: Res<Hitstop>,
    assets: Option<Res<VfxAssets>>,
    mut trails: Query<(Entity, &mut SwingTrail, &GlobalTransform, Option<&TrailPrev>)>,
) {
    let Some(assets) = assets else { return };
    // A trail laid down during hit-stop would draw a stripe through the frozen frame.
    if hitstop.frozen() {
        return;
    }
    let dt = time.delta_secs();

    for (entity, mut trail, gtf, prev) in &mut trails {
        let (scale, rot, pos) = gtf.to_scale_rotation_translation();
        if !trail.hot {
            trail.accum = 0.0;
            commands.entity(entity).remove::<TrailPrev>();
            continue;
        }

        // Emit at a fixed rate rather than once per frame, so the ribbon's density is
        // the same at 30 fps and 144 fps.
        trail.accum += dt * TRAIL_RATE;
        let steps = trail.accum.floor();
        trail.accum -= steps;

        // Spread this frame's segments ALONG the path the blade actually travelled
        // since last frame. Stamping them all at the current transform is what the
        // first render showed: one slat per frame with clean gaps between, so a fast
        // swing read as a venetian blind instead of a ribbon. A fast arc covers tens
        // of centimetres per frame — the gaps are the frame boundaries made visible.
        let (p0, r0) = prev.map(|p| (p.pos, p.rot)).unwrap_or((pos, rot));
        let n = steps as u32;
        for i in 0..n {
            let f = (i as f32 + 1.0) / n as f32;
            let born = trail.half * 2.0 * scale;
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.trail.clone()),
                Visibility::default(),
                Transform {
                    translation: p0.lerp(pos, f),
                    rotation: r0.slerp(rot, f),
                    scale: born,
                },
                Particle {
                    // A trail segment is *stationary* — the arc comes from the weapon
                    // having moved between segments, exactly like a real motion trail.
                    vel: Vec3::ZERO,
                    gravity: 0.0,
                    drag: 0.0,
                    spin: Vec3::ZERO,
                    age: 0.0,
                    ttl: TRAIL_TTL,
                    born,
                    // Hold most of the width, then taper: shrinking from birth made
                    // each segment thinner than its neighbour's gap, which reopened
                    // the comb the interpolation above just closed.
                    hold: 0.45,
                    delay: 0.0,
                },
            ));
        }
        commands.entity(entity).insert(TrailPrev { pos, rot });
    }
}

// ===========================================================================
// ④ Campfire — coal bed, embers, flicker
// ===========================================================================

fn campfire_build(
    mut commands: Commands,
    mut rng: ResMut<VfxRng>,
    assets: Option<Res<VfxAssets>>,
    fires: Query<(Entity, &CampfireVfx), Without<CampfireBuilt>>,
) {
    let Some(assets) = assets else { return };

    for (e, fire) in &fires {
        commands.entity(e).insert(CampfireBuilt { accum: 0.0 });
        commands.entity(e).with_children(|p| {
            // Coal bed: a scatter of low, dark-red blocks that breathe. This is what
            // makes a fire read as *burning down into something* rather than as a
            // flame decal — and it survives even when the flame is off-frame.
            for _ in 0..11 {
                let a = rng.unit() * std::f32::consts::TAU;
                let r = fire.radius * rng.range(0.15, 0.95);
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.coal.clone()),
                    Visibility::default(),
                    Transform::from_xyz(a.cos() * r, rng.range(0.03, 0.09), a.sin() * r)
                        .with_scale(Vec3::new(
                            rng.range(0.10, 0.20),
                            rng.range(0.05, 0.10),
                            rng.range(0.10, 0.20),
                        ))
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, rng.unit() * 1.57)),
                    Coal { phase: rng.unit() * std::f32::consts::TAU },
                ));
            }
            // This module's own flicker light. It never touches whatever light the
            // scene lane already put here — two warm lights read as one richer fire,
            // and cross-lane edits stay at zero.
            p.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.62, 0.28),
                    intensity: fire.light,
                    range: 12.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.42, 0.0),
                Visibility::default(),
                FireFlicker { base: fire.light },
            ));
        });
    }
}

fn campfire_pulse(
    mut commands: Commands,
    time: Res<Time>,
    mut rng: ResMut<VfxRng>,
    assets: Option<Res<VfxAssets>>,
    mut fires: Query<(&CampfireVfx, &mut CampfireBuilt, &GlobalTransform)>,
    mut lights: Query<(&mut PointLight, &FireFlicker)>,
) {
    let Some(assets) = assets else { return };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    // Flicker: three detuned sines instead of random noise. Random per-frame
    // intensity strobes badly on video; layered sines wander the way a real flame
    // does and stay smooth at any frame rate.
    for (mut light, flicker) in &mut lights {
        let f = 1.0
            + 0.085 * (t * 11.0).sin()
            + 0.055 * (t * 6.3 + 1.7).sin()
            + 0.030 * (t * 23.0 + 0.4).sin();
        light.intensity = flicker.base * f;
    }

    for (fire, mut built, gtf) in &mut fires {
        built.accum += dt;
        while built.accum >= EMBER_EVERY {
            built.accum -= EMBER_EVERY;
            let a = rng.unit() * std::f32::consts::TAU;
            let r = fire.radius * rng.range(0.0, 0.7);
            let base = gtf.translation() + Vec3::new(a.cos() * r, 0.20, a.sin() * r);
            let born = Vec3::splat(rng.range(0.022, 0.05));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.ember.clone()),
                Visibility::default(),
                Transform::from_translation(base).with_scale(born),
                Particle {
                    // Embers rise, wander sideways, and fade — negative gravity plus
                    // heavy drag gives the slow lift; the sideways component is what
                    // stops them looking like a vertical particle column.
                    vel: Vec3::new(rng.signed() * 0.45, rng.range(0.9, 1.8), rng.signed() * 0.45),
                    gravity: -1.15,
                    drag: 1.35,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 3.0,
                    age: 0.0,
                    ttl: rng.range(EMBER_TTL.0, EMBER_TTL.1),
                    born,
                    hold: 0.25,
                    delay: 0.0,
                },
            ));

            // Smoke: a cool grey puff every few embers, so the fire reads as
            // breathing rather than a shower of sparks alone.
            if rng.unit() < SMOKE_CHANCE {
                let born = Vec3::splat(rng.range(0.12, 0.30));
                commands.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.smoke.clone()),
                    Visibility::default(),
                    Transform::from_translation(base + Vec3::Y * 0.12).with_scale(born),
                    Particle {
                        vel: Vec3::new(rng.signed() * 0.5, rng.range(0.8, 1.5), rng.signed() * 0.5),
                        gravity: -0.6,
                        drag: 1.6,
                        spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 0.6,
                        age: 0.0,
                        ttl: rng.range(1.4, 2.4),
                        born,
                        hold: 0.25,
                        delay: 0.0,
                    },
                ));
            }

            // Spark pop: an occasional bright chip that jumps clear of the coals —
            // the crackle that keeps a campfire from reading as a candle.
            if rng.unit() < SPARK_CHANCE {
                let tone = assets.spark[(rng.unit() * 3.0) as usize % 3].clone();
                let born = Vec3::splat(rng.range(0.02, 0.045));
                commands.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(tone),
                    Visibility::default(),
                    Transform::from_translation(base).with_scale(born),
                    Particle {
                        vel: Vec3::new(rng.signed() * 1.8, rng.range(2.0, 3.6), rng.signed() * 1.8),
                        gravity: 4.0,
                        drag: 1.8,
                        spin: Vec3::ZERO,
                        age: 0.0,
                        ttl: rng.range(0.25, 0.5),
                        born,
                        hold: 0.1,
                        delay: 0.0,
                    },
                ));
            }
        }
    }
}

fn tick_coals(
    time: Res<Time>,
    assets: Option<Res<VfxAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    coals: Query<&Coal>,
) {
    let Some(assets) = assets else { return };
    // All coals share one material, so breathe it as a whole against the *average*
    // phase. Per-coal materials would be prettier but would cost a material per coal
    // per campfire; the flicker light already breaks up the bed's uniformity.
    let Some(mut mat) = materials.get_mut(&assets.coal) else { return };
    let n = coals.iter().count().max(1) as f32;
    let mut phase = 0.0;
    for c in &coals {
        phase += c.phase;
    }
    let t = time.elapsed_secs() * 2.4 + phase / n;
    let g = 0.78 + 0.22 * t.sin();
    mat.emissive = LinearRgba::rgb(4.2 * g, 1.1 * g, 0.15 * g);
}

// ===========================================================================
// Ambient dust — the scene breathes even when nothing is fighting
// ===========================================================================

/// Seconds between ambient dust motes per camera, and how long one drifts.
/// Tuned so dust is a faint texture in the air, never a cloud that draws the eye.
const AMBIENT_EVERY: f32 = 0.14;
const AMBIENT_TTL: (f32, f32) = (2.2, 4.0);

/// One ambient-dust emitter per active camera. Spawned lazily by [`ambient_dust`]
/// the first frame it sees a `Camera3d` without one; the mote field lives on the
/// camera so dust always hangs where the player is looking.
#[derive(Component)]
struct AmbientEmitter {
    accum: f32,
}

fn ambient_dust(
    mut commands: Commands,
    time: Res<Time>,
    mut rng: ResMut<VfxRng>,
    assets: Option<Res<VfxAssets>>,
    mut cams: Query<(&GlobalTransform, &mut AmbientEmitter)>,
    cam_needs: Query<Entity, (With<Camera3d>, Without<AmbientEmitter>)>,
) {
    let Some(assets) = assets else { return };

    // Lazy-attach one emitter per camera — dust hangs where the player looks, so it
    // rides the camera rather than being a fixed cloud anchored in the world.
    for e in &cam_needs {
        commands.entity(e).insert(AmbientEmitter { accum: 0.0 });
    }

    let dt = time.delta_secs();
    for (gtf, mut emitter) in &mut cams {
        emitter.accum += dt;
        while emitter.accum >= AMBIENT_EVERY {
            emitter.accum -= AMBIENT_EVERY;

            // A shell in front of the camera, never behind it — a mote behind the
            // lens is a wasted draw on a frame the player can't see.
            let cam = gtf.translation();
            let p = cam
                + *gtf.forward() * rng.range(1.0, 6.0)
                + *gtf.right() * rng.signed() * 2.5
                + *gtf.up() * rng.range(-0.5, 2.0);
            let born = Vec3::splat(rng.range(0.02, 0.05));
            commands.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.air_dust.clone()),
                Visibility::default(),
                Transform::from_translation(p).with_scale(born),
                Particle {
                    // Dust hangs and wanders; it does not travel.
                    vel: Vec3::new(
                        rng.signed() * 0.12,
                        rng.range(-0.05, 0.15),
                        rng.signed() * 0.12,
                    ),
                    gravity: 0.0,
                    drag: 0.15,
                    spin: Vec3::new(rng.signed(), rng.signed(), rng.signed()) * 0.5,
                    age: 0.0,
                    ttl: rng.range(AMBIENT_TTL.0, AMBIENT_TTL.1),
                    born,
                    hold: 0.85,
                    delay: 0.0,
                },
            ));
        }
    }
}

// ===========================================================================
// Integration / lifetime
// ===========================================================================

fn tick_hitstop(time: Res<Time>, mut hitstop: ResMut<Hitstop>) {
    if hitstop.left > 0.0 {
        hitstop.left = (hitstop.left - time.delta_secs()).max(0.0);
    }
}

fn tick_particles(
    mut commands: Commands,
    time: Res<Time>,
    hitstop: Res<Hitstop>,
    mut q: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    // §5.2: the hit-stop freezes the *affected actors* — for this module that is the
    // debris. `Time` itself is untouched, so camera, audio and gameplay keep running,
    // which is what the design doc explicitly asks for.
    let frozen = hitstop.frozen();

    if frozen {
        // A true freeze: age does not advance either. Advancing it would quietly eat
        // 80–150 ms off every particle's life, so the debris from the hit you just
        // froze on would die early — the opposite of the beat hit-stop exists to sell.
        return;
    }

    for (e, mut p, mut tf) in &mut q {
        p.age += dt;
        if p.age >= p.ttl {
            commands.entity(e).despawn();
            continue;
        }
        if p.age < p.delay {
            continue;
        }

        p.vel.y -= p.gravity * dt;
        // Read the drag factor out first: `p.vel *= ..p.drag..` would borrow `*p`
        // mutably for the assignment while still reading it on the right-hand side.
        let damp = 1.0 - (p.drag * dt).min(1.0);
        p.vel *= damp;
        tf.translation += p.vel * dt;

        if p.spin != Vec3::ZERO {
            tf.rotation *= Quat::from_euler(
                EulerRot::XYZ,
                p.spin.x * dt,
                p.spin.y * dt,
                p.spin.z * dt,
            );
        }

        // Shrink toward nothing after the hold window. Scaling to zero is the voxel
        // way to "fade" — no per-particle alpha material, no sorting cost, and it
        // literally is the story-bible's "losing cohesion". Driven from the birth
        // scale (not the current one) so the curve can't drift over a long life.
        //
        // The clock starts at `delay`, not at spawn: a staggered dissolve block would
        // otherwise begin shrinking while it was still sitting perfectly still, and
        // the feet would fade before they ever moved.
        let t = (p.age - p.delay).max(0.0) / (p.ttl - p.delay).max(1e-4);
        tf.scale = if t > p.hold {
            let k = ((1.0 - t) / (1.0 - p.hold)).clamp(0.0, 1.0);
            p.born * k
        } else {
            p.born
        };
    }
}

fn tick_ephemeral(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        Entity,
        &mut Ephemeral,
        &mut Transform,
        Option<&mut PointLight>,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
) {
    let dt = time.delta_secs();
    for (e, mut eph, mut tf, light, mat) in &mut q {
        eph.age += dt;
        if eph.age >= eph.ttl {
            commands.entity(e).despawn();
            continue;
        }
        let t = (eph.age / eph.ttl).clamp(0.0, 1.0);
        let fall = (1.0 - t) * (1.0 - t);
        tf.scale = eph.born_scale * (1.0 + (eph.grow_to - 1.0) * t);
        if let Some(mut l) = light {
            // Quadratic falloff — a linear fade on a light reads as a lamp being
            // dimmed; quadratic reads as a flash.
            l.intensity = eph.light0 * fall;
        }
        // Flash shells own a private material (one per hit), so fading it here is
        // safe and is what stops the shell popping out of existence at end of life.
        if let Some(handle) = mat {
            if let Some(mut m) = materials.get_mut(&handle.0) {
                let (b, e0) = (eph.base0, eph.emissive0);
                m.base_color = Color::srgba(b[0], b[1], b[2], b[3] * fall);
                m.emissive = LinearRgba::rgb(e0[0] * fall, e0[1] * fall, e0[2] * fall);
            }
        }
    }
}

fn apply_cam_kick(
    time: Res<Time>,
    hitstop: Res<Hitstop>,
    mut kick: ResMut<CamKick>,
    mut cams: Query<&mut Transform, With<VfxCamera>>,
) {
    let Ok(mut tf) = cams.single_mut() else { return };

    // Take back last frame's offset FIRST. A kick is a delta on top of whatever the
    // camera rig decided, so it has to be removed before the next one goes on —
    // `translation +=` / `rotation *=` alone are cumulative, and the camera keeps
    // every push it ever got. The first render of the showcase caught exactly that:
    // three hits left the horizon rolled ~10° and it never came back.
    tf.translation -= kick.applied_pos;
    tf.rotation *= Quat::from_rotation_z(-kick.applied_roll);
    kick.applied_pos = Vec3::ZERO;
    kick.applied_roll = 0.0;

    if kick.amp <= 0.0 || kick.dur <= 0.0 {
        return;
    }
    // Hold the kick's own clock through hit-stop, for the same reason
    // `tick_particles` holds particle `age` there: advancing it while frozen
    // burns most of the punch before the freeze ever releases, so by the time
    // gameplay unpauses the snap the player is supposed to feel has already
    // half-decayed. The offset itself keeps being applied every frame either
    // way — only the decay clock pauses.
    if !hitstop.frozen() {
        kick.age += time.delta_secs();
    }
    if kick.age >= kick.dur {
        kick.amp = 0.0;
        return;
    }
    // Snap out, ease back: the punch is front-loaded (1-t)² so the first frame after
    // impact carries most of the displacement, which is what sells weight. A
    // symmetric curve feels like the camera is bouncing on a spring instead.
    let t = kick.age / kick.dur;
    let f = (1.0 - t) * (1.0 - t);
    let offset = kick.dir * kick.amp * f;
    let roll = kick.roll * f;
    tf.translation += offset;
    tf.rotation *= Quat::from_rotation_z(roll);
    kick.applied_pos = offset;
    kick.applied_roll = roll;
}

// ===========================================================================
// Showcase stage — the scene the before/after beauty shots are rendered from
// ===========================================================================

/// Which beat the showcase renders.
/// Set with `VOXELFORGE_VFX=off|impact|dissolve|fire|parry|stagger`.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VfxShot {
    /// Same stage, same camera, VFX layer loaded but nothing fired — the honest
    /// "before" plate for the comparison.
    Off,
    Impact,
    Dissolve,
    Fire,
    /// A successful parry — the ring flash beat.
    Parry,
    /// The husk taking a stagger break: it reels back and the ground kicks up dust.
    Stagger,
    /// The weapon ribbon alone: the blade sweeps with the trail hot and nothing
    /// else fires. Isolates the swing-trail for a before/after that a debris-laden
    /// impact frame would drown out.
    Trail,
}

impl VfxShot {
    pub fn from_env() -> Option<Self> {
        match std::env::var("VOXELFORGE_VFX").ok()?.trim().to_ascii_lowercase().as_str() {
            "off" | "before" => Some(VfxShot::Off),
            "impact" | "hit" => Some(VfxShot::Impact),
            "dissolve" | "death" | "unravel" => Some(VfxShot::Dissolve),
            "fire" | "campfire" => Some(VfxShot::Fire),
            "parry" | "riposte" => Some(VfxShot::Parry),
            "stagger" | "stun" | "dust" => Some(VfxShot::Stagger),
            "trail" | "ribbon" => Some(VfxShot::Trail),
            _ => None,
        }
    }
}

/// Suppresses the effect layer while leaving the beat's staging untouched.
/// Set with `VOXELFORGE_VFX_MUTE=1`.
///
/// This is what makes a before/after pair honest. The old way of shooting a
/// "before" was `VOXELFORGE_VFX=off`, but `off` is its own branch: it does not run
/// the chosen beat's staging, so the stagger pair came out with a reeled husk in
/// the after plate and an upright one in the before plate. The pair then differed
/// in POSE as well as in VFX and proved nothing.
///
/// With mute, both plates render the SAME beat through the SAME code path at the
/// SAME timestamps — the husk reels in both, the blade sweeps in both — and the one
/// difference is that the emitters never fire. Whatever the eye picks up between
/// the two images is therefore the effect layer and nothing else.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct VfxMute(pub bool);

impl VfxMute {
    pub fn from_env() -> Self {
        Self(matches!(
            std::env::var("VOXELFORGE_VFX_MUTE").ok().as_deref().map(str::trim),
            Some("1") | Some("on") | Some("true") | Some("yes")
        ))
    }
}

/// Read one showcase look knob from the environment.
///
/// Only the showcase stage uses these — the effects themselves are hard-tuned to
/// `combat-design.md`. Exposure/key/fill live behind env vars because grading a
/// stage takes a dozen renders and a Bevy rebuild is minutes; the defaults here ARE
/// the graded values, so `bash scripts/render_vfx.sh` with no env reproduces the
/// shipped PNGs exactly.
fn env_f32(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

/// A comma-separated triple, as raw numbers.
fn env_rgb_raw(key: &str, default: [f32; 3]) -> [f32; 3] {
    std::env::var(key)
        .ok()
        .and_then(|v| {
            let p: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            (p.len() == 3).then(|| [p[0], p[1], p[2]])
        })
        .unwrap_or(default)
}

/// Same, read as an sRGB colour.
fn env_rgb(key: &str, default: [f32; 3]) -> Color {
    let [r, g, b] = env_rgb_raw(key, default);
    Color::srgb(r, g, b)
}

/// Showcase swing arc: when the blade travels, and through what angle. Timed so it
/// is still mid-arc at the 3.2 s screenshot — a finished swing has no ribbon left.
const SWING_FROM: f32 = 2.80;
const SWING_TO: f32 = 3.34;
const SWING_START: f32 = 1.15;
const SWING_END: f32 = -0.60;

/// Marks the swinging arm so the timeline can turn its trail on and off.
#[derive(Component)]
pub struct ShowcaseBlade;

/// Root of the showcase husk, so the stagger beat can reel the whole body.
#[derive(Component)]
pub struct ShowcaseHusk;

/// When the stagger beat lands and how long the body takes to reel back.
const STAGGER_AT: f32 = 2.94;
const STAGGER_REEL: f32 = 0.42;
/// When the parry ring fires. Late on purpose: the ring lives ~160 ms by design
/// (a parry read has to be a *flash*), so it must be caught close to its peak.
const PARRY_AT: f32 = 3.13;

/// Drives the shot so the chosen beat peaks at the 3.2 s screenshot.
#[derive(Resource, Default)]
pub struct ShowcaseTimeline {
    fired: u32,
}

/// Positions the showcase stage. Kept here (not in `hero.rs`) on purpose: `hero.rs`
/// is the LOCKED golden kitchen beauty shot and re-framing it is forbidden, so VFX
/// get their own stage rather than being dropped into a kitchen they'd make no
/// sense in.
pub fn setup_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Same module paths hero.rs uses — the post stack below is lifted from it, so
    // the two shots are graded through the same pipeline (see the TAA note).
    use bevy::camera::{ClearColorConfig, Exposure, PerspectiveProjection, Projection};
    use bevy::core_pipeline::tonemapping::Tonemapping;
    use bevy::light::{AmbientLight, ShadowFilteringMethod};
    use bevy::pbr::{
        DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion,
        ScreenSpaceAmbientOcclusionQualityLevel,
    };
    use bevy::post_process::bloom::Bloom;
    use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
    use bevy::render::view::Msaa;
    use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let put = |c: &mut Commands,
               mesh: &Handle<Mesh>,
               mat: &Handle<StandardMaterial>,
               pos: Vec3,
               scale: Vec3| {
        c.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Visibility::default(),
            Transform::from_translation(pos).with_scale(scale),
        ));
    };

    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.33, 0.24),
        perceptual_roughness: 0.95,
        ..default()
    });
    let road = materials.add(StandardMaterial {
        base_color: Color::srgb(0.44, 0.43, 0.41),
        perceptual_roughness: 0.9,
        ..default()
    });
    let wall = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.35, 0.31),
        perceptual_roughness: 0.92,
        ..default()
    });
    let husk_armor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.34, 0.40),
        perceptual_roughness: 0.55,
        metallic: 0.3,
        ..default()
    });
    let hero_cloth = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.34, 0.16),
        perceptual_roughness: 0.75,
        ..default()
    });
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.74, 0.78),
        perceptual_roughness: 0.28,
        metallic: 0.85,
        ..default()
    });
    let log_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.18, 0.10),
        perceptual_roughness: 0.9,
        ..default()
    });

    // ---- ground + a strip of the stone road (the village's leading line) ----
    put(&mut commands, &cube, &ground, Vec3::new(0.0, -0.5, 0.0), Vec3::new(46.0, 1.0, 46.0));
    for i in -12..14 {
        put(
            &mut commands,
            &cube,
            &road,
            Vec3::new(0.0, 0.02, i as f32),
            Vec3::new(5.0, 0.06, 1.0),
        );
    }

    // ---- ruined wall behind the fight: gives shadows something to fall on,
    //      and gives the frame a dark value to read sparks against ----
    for i in 0..9 {
        let h = 3 - (i % 3);
        for y in 0..h {
            put(
                &mut commands,
                &cube,
                &wall,
                Vec3::new(-5.5 + i as f32, y as f32 + 0.5, -5.0),
                Vec3::splat(0.98),
            );
        }
    }

    // ---- campfire (④) — logs + the VFX marker that grows coals/embers/flicker ----
    for i in 0..3 {
        let a = i as f32 * std::f32::consts::PI / 3.0;
        commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(log_mat.clone()),
            Visibility::default(),
            Transform::from_xyz(4.6, 0.16, 2.6)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, a))
                .with_scale(Vec3::new(1.05, 0.18, 0.18)),
        ));
    }
    commands.spawn((
        Transform::from_xyz(4.6, 0.06, 2.6),
        Visibility::default(),
        CampfireVfx::default(),
    ));

    // ---- the husk taking the hit ----
    // Built under ONE root so the stagger beat can reel the whole body back around
    // its feet. The root also means the reel is applied identically in the before and
    // after plates (`showcase_timeline` drives it in every mode that uses it), so the
    // pair still differs by VFX only.
    let husk_feet = Vec3::new(0.0, 0.0, -1.2);
    commands
        .spawn((
            Transform::from_translation(husk_feet),
            Visibility::default(),
            ShowcaseHusk,
        ))
        .with_children(|p| {
            for (dy, sx, sy, sz) in [
                (0.45f32, 0.75f32, 0.9f32, 0.55f32), // legs
                (1.35, 0.95, 0.9, 0.62),             // torso
                (2.05, 0.62, 0.5, 0.55),             // head
            ] {
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(husk_armor.clone()),
                    Visibility::default(),
                    Transform::from_translation(Vec3::Y * dy)
                        .with_scale(Vec3::new(sx, sy, sz)),
                ));
            }
        });

    // ---- the player mid-swing, blade carrying the trail emitter (②) ----
    let hero_feet = Vec3::new(-1.9, 0.0, 0.6);
    for (dy, sx, sy, sz) in [(0.45f32, 0.7f32, 0.9f32, 0.5f32), (1.3, 0.85, 0.85, 0.55), (1.95, 0.55, 0.45, 0.5)] {
        put(&mut commands, &cube, &hero_cloth, hero_feet + Vec3::Y * dy, Vec3::new(sx, sy, sz));
    }
    // The blade is posed at the end of its arc, right at the husk's chest. The trail
    // system reads its GlobalTransform, so in-game it follows the real animation.
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(steel.clone()),
        Visibility::default(),
        Transform::from_translation(hero_feet + Vec3::new(0.95, 1.45, -0.65))
            .with_rotation(Quat::from_rotation_z(-0.9) * Quat::from_rotation_y(0.5))
            .with_scale(Vec3::new(0.10, 1.5, 0.16)),
        ShowcaseBlade,
        // The ribbon traces the OUTER band of the blade (0.84 m of a 1.5 m blade),
        // not its whole length. Sweeping the full length draws a wedge that reaches
        // all the way back to the shoulder pivot — geometrically true, but it reads
        // as a paper fan rather than a weapon trail.
        SwingTrail { hot: false, half: Vec3::new(0.06, 0.42, 0.09), accum: 0.0 },
    ));

    // ---- key light: 18° above the horizon, per look-bible §2 (15–25°) ----
    // The game's own sun currently sits at ~59°, which is why game frames don't look
    // like the bible; this stage shows what the VFX read like under the intended key.
    let elev: f32 = 18f32.to_radians();
    let azim: f32 = 128f32.to_radians();
    let sun_dir = Vec3::new(elev.cos() * azim.cos(), elev.sin(), elev.cos() * azim.sin());
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.84, 0.64),
            illuminance: env_f32("VOXELFORGE_VFX_SUN", 11_000.0),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(sun_dir * 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // ---- camera: hero.rs's post stack, minus TAA -------------------------
    // TAA is dropped ON PURPOSE here. It accumulates several frames to resolve PCSS
    // and SSAO noise, which is right for a still kitchen — but particles move metres
    // per frame and newly spawned ones have no motion-vector history, so TAA smears
    // them into ghost trails and the shot would be grading the anti-aliaser, not the
    // VFX. Shadows use the Gaussian filter instead, which needs no history.
    let grade = env_rgb_raw("VOXELFORGE_VFX_GRADE", [0.0, 1.06, 1.06]);
    // Framing knobs, same reason as the grade knobs: `VOXELFORGE_VFX_EYE` /
    // `_AIM` = x,y,z. Defaults ARE the shipped framing.
    // Shot from the SIDE of the fight, not down its axis. The first framing put the
    // camera on the hero→husk line, so the hero stood squarely in front of the thing
    // he was hitting and every spark landed behind him.
    let eye = Vec3::from(env_rgb_raw("VOXELFORGE_VFX_EYE", [-7.9, 3.5, 1.7]));
    let target = Vec3::from(env_rgb_raw("VOXELFORGE_VFX_AIM", [0.15, 1.45, -0.95]));
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(env_rgb(
                "VOXELFORGE_VFX_SKY",
                [0.44, 0.52, 0.64],
            )),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection { fov: 0.72, near: 0.05, ..default() }),
        Transform::from_translation(eye).looking_at(target, Vec3::Y),
        Msaa::Off, // SSAO requires MSAA off
        AmbientLight {
            color: Color::srgb(0.58, 0.62, 0.76),
            brightness: env_f32("VOXELFORGE_VFX_AMB", 2600.0),
            affects_lightmapped_meshes: false,
        },
        Exposure { ev100: env_f32("VOXELFORGE_VFX_EV", 10.3) },
        Tonemapping::AcesFitted,
        // Grade knobs: `VOXELFORGE_VFX_GRADE=temperature,post_saturation,contrast`.
        // The first render of this stage came back FIRE-RED — the same trap hero.rs
        // documents ("under this honey the frame collapses to FIRE-RED"): a warm
        // temperature on top of AcesFitted collapses a warm-lit frame to one hue.
        // The kitchen can carry it (it IS a warm room); a dusk fight can't, and the
        // sparks stop reading as hot if the whole frame is already hot. So this
        // stage grades NEUTRAL and lets the emissives be the only warm pixels.
        ColorGrading {
            global: ColorGradingGlobal {
                temperature: grade[0],
                post_saturation: grade[1],
                ..default()
            },
            // Shadows stay neutral for the same reason hero.rs holds them there:
            // contrast on shadows crushes the open shade to pure black, and debris
            // that lands in shade would simply vanish.
            shadows: ColorGradingSection { contrast: 1.0, ..default() },
            midtones: ColorGradingSection { contrast: grade[2], ..default() },
            highlights: ColorGradingSection { contrast: 1.0, ..default() },
        },
        ShadowFilteringMethod::Gaussian,
        Bloom {
            // Higher than the kitchen's 0.26: sparks and embers ARE the subject here,
            // and their glow is what separates a hit from a puff of grey cubes.
            intensity: 0.34,
            ..Bloom::NATURAL
        },
        DepthOfField {
            mode: DepthOfFieldMode::Bokeh,
            // Focus the husk, not the hero: the hit is the subject. Aperture opened
            // from f/2.6 to f/4.0 — at 2.6 the ruined wall dissolved into mush and
            // the sparks had nothing crisp to read against.
            focal_distance: env_f32("VOXELFORGE_VFX_FOCUS", eye.distance(target)),
            aperture_f_stops: env_f32("VOXELFORGE_VFX_FSTOP", 4.0),
            sensor_height: 0.35,
            ..default()
        },
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
            constant_object_thickness: 1.45,
        },
        DistanceFog {
            // Cool, so distance recedes away from the warm key and the sparks/embers
            // are the only hot pixels in the frame.
            color: Color::srgb(0.24, 0.26, 0.33),
            falloff: FogFalloff::Exponential {
                density: env_f32("VOXELFORGE_VFX_FOG", 0.022),
            },
            ..default()
        },
        VfxCamera,
    ));
}

/// Fires the chosen beat on a fixed clock so the 3.2 s screenshot lands mid-effect.
/// Deterministic: same seed + same timings ⇒ the same PNG on a re-render, which is
/// what makes the before/after pair honest (only the VFX differ).
pub fn showcase_timeline(
    time: Res<Time>,
    shot: Res<VfxShot>,
    mute: Res<VfxMute>,
    mut tl: ResMut<ShowcaseTimeline>,
    mut impacts: bevy::ecs::message::MessageWriter<Impact>,
    mut unravels: bevy::ecs::message::MessageWriter<Unravel>,
    mut blades: Query<(&mut SwingTrail, &mut Transform), With<ShowcaseBlade>>,
    mut husks: Query<&mut Transform, (With<ShowcaseHusk>, Without<ShowcaseBlade>)>,
) {
    let t = time.elapsed_secs();
    let chest = Vec3::new(0.0, 1.35, -1.2);

    // ---- the husk reels, in the stagger beat only -----------------------
    // Pose, NOT VFX — so it runs before the mute gate below and therefore lands in
    // the before plate and the after plate alike. That ordering is the whole point:
    // a reel that only happened in the after plate would make the pair differ in
    // body pose, and the reader could no longer tell which difference is the effect.
    if *shot == VfxShot::Stagger {
        let k = ((t - STAGGER_AT) / STAGGER_REEL).clamp(0.0, 1.0);
        // Snap back, settle: the body is thrown, it does not ease into it.
        let punch = (k * std::f32::consts::PI).sin() * (1.0 - k * 0.35);
        for mut tf in &mut husks {
            tf.rotation = Quat::from_rotation_x(-0.40 * punch);
            tf.translation = Vec3::new(0.0, 0.0, -1.2) + Vec3::new(0.08, 0.0, -0.26) * punch;
        }
    }

    // ---- the swing itself, run for EVERY mode ---------------------------
    // The blade actually travels; the trail system only ever reads its real
    // transform, so a static blade produced a ribbon stacked on one spot (that is
    // what the first render showed). Sweeping it here proves the ribbon follows
    // real animation — which is exactly what it will do in game.
    //
    // Driven in all four modes on purpose: the "before" plate has to differ from the
    // "after" plate by VFX ONLY. If the blade moved only in the impact shot, the
    // pair would also differ in geometry and the comparison would be worthless.
    let shoulder = Vec3::new(-1.55, 1.52, 0.50);
    let sweep = ((t - SWING_FROM) / (SWING_TO - SWING_FROM)).clamp(0.0, 1.0);
    let a = SWING_START + (SWING_END - SWING_START) * sweep;
    let arm = Quat::from_rotation_y(a);
    for (_, mut tf) in &mut blades {
        tf.translation = shoulder + arm * Vec3::new(1.02, -0.06, -0.72);
        tf.rotation = arm * Quat::from_rotation_z(-0.92);
    }

    // ---- everything past here IS the effect layer -----------------------
    // The mute plate stops exactly here: same stage, same swing, same reel, same
    // timestamps — no emitters. Held down rather than branched to a different beat
    // so the two plates cannot drift apart as the beats get retuned.
    if mute.0 {
        for (mut b, _) in &mut blades {
            b.hot = false;
        }
        return;
    }

    // The ribbon is hot only while the blade is actually travelling — the combat
    // lane's one line is exactly this predicate. Hoisted out of the impact arm so
    // every beat that swings a blade gets the trail: the parry and stagger plates
    // show a real weapon arc for the same reason the impact one does. `Off` is left
    // out on purpose — it is the legacy bare "before" — and `Fire` has no swing.
    if matches!(*shot, VfxShot::Impact | VfxShot::Parry | VfxShot::Stagger | VfxShot::Trail) {
        let hot = (SWING_FROM..SWING_TO).contains(&t);
        for (mut b, _) in &mut blades {
            b.hot = hot;
        }
    }

    match *shot {
        VfxShot::Off | VfxShot::Fire | VfxShot::Trail => {}
        VfxShot::Impact => {
            // Three staggered hits (light, light, heavy) so the frame shows debris at
            // three different ages — one lone burst reads as a single freeze-frame.
            // The first two land early enough that their debris is well clear of the
            // body by the 3.2 s grab; the third lands late so the shot still catches
            // a live hit flash (⑥) on the husk.
            for (i, at) in [2.40f32, 2.72, 3.11].iter().enumerate() {
                if t >= *at && tl.fired <= i as u32 {
                    tl.fired = i as u32 + 1;
                    impacts.write(Impact {
                        pos: chest + Vec3::new(0.0, i as f32 * 0.06, 0.0),
                        dir: Vec3::new(0.35, 0.0, -0.94).normalize(),
                        power: if i == 2 { 1.9 } else { 1.0 },
                        flavor: HitFlavor::Husk,
                        body_half: Some(Vec3::new(0.48, 1.15, 0.32)),
                    });
                }
            }
        }
        VfxShot::Dissolve => {
            // One hit that kills, then the body lets go 0.18 s later — the beat order
            // a player actually sees.
            if t >= 2.60 && tl.fired == 0 {
                tl.fired = 1;
                impacts.write(Impact {
                    pos: chest,
                    dir: Vec3::new(0.35, 0.0, -0.94).normalize(),
                    power: 2.2,
                    flavor: HitFlavor::Husk,
                    body_half: Some(Vec3::new(0.48, 1.15, 0.32)),
                });
            }
            if t >= 2.78 && tl.fired == 1 {
                tl.fired = 2;
                unravels.write(Unravel {
                    pos: Vec3::new(0.0, 1.25, -1.2),
                    half: Vec3::new(0.48, 1.25, 0.32),
                    tint: Color::srgb(0.32, 0.34, 0.40),
                });
            }
        }
        VfxShot::Parry => {
            // The parry lands where the two blades meet — between the bodies at
            // guard height, not on the husk's chest. Nothing broke, so no wrap flash.
            if t >= PARRY_AT && tl.fired == 0 {
                tl.fired = 1;
                impacts.write(Impact {
                    pos: Vec3::new(-0.72, 1.52, -0.62),
                    dir: Vec3::new(-0.42, 0.05, 0.90).normalize(),
                    power: 1.3,
                    flavor: HitFlavor::Parry,
                    body_half: None,
                });
            }
        }
        VfxShot::Stagger => {
            // A stagger break is a heavy hit that connects and then the guard
            // collapses. The hit is fired in both plates; only the ground reaction
            // is new, which is exactly what this pair is meant to show.
            if t >= STAGGER_AT && tl.fired == 0 {
                tl.fired = 1;
                impacts.write(Impact {
                    pos: chest,
                    dir: Vec3::new(0.35, 0.0, -0.94).normalize(),
                    power: 2.4,
                    flavor: HitFlavor::Husk,
                    body_half: Some(Vec3::new(0.48, 1.15, 0.32)),
                });
            }
        }
    }
}
