//! New enemy silhouettes (Monanisa) — the "hand-tooled monster" answer to the
//! CEO brief: the only enemy in the live game (`combat::spawn_guard_husk`) is
//! three flat grey cuboids with no eyes, no claws, no asymmetry. This module
//! designs three replacements that read as threats at a glance: broken
//! silhouettes (overlong arm, hunched spine, elongated skull), dark matte
//! armour/hide that eats light instead of a lit-panel grey, and glowing eyes
//! as the one thing that reads in the dark.
//!
//! Bevy-only, no `crate::` reference anywhere — same rule `characters.rs`
//! documents and for the same reason: it lets `shot_main.rs` `#[path]`-include
//! this file to render/prove the designs without a `combat.rs` edit (Rose's
//! lane) or a `characters.rs` edit (Flamingo's lane). Wiring an `EnemyKind`
//! into the real spawn path is `combat.rs`'s call — this file only proposes
//! the geometry and the swap points (see `docs/enemy-design.md`).
//!
//! Authoring units mirror `characters.rs`: `VX` = ⅛ of a world block, origin =
//! feet centre, +Y up, facing −Z (same convention as the husk and the cast).

use bevy::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::AmbientLight;
use bevy::prelude::*;
use bevy::render::view::Msaa;

pub const VX: f32 = 0.125;

// ---------------------------------------------------------------------------
// Palette — grim, dark, desaturated. Every hex is quoted in docs/enemy-design.md.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    /// `#2E2A26` rotted hide — the base skin tone for the organic pair.
    Hide,
    /// `#1C1916` — crevice/shadow hide, one step darker so a hunch still reads
    /// as two surfaces instead of one flat mass.
    HideDark,
    /// `#C9BFA0` dulled ivory — claws, tusks, spine spikes, exposed bone.
    Bone,
    /// `#1F1A16` — torn rag wraps, near-black so they read as absence, not cloth.
    Rag,
    /// `#23262B` dark charcoal iron — the Sentinel's armour plate.
    Iron,
    /// `#14161A` — soot/damage iron, one step darker than `Iron`.
    IronDark,
    /// `#6E1B14` dried-blood crack — dull emissive, corruption made visible.
    BloodCrack,
    /// `#7CFF6E` sickly toxic green — the Reaver's eyes. Strong emissive; the
    /// one thing this body is allowed to be bright.
    GhoulGlow,
    /// `#8FD6FF` cold dead-iron blue — the Sentinel's eye-slits. Deliberately
    /// NOT amber: Garren (`characters.rs`, Flamingo's NPC husk) already owns
    /// warm amber corruption-cracks, and the Sentinel's own `BloodCrack` sits
    /// close to that hue too. A cold blue glow reads as "dead and watching"
    /// rather than "burning," and keeps three different warm-toned reads
    /// (Garren amber, BloodCrack dried-blood, StalkerGlow hot red) from
    /// collapsing into one indistinct "orange-eyed monster" silhouette.
    SentinelGlow,
    /// `#FF3B1E` low predator red-orange — the Stalker's eyes.
    StalkerGlow,
}

impl Tone {
    fn srgb(self) -> (u8, u8, u8) {
        match self {
            Tone::Hide => (0x2E, 0x2A, 0x26),
            Tone::HideDark => (0x1C, 0x19, 0x16),
            Tone::Bone => (0xC9, 0xBF, 0xA0),
            Tone::Rag => (0x1F, 0x1A, 0x16),
            Tone::Iron => (0x23, 0x26, 0x2B),
            Tone::IronDark => (0x14, 0x16, 0x1A),
            Tone::BloodCrack => (0x6E, 0x1B, 0x14),
            Tone::GhoulGlow => (0x7C, 0xFF, 0x6E),
            Tone::SentinelGlow => (0x8F, 0xD6, 0xFF),
            Tone::StalkerGlow => (0xFF, 0x3B, 0x1E),
        }
    }

    /// Eyes and cracks are the only emissive surfaces — same "emissive is
    /// narrative" rule `characters.rs` uses for Auren's ember and Maren's
    /// lantern. Everything else stays dead matte on purpose (§ dark-vs-env).
    fn emissive(self) -> LinearRgba {
        match self {
            Tone::BloodCrack => LinearRgba::rgb(1.7, 0.36, 0.20),
            Tone::GhoulGlow => LinearRgba::rgb(1.1, 4.6, 0.9),
            Tone::SentinelGlow => LinearRgba::rgb(1.3, 3.4, 4.8),
            Tone::StalkerGlow => LinearRgba::rgb(4.6, 0.7, 0.25),
            _ => LinearRgba::BLACK,
        }
    }

    fn roughness(self) -> f32 {
        match self {
            Tone::Iron | Tone::IronDark => 0.55,
            Tone::Bone => 0.7,
            Tone::GhoulGlow | Tone::SentinelGlow | Tone::StalkerGlow => 0.3,
            _ => 0.92, // hide, rag: dead matte, eats light instead of shining
        }
    }

    fn idx(self) -> usize {
        match self {
            Tone::Hide => 0,
            Tone::HideDark => 1,
            Tone::Bone => 2,
            Tone::Rag => 3,
            Tone::Iron => 4,
            Tone::IronDark => 5,
            Tone::BloodCrack => 6,
            Tone::GhoulGlow => 7,
            Tone::SentinelGlow => 8,
            Tone::StalkerGlow => 9,
        }
    }

    const ALL: [Tone; 10] = [
        Tone::Hide,
        Tone::HideDark,
        Tone::Bone,
        Tone::Rag,
        Tone::Iron,
        Tone::IronDark,
        Tone::BloodCrack,
        Tone::GhoulGlow,
        Tone::SentinelGlow,
        Tone::StalkerGlow,
    ];
}

pub struct Palette {
    mats: [Handle<StandardMaterial>; 10],
}

impl Palette {
    pub fn build(materials: &mut Assets<StandardMaterial>) -> Self {
        let mats = Tone::ALL.map(|t| {
            let (r, g, b) = t.srgb();
            materials.add(StandardMaterial {
                base_color: Color::srgb_u8(r, g, b),
                emissive: t.emissive(),
                perceptual_roughness: t.roughness(),
                metallic: if matches!(t, Tone::Iron | Tone::IronDark) { 0.35 } else { 0.0 },
                ..default()
            })
        });
        Palette { mats }
    }

    fn get(&self, t: Tone) -> Handle<StandardMaterial> {
        self.mats[t.idx()].clone()
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Bx {
    lo: [f32; 3],
    hi: [f32; 3],
    tone: Tone,
}

const fn b(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, tone: Tone) -> Bx {
    Bx { lo: [x0, y0, z0], hi: [x1, y1, z1], tone }
}

impl Bx {
    fn size(&self) -> Vec3 {
        Vec3::new(
            (self.hi[0] - self.lo[0]) * VX,
            (self.hi[1] - self.lo[1]) * VX,
            (self.hi[2] - self.lo[2]) * VX,
        )
    }
    fn centre(&self) -> Vec3 {
        Vec3::new(
            (self.hi[0] + self.lo[0]) * 0.5 * VX,
            (self.hi[1] + self.lo[1]) * 0.5 * VX,
            (self.hi[2] + self.lo[2]) * 0.5 * VX,
        )
    }
}

// ---------------------------------------------------------------------------
// The three designs
// ---------------------------------------------------------------------------

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyBody(pub EnemyKind);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyKind {
    /// Low-tier scavenger. Hunched, one overlong dragging claw-arm, elongated
    /// skull, sickly green eyes. The "many, fast, unsettling" archetype.
    Reaver,
    /// The Guard Husk replacement — same combat role, a genuinely dangerous
    /// silhouette: asymmetric horned pauldron, cracked amber eye-slits, a
    /// jagged cleaver-glaive standing above the helm.
    Sentinel,
    /// Low predator build, spine ridge tapering to a barbed tail, one
    /// oversized sickle claw, close-set red eyes low to the ground.
    Stalker,
}

impl EnemyKind {
    pub fn id(self) -> &'static str {
        match self {
            EnemyKind::Reaver => "reaver",
            EnemyKind::Sentinel => "sentinel",
            EnemyKind::Stalker => "stalker",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            EnemyKind::Reaver => "Ghoul Reaver",
            EnemyKind::Sentinel => "Bone Sentinel",
            EnemyKind::Stalker => "Thornclaw Stalker",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "reaver" | "ghoul" => Some(EnemyKind::Reaver),
            "sentinel" | "bone" | "husk2" => Some(EnemyKind::Sentinel),
            "stalker" | "thornclaw" => Some(EnemyKind::Stalker),
            _ => None,
        }
    }

    pub const ALL: [EnemyKind; 3] = [EnemyKind::Reaver, EnemyKind::Sentinel, EnemyKind::Stalker];

    fn boxes(self) -> &'static [Bx] {
        match self {
            EnemyKind::Reaver => REAVER,
            EnemyKind::Sentinel => SENTINEL,
            EnemyKind::Stalker => STALKER,
        }
    }

    pub fn height(self) -> f32 {
        self.boxes().iter().fold(0.0f32, |m, bx| m.max(bx.hi[1])) * VX
    }

    /// Local-space (unrotated, feet-at-origin) bounding box in world units —
    /// the real geometric extent, not the eyeballed "roughly this tall"
    /// numbers the shot camera used to hardcode. Feeds `placed_bounds` /
    /// `fit_camera` so every solo portrait and the lineup auto-fit to
    /// whatever the model actually measures, arms/glaive/tail included,
    /// instead of a magic constant that goes stale the moment the sculpt
    /// changes by half a block.
    fn bounds(self) -> (Vec3, Vec3) {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for bx in self.boxes() {
            let c = bx.centre();
            let s = bx.size() * 0.5;
            lo = lo.min(c - s);
            hi = hi.max(c + s);
        }
        (lo, hi)
    }
}

/// World-space AABB of one placed (translated + Y-yawed) enemy — rotates all
/// 8 corners of its local bounds rather than the box itself, so a yawed model
/// still reports its true swept extent instead of the unrotated footprint.
fn placed_bounds(kind: EnemyKind, feet: Vec3, yaw: f32) -> (Vec3, Vec3) {
    let (lo, hi) = kind.bounds();
    let corners = [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ];
    let rot = Quat::from_axis_angle(Vec3::Y, yaw);
    let mut wlo = Vec3::splat(f32::INFINITY);
    let mut whi = Vec3::splat(f32::NEG_INFINITY);
    for c in corners {
        let w = rot * c + feet;
        wlo = wlo.min(w);
        whi = whi.max(w);
    }
    (wlo, whi)
}

/// Merge two AABBs (component-wise min/max) — used to grow a single-enemy
/// box into the lineup's combined box across all three placements.
fn merge_bounds(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> (Vec3, Vec3) {
    (a.0.min(b.0), a.1.max(b.1))
}

/// Auto-fit camera: back the eye off along `angle_deg` (measured from the
/// world −Z axis, same convention the old hardcoded `eye.z < 0` used) until
/// the AABB's real half-height AND real half-width both clear the frame with
/// `headroom` to spare on every side, then aim it at the box's true centre —
/// not `height * 0.5`, which silently assumes the model is vertically
/// symmetric around its own midpoint the way none of these three are (the
/// Sentinel's glaive and the Reaver's dragging arm both throw that off).
/// Whichever axis needs more distance wins, so the other axis ends up with
/// even MORE headroom than requested rather than clipping.
fn fit_camera(lo: Vec3, hi: Vec3, aspect: f32, fov_deg: f32, angle_deg: f32, headroom: f32) -> (Vec3, Vec3) {
    let center = (lo + hi) * 0.5;
    let half_h = ((hi.y - lo.y) * 0.5).max(0.05);
    let half_x = (hi.x - lo.x) * 0.5;
    let half_z = (hi.z - lo.z) * 0.5;
    // Diagonal radius, not just the wider of the two: a yawed box can present
    // any horizontal cross-section between half_x and half_z depending on
    // camera angle, so sizing off the smaller one risks a side clip.
    let half_horiz = (half_x * half_x + half_z * half_z).sqrt().max(0.05);

    let vfov = fov_deg.to_radians();
    let hfov = 2.0 * ((vfov * 0.5).tan() * aspect).atan();
    let margin = 1.0 + headroom;
    let dist_v = (half_h * margin) / (vfov * 0.5).tan();
    let dist_h = (half_horiz * margin) / (hfov * 0.5).tan();
    let dist = dist_v.max(dist_h).max(0.5);

    let angle = angle_deg.to_radians();
    // Eye sits a touch above centre for a slight downward look — the same
    // "camera looks a bit down at its subject" read the old hardcoded
    // eye/target split had, just derived from the box instead of guessed.
    let eye = center + Vec3::new(dist * angle.sin(), half_h * 0.18, -dist * angle.cos());
    (eye, center)
}

/// `VOXELFORGE_RES=1440,1080` → aspect `1440.0/1080.0`. Mirrors
/// `char_shot_main.rs`'s `env_res`, but this module stays crate-free (see the
/// file header), so it re-reads the env var rather than importing that
/// binary's parser. Defaults to the 16:9 both shot bins render at.
fn env_aspect() -> f32 {
    std::env::var("VOXELFORGE_RES")
        .ok()
        .and_then(|raw| {
            let (w, h) = raw.split_once(&[',', 'x'][..])?;
            let w: f32 = w.trim().parse().ok()?;
            let h: f32 = h.trim().parse().ok()?;
            (w > 0.0 && h > 0.0).then_some(w / h)
        })
        .unwrap_or(16.0 / 9.0)
}

// --- Ghoul Reaver ------------------------------------------------------------
//
// The read-point is the right arm: a shoulder, a forearm reaching PAST the
// knee, and three splayed bone claws that end within half a block of the
// ground — "overlong arm" as geometry, not a texture trick. The torso is two
// stacked slabs, the upper one pushed forward+down in Z as it rises, so the
// creature reads as hunching even in a static pose. Spine spikes are
// irregular in height and spacing on purpose — evenly-spaced spikes read as
// architecture, not anatomy.
#[rustfmt::skip]
static REAVER: &[Bx] = &[
    // legs — asymmetric stance, one shorter (it limps)
    b(-2.4, 0.0, -1.2,  -0.6,  6.2,  1.2, Tone::Hide),
    b( 0.6, 0.0, -1.0,   2.2,  5.6,  1.4, Tone::Hide),
    b(-2.6, 0.0, -1.6,  -1.0,  0.9,  1.8, Tone::HideDark),
    b( 0.4, 0.0, -1.4,   2.4,  0.8,  2.0, Tone::HideDark),
    b(-2.0, 0.5, -2.4,  -1.2,  1.3, -1.6, Tone::Bone),      // toe-claw L
    b( 1.0, 0.5, -2.2,   1.9,  1.3, -1.4, Tone::Bone),      // toe-claw R

    // torso — hunches forward (more −Z) as it rises
    b(-2.4, 6.2, -1.0,   2.4,  9.6,  1.6, Tone::Hide),
    b(-2.6, 9.2, -3.4,   2.6, 12.8,  0.2, Tone::Hide),
    b(-1.6, 7.2, -1.4,   1.6,  9.2,  1.0, Tone::HideDark), // sunken belly shadow

    // spine ridge — irregular height + spacing, on purpose
    b(-0.5, 13.4, -1.4,  0.5, 15.0, -0.2, Tone::Bone),
    b(-0.4, 11.6, -0.2,  0.4, 12.8,  1.0, Tone::Bone),
    b(-0.3,  9.6,  0.6,  0.3, 10.5,  1.7, Tone::Bone),

    // head — elongated skull thrust forward off the hunch, protruding jaw
    b(-1.5, 11.6, -6.0,  1.5, 15.0, -3.0, Tone::Hide),
    b(-1.0, 11.0, -7.4,  1.0, 12.1, -5.8, Tone::Bone),      // jaw/snout
    b(-1.1, 13.8, -6.15,-0.5, 14.3, -5.9, Tone::GhoulGlow), // eye L
    b( 0.5, 13.4, -6.15, 1.1, 13.9, -5.9, Tone::GhoulGlow), // eye R — uneven height, unsettling

    // right arm — the overlong one, hand drags near the ground
    b( 2.4, 10.0, -1.8,  3.6, 13.2,  1.2, Tone::Hide),
    b( 2.6,  2.2, -2.0,  3.8, 10.2,  1.0, Tone::Hide),      // forearm past the knee
    b( 2.3,  0.8, -2.8,  3.0,  2.2,  0.2, Tone::Bone),      // claw 1
    b( 3.4,  0.8, -2.6,  4.1,  2.3,  0.4, Tone::Bone),      // claw 2
    b( 2.8,  0.5, -3.4,  3.4,  1.8, -1.8, Tone::Bone),      // claw 3, splayed forward

    // left arm — normal reach, for contrast
    b(-3.7, 10.4, -1.6, -2.5, 13.2,  1.2, Tone::Hide),
    b(-3.9,  6.8, -1.8, -2.7, 10.6,  1.0, Tone::Hide),
    b(-3.5,  5.8, -2.4, -2.8,  7.0, -1.4, Tone::Bone),

    // torn rag wraps — break the smooth-hide read
    b(-2.8,  7.2, -1.4, -1.8,  9.4,  1.8, Tone::Rag),
    b( 1.0,  6.4, -1.2,  2.2,  8.8,  1.6, Tone::Rag),
];

// --- Bone Sentinel -----------------------------------------------------------
//
// The Guard Husk archetype, redesigned: the weapon-side pauldron carries a
// horn spike a full block above the helm (breaks the outline the way Garren's
// spear does), the OTHER pauldron is small and chipped — the asymmetry itself
// reads as "this thing has been in a fight and won." The helm keeps the
// original husk's "blank slab = the emptiness is the horror beat" idea but
// adds two off-level glowing slits, so it reads as watching you instead of
// being merely faceless. Cracks are dried-blood red, not the amber the
// existing NPC Garren (Flamingo's file, untouched here) uses — different
// character, different corruption colour, no collision.
#[rustfmt::skip]
static SENTINEL: &[Bx] = &[
    // legs — bowed, wide stance
    b(-3.6, 0.0, -2.2,  -0.8,  2.0,  2.2, Tone::IronDark),
    b( 0.8, 0.0, -2.2,   3.6,  2.4,  2.2, Tone::IronDark),
    b(-3.8, 2.0, -2.0,  -0.7,  8.4,  2.0, Tone::Iron),
    b( 0.7, 2.4, -2.0,   3.8,  9.0,  2.0, Tone::Iron),      // weapon-side leg reads heavier

    // cuirass
    b(-4.8, 8.6, -3.2,   4.8, 16.2,  3.2, Tone::Iron),
    b(-5.0, 8.6, -3.4,   5.0,  9.6,  3.4, Tone::IronDark), // crumbling lower edge
    b(-1.0, 10.2,-3.5,   1.0, 14.0, -3.1, Tone::BloodCrack),
    b( 1.6, 11.6,-3.5,   2.6, 13.4, -3.1, Tone::BloodCrack), // off-centre second crack

    // pauldrons — deliberately unequal
    b( 4.0, 13.8,-3.6,   8.6, 17.2,  3.6, Tone::Iron),      // R — the bulk
    b( 4.4, 17.2,-2.4,   6.6, 20.0, -0.8, Tone::Iron),      // horn spike, jutting up+forward
    b( 4.8, 20.0,-2.0,   6.0, 22.2, -1.2, Tone::IronDark),  // horn tip
    b(-6.8, 13.8,-3.2,  -4.2, 16.0,  3.2, Tone::Iron),      // L — small
    b(-7.0, 13.8,-3.3,  -6.2, 15.2,  3.3, Tone::IronDark),  // chipped edge

    // neck + helm
    b(-1.6, 16.2,-1.6,   1.6, 17.2,  1.6, Tone::IronDark),
    b(-2.8, 17.2,-2.8,   2.8, 21.0,  2.8, Tone::Iron),      // one blank slab
    b(-1.7, 18.6,-2.85, -0.6, 18.95,-2.7, Tone::SentinelGlow), // eye slit L
    b( 0.5, 18.1,-2.85,  1.7, 18.4, -2.7, Tone::SentinelGlow), // eye slit R — lower, off-kilter
    b(-0.4, 19.8,-2.85,  0.5, 20.05,-2.7, Tone::BloodCrack),   // crack across the brow

    // arms
    b( 4.4,  8.6, -2.6,  6.6, 13.4,  2.6, Tone::Iron),      // weapon arm, upper (R)
    b( 4.6,  4.2, -2.4,  6.4,  8.8,  2.4, Tone::IronDark),  // weapon arm, forearm/gauntlet
    b(-6.4,  9.4, -2.2, -4.4, 13.2,  2.2, Tone::Iron),      // off-arm, upper (L)
    b(-6.6,  5.8, -2.0, -4.6,  9.6,  2.0, Tone::IronDark),  // off-arm, forearm
    b(-6.8,  4.6, -2.6, -5.2,  6.0, -1.0, Tone::Bone),      // off-hand claw, torn glove

    // jagged cleaver-glaive, standing above the helm — the "guard" read at a distance
    b( 6.6,  0.8, -3.5,  7.6, 22.4, -2.6, Tone::IronDark),  // haft
    b( 6.9,  6.6, -3.7,  7.9,  7.8, -2.4, Tone::Bone),      // grip wrap
    b( 7.2, 22.4, -3.6, 10.4, 27.0, -1.6, Tone::Iron),      // cleaver blade
    b( 7.2, 22.4, -4.4,  8.6, 23.8, -3.6, Tone::IronDark),  // serration notch

    // waist — jagged broken hem instead of one clean skirt
    b(-3.6,  7.2, -3.0,  3.6,  8.6,  3.0, Tone::IronDark),
    b(-3.6,  5.8, -2.8, -1.4,  7.4,  2.8, Tone::Iron),
    b( 0.4,  6.2, -2.8,  2.4,  7.6,  2.8, Tone::IronDark),  // shorter — broken
];

// --- Thornclaw Stalker --------------------------------------------------------
//
// Bipedal-compatible crouch (haunches, not four separate legs — this stays
// walkable on the existing AI/animation rig, which is Rose/Yamamoto's lane,
// not mine). Read-points: a spine ridge that tapers straight into a thin
// barbed tail, one grossly oversized sickle claw held low and forward
// (mantis-strike read), and close-set low eyes — a predator's gaze height,
// not a person's.
#[rustfmt::skip]
static STALKER: &[Bx] = &[
    // legs — crouched, digitigrade
    b(-2.6, 0.0, -1.6,  -0.8,  2.4,  1.6, Tone::HideDark),
    b( 0.8, 0.0, -1.6,   2.6,  2.6,  1.6, Tone::HideDark),
    b(-3.0, 2.4, -2.0,  -0.6,  6.0,  2.2, Tone::Hide),      // haunch L
    b( 0.6, 2.6, -2.0,   3.0,  6.2,  2.2, Tone::Hide),      // haunch R
    b(-2.0, 3.0, -1.6,  -0.4,  5.4,  1.8, Tone::HideDark),  // mottled patch
    b( 0.6, 3.2, -1.6,   2.2,  5.6,  1.8, Tone::HideDark),

    // spine — low, near-horizontal, hunched
    b(-2.4, 5.6, -2.0,   2.4,  8.4,  1.6, Tone::Hide),
    b(-2.2, 7.0, -3.4,   2.2,  9.6, -0.4, Tone::Hide),      // chest, pushed low+forward

    // spike ridge, tapering toward the tail
    b(-0.4, 9.2, -2.4,   0.4, 10.6, -1.6, Tone::Bone),
    b(-0.3, 8.6, -0.4,   0.3,  9.6,  0.6, Tone::Bone),
    b(-0.25,8.0,  1.4,   0.25, 8.8,  2.2, Tone::Bone),

    // tail — thin, whip-like, barbed tip
    b(-0.5, 6.6,  1.8,   0.5,  7.4,  4.0, Tone::HideDark),
    b(-0.35,6.2,  3.8,   0.35, 6.8,  6.4, Tone::HideDark),
    b(-0.4, 6.0,  6.2,   0.4,  6.6,  7.4, Tone::Bone),      // barb

    // head — low, thrust forward, close-set eyes
    b(-1.6, 7.6, -5.4,   1.6, 10.0, -3.0, Tone::Hide),
    b(-1.1, 7.2, -6.4,   1.1,  8.2, -5.2, Tone::HideDark), // snout
    b(-1.0, 9.0, -5.6,  -0.4,  9.5, -5.35, Tone::StalkerGlow),
    b( 0.4, 9.0, -5.6,   1.0,  9.5, -5.35, Tone::StalkerGlow),

    // arms — one oversized sickle claw (front-right), smaller other
    b( 2.2, 6.8, -3.0,   3.6,  9.2,  0.6, Tone::Hide),
    b( 2.6, 3.0, -3.6,   4.2,  7.0,  0.2, Tone::Hide),
    b( 2.4, 1.4, -4.6,   3.4,  3.4, -3.2, Tone::Bone),      // sickle base
    b( 1.6, 0.2, -5.6,   3.0,  1.8, -4.2, Tone::Bone),      // sickle hook, curving fwd+down
    b(-3.8, 7.2, -2.6,  -2.4,  9.4,  0.4, Tone::Hide),
    b(-4.0, 4.6, -2.8,  -2.8,  7.4,  0.0, Tone::Hide),
    b(-3.6, 3.6, -3.6,  -2.8,  4.8, -2.6, Tone::Bone),
];

// ---------------------------------------------------------------------------
// Spawning — the real design deliverable
// ---------------------------------------------------------------------------

/// Build one enemy and stand it with its **feet** at `feet`, facing `yaw`
/// (radians, yaw 0 = facing −Z — matches the husk and the cast). Returns the
/// root entity so a caller can move/hide/despawn the whole body with one
/// handle, same contract as `characters::spawn_character`.
pub fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: EnemyKind,
    feet: Vec3,
    yaw: f32,
) -> Entity {
    let pal = Palette::build(materials);
    let root = commands
        .spawn((
            Transform::from_translation(feet).with_rotation(Quat::from_axis_angle(Vec3::Y, yaw)),
            Visibility::default(),
            EnemyBody(kind),
            Name::new(kind.display()),
        ))
        .id();

    commands.entity(root).with_children(|p| {
        for bx in kind.boxes() {
            let size = bx.size();
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(pal.get(bx.tone)),
                Transform::from_translation(bx.centre()),
                Visibility::default(), // see characters.rs's note: never omit this
            ));
        }
    });

    println!(
        "ENEMY_SPAWN kind={} parts={} height={:.2}b feet=({:.1},{:.1},{:.1}) yaw={:.2}",
        kind.id(),
        kind.boxes().len(),
        kind.height(),
        feet.x,
        feet.y,
        feet.z,
        yaw
    );
    root
}

/// Same geometry as `spawn_enemy`, but every box shares one flat black unlit
/// material instead of the `Tone` palette — the "flat black cutout" test
/// design rule 1 (`docs/enemy-design.md`) claims every new body passes, run
/// for real instead of asserted in prose. `shot_main.rs`-only: never called
/// from the real spawn path, so it stays here rather than growing a
/// `silhouette: bool` param onto `spawn_enemy` (which would ripple into
/// `combat.rs`'s call site, another lane's file).
fn spawn_enemy_silhouette(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: EnemyKind,
    feet: Vec3,
    yaw: f32,
) -> Entity {
    let black = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
    let root = commands
        .spawn((
            Transform::from_translation(feet).with_rotation(Quat::from_axis_angle(Vec3::Y, yaw)),
            Visibility::default(),
            EnemyBody(kind),
            Name::new(kind.display()),
        ))
        .id();

    commands.entity(root).with_children(|p| {
        for bx in kind.boxes() {
            let size = bx.size();
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(black.clone()),
                Transform::from_translation(bx.centre()),
                Visibility::default(),
            ));
        }
    });

    println!(
        "ENEMY_SPAWN_SILHOUETTE kind={} parts={} feet=({:.1},{:.1},{:.1}) yaw={:.2}",
        kind.id(),
        kind.boxes().len(),
        feet.x,
        feet.y,
        feet.z,
        yaw
    );
    root
}

/// A byte-for-byte reproduction of the CURRENTLY SHIPPED enemy —
/// `combat::spawn_guard_husk` (client/src/combat.rs, "Spawning" section) —
/// for the mandatory before/after proof. NOT a new design: same three
/// cuboids, same sizes, same colours, same offsets as the live code. Kept
/// here (not in `combat.rs`, which is off-limits) purely so the isolated
/// shot bin can render an honest "before" without pulling in the full
/// gameplay crate graph.
pub fn spawn_legacy_guard_husk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    feet: Vec3,
) -> Entity {
    let armor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.34, 0.40),
        perceptual_roughness: 0.55,
        metallic: 0.3,
        ..default()
    });
    let head_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.19, 0.24),
        perceptual_roughness: 0.5,
        ..default()
    });
    let torso = meshes.add(Cuboid::new(0.9, 1.4, 0.6));
    let head = meshes.add(Cuboid::new(0.55, 0.55, 0.55));
    let arm = meshes.add(Cuboid::new(0.28, 1.0, 0.28));

    let root = commands
        .spawn((Transform::from_translation(feet), Visibility::default(), Name::new("Guard Husk (legacy)")))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((Mesh3d(torso), MeshMaterial3d(armor.clone()), Transform::from_xyz(0.0, 1.0, 0.0)));
        p.spawn((Mesh3d(head), MeshMaterial3d(head_mat), Transform::from_xyz(0.0, 2.0, 0.0)));
        p.spawn((Mesh3d(arm), MeshMaterial3d(armor), Transform::from_xyz(0.55, 1.1, -0.2)));
    });
    println!("ENEMY_SPAWN kind=legacy_husk parts=3 height=2.28b feet=({:.1},{:.1},{:.1})", feet.x, feet.y, feet.z);
    root
}

// ---------------------------------------------------------------------------
// The enemy shot (`shot_main.rs` stage)
// ---------------------------------------------------------------------------

/// `VOXELFORGE_ENEMYSHOT` — the contact-sheet stage inside the shot binary.
///
/// ```text
/// VOXELFORGE_ENEMYSHOT=before         the live combat.rs husk, daylight (the "before")
/// VOXELFORGE_ENEMYSHOT=line           all three new kinds, one frame, dark/moonlit
/// VOXELFORGE_ENEMYSHOT=sentinel       solo portrait, dark/moonlit (default — sells the eye-glow)
/// VOXELFORGE_ENEMYSHOT=sentinel:day   solo portrait, daylight (direct A/B vs. `before`)
/// VOXELFORGE_ENEMYSHOT=sentinel:sil   solo, flat black unlit cutout on a bright card — the
///                                     design-rule-1 "broken silhouette" claim rendered for real
/// VOXELFORGE_ENEMYSHOT=reaver|stalker same solo modifiers
/// ```
#[derive(Resource, Clone, Copy, Debug)]
pub struct EnemyShot {
    pub mode: EnemyShotMode,
    pub daylight: bool,
    /// Flat black unlit cutout on a bright card, same convention
    /// `characters.rs`'s `Stage::Silhouette` uses. Only meaningful for
    /// `Solo` — that's the design-rule-1 claim under test.
    pub silhouette: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum EnemyShotMode {
    Before,
    Line,
    Solo(EnemyKind),
}

impl EnemyShot {
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("VOXELFORGE_ENEMYSHOT").ok()?;
        let raw = raw.trim();
        if raw.is_empty() || raw == "0" || raw.eq_ignore_ascii_case("off") {
            return None;
        }
        let mut parts = raw.splitn(2, ':');
        let head = parts.next().unwrap_or("").trim();
        let modifier = parts.next().map(|s| s.trim().to_ascii_lowercase());

        let (mode, default_day) = if head.eq_ignore_ascii_case("before") {
            (EnemyShotMode::Before, true)
        } else if head.eq_ignore_ascii_case("line") {
            (EnemyShotMode::Line, false)
        } else if let Some(k) = EnemyKind::from_name(head) {
            (EnemyShotMode::Solo(k), false)
        } else {
            return None;
        };

        let (daylight, silhouette) = match modifier.as_deref() {
            Some("day") => (true, false),
            Some("night") => (false, false),
            Some("sil") | Some("silhouette") => (default_day, true),
            _ => (default_day, false),
        };
        Some(EnemyShot { mode, daylight, silhouette })
    }
}

fn env_f32(key: &str) -> Option<f32> {
    std::env::var(key).ok()?.trim().parse().ok()
}

fn env_floats<const N: usize>(key: &str) -> Option<[f32; N]> {
    let raw = std::env::var(key).ok()?;
    let parts: Vec<f32> = raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    if parts.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    out.copy_from_slice(&parts);
    Some(out)
}

pub fn setup_enemyshot(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shot: Res<EnemyShot>,
) {
    let day = shot.daylight;
    let sil = shot.silhouette;

    // ---- the enemy/enemies -------------------------------------------------
    // Alongside the names, collect the world-space AABB of whatever actually
    // got spawned (`None` for `Before`, which keeps its old hardcoded cam —
    // it's the legacy mesh husk, not an `EnemyKind`, so it has no `.boxes()`
    // to measure). Computing this HERE, from the same `x`/`yaw` values the
    // spawn loop uses, is what keeps the camera below honest: it fits the
    // real placement instead of a second hand-copied formula that could
    // silently drift from this one.
    let (placed_names, placed_bounds_all): (Vec<&'static str>, Option<(Vec3, Vec3)>) = match shot.mode {
        EnemyShotMode::Before => {
            spawn_legacy_guard_husk(&mut commands, &mut meshes, &mut materials, Vec3::ZERO);
            (vec!["legacy_husk"], None)
        }
        EnemyShotMode::Solo(k) => {
            let yaw = 0.22;
            if shot.silhouette {
                spawn_enemy_silhouette(&mut commands, &mut meshes, &mut materials, k, Vec3::ZERO, yaw);
            } else {
                spawn_enemy(&mut commands, &mut meshes, &mut materials, k, Vec3::ZERO, yaw);
            }
            (vec![k.id()], Some(placed_bounds(k, Vec3::ZERO, yaw)))
        }
        EnemyShotMode::Line => {
            let gap = 2.6;
            let order = EnemyKind::ALL;
            let mut names = Vec::new();
            let mut bounds = None;
            for (i, k) in order.iter().enumerate() {
                let x = (i as f32 - (order.len() as f32 - 1.0) * 0.5) * gap;
                let yaw = 0.18 * if x < 0.0 { 1.0 } else { -1.0 };
                let feet = Vec3::new(x, 0.0, 0.0);
                spawn_enemy(&mut commands, &mut meshes, &mut materials, *k, feet, yaw);
                names.push(k.id());
                let b = placed_bounds(*k, feet, yaw);
                bounds = Some(match bounds {
                    Some(acc) => merge_bounds(acc, b),
                    None => b,
                });
            }
            (names, bounds)
        }
    };

    // ---- ground --------------------------------------------------------
    let ground_mat = materials.add(if sil {
        // Bright card, same convention `characters.rs`'s `Stage::Silhouette`
        // grades against — the cutout has to read against light, not dark.
        StandardMaterial { base_color: Color::srgb(0.93, 0.90, 0.84), unlit: true, ..default() }
    } else {
        StandardMaterial {
            base_color: if day { Color::srgb_u8(0x4A, 0x3A, 0x24) } else { Color::srgb_u8(0x14, 0x16, 0x1A) },
            perceptual_roughness: 0.95,
            ..default()
        }
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(60.0, 1.0, 60.0))),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, -0.5, 0.0),
        Visibility::default(),
    ));

    // ---- lighting --------------------------------------------------------
    // Unlit black-on-card silhouette needs no light at all — same rule
    // `characters.rs` uses (`if !sil`).
    if !sil && day {
        // Same golden-hour convention `characters.rs` grades against, so a
        // solo `:day` shot is an honest, same-light A/B vs. `before`.
        let [elev, azim, illum] = env_floats::<3>("VOXELFORGE_SUN").unwrap_or([26.0, 208.0, 12000.0]);
        let (e, a) = (elev.to_radians(), azim.to_radians());
        let dir = Vec3::new(a.sin() * e.cos(), -e.sin(), a.cos() * e.cos()).normalize();
        commands.spawn((
            DirectionalLight {
                color: Color::srgb(1.0, 0.80, 0.46),
                illuminance: illum,
                shadow_maps_enabled: true,
                shadow_depth_bias: 0.06,
                shadow_normal_bias: 1.4,
                ..default()
            },
            Transform::from_translation(-dir * 40.0).looking_to(dir, Vec3::Y),
        ));
        commands.spawn((
            DirectionalLight {
                color: Color::srgb(0.70, 0.72, 0.80),
                illuminance: 1500.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(-14.0, 8.0, 14.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        ));
    } else if !sil {
        // Dim cool moonlight — just enough to read the silhouette + matte
        // armour, so the emissive eyes/cracks are the loudest thing in frame.
        let dir = Vec3::new(0.35, -0.55, 0.45).normalize();
        commands.spawn((
            DirectionalLight {
                color: Color::srgb(0.55, 0.62, 0.85),
                illuminance: env_f32("VOXELFORGE_SUN_NIGHT").unwrap_or(650.0),
                shadow_maps_enabled: true,
                shadow_depth_bias: 0.06,
                shadow_normal_bias: 1.4,
                ..default()
            },
            Transform::from_translation(-dir * 40.0).looking_to(dir, Vec3::Y),
        ));
    }

    // ---- camera ------------------------------------------------------------
    // Auto-fit from the real bounding box (see `fit_camera`) instead of a
    // hardcoded eye/target guess — that guess is exactly what let
    // `sentinel_after.png` / `reaver_silhouette.png` ship head-cropped: a
    // constant tuned for one body doesn't notice when another body's glaive
    // or dragging arm is taller/wider than the number assumed. 20% headroom
    // on every side, front-on for the lineup (`angle=0`) and a 20° 3/4 turn
    // for the solo portraits (matches the old hardcoded eye.x/eye.z ratio).
    let aspect = env_aspect();
    let default_cam: [f32; 7] = match (shot.mode, placed_bounds_all) {
        (EnemyShotMode::Solo(_), Some((lo, hi))) => {
            let (eye, target) = fit_camera(lo, hi, aspect, 30.0, 20.0, 0.20);
            [eye.x, eye.y, eye.z, target.x, target.y, target.z, 30.0]
        }
        (EnemyShotMode::Line, Some((lo, hi))) => {
            let (eye, target) = fit_camera(lo, hi, aspect, 36.0, 0.0, 0.20);
            [eye.x, eye.y, eye.z, target.x, target.y, target.z, 36.0]
        }
        // `Before` (no `EnemyKind`, nothing to measure) and any mode whose
        // bounds we didn't compute fall back to the original hand-tuned cam.
        _ => [1.15, 1.60, -3.60, 0.0, 1.30, 0.0, 30.0],
    };
    let cam = env_floats::<7>("VOXELFORGE_CAM").unwrap_or(default_cam);
    let eye = Vec3::new(cam[0], cam[1], cam[2]);
    let target = Vec3::new(cam[3], cam[4], cam[5]);

    let clear = if sil {
        Color::srgb(0.93, 0.90, 0.84)
    } else if day {
        Color::srgb(0.055, 0.038, 0.028)
    } else {
        Color::srgb(0.012, 0.014, 0.020)
    };
    let mut cam_cmd = commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(clear), ..default() },
        Projection::Perspective(PerspectiveProjection { fov: cam[6].to_radians(), near: 0.05, ..default() }),
        Transform::from_translation(eye).looking_at(target, Vec3::Y),
        Msaa::Off,
        Tonemapping::AcesFitted,
        Exposure {
            ev100: env_f32("VOXELFORGE_EXPOSURE").unwrap_or(if sil { 9.7 } else if day { 9.9 } else { 8.6 }),
        },
    ));
    if !sil {
        cam_cmd.insert(AmbientLight {
            color: if day { Color::srgb(0.784, 0.541, 0.180) } else { Color::srgb(0.10, 0.13, 0.22) },
            brightness: env_f32("VOXELFORGE_AMBIENT").unwrap_or(if day { 2600.0 } else { 220.0 }),
            affects_lightmapped_meshes: false,
        });
    }

    println!(
        "ENEMYSHOT mode={:?} day={} sil={} cast=[{}] cam eye=({:.2},{:.2},{:.2}) -> ({:.2},{:.2},{:.2}) fov={:.1}",
        shot.mode,
        day,
        sil,
        placed_names.join(","),
        eye.x,
        eye.y,
        eye.z,
        target.x,
        target.y,
        target.z,
        cam[6]
    );
}
