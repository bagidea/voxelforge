//! The Act I cast, as real in-engine bodies — now built as **base body + six
//! swappable equipment slots**.
//!
//! ## What changed in this pass (2026-08-14)
//!
//! The first pass rendered each character as ONE frozen `&[Bx]` list wearing one
//! flat colour per region. Two things were wrong with it and both are fixed here:
//!
//! 1. **Auren didn't read as a hero.** The old body was 17.1 vx tall (2.14 blocks)
//!    with a uniform 7.6-vx-wide torso — a 3.6-heads-tall box. The new body is
//!    20.5 vx (2.56 blocks) at 5.5 heads tall, with a real V-taper: an 11.8-vx
//!    shoulder span over a 6.0-vx waist (ratio 1.97, was 1.00). See [`AUREN_BODY`]
//!    for the measured ladder; [`AUREN_V1`] is kept verbatim so one binary can
//!    shoot the before/after pair at one camera.
//! 2. **Everything was one material.** The old palette was 11 colours sharing a
//!    single roughness. Gear now lives in `equipment.rs` on real material classes
//!    — cloth is matte, leather has a broad sheen, steel is `metallic: 1.0` and
//!    actually glints. A cuirass and a cloak lit by the same sun no longer differ
//!    only in hue.
//!
//! ## Slots
//!
//! A character is a **root entity** carrying a [`Wardrobe`], with one child node
//! per slot (head / torso / legs / hands / weapon / back). Changing a slot is
//! `equipment::equip` — despawn that node, spawn the new part. The root, the
//! transform and the other five slots are untouched, which is exactly the property
//! a future item/inventory system needs. `VOXELFORGE_CHARSHOT=swap` proves it by
//! running three complete outfit changes on one entity inside one process and
//! printing that entity's id beside every capture.
//!
//! ## Why this module reaches into no other lane
//!
//! It imports `crate::equipment` and nothing else. Both crate roots that use it
//! (`main.rs` for `voxelforge`, `shot_main.rs` for `voxelforge_shot`) declare
//! `mod equipment;` and `mod characters;`, so the path resolves in both — and the
//! playable scene and the contact sheet therefore build the cast through **one**
//! code path. What the sheet shows is literally what the game spawns.

use bevy::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::AmbientLight;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::view::Msaa;

use crate::equipment::{
    self as eq, b, br, Bx, Loadout, Mat, Palette, Slot, Surf, Wardrobe, SLOT_COUNT,
};

// ---------------------------------------------------------------------------
// Who
// ---------------------------------------------------------------------------

/// Marks a body built by this module, so a later pass can find the cast without
/// guessing at mesh shapes.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Character(pub Who);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    /// The hero. `docs/character-bible.md` §1.
    Auren,
    /// Elder Maren, keeper of a Forge-point. §3.
    Maren,
    /// Toma, the child of Edhari. §4.
    Toma,
    /// Garren — the Guard Husk of `docs/act1-script.md` q3. §2.
    Garren,
}

impl Who {
    /// The `act1.json` / `quest.rs` npc id, where one exists.
    pub fn id(self) -> &'static str {
        match self {
            Who::Auren => "auren",
            Who::Maren => "maren",
            Who::Toma => "toma",
            Who::Garren => "garren",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            Who::Auren => "Auren",
            Who::Maren => "Elder Maren",
            Who::Toma => "Toma",
            Who::Garren => "Garren (Guard Husk)",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auren" | "hero" => Some(Who::Auren),
            "maren" | "elder" | "elder-maren" => Some(Who::Maren),
            "toma" | "child" => Some(Who::Toma),
            "garren" | "husk" | "guard-husk" => Some(Who::Garren),
            _ => None,
        }
    }

    pub const ALL: [Who; 4] = [Who::Auren, Who::Maren, Who::Toma, Who::Garren];

    /// The parts of the body that are never removed: skin, hair, and (for the
    /// three not yet slotted) their whole monolithic silhouette.
    pub fn body(self) -> &'static [Bx] {
        match self {
            // `VOXELFORGE_SCULPT=0` drops Auren back to the pre-sculpt v2 table —
            // the same lever `equipment::boxes_for` uses on the gear, so ONE
            // process can shoot the whole before/after pair. See
            // `equipment::sculpt_on` for why two binaries would not do.
            Who::Auren if !eq::sculpt_on() => AUREN_V2,
            Who::Auren => AUREN_BODY,
            Who::Maren => MAREN,
            Who::Toma => TOMA,
            Who::Garren => GARREN,
        }
    }

    /// What this character wears when nobody says otherwise.
    ///
    /// Auren is the fully-slotted body. Garren carries exactly one slotted part
    /// (his spear) — deliberately, so the swap machinery is proven on a second
    /// character and not just on the hero. Maren and Toma are still monolithic;
    /// slotting them is the next pass and is tracked in `docs/character-equipment.md`.
    pub fn default_loadout(self) -> Loadout {
        match self {
            Who::Auren => eq::adventurer(),
            Who::Garren => Loadout::EMPTY.with(&eq::SPEAR_GUARD),
            _ => Loadout::EMPTY,
        }
    }

    /// Top of body+gear in world metres — used to frame a portrait and to print a
    /// height the design docs can be graded against.
    pub fn height_with(self, l: Loadout) -> f32 {
        let body = self.body().iter().fold(0.0f32, |m, bx| m.max(bx.top()));
        l.parts
            .iter()
            .flatten()
            .flat_map(|p| eq::boxes_for(p).iter())
            .fold(body, |m, bx| m.max(bx.top()))
    }
}

// ===========================================================================
// AUREN — the hero body, re-proportioned
// ===========================================================================
//
// The measured ladder (all vx; 8 vx = 1 world block):
//
//   region        y span     half-width      note
//   -----------------------------------------------------------------------
//   boot/foot     0.0– 1.1     1.45          right foot 1.2 forward of the left
//   shin          1.1– 5.0     1.25
//   thigh         5.0– 9.4     1.50
//   pelvis        9.0–10.6     3.60
//   WAIST        10.4–12.6     3.00          <- the pinch
//   chest        12.4–15.4     4.40
//   deltoid      13.8–15.7     5.90 (outer)  <- shoulder span 11.8
//   neck         15.6–16.4     1.50
//   head         16.2–19.9     2.20          <- 3.7 tall
//   hair         16.4–20.5     2.50
//
//   total 20.5 vx = 2.56 blocks · 5.5 heads tall · shoulder:waist = 1.97
//   (v1 was 17.1 vx = 2.14 blocks · 3.6 heads tall · shoulder:waist = 1.00)
//
// Stance is baked, not animated: the right foot is 1.2 vx forward and the whole
// upper body is pushed −Z with height, so the silhouette leans into the walk from
// a dead-static pose.
//
// ===========================================================================
// AUREN v3 — the sculpt pass (2026-08-14, Flamingo)
// ===========================================================================
//
// v2 (below, kept verbatim as `AUREN_V2`) fixed the PROPORTIONS and nothing else:
// the ladder, the V-taper and the 5.5-head count are all still exactly what v2
// measured, and this table does not move a single one of the anchors the gear in
// `equipment.rs` is authored against (skull top y=19.9, shoulder span ±5.9,
// waist ±3.0, hand centre 4.85/7.3/−0.35). Re-proportioning was the right fix and
// it is not being re-litigated.
//
// What v2 still looked like, though, was a correctly-proportioned pile of crates,
// and the three reasons are all form, not colour:
//
//   1. **No diagonals.** Every one of v2's 28 boxes was axis-aligned, so every
//      silhouette edge was vertical or horizontal. The eye reads that as masonry.
//      v3 rotates the load-bearing ones — deltoids roll 14° so the shoulder line
//      SLOPES, arms hang 4–6° out, the fringe is three locks at 14/−6/−16°, the
//      feet toe out 7°. Rotation is new machinery: `equipment::br` + `Bx::rot`.
//   2. **No taper.** A limb was one box from joint to joint, so it read as a pipe.
//      Every limb here is a stack — bicep → elbow → forearm → wrist → palm →
//      knuckles → fingers → thumb — each step narrower than the last.
//   3. **No face.** The head was ONE skin cuboid. At the shot camera that is
//      ~110 px of blank tan, and it is the single loudest "this is a voxel dummy"
//      signal on the body. v3 builds brow ridge, socket, eye, pupil, nose with a
//      flushed tip, two lips, a jaw that narrows to a chin, and yawed ears — 24
//      boxes for the head alone, on the theory that the face is where a viewer
//      looks first and therefore where the budget belongs.
//
// The other half of the readability fix is `Surf::SkinShade`/`SkinFlush`: at this
// scale a 3-vx-deep eye socket casts no shadow the renderer can resolve, so the
// value change has to be authored. That is painted-in occlusion, deliberately.
//
// 28 boxes → 113, of which 49 are rotated. Still one box per mesh sharing one
// material per surface;
// at four bodies on screen that is noise, and the shot lane is not perf-bound.
#[rustfmt::skip]
static AUREN_BODY: &[Bx] = &[
    // ---- feet: heel / mid-foot / dropped toe, each toed out 7° ---------------
    // A foot that is one box ends the leg in a shelf. Three boxes with the toe
    // box LOWER than the heel gives an arch, and the yaw gives a stance.
    b (-3.30, 0.00,  0.85,  -0.60,  1.35,  2.40, Surf::Skin),                 // heel L
    br(-3.35, 0.00, -1.00,  -0.55,  1.20,  1.00,  0.0,  7.0, 0.0, Surf::Skin),// mid-foot L
    br(-3.15, 0.00, -2.20,  -0.75,  0.80, -0.90,  0.0,  7.0, 0.0, Surf::Skin),// toes L (dropped)
    b (-3.05, 1.10, -0.90,  -0.85,  2.10,  1.05, Surf::Skin),                 // ankle L
    b (-3.00, 0.00, -1.05,  -0.90,  0.28,  1.10, Surf::SkinShade),            // sole contact L
    b ( 0.60, 0.00, -0.35,   3.30,  1.35,  1.20, Surf::Skin),                 // heel R
    br( 0.55, 0.00, -2.20,   3.35,  1.20, -0.20,  0.0, -7.0, 0.0, Surf::Skin),// mid-foot R
    br( 0.75, 0.00, -3.40,   3.15,  0.80, -2.10,  0.0, -7.0, 0.0, Surf::Skin),// toes R (dropped)
    b ( 0.85, 1.10, -2.10,   3.05,  2.10, -0.15, Surf::Skin),                 // ankle R
    b ( 0.90, 0.00, -2.25,   3.00,  0.28, -0.10, Surf::SkinShade),            // sole contact R

    // ---- lower legs: shin ridge in front, calf swell behind ------------------
    b (-3.00, 1.60, -1.35,  -0.90,  3.20,  1.50, Surf::Skin),                 // shin L
    b (-2.65, 1.70, -1.45,  -1.25,  3.60, -1.15, Surf::Skin),                 // shin ridge L
    b (-3.25, 2.90, -1.50,  -0.65,  4.90,  1.95, Surf::Skin),                 // calf L
    b (-3.05, 3.20,  1.30,  -0.85,  4.70,  2.15, Surf::Skin),                 // calf swell L
    b ( 0.90, 1.60, -2.55,   3.00,  3.20,  0.30, Surf::Skin),                 // shin R
    b ( 1.25, 1.70, -2.65,   2.65,  3.60, -2.35, Surf::Skin),                 // shin ridge R
    b ( 0.65, 2.90, -2.70,   3.25,  4.90,  0.75, Surf::Skin),                 // calf R
    b ( 0.85, 3.20,  0.10,   3.05,  4.70,  0.95, Surf::Skin),                 // calf swell R

    // ---- knees + thighs: two-stage taper, legs 2° out of vertical ------------
    b (-3.30, 4.70, -1.75,  -0.60,  5.75,  1.80, Surf::Skin),                 // knee L
    b (-3.20, 4.70, -1.80,  -0.70,  4.95,  1.70, Surf::SkinShade),            // knee crease L
    br(-3.35, 5.40, -1.80,  -0.65,  7.50,  1.85,  0.0, 0.0,  2.0, Surf::Skin),// thigh lower L
    br(-3.60, 7.30, -1.90,  -0.40,  9.40,  2.00,  0.0, 0.0,  2.0, Surf::Skin),// thigh upper L
    b ( 0.60, 4.70, -2.58,   3.30,  5.75,  0.90, Surf::Skin),                 // knee R
    b ( 0.70, 4.70, -2.52,   3.20,  4.95,  0.80, Surf::SkinShade),            // knee crease R
    br( 0.65, 5.40, -2.50,   3.35,  7.50,  1.15,  0.0, 0.0, -2.0, Surf::Skin),// thigh lower R
    br( 0.40, 7.30, -2.35,   3.60,  9.40,  1.55,  0.0, 0.0, -2.0, Surf::Skin),// thigh upper R

    // ---- pelvis: a linen underwrap, so the BARE body still reads deliberate
    // instead of looking like an unfinished mannequin when the legs slot is empty.
    // Now with a hanging fold and an off-centre knot, because a flat band was the
    // one place the bare plate still looked like a placeholder.
    b (-3.60, 9.00, -2.10,   3.60, 10.60,  1.90, Surf::ClothLinen),
    b (-3.70, 9.55, -2.20,   3.70, 10.15,  2.00, Surf::ClothLinen),           // wrap band
    br(-3.75, 9.10, -2.15,  -1.20, 10.40,  1.95,  0.0, 0.0,  5.0, Surf::ClothLinen), // fold L
    br( 1.10, 9.25, -2.35,   2.60, 10.60, -1.85,  0.0, 0.0,-12.0, Surf::ClothWalnut),// knot, off-centre
    b (-3.55, 9.00, -2.05,   3.55,  9.30,  1.85, Surf::SkinShade),            // hip line under the wrap

    // ---- torso: the V. Waist 6.0 wide, chest 8.8, deltoids out to 11.8 ------
    b (-3.00,10.40, -2.00,   3.00, 12.60,  1.90, Surf::Skin),                 // waist — the pinch
    br(-3.30,11.20, -1.90,  -2.55, 12.90,  1.80,  0.0, 0.0,  9.0, Surf::Skin),// oblique L
    br( 2.55,11.20, -1.90,   3.30, 12.90,  1.80,  0.0, 0.0, -9.0, Surf::Skin),// oblique R
    b (-0.18,10.60, -2.06,   0.18, 12.55, -1.90, Surf::SkinShade),            // linea alba
    b (-1.30,11.55, -2.06,   1.30, 11.75, -1.92, Surf::SkinShade),            // ab crease
    b (-4.40,12.40, -2.45,  -0.20, 15.10,  2.05, Surf::Skin),                 // chest L
    b ( 0.20,12.40, -2.45,   4.40, 15.10,  2.05, Surf::Skin),                 // chest R
    b (-0.22,12.40, -2.50,   0.22, 15.20, -2.02, Surf::SkinShade),            // sternum
    b (-4.20,12.85, -2.52,  -0.35, 13.05, -2.10, Surf::SkinShade),            // pec underline L
    b ( 0.35,12.85, -2.52,   4.20, 13.05, -2.10, Surf::SkinShade),            // pec underline R
    b (-4.30,15.00, -2.35,   4.30, 15.55,  2.00, Surf::Skin),                 // upper chest
    br(-3.90,15.20, -2.45,  -0.90, 15.70, -1.75,  0.0, 0.0,  7.0, Surf::Skin),// clavicle L
    br( 0.90,15.20, -2.45,   3.90, 15.70, -1.75,  0.0, 0.0, -7.0, Surf::Skin),// clavicle R
    b (-4.00,15.10, -2.20,   4.00, 15.90,  1.90, Surf::Skin),                 // trapezius
    br(-4.30,15.30, -1.90,  -1.20, 16.05,  1.20,  0.0, 0.0,-10.0, Surf::Skin),// trap slope L
    br( 1.20,15.30, -1.90,   4.30, 16.05,  1.20,  0.0, 0.0, 10.0, Surf::Skin),// trap slope R
    b (-4.40,13.40, -1.90,  -3.85, 14.20,  1.60, Surf::SkinShade),            // armpit L
    b ( 3.85,13.40, -1.90,   4.40, 14.20,  1.60, Surf::SkinShade),            // armpit R

    // ---- deltoids: the 14° roll IS the shoulder line ------------------------
    // Same span v2 had (out to ±5.9). Rotated about its own centre so the OUTER
    // end drops — one diagonal, and the top of the body stops being a lintel.
    br(-5.90,13.80, -2.20,  -3.90, 15.70,  1.90,  0.0, 0.0, 14.0, Surf::Skin),// deltoid L
    br( 3.90,13.80, -2.20,   5.90, 15.70,  1.90,  0.0, 0.0,-14.0, Surf::Skin),// deltoid R
    br(-5.95,15.00, -2.10,  -4.10, 15.85,  1.80,  0.0, 0.0, 14.0, Surf::Skin),// deltoid cap L
    br( 4.10,15.00, -2.10,   5.95, 15.85,  1.80,  0.0, 0.0,-14.0, Surf::Skin),// deltoid cap R

    // ---- arms: four-stage taper, hanging 4–6° off the body ------------------
    br(-5.70,11.40, -2.00,  -4.05, 14.00,  1.60,  0.0, 0.0,  4.0, Surf::Skin),// bicep L
    br( 4.05,11.40, -2.00,   5.70, 14.00,  1.60,  0.0, 0.0, -4.0, Surf::Skin),// bicep R
    br(-5.55,10.60, -1.95,  -4.15, 11.55,  1.55,  0.0, 0.0,  5.0, Surf::Skin),// elbow L
    br( 4.15,10.60, -1.95,   5.55, 11.55,  1.55,  0.0, 0.0, -5.0, Surf::Skin),// elbow R
    br(-5.60, 8.20, -1.90,  -4.10, 11.10,  1.50,  0.0, 0.0,  6.0, Surf::Skin),// forearm L
    br( 4.10, 8.20, -1.90,   5.60, 11.10,  1.50,  0.0, 0.0, -6.0, Surf::Skin),// forearm R
    br(-5.35, 7.85, -1.80,  -4.35,  8.50,  1.35,  0.0, 0.0,  6.0, Surf::Skin),// wrist L
    br( 4.35, 7.85, -1.80,   5.35,  8.50,  1.35,  0.0, 0.0, -6.0, Surf::Skin),// wrist R

    // ---- hands: palm, knuckle row, finger block, splayed thumb --------------
    // Everything stays inside y 6.4–8.1 so the glove and the gauntlet still cover
    // a hand completely — a finger poking through a plate gauntlet is the kind of
    // detail that makes the whole set look cheaper, not richer.
    br(-5.70, 6.95, -2.00,  -4.10,  8.10,  1.25,  0.0, 0.0,  6.0, Surf::Skin),     // palm L
    br( 4.10, 6.95, -2.00,   5.70,  8.10,  1.25,  0.0, 0.0, -6.0, Surf::Skin),     // palm R
    br(-5.75, 7.05, -2.05,  -4.05,  7.40,  1.30,  0.0, 0.0,  6.0, Surf::SkinFlush),// knuckles L
    br( 4.05, 7.05, -2.05,   5.75,  7.40,  1.30,  0.0, 0.0, -6.0, Surf::SkinFlush),// knuckles R
    br(-5.60, 6.40, -1.85,  -4.20,  7.05,  1.05,  0.0, 0.0,  6.0, Surf::Skin),     // fingers L
    br( 4.20, 6.40, -1.85,   5.60,  7.05,  1.05,  0.0, 0.0, -6.0, Surf::Skin),     // fingers R
    br(-5.62, 6.52, -1.90,  -4.18,  6.68,  1.10,  0.0, 0.0,  6.0, Surf::SkinShade),// finger split L
    br( 4.18, 6.52, -1.90,   5.62,  6.68,  1.10,  0.0, 0.0, -6.0, Surf::SkinShade),// finger split R
    br(-4.42, 6.90, -1.60,  -3.92,  7.95, -0.30,  0.0, 0.0,-16.0, Surf::Skin),     // thumb L
    br( 3.92, 6.90, -1.60,   4.42,  7.95, -0.30,  0.0, 0.0, 16.0, Surf::Skin),     // thumb R

    // ---- neck: tapered, with the under-jaw shadow authored in ---------------
    b (-1.55,15.55, -1.45,   1.55, 16.45,  1.05, Surf::Skin),                 // neck
    b (-1.45,16.10, -1.35,   1.45, 16.50,  0.95, Surf::SkinShade),            // under-jaw shadow
    br(-1.80,15.40, -1.20,  -0.95, 16.30,  0.60,  0.0, 0.0, 10.0, Surf::Skin),// neck cord L
    br( 0.95,15.40, -1.20,   1.80, 16.30,  0.60,  0.0, 0.0,-10.0, Surf::Skin),// neck cord R

    // ---- head. Head is 3.7 tall against a 20.5 body = 5.5 heads, and 4.4 wide
    // against an 11.8 shoulder span = 2.7 head-widths. That ratio pair IS the
    // "reads as a hero" fix from v2 and is untouched. What is new is that there
    // is now a FACE inside the envelope instead of a blank tan slab.
    //
    // Depth order, front to back: pupil −2.68 · eye −2.62 · brow −2.72 (it
    // overhangs, so the socket sits in its shadow) · nose tip −2.92.
    b (-1.85,16.20, -2.35,   1.85, 17.35,  1.70, Surf::Skin),                 // jaw, narrower than the skull
    b (-1.75,16.20, -2.25,   1.75, 16.55,  1.60, Surf::SkinShade),            // jaw underside
    b (-0.95,16.30, -2.55,   0.95, 17.20, -2.15, Surf::Skin),                 // chin
    b (-2.10,17.25, -2.50,   2.10, 18.70,  1.85, Surf::Skin),                 // midface
    br(-2.22,17.55, -2.45,  -1.35, 18.35, -1.10,  0.0, 0.0,  6.0, Surf::Skin),// cheekbone L
    br( 1.35,17.55, -2.45,   2.22, 18.35, -1.10,  0.0, 0.0, -6.0, Surf::Skin),// cheekbone R
    b (-2.20,18.55, -2.40,   2.20, 19.90,  1.90, Surf::Skin),                 // cranium
    b (-2.10,18.35, -2.72,   2.10, 18.80, -2.05, Surf::Skin),                 // brow ridge (overhangs)
    b (-1.90,18.55, -2.80,  -0.55, 18.85, -2.40, Surf::Hair),                 // brow L
    b ( 0.55,18.55, -2.80,   1.90, 18.85, -2.40, Surf::Hair),                 // brow R
    b (-1.85,17.90, -2.55,  -0.50, 18.45, -2.05, Surf::SkinShade),            // eye socket L
    b ( 0.50,17.90, -2.55,   1.85, 18.45, -2.05, Surf::SkinShade),            // eye socket R
    b (-1.70,18.00, -2.62,  -0.65, 18.38, -2.28, Surf::EyeWhite),             // eye L
    b ( 0.65,18.00, -2.62,   1.70, 18.38, -2.28, Surf::EyeWhite),             // eye R
    b (-1.45,18.05, -2.68,  -0.90, 18.33, -2.42, Surf::EyePupil),             // pupil L
    b ( 0.90,18.05, -2.68,   1.45, 18.33, -2.42, Surf::EyePupil),             // pupil R
    b (-0.50,18.10, -2.72,   0.50, 18.75, -2.30, Surf::Skin),                 // nose bridge
    b (-0.55,17.55, -2.86,   0.55, 18.20, -2.35, Surf::Skin),                 // nose
    b (-0.40,17.60, -2.92,   0.40, 17.95, -2.80, Surf::SkinFlush),            // nose tip
    b (-0.58,17.52, -2.86,  -0.22, 17.70, -2.55, Surf::SkinShade),            // nostril L
    b ( 0.22,17.52, -2.86,   0.58, 17.70, -2.55, Surf::SkinShade),            // nostril R
    b (-0.65,17.14, -2.62,   0.65, 17.34, -2.36, Surf::SkinFlush),            // upper lip
    b (-0.75,17.02, -2.60,   0.75, 17.14, -2.34, Surf::SkinShade),            // mouth line
    b (-0.60,16.82, -2.62,   0.60, 17.02, -2.36, Surf::SkinFlush),            // lower lip
    br(-2.55,17.60, -1.20,  -2.10, 18.65,  0.20,  0.0, 0.0, -7.0, Surf::Skin),// ear L
    br( 2.10,17.60, -1.20,   2.55, 18.65,  0.20,  0.0, 0.0,  7.0, Surf::Skin),// ear R
    b (-2.45,17.85, -0.85,  -2.25, 18.40, -0.05, Surf::SkinShade),            // ear hollow L
    b ( 2.25,17.85, -0.85,   2.45, 18.40, -0.05, Surf::SkinShade),            // ear hollow R

    // ---- hair: a layered mass with real thickness, per the §7 review ---------
    // The fringe is now three separate locks at 14° / −6° / −16° instead of one
    // slab. Three diagonals over the brow is the cheapest hair in the world and it
    // is the difference between "hair" and "helmet".
    b (-2.50,19.40, -2.70,   2.50, 20.50,  2.20, Surf::Hair),                 // crown
    b (-2.50,16.40,  1.50,   2.50, 19.80,  2.60, Surf::Hair),                 // back mass
    b (-2.70,16.60, -2.70,  -2.10, 19.80,  2.20, Surf::Hair),                 // side L
    b ( 2.10,16.60, -2.70,   2.70, 19.80,  2.20, Surf::Hair),                 // side R
    br(-2.05,18.70, -2.86,  -0.55, 19.70, -2.30,  0.0, 0.0, 14.0, Surf::Hair),// fringe lock L
    br(-0.60,18.85, -2.90,   0.70, 19.75, -2.34,  0.0, 0.0, -6.0, Surf::Hair),// fringe lock C
    br( 0.75,18.60, -2.86,   2.10, 19.70, -2.30,  0.0, 0.0,-16.0, Surf::Hair),// fringe lock R
    br(-2.30,19.90, -1.40,   0.40, 20.60,  1.60,-10.0, 0.0,  7.0, Surf::Hair),// swept crown lock
    b (-1.90,15.40,  1.80,   1.90, 17.00,  3.00, Surf::Hair),                 // nape tail past the collar
    br(-1.10,14.60,  1.90,   0.60, 17.20,  2.95,  8.0, 0.0,  6.0, Surf::Hair),// a strand off the tail
];

/// Auren **v2** — the 2026-08-14 proportion pass, kept verbatim as the *before*
/// plate for the sculpt pass above.
///
/// Same reason `AUREN_V1` is kept: `VOXELFORGE_CHARSHOT=auren-v2` and `=auren` are
/// two stages of ONE binary at ONE camera, so a before/after pair cannot differ by
/// build, shader cache, driver or framing — only by the geometry under test. v2's
/// proportions are correct and unchanged in v3; what the pair shows is form.
#[rustfmt::skip]
static AUREN_V2: &[Bx] = &[
    // ---- legs: right foot forward, weight on it ----------------------------
    b(-3.4,  0.0, -2.2,  -0.5,  1.1,  2.4, Surf::Skin),        // foot L (back)
    b(-3.2,  1.1, -1.5,  -0.7,  5.0,  1.6, Surf::Skin),        // shin L
    b(-3.3,  4.7, -1.7,  -0.6,  5.7,  1.8, Surf::Skin),        // knee L
    b(-3.5,  5.0, -1.8,  -0.5,  9.4,  1.9, Surf::Skin),        // thigh L
    b( 0.5,  0.0, -3.4,   3.4,  1.1,  1.2, Surf::Skin),        // foot R (forward)
    b( 0.7,  1.1, -2.7,   3.2,  5.0,  0.4, Surf::Skin),        // shin R
    b( 0.6,  4.7, -2.6,   3.3,  5.7,  0.7, Surf::Skin),        // knee R
    b( 0.5,  5.0, -2.2,   3.5,  9.4,  1.4, Surf::Skin),        // thigh R

    // ---- pelvis: a linen underwrap, so the BARE body still reads deliberate
    // instead of looking like an unfinished mannequin when the legs slot is empty.
    b(-3.6,  9.0, -2.1,   3.6, 10.6,  1.9, Surf::ClothLinen),

    // ---- torso: the V. Waist 6.0 wide, chest 8.8, deltoids out to 11.8 ------
    b(-3.0, 10.4, -2.0,   3.0, 12.6,  1.9, Surf::Skin),        // waist — the pinch
    b(-4.4, 12.4, -2.4,   4.4, 15.4,  2.1, Surf::Skin),        // chest
    b(-4.0, 15.1, -2.2,   4.0, 15.9,  1.9, Surf::Skin),        // trapezius
    b(-5.9, 13.8, -2.2,  -3.9, 15.7,  1.9, Surf::Skin),        // deltoid L
    b( 3.9, 13.8, -2.2,   5.9, 15.7,  1.9, Surf::Skin),        // deltoid R

    // ---- arms, hanging ------------------------------------------------------
    b(-5.7, 11.2, -2.0,  -4.0, 14.0,  1.6, Surf::Skin),        // upper arm L
    b( 4.0, 11.2, -2.0,   5.7, 14.0,  1.6, Surf::Skin),        // upper arm R
    b(-5.6,  7.9, -1.9,  -4.1, 11.4,  1.5, Surf::Skin),        // forearm L
    b( 4.1,  7.9, -1.9,   5.6, 11.4,  1.5, Surf::Skin),        // forearm R
    b(-5.7,  6.5, -2.1,  -4.0,  8.1,  1.4, Surf::Skin),        // hand L
    b( 4.0,  6.5, -2.1,   5.7,  8.1,  1.4, Surf::Skin),        // hand R

    // ---- neck + head. Head is 3.7 tall against a 20.5 body = 5.5 heads, and
    // 4.4 wide against an 11.8 shoulder span = 2.7 head-widths. That ratio pair
    // IS the "reads as a hero" fix — nothing about it is a texture or a decal.
    b(-1.5, 15.6, -1.4,   1.5, 16.4,  1.0, Surf::Skin),        // neck
    b(-2.2, 16.2, -2.5,   2.2, 19.9,  1.9, Surf::Skin),        // head

    // ---- hair: a layered mass with real thickness, per the §7 review ---------
    b(-2.5, 19.4, -2.7,   2.5, 20.5,  2.2, Surf::Hair),        // crown
    b(-2.5, 16.4,  1.5,   2.5, 19.8,  2.6, Surf::Hair),        // back mass
    b(-2.7, 16.6, -2.7,  -2.1, 19.8,  2.2, Surf::Hair),        // side L
    b( 2.1, 16.6, -2.7,   2.7, 19.8,  2.2, Surf::Hair),        // side R
    b(-2.1, 18.8, -2.75,  2.1, 19.65,-2.2, Surf::Hair),        // fringe
    b(-1.9, 15.4,  1.8,   1.9, 17.0,  3.0, Surf::Hair),        // nape tail past the collar
];

// --- Auren v1 (kept for the before/after plate) -----------------------------
//
// The 2026-08-13 body, VERBATIM apart from the mechanical `Tone`→`Surf` rename.
// It exists for one reason: `VOXELFORGE_CHARSHOT=auren-v1` and `=auren` are two
// stages of ONE binary at ONE camera, so the before/after pair cannot differ by
// build, shader cache, driver or framing — only by the geometry under test.
#[rustfmt::skip]
static AUREN_V1: &[Bx] = &[
    b(-3.0, 0.0,  0.2,  -0.5, 2.6,  3.2, Surf::ClothEspresso),
    b(-3.0, 2.6,  0.2,  -0.5, 7.2,  3.0, Surf::LeatherStrap),
    b( 0.5, 0.0, -3.2,   3.0, 2.6, -0.2, Surf::ClothEspresso),
    b( 0.5, 2.6, -3.0,   3.0, 7.2, -0.2, Surf::LeatherStrap),

    b(-3.5, 6.8, -2.6,   3.5, 12.4,  1.4, Surf::ClothWalnut),
    b(-3.8, 11.0,-2.9,   3.8, 12.6,  1.7, Surf::ClothEspresso),
    b(-3.9, 8.3, -2.9,   3.9,  9.2,  1.7, Surf::LeatherStrap),
    b(-1.2, 9.2, -3.0,   1.2, 12.2, -2.6, Surf::LeatherStrap),

    b( 2.4, 7.5, -3.4,   4.2,  9.1, -1.8, Surf::EmberHousing),
    b( 2.7, 7.8, -3.7,   3.9,  8.8, -3.3, Surf::Ember),

    b( 3.5, 9.0, -2.4,   5.5, 12.2,  1.2, Surf::ClothWalnut),
    b( 3.6, 6.6, -2.3,   5.4,  9.0,  1.1, Surf::Skin),
    b(-5.5, 9.0, -2.4,  -3.5, 12.2,  1.2, Surf::ClothWalnut),
    b(-5.4, 6.6, -2.3,  -3.6,  9.0,  1.1, Surf::Skin),

    b(-2.4, 12.4,-2.8,   2.4, 16.6,  2.0, Surf::Skin),
    b(-2.7, 15.9,-3.0,   2.7, 17.1,  2.3, Surf::Hair),
    b(-2.7, 12.6, 1.6,   2.7, 16.2,  2.6, Surf::Hair),
    b(-2.9, 12.8,-3.0,  -2.3, 16.2,  2.3, Surf::Hair),
    b( 2.3, 12.8,-3.0,   2.9, 16.2,  2.3, Surf::Hair),
    b(-2.3, 15.3,-3.1,   2.3, 16.1, -2.5, Surf::Hair),

    b(-6.2, 11.4,-3.2,   0.6, 13.0,  2.6, Surf::ClothEspresso),
    b(-6.4,  5.4,-3.2,  -1.6, 11.6,  2.8, Surf::ClothEspresso),
    b(-6.8,  2.2, 0.4,  -2.4,  5.6,  3.6, Surf::ClothEspresso),

    b(-2.6, 10.4, 2.2,   2.6, 13.0,  4.6, Surf::LeatherStrap),
    b(-3.3, 12.4, 2.6,   3.3, 13.8,  4.4, Surf::ClothWalnut),

    b( 3.2,  3.4, 0.4,   4.2,  9.0,  1.8, Surf::LeatherStrap),
    b( 3.3,  9.0, 0.1,   4.1, 10.3,  1.5, Surf::LeatherStrap),
    b( 2.4,  9.6, 0.3,   5.0, 10.2,  1.6, Surf::Stone),
];

// --- Elder Maren -----------------------------------------------------------
//
// The opposite silhouette grammar from every combat body in the cast: a wide,
// stepped, ground-hugging triangle with a head pushed forward into a stoop, and a
// THIRD ground contact — the cane — set clear of the widest robe box so it
// survives as its own shape in a black cutout.
#[rustfmt::skip]
static MAREN: &[Bx] = &[
    b(-6.0, 0.0, -4.0,   6.0,  3.0,  4.0, Surf::ClothWalnut),
    b(-5.2, 3.0, -3.6,   5.2,  6.5,  3.6, Surf::ClothWalnut),
    b(-4.4, 6.5, -3.0,   4.4,  9.5,  3.0, Surf::ClothWalnut),
    b(-3.6, 9.5, -2.6,   3.6, 11.8,  2.6, Surf::ClothWalnut),
    b(-6.2, 0.0, -4.2,   6.2,  0.9,  4.2, Surf::ClothEspresso), // hem band
    b(-5.4, 6.2, -3.8,   5.4,  7.0,  3.8, Surf::ClothEspresso), // waist sash

    b(-5.6, 11.0,-3.2,   5.6, 12.6,  3.2, Surf::ClothEspresso), // shawl
    b(-4.6,  8.4, 2.4,   4.6, 11.2,  3.4, Surf::ClothEspresso), // shawl drape

    b(-2.0, 12.4,-3.4,   2.0, 15.2,  0.4, Surf::Skin),
    b(-2.2, 14.7,-3.6,   2.2, 15.7,  0.6, Surf::HairGrey),
    b(-2.2, 12.6, 0.0,   2.2, 15.0,  0.8, Surf::HairGrey),
    b(-2.4, 12.8,-3.5,  -1.9, 15.3,  0.6, Surf::HairGrey),
    b( 1.9, 12.8,-3.5,   2.4, 15.3,  0.6, Surf::HairGrey),

    // right arm holds the forge-ember lantern out in FRONT of the body
    b( 3.4, 8.6, -3.0,   5.0, 11.6, -0.6, Surf::ClothWalnut),
    b( 3.5, 7.5, -3.4,   4.9,  8.7, -1.4, Surf::Skin),
    b( 4.0, 7.0, -3.0,   4.4,  7.6, -2.6, Surf::LeatherStrap),
    b( 3.2, 6.5, -3.9,   5.2,  7.1, -1.8, Surf::EmberHousing),
    b( 3.4, 5.1, -3.7,   5.0,  6.6, -2.0, Surf::Ember),
    b( 3.2, 4.6, -3.9,   5.2,  5.2, -1.8, Surf::EmberHousing),

    // left arm reaches OUT to the cane
    b(-6.0, 10.4,-2.8,  -3.6, 11.8, -1.0, Surf::ClothWalnut),
    b(-7.6,  9.4,-2.9,  -5.6, 10.6, -1.1, Surf::ClothWalnut),
    b(-7.9,  8.6,-2.9,  -6.9,  9.6, -1.3, Surf::Skin),

    b(-7.9, 0.0, -2.5,  -7.0, 14.0, -1.5, Surf::WoodHaft),      // the cane
    b(-8.3, 13.5,-2.9,  -6.6, 14.5, -1.1, Surf::WoodPale),      // knob
];

// --- Toma ------------------------------------------------------------------
//
// Half an adult's height and a head deliberately WIDER than the shoulders —
// "small = few, chunky blocks", never a scaled-down adult. The carved toy is held
// out past the chest so it clears the body in a cutout, and it is plain walnut:
// the one crafted object in the cast that does not glow, which is the whole point
// of the character.
#[rustfmt::skip]
static TOMA: &[Bx] = &[
    b(-2.0, 0.0, -1.2,  -0.4,  3.0,  1.2, Surf::ClothEspresso),
    b( 0.4, 0.0, -1.2,   2.0,  3.0,  1.2, Surf::ClothEspresso),
    b( 0.5, 1.0, -1.4,   2.1,  2.3, -1.0, Surf::ClothWalnut),   // the mismatched patch
    b(-2.4, 3.0, -1.6,   2.4,  5.6,  1.6, Surf::ClothEspresso), // overalls
    b(-2.6, 5.6, -1.8,   2.6,  6.7,  1.8, Surf::ClothLinen),
    b(-1.4, 3.4, -1.7,   1.4,  5.6, -1.5, Surf::ClothLinen),

    b( 2.6, 4.2, -1.8,   3.8,  6.4,  1.0, Surf::ClothLinen),
    b(-3.8, 4.2, -1.8,  -2.6,  6.4,  1.0, Surf::ClothLinen),
    b( 1.4, 4.2, -2.8,   3.0,  5.2, -1.4, Surf::Skin),
    b(-3.0, 4.2, -2.8,  -1.4,  5.2, -1.4, Surf::Skin),

    b(-2.8, 6.7, -2.4,   2.8, 10.2,  2.4, Surf::Skin),          // oversized head
    b(-3.0, 9.6, -2.6,   3.0, 10.7,  2.6, Surf::Hair),
    b(-3.0, 7.0,  2.0,   3.0, 10.0,  2.8, Surf::Hair),
    b(-2.7, 9.0, -2.7,   2.7, 10.0, -2.2, Surf::Hair),

    b(-0.9, 4.2, -3.4,   0.9,  5.4, -2.6, Surf::WoodHaft),      // the carved toy
    b(-0.6, 5.4, -3.3,   0.6,  6.3, -2.7, Surf::WoodHaft),
    b(-1.8, 4.7, -3.3,   1.8,  5.1, -2.7, Surf::WoodHaft),
];

// --- Garren, the Guard Husk ------------------------------------------------
//
// The armour is `HuskPlate` — half-metallic and rough, so it reads as iron that
// stopped being maintained rather than as fresh steel next to Auren's warplate.
// Two rules are enforced by absence: the helm is one unbroken slab with ZERO
// negative space (the emptiness is the horror beat), and the cracks are dull amber
// `Crack`, never teal. His spear lives in the WEAPON slot (`eq::SPEAR_GUARD`) —
// a second body proving the slot machinery is general.
#[rustfmt::skip]
static GARREN: &[Bx] = &[
    b(-3.5, 0.0, -2.0,  -0.8,  2.0,  2.0, Surf::HuskPlateDark), // boot L
    b( 0.8, 0.0, -2.0,   3.5,  2.0,  2.0, Surf::HuskPlateDark), // boot R
    b(-3.5, 2.0, -2.0,  -0.8,  8.2,  2.0, Surf::HuskPlate),     // greave L
    b( 0.8, 2.0, -2.0,   3.5,  8.2,  2.0, Surf::HuskPlate),     // greave R

    b(-4.5, 8.0, -3.0,   4.5, 15.2,  3.0, Surf::HuskPlate),     // cuirass
    b(-4.7, 8.0, -3.2,   4.7,  8.9,  3.2, Surf::HuskPlateDark), // crumbling lower edge
    b(-7.6, 12.9,-3.4,  -4.0, 15.6,  3.4, Surf::HuskPlate),     // pauldron L (the bulk)
    b( 4.0, 12.9,-3.4,   7.6, 15.6,  3.4, Surf::HuskPlate),     // pauldron R
    b(-7.8, 12.9,-3.5,  -6.9, 15.7,  3.5, Surf::HuskPlateDark), // chipped edge L
    b( 6.9, 12.9,-3.5,   7.8, 15.7,  3.5, Surf::HuskPlateDark), // chipped edge R

    b(-0.9, 9.2, -3.3,   0.9, 13.4, -2.9, Surf::Crack),
    b( 1.5, 10.2,-3.3,   2.6, 12.2, -2.9, Surf::Crack),
    b(-3.0, 11.6,-3.3,  -1.9, 13.8, -2.9, Surf::Crack),
    b(-4.8, 10.0,-1.2,  -4.4, 13.0,  1.2, Surf::Crack),

    b(-1.6, 15.0,-1.6,   1.6, 15.9,  1.6, Surf::HuskPlateDark), // neck
    b(-2.7, 15.7,-2.7,   2.7, 19.4,  2.7, Surf::HuskPlate),     // helm — one blank slab

    b( 4.6, 7.2, -2.4,   6.8, 13.6,  2.4, Surf::HuskPlate),     // arm R (on the haft)
    b(-6.8, 7.2, -2.4,  -4.6, 13.6,  2.4, Surf::HuskPlate),     // arm L
];

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

/// Build one character with an explicit loadout and stand it with its **feet** at
/// `feet`, facing `yaw` (radians, yaw 0 = facing −Z).
///
/// Returns the root entity. Every box — body and gear alike — is a descendant, so
/// a caller can move, hide or despawn a whole body with one handle, and
/// [`eq::equip`] can replace one slot without disturbing the rest.
pub fn spawn_character_with(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    who: Who,
    loadout: Loadout,
    feet: Vec3,
    yaw: f32,
) -> Entity {
    let root = commands
        .spawn((
            Transform::from_translation(feet).with_rotation(Quat::from_axis_angle(Vec3::Y, yaw)),
            Visibility::default(),
            Character(who),
            Name::new(who.display()),
        ))
        .id();

    // The base body goes straight under the root — it is not a slot and must
    // survive every equipment change.
    commands.entity(root).with_children(|p| {
        for bx in who.body() {
            let size = bx.size();
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(pal.get(bx.surf)),
                bx.transform(),
                Visibility::default(),
            ));
        }
    });

    let mut wardrobe = Wardrobe::new(loadout);
    for slot in Slot::ALL {
        let node = eq::spawn_slot(commands, meshes, pal, root, slot, loadout.get(slot));
        wardrobe.nodes[slot.idx()] = Some(node);
    }
    commands.entity(root).insert(wardrobe);

    println!(
        "CAST_SPAWN who={} root={:?} body_boxes={} gear_boxes={} slots_filled={}/{} \
         height={:.2}b feet=({:.1},{:.1},{:.1}) yaw={:.2}",
        who.id(),
        root,
        who.body().len(),
        loadout.box_count(),
        loadout.part_count(),
        SLOT_COUNT,
        who.height_with(loadout),
        feet.x,
        feet.y,
        feet.z,
        yaw
    );
    println!("CAST_WEAR who={} {}", who.id(), loadout.describe());
    report_materials(who, loadout);
    root
}

/// Back-compatible entry point for `scene.rs` (the playable village). Same
/// signature it has always had, so the gameplay lane needs no edit: builds a
/// palette, uses the character's default loadout, and spawns.
pub fn spawn_character(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    who: Who,
    feet: Vec3,
    yaw: f32,
    silhouette: bool,
) -> Entity {
    let pal = Palette::build(materials, silhouette);
    spawn_character_with(commands, meshes, &pal, who, who.default_loadout(), feet, yaw)
}

/// Count boxes per material class and print it. This is the evidence that the
/// "layered materials" claim is geometry and not a caption: a villager comes out
/// pure cloth, the warplate set comes out majority metal, and the numbers are
/// derived from the same tables the renderer reads.
fn report_materials(who: Who, l: Loadout) {
    let mut counts: Vec<(Mat, usize)> = Vec::new();
    let mut bump = |m: Mat| {
        if let Some(e) = counts.iter_mut().find(|(k, _)| *k == m) {
            e.1 += 1;
        } else {
            counts.push((m, 1));
        }
    };
    for bx in who.body() {
        bump(bx.surf.mat());
    }
    for bx in l.parts.iter().flatten().flat_map(|p| eq::boxes_for(p).iter()) {
        bump(bx.surf.mat());
    }
    let total: usize = counts.iter().map(|(_, n)| n).sum();
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    println!(
        "CAST_MATERIALS who={} total={} {}",
        who.id(),
        total,
        counts
            .iter()
            .map(|(m, n)| format!("{}={}", m.label(), n))
            .collect::<Vec<_>>()
            .join(" ")
    );
}

// ---------------------------------------------------------------------------
// The character shot (`shot_main.rs` stage)
// ---------------------------------------------------------------------------

/// What `VOXELFORGE_CHARSHOT` asked for.
///
/// ```text
///   line                     all four, one frame, shortest → tallest
///   silhouette | sil         the same line-up as flat black on a bright card
///   auren                    the NEW hero body, adventurer loadout
///   auren-v1 | v1            the 2026-08-13 body, same camera — the before plate
///   auren-v2 | v2            the 2026-08-14 proportion pass, pre-sculpt
///   quad | ladder | gear     ONE entity, FOUR sets: bare → villager → adventurer
///                            → warplate. Four captures, one camera, one sun.
///   auren-bare               the base body with every slot empty
///   auren-villager           \  the same body, other presets
///   auren-warplate           /
///   maren | toma | garren    one body, portrait framing
///   swap                     ONE entity, three outfit changes, three captures
///   weapons                  ONE entity, three weapon changes, three captures
///   catalog                  dump the equipment registry to JSON and exit
/// ```
///
/// Camera/sun/exposure take the same env overrides the rest of the shot lane uses
/// (`VOXELFORGE_CAM`, `VOXELFORGE_SUN`, `VOXELFORGE_AMBIENT`,
/// `VOXELFORGE_EXPOSURE`), so a re-frame costs a re-run, not a rebuild.
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    Line,
    Silhouette,
    /// `legacy` = an archived body table to draw INSTEAD of `who.body()`, with no
    /// slots. `None` is the live body plus the loadout.
    Solo { who: Who, loadout: Loadout, label: &'static str, legacy: Option<(&'static [Bx], &'static str)> },
    SwapOutfits,
    SwapWeapons,
    /// The four-plate gear ladder — bare → villager → adventurer → warplate — run
    /// on ONE entity at ONE camera under ONE sun. This is the stage the CEO's
    /// "does the gear actually go ON, and does it actually LOOK different" question
    /// is answered with, and it is deliberately not four separate processes: four
    /// runs could differ by driver state, shader-cache warmth or window placement,
    /// and then the sheet would be measuring the harness instead of the gear.
    GearLadder,
    Catalog,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct CharShot {
    pub stage: Stage,
}

impl CharShot {
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("VOXELFORGE_CHARSHOT").ok()?;
        let raw = raw.trim().to_ascii_lowercase();
        if raw.is_empty() || raw == "0" || raw == "off" {
            return None;
        }
        Some(CharShot { stage: Stage::parse(&raw) })
    }

    /// True when this stage drives its OWN captures (several per run) and
    /// `shot_main::screenshot_once` must therefore stand down. Anything else is a
    /// single still and goes through the shot binary's normal one-grab path.
    pub fn owns_capture(&self) -> bool {
        matches!(
            self.stage,
            Stage::SwapOutfits | Stage::SwapWeapons | Stage::GearLadder | Stage::Catalog
        )
    }
}

impl Stage {
    fn parse(raw: &str) -> Stage {
        match raw {
            "line" | "cast" => return Stage::Line,
            "silhouette" | "sil" => return Stage::Silhouette,
            "swap" | "outfits" | "loadouts" => return Stage::SwapOutfits,
            "weapons" | "weapon" | "arms" => return Stage::SwapWeapons,
            "quad" | "ladder" | "gear" | "sets" => return Stage::GearLadder,
            "catalog" | "catalogue" | "dump" => return Stage::Catalog,
            "auren-v1" | "auren_v1" | "v1" | "before" => {
                return Stage::Solo {
                    who: Who::Auren,
                    loadout: Loadout::EMPTY,
                    label: "auren-v1",
                    legacy: Some((AUREN_V1, "v1")),
                }
            }
            "auren-v2" | "auren_v2" | "v2" | "presculpt" => {
                return Stage::Solo {
                    who: Who::Auren,
                    loadout: Loadout::EMPTY,
                    label: "auren-v2",
                    legacy: Some((AUREN_V2, "v2")),
                }
            }
            _ => {}
        }
        // `auren-warplate`, `auren-bare`, … — a body plus a preset name.
        if let Some((body, outfit)) = raw.split_once('-') {
            if let (Some(who), Some((l, label))) = (Who::from_name(body), eq::preset(outfit)) {
                return Stage::Solo { who, loadout: l, label, legacy: None };
            }
        }
        if let Some(who) = Who::from_name(raw) {
            return Stage::Solo {
                who,
                loadout: who.default_loadout(),
                label: match who {
                    Who::Auren => "auren",
                    Who::Maren => "maren",
                    Who::Toma => "toma",
                    Who::Garren => "garren",
                },
                legacy: None,
            };
        }
        eprintln!("CHARSHOT: unknown stage {raw:?} — falling back to `line`");
        Stage::Line
    }

    fn silhouette(self) -> bool {
        matches!(self, Stage::Silhouette)
    }

    fn label(self) -> &'static str {
        match self {
            Stage::Line => "line",
            Stage::Silhouette => "silhouette",
            Stage::Solo { label, .. } => label,
            Stage::SwapOutfits => "swap",
            Stage::SwapWeapons => "weapons",
            Stage::GearLadder => "quad",
            Stage::Catalog => "catalog",
        }
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

/// The one camera the Auren before/after pair is shot at.
///
/// It is a CONSTANT, not a function of the subject's height, precisely because
/// the two bodies are different heights: auto-framing each one would silently
/// normalise away the very thing the plate is meant to show. 3.4 m of vertical
/// coverage holds v1 (2.14 b) and v2 (2.56 b) with the same headroom.
const AB_CAM: [f32; 7] = [-2.05, 1.62, -5.35, 0.0, 1.30, 0.0, 32.0];

/// The subject yaw for the A/B and swap plates: a few degrees off dead-on so the
/// cloak, the pauldron and the weapon are not edge-on to the lens.
const AB_YAW: f32 = 0.34;

/// Ground plane + golden-hour key + camera + the cast.
pub fn setup_charshot(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shot: Res<CharShot>,
) {
    let stage = shot.stage;
    let sil = stage.silhouette();

    // Printed on EVERY run, not just the before ones: the line that says the
    // pre-sculpt counts are real prefixes is only worth anything if it appears in
    // the after runlog too, next to the plate it is vouching for.
    let bad = eq::assert_presculpt_prefixes();
    if bad > 0 {
        eprintln!("CHARSHOT: {bad} part(s) have no usable pre-sculpt count — the A/B pair is NOT trustworthy");
    }

    // The catalogue dump is a build artefact, not a render: write it before the
    // scene so a failed frame still leaves the JSON on disk.
    if matches!(stage, Stage::Catalog) {
        let out = std::env::var("VOXELFORGE_CATALOG_OUT")
            .unwrap_or_else(|_| "assets/characters/equipment-catalog.json".to_string());
        if let Some(dir) = std::path::Path::new(&out).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&out, eq::catalog_json()) {
            Ok(()) => println!("CATALOG wrote {} parts to {out}", eq::PARTS.len()),
            Err(e) => eprintln!("CATALOG FAILED to write {out}: {e}"),
        }
    }

    let pal = Palette::build(&mut materials, sil);

    // ---- the cast ----------------------------------------------------------
    match stage {
        Stage::Line | Stage::Silhouette => {
            // Shortest → tallest, so the height ladder is the first thing the eye
            // gets: Toma 1.3 → Maren 2.0-but-wide → Auren 2.6 → Garren 3.6 (spear).
            let order = [Who::Toma, Who::Maren, Who::Auren, Who::Garren];
            let gap = 2.15;
            for (i, who) in order.iter().enumerate() {
                let x = (i as f32 - (order.len() as f32 - 1.0) * 0.5) * gap;
                let yaw = 0.20 * if x < 0.0 { 1.0 } else { -1.0 };
                spawn_character_with(
                    &mut commands,
                    &mut meshes,
                    &pal,
                    *who,
                    who.default_loadout(),
                    Vec3::new(x, 0.0, 0.0),
                    yaw,
                );
            }
        }
        Stage::Solo { who, loadout, legacy, .. } => {
            if let Some((boxes, tag)) = legacy {
                spawn_legacy_auren(
                    &mut commands,
                    &mut meshes,
                    &pal,
                    boxes,
                    tag,
                    Vec3::ZERO,
                    AB_YAW,
                );
            } else {
                spawn_character_with(
                    &mut commands,
                    &mut meshes,
                    &pal,
                    who,
                    loadout,
                    Vec3::ZERO,
                    AB_YAW,
                );
            }
        }
        Stage::SwapOutfits | Stage::SwapWeapons | Stage::GearLadder | Stage::Catalog => {
            // One body. The timeline below never despawns it.
            let start = match stage {
                Stage::SwapWeapons => eq::adventurer(),
                // The ladder opens on the BARE body — plate 1 is the base sculpt
                // with all six slots empty, which is what makes plates 2–4 a
                // statement about the gear rather than about the character.
                Stage::GearLadder => Loadout::EMPTY,
                _ => eq::villager(),
            };
            spawn_character_with(
                &mut commands,
                &mut meshes,
                &pal,
                Who::Auren,
                start,
                Vec3::ZERO,
                AB_YAW,
            );
        }
    }

    commands.insert_resource(pal.clone());

    // ---- ground ------------------------------------------------------------
    let ground_mat = if sil {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.93, 0.90, 0.84),
            unlit: true,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb_u8(0x4A, 0x3A, 0x24),
            perceptual_roughness: 0.95,
            ..default()
        })
    };
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(60.0, 1.0, 60.0))),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, -0.5, 0.0),
        Visibility::default(),
    ));

    // ---- key light ---------------------------------------------------------
    // Low, raking, golden-hour — the direction the beauty shot is graded on
    // (look-bible §4). It matters more now than it did last pass: a raking key is
    // what makes a `metallic: 1.0` pauldron throw a specular streak that a cloth
    // cloak at the same hue physically cannot. Flatten the sun and the material
    // separation this whole pass is about disappears.
    if !sil {
        let [elev, azim, illum] =
            env_floats::<3>("VOXELFORGE_SUN").unwrap_or([24.0, 214.0, 13000.0]);
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
        // A dim, cool-neutral rim from the opposite side. Without it the espresso
        // cloak and the umber hair merge into one black mass exactly where the
        // design says they should read as two materials.
        commands.spawn((
            DirectionalLight {
                color: Color::srgb(0.70, 0.72, 0.80),
                illuminance: 1800.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(-14.0, 8.0, 14.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
        ));
    }

    // ---- camera ------------------------------------------------------------
    let default_cam: [f32; 7] = match stage {
        Stage::Line | Stage::Silhouette => [0.0, 1.75, -10.6, 0.0, 1.45, 0.0, 34.0],
        // Every single-body stage — before, after, all three outfits, the weapon
        // ladder — shares ONE camera. That is what makes the plates comparable.
        _ => AB_CAM,
    };
    let cam = env_floats::<7>("VOXELFORGE_CAM").unwrap_or(default_cam);
    let eye = Vec3::new(cam[0], cam[1], cam[2]);
    let target = Vec3::new(cam[3], cam[4], cam[5]);

    let clear = if sil {
        Color::srgb(0.93, 0.90, 0.84)
    } else {
        Color::srgb(0.055, 0.038, 0.028)
    };
    let mut cam_cmd = commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(clear), ..default() },
        Projection::Perspective(PerspectiveProjection {
            fov: cam[6].to_radians(),
            near: 0.05,
            ..default()
        }),
        Transform::from_translation(eye).looking_at(target, Vec3::Y),
        Msaa::Off,
        Tonemapping::AcesFitted,
        Exposure { ev100: env_f32("VOXELFORGE_EXPOSURE").unwrap_or(if sil { 9.7 } else { 9.9 }) },
    ));
    if !sil {
        cam_cmd.insert(AmbientLight {
            color: Color::srgb(0.784, 0.541, 0.180),
            brightness: env_f32("VOXELFORGE_AMBIENT").unwrap_or(2400.0),
            affects_lightmapped_meshes: false,
        });
    }

    println!(
        "CHARSHOT stage={} cam eye=({:.2},{:.2},{:.2}) -> ({:.2},{:.2},{:.2}) fov={:.1}",
        stage.label(),
        eye.x,
        eye.y,
        eye.z,
        target.x,
        target.y,
        target.z,
        cam[6]
    );
}

/// Spawn an ARCHIVED body table (v1 or v2) with no slots at all.
///
/// v1 has no slots by definition — it predates them; v2's gear lives in
/// `equipment.rs` and is shared with v3, so shooting v2 bare is the honest way to
/// isolate "what changed in the BODY". Either way this goes through its own tiny
/// path rather than faking a Loadout, so a before plate is literally the old
/// table's geometry and not a reconstruction of it.
fn spawn_legacy_auren(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    boxes: &'static [Bx],
    tag: &'static str,
    feet: Vec3,
    yaw: f32,
) -> Entity {
    let root = commands
        .spawn((
            Transform::from_translation(feet).with_rotation(Quat::from_axis_angle(Vec3::Y, yaw)),
            Visibility::default(),
            Character(Who::Auren),
            Name::new(format!("Auren ({tag})")),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        for bx in boxes {
            let size = bx.size();
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(pal.get(bx.surf)),
                bx.transform(),
                Visibility::default(),
            ));
        }
    });
    let h = boxes.iter().fold(0.0f32, |m, bx| m.max(bx.top()));
    println!(
        "CAST_SPAWN who=auren-{tag} root={root:?} body_boxes={} gear_boxes=0 slots_filled=0/{SLOT_COUNT} \
         height={h:.2}b feet=({:.1},{:.1},{:.1}) yaw={yaw:.2}",
        boxes.len(),
        feet.x,
        feet.y,
        feet.z
    );
    root
}

// ---------------------------------------------------------------------------
// The swap proof
// ---------------------------------------------------------------------------

/// Frame numbers, not seconds. A wall-clock deadline overshoots by however big the
/// last delta was, which would let a busy machine catch a capture mid-swap; a
/// counted frame cannot.
const WARM_FRAMES: u32 = 96;
const STEP_FRAMES: u32 = 54;
const TAIL_FRAMES: u32 = 60;

#[derive(Resource)]
pub struct SwapRun {
    frame: u32,
    /// How many captures have been requested so far.
    taken: usize,
    base: String,
    /// Filled on the first frame, once `spawn_character_with`'s commands have been
    /// applied and the root is actually queryable.
    root: Option<Entity>,
}

impl Default for SwapRun {
    fn default() -> Self {
        let base = std::env::var("VOXELFORGE_SHOT")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "docs/assets/characters/_fl_charshot.png".to_string());
        SwapRun { frame: 0, taken: 0, base, root: None }
    }
}

impl SwapRun {
    /// `…/_fl_swap.png` + (2, "adventurer") → `…/_fl_swap-2-adventurer.png`.
    fn path_for(&self, n: usize, label: &str) -> String {
        let stem = self.base.strip_suffix(".png").unwrap_or(&self.base);
        format!("{stem}-{n}-{label}.png")
    }
}

/// Runs the outfit / weapon ladders on ONE entity and captures between each step.
///
/// The whole point is what it does NOT do: no `despawn` of the root, no second
/// `spawn_character`, no reload. Every capture prints the root entity id, so the
/// three frames can be checked to have come off one body — that check is the
/// evidence, not the prose above it.
pub fn charshot_timeline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    shot: Res<CharShot>,
    pal: Option<Res<Palette>>,
    mut run: ResMut<SwapRun>,
    mut wardrobes: Query<(Entity, &mut Wardrobe), With<Character>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !shot.owns_capture() {
        return;
    }
    run.frame += 1;

    // The catalogue stage has nothing to render; it wrote its JSON in Startup.
    if matches!(shot.stage, Stage::Catalog) {
        if run.frame >= 4 {
            exit.write(AppExit::Success);
        }
        return;
    }

    let Some(pal) = pal else { return };
    let Ok((root, mut wardrobe)) = wardrobes.single_mut() else { return };
    if run.root.is_none() {
        run.root = Some(root);
        println!("SWAP_ROOT root={root:?} — every capture below is this same entity");
    }

    // The gear ladder is four plates (bare + three outfits); the two swap proofs
    // are three. Derived from the stage rather than hardcoded, because the exit
    // frame below is computed from it and a mismatch silently truncates the sheet.
    let steps: usize = match shot.stage {
        Stage::GearLadder => eq::gear_ladder().len(),
        _ => 3,
    };
    // Capture at WARM, WARM+STEP, WARM+2·STEP…; equip the next set two frames after
    // each capture so the grab cannot race the change it is meant to precede.
    for n in 0..steps {
        let cap_at = WARM_FRAMES + STEP_FRAMES * n as u32;
        if run.taken == n && run.frame == cap_at {
            let label = current_label(shot.stage, n);
            let path = run.path_for(n + 1, label);
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
            run.taken = n + 1;
            println!(
                "SWAP_CAPTURE {}/{} root={root:?} wearing[{}] -> {path}",
                n + 1,
                steps,
                wardrobe.loadout.describe()
            );
        }
        if run.taken == n + 1 && run.frame == cap_at + 2 && n + 1 < steps {
            match shot.stage {
                Stage::GearLadder => {
                    let (label, next) = eq::gear_ladder()[n + 1];
                    println!("SWAP_APPLY set={label} on root={root:?} (root NOT respawned)");
                    eq::equip_loadout(
                        &mut commands,
                        &mut meshes,
                        &pal,
                        root,
                        &mut wardrobe,
                        next,
                    );
                }
                Stage::SwapOutfits => {
                    let (label, next) = eq::outfit_ladder()[n + 1];
                    println!("SWAP_APPLY outfit={label} on root={root:?} (root NOT respawned)");
                    eq::equip_loadout(
                        &mut commands,
                        &mut meshes,
                        &pal,
                        root,
                        &mut wardrobe,
                        next,
                    );
                }
                Stage::SwapWeapons => {
                    let (label, part) = eq::weapon_ladder()[n + 1];
                    println!("SWAP_APPLY weapon={label} on root={root:?} (weapon slot only)");
                    eq::equip(
                        &mut commands,
                        &mut meshes,
                        &pal,
                        root,
                        &mut wardrobe,
                        Slot::Weapon,
                        Some(part),
                    );
                }
                _ => {}
            }
        }
    }

    if run.frame >= WARM_FRAMES + STEP_FRAMES * (steps as u32 - 1) + TAIL_FRAMES {
        println!("SWAP_DONE captures={} root={root:?}", run.taken);
        exit.write(AppExit::Success);
    }
}

fn current_label(stage: Stage, n: usize) -> &'static str {
    match stage {
        Stage::GearLadder => eq::gear_ladder()[n].0,
        Stage::SwapOutfits => eq::outfit_ladder()[n].0,
        Stage::SwapWeapons => eq::weapon_ladder()[n].0,
        _ => "frame",
    }
}
