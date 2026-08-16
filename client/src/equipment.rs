//! Slot-based, hot-swappable equipment for the Voxelforge cast.
//!
//! ## What this file is for
//!
//! Before it, a character was ONE static `&[Bx]` — a single frozen list of voxel
//! boxes. Changing what someone wore meant editing that list and rebuilding, and
//! "give the player a new sword" had no landing site at all.
//!
//! This module splits a body into a **base body** (skin, hair, the parts nobody
//! takes off) plus **six independent equipment slots**:
//!
//! ```text
//!   Head    hood / kerchief / helm
//!   Torso   tunic / jerkin / cuirass
//!   Legs    trousers / breeches+boots / greaves
//!   Hands   wraps / gloves+bracers / gauntlets
//!   Weapon  whatever is in the right hand
//!   Back    cloak, pack, cape
//! ```
//!
//! Each slot is a separate child entity under the character root, tagged with
//! [`SlotNode`]. Swapping one is [`equip`]: despawn that slot's node, spawn the new
//! part's boxes in its place. **Nothing else on the body is touched** — same root
//! entity, same transform, same other five slots. That is the property the whole
//! future item system needs, so it is proven by construction rather than asserted:
//! `VOXELFORGE_CHARSHOT=swap` runs three full outfit changes on one entity inside
//! one process and prints the entity id each time.
//!
//! ## Materials are layered for real (the second half of the brief)
//!
//! The old palette was 11 flat colours that all shared one roughness — the whole
//! cast rendered as "voxel painted N colours". [`Surf`] replaces it with 26
//! surfaces carrying a real **material class**:
//!
//! | class   | roughness | metallic | what it does under the key light          |
//! |---------|-----------|----------|-------------------------------------------|
//! | cloth   | 0.94–0.97 | 0.0      | no highlight at all — pure diffuse falloff |
//! | leather | 0.44–0.62 | 0.0      | a broad soft sheen along the top faces     |
//! | metal   | 0.22–0.40 | 0.85–1.0 | a hard specular streak; tints the bounce   |
//! | skin    | 0.74      | 0.0      | soft, low reflectance                      |
//! | wood    | 0.80–0.92 | 0.0      | flat, slightly warm                        |
//!
//! So a steel pauldron and an espresso cloak lit by the same sun no longer differ
//! only in hue — one glints and one does not, from any angle.
//!
//! ## Why it is `bevy`-only
//!
//! Same rule as `characters.rs` and `vfx.rs`: zero `crate::` references, so this
//! file can be `#[path]`-included by the isolated shot binary AND `mod`-declared by
//! `main.rs`. The contact sheet is therefore rendered from the *same* gear tables
//! the game spawns, not from a look-alike authored twice.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Authoring units + geometry
// ---------------------------------------------------------------------------

/// One authoring unit = ⅛ of a world block. Every coordinate in this file and in
/// `characters.rs` is in these, so "the hero is 2.5 blocks tall" reads as `y`
/// topping out at 20 and a body can be re-proportioned without touching a metre.
pub const VX: f32 = 0.125;

/// One axis-aligned voxel box in `VX` units, relative to the character's own
/// origin: **feet centre**, `+Y` up, facing `−Z` (the engine's yaw-0 convention).
///
/// Equipment boxes live in the *same* space as body boxes — a gauntlet is authored
/// where the forearm is, not in some glove-local frame. That is deliberate: it
/// means a part can be read, diffed and eyeballed against the body it covers
/// without mentally composing two transforms.
#[derive(Clone, Copy, Debug)]
pub struct Bx {
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    pub surf: Surf,
    /// Euler XYZ in **degrees**, applied about this box's own centre.
    ///
    /// This is the single most important field in the file for how the cast reads.
    /// An all-axis-aligned body is the thing that makes voxel characters look like
    /// stacked crates no matter how many boxes you spend: every silhouette edge is
    /// either vertical or horizontal, so the eye reads *masonry*, not anatomy. One
    /// 12° roll on a deltoid, 8° on a hanging forearm and 30° on a cloak flare puts
    /// diagonals in the outline, and diagonals are what the brain reads as a
    /// shoulder, a limb and cloth.
    ///
    /// `[0,0,0]` (what [`b`] builds) is a plain axis-aligned box, so every table
    /// written before this field existed is unchanged and un-rotated.
    pub rot: [f32; 3],
}

/// An axis-aligned box — the workhorse.
pub const fn b(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, surf: Surf) -> Bx {
    Bx { lo: [x0, y0, z0], hi: [x1, y1, z1], surf, rot: [0.0, 0.0, 0.0] }
}

/// A **rotated** box: same span, then turned about its own centre by
/// `(rx, ry, rz)` degrees. See [`Bx::rot`] for why this exists.
///
/// A 45° roll on a thin slab is also the cheap chamfer used all over the armour —
/// a bevel that catches the key light in a band no flat plate face can produce.
#[allow(clippy::too_many_arguments)]
pub const fn br(
    x0: f32,
    y0: f32,
    z0: f32,
    x1: f32,
    y1: f32,
    z1: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    surf: Surf,
) -> Bx {
    Bx { lo: [x0, y0, z0], hi: [x1, y1, z1], surf, rot: [rx, ry, rz] }
}

impl Bx {
    pub fn size(&self) -> Vec3 {
        Vec3::new(
            (self.hi[0] - self.lo[0]) * VX,
            (self.hi[1] - self.lo[1]) * VX,
            (self.hi[2] - self.lo[2]) * VX,
        )
    }
    pub fn centre(&self) -> Vec3 {
        Vec3::new(
            (self.hi[0] + self.lo[0]) * 0.5 * VX,
            (self.hi[1] + self.lo[1]) * 0.5 * VX,
            (self.hi[2] + self.lo[2]) * 0.5 * VX,
        )
    }

    pub fn rotation(&self) -> Quat {
        if self.rot == [0.0, 0.0, 0.0] {
            Quat::IDENTITY
        } else {
            Quat::from_euler(
                EulerRot::XYZ,
                self.rot[0].to_radians(),
                self.rot[1].to_radians(),
                self.rot[2].to_radians(),
            )
        }
    }

    /// Where this box's cuboid mesh goes. Rotation is about the centre, so the
    /// authored span still says where the part sits — only its facing changes.
    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.centre()).with_rotation(self.rotation())
    }

    /// World-space top of the box **after** rotation.
    ///
    /// Not `hi[1] * VX`: a rolled box reaches higher than its authored span, and
    /// height reports / portrait framing read this. Standard rotated-AABB extent —
    /// the Y half-extent is `Σ|R[1][j]| · half[j]`.
    pub fn top(&self) -> f32 {
        if self.rot == [0.0, 0.0, 0.0] {
            return self.hi[1] * VX;
        }
        let m = Mat3::from_quat(self.rotation());
        let h = self.size() * 0.5;
        let ext = m.x_axis.y.abs() * h.x + m.y_axis.y.abs() * h.y + m.z_axis.y.abs() * h.z;
        self.centre().y + ext
    }
}

// ---------------------------------------------------------------------------
// Surfaces — colour AND material class
// ---------------------------------------------------------------------------

/// Which physical family a surface belongs to. Only used to derive PBR numbers,
/// but kept as a named type so "this is leather" is a statement in the geometry
/// tables rather than a roughness float somebody has to reverse-engineer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mat {
    Cloth,
    Leather,
    Metal,
    Skin,
    Hair,
    Wood,
    Stone,
    Glow,
}

impl Mat {
    pub fn label(self) -> &'static str {
        match self {
            Mat::Cloth => "cloth",
            Mat::Leather => "leather",
            Mat::Metal => "metal",
            Mat::Skin => "skin",
            Mat::Hair => "hair",
            Mat::Wood => "wood",
            Mat::Stone => "stone",
            Mat::Glow => "glow",
        }
    }
}

/// Every surface a body in this cast is allowed to wear.
///
/// Deliberately an enum, not free hex: a new colour has to be added *here*, next
/// to its material class, which is where the look-bible check happens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surf {
    // ---- cloth: matte, zero specular. 85% of every villager. ----------------
    ClothWalnut,
    ClothEspresso,
    ClothLinen,
    ClothMoss,
    ClothCrimson,
    // ---- leather: the middle material. Broad soft sheen, no metallic tint. --
    LeatherDark,
    LeatherMid,
    LeatherWorn,
    LeatherStrap,
    // ---- metal: the only surfaces that glint. --------------------------------
    SteelBright,
    SteelDark,
    SteelRust,
    Brass,
    // ---- husk plate: dead, half-metallic. Corrupted iron, not fresh steel. ---
    HuskPlate,
    HuskPlateDark,
    // ---- organic ------------------------------------------------------------
    Skin,
    /// Skin in its own shadow — under the jaw, inside an eye socket, the crease of
    /// an elbow. Painted-in occlusion: at this scale a 3-vx-deep socket produces no
    /// real shadow of its own, so the *form* has to carry the value change or the
    /// face flattens back into one lit slab.
    SkinShade,
    /// Warmer, redder skin: lips, the tip of the nose, knuckles.
    SkinFlush,
    /// Eye white. Not paper-white — a warm off-white, because a pure #FFF sclera at
    /// this size reads as a hole punched in the head.
    EyeWhite,
    /// Iris + pupil as one dark, low-roughness box: the only place on a face where
    /// a specular pin is *wanted*, and the thing that decides whether a character
    /// is looking at you or is a mannequin.
    EyePupil,
    Hair,
    HairGrey,
    // ---- mundane craft ------------------------------------------------------
    WoodHaft,
    WoodPale,
    Wicker,
    Stone,
    StoneDark,
    // ---- narrative emissive (bible rule #4) ---------------------------------
    Ember,
    EmberHousing,
    Crack,
}

/// The full PBR description of one surface.
pub struct Props {
    pub rgb: (u8, u8, u8),
    pub mat: Mat,
    pub rough: f32,
    pub metal: f32,
    pub refl: f32,
    pub emissive: LinearRgba,
}

const fn p(rgb: (u8, u8, u8), mat: Mat, rough: f32, metal: f32, refl: f32) -> Props {
    Props { rgb, mat, rough, metal, refl, emissive: LinearRgba::BLACK }
}

const fn pe(rgb: (u8, u8, u8), rough: f32, refl: f32, e: LinearRgba) -> Props {
    Props { rgb, mat: Mat::Glow, rough, metal: 0.0, refl, emissive: e }
}

impl Surf {
    pub fn props(self) -> Props {
        use Mat::*;
        match self {
            // Cloth: roughness pinned near 1.0 and reflectance floored, so a
            // tunic has literally nowhere for a highlight to form.
            Surf::ClothWalnut => p((0x6B, 0x4A, 0x2E), Cloth, 0.96, 0.0, 0.06),
            Surf::ClothEspresso => p((0x3A, 0x27, 0x16), Cloth, 0.97, 0.0, 0.05),
            Surf::ClothLinen => p((0xC4, 0xA8, 0x7E), Cloth, 0.94, 0.0, 0.08),
            Surf::ClothMoss => p((0x5A, 0x60, 0x42), Cloth, 0.95, 0.0, 0.07),
            Surf::ClothCrimson => p((0x7A, 0x34, 0x28), Cloth, 0.95, 0.0, 0.07),

            // Leather: the tell is REFLECTANCE, not colour. 0.42 at roughness
            // 0.48 gives a wide soft band down the top of a bracer that cloth at
            // the same hue simply cannot produce.
            Surf::LeatherDark => p((0x33, 0x24, 0x1A), Leather, 0.48, 0.0, 0.42),
            Surf::LeatherMid => p((0x59, 0x40, 0x2A), Leather, 0.44, 0.0, 0.46),
            Surf::LeatherWorn => p((0x7A, 0x5C, 0x3C), Leather, 0.62, 0.0, 0.30),
            Surf::LeatherStrap => p((0x4A, 0x32, 0x20), Leather, 0.55, 0.0, 0.36),

            // Metal: metallic 1.0 means base_color tints the SPECULAR, so brass
            // throws a warm streak and steel a neutral one — the cheapest way to
            // make two greys read as two different metals in a voxel frame.
            Surf::SteelBright => p((0xC6, 0xC2, 0xB6), Metal, 0.24, 1.0, 0.5),
            Surf::SteelDark => p((0x7E, 0x7A, 0x70), Metal, 0.38, 1.0, 0.5),
            Surf::SteelRust => p((0x8A, 0x5A, 0x38), Metal, 0.68, 0.85, 0.4),
            Surf::Brass => p((0xC0, 0x8A, 0x3E), Metal, 0.28, 1.0, 0.5),

            // Husk plate: half-metallic and rough. Reads as iron that stopped
            // being maintained a long time ago — never as polished armour.
            Surf::HuskPlate => p((0xB9, 0xA9, 0x8C), Metal, 0.72, 0.60, 0.35),
            Surf::HuskPlateDark => p((0x8A, 0x7A, 0x5C), Metal, 0.80, 0.50, 0.30),

            Surf::Skin => p((0xD9, 0xB0, 0x8C), Skin, 0.74, 0.0, 0.22),
            // ~62% of `Skin`'s luminance. Deep enough to read as an eye socket
            // under a flat sky, shallow enough that it never reads as dirt.
            Surf::SkinShade => p((0x92, 0x6C, 0x52), Skin, 0.78, 0.0, 0.18),
            Surf::SkinFlush => p((0xC4, 0x87, 0x72), Skin, 0.70, 0.0, 0.26),
            Surf::EyeWhite => p((0xE8, 0xDC, 0xCA), Skin, 0.42, 0.0, 0.35),
            // Roughness 0.14 is the whole point: it is the sharpest highlight on
            // the entire body, so the eye lands on the face first.
            Surf::EyePupil => p((0x22, 0x18, 0x14), Skin, 0.14, 0.0, 0.55),
            // Hair gets a real sheen — without it, umber hair and espresso cloth
            // merge into one black mass exactly where they should read as two.
            Surf::Hair => p((0x2A, 0x1B, 0x12), Hair, 0.55, 0.0, 0.24),
            Surf::HairGrey => p((0xC9, 0xC0, 0xB4), Hair, 0.60, 0.0, 0.22),

            Surf::WoodHaft => p((0x6B, 0x4A, 0x2E), Wood, 0.80, 0.0, 0.14),
            Surf::WoodPale => p((0xA9, 0x8A, 0x5C), Wood, 0.82, 0.0, 0.14),
            Surf::Wicker => p((0xB5, 0x8C, 0x50), Wood, 0.92, 0.0, 0.08),
            Surf::Stone => p((0xB9, 0xA9, 0x8C), Stone, 0.84, 0.0, 0.10),
            Surf::StoneDark => p((0x8A, 0x7A, 0x5C), Stone, 0.88, 0.0, 0.09),

            Surf::Ember => pe((0xFF, 0xD9, 0x8A), 0.55, 0.3, LinearRgba::rgb(3.4, 1.9, 0.62)),
            Surf::EmberHousing => p((0xC8, 0x8A, 0x4A), Metal, 0.42, 0.85, 0.45),
            // A husk is corrupted, not Shaper-lit: a third of the ember's punch,
            // and dull amber — never teal, because the escalation ladder needs
            // somewhere to go.
            Surf::Crack => pe((0xC8, 0x76, 0x3C), 0.70, 0.2, LinearRgba::rgb(1.10, 0.42, 0.11)),
        }
    }

    pub fn mat(self) -> Mat {
        self.props().mat
    }

    fn idx(self) -> usize {
        Surf::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    pub const ALL: [Surf; 30] = [
        Surf::ClothWalnut,
        Surf::ClothEspresso,
        Surf::ClothLinen,
        Surf::ClothMoss,
        Surf::ClothCrimson,
        Surf::LeatherDark,
        Surf::LeatherMid,
        Surf::LeatherWorn,
        Surf::LeatherStrap,
        Surf::SteelBright,
        Surf::SteelDark,
        Surf::SteelRust,
        Surf::Brass,
        Surf::HuskPlate,
        Surf::HuskPlateDark,
        Surf::Skin,
        Surf::SkinShade,
        Surf::SkinFlush,
        Surf::EyeWhite,
        Surf::EyePupil,
        Surf::Hair,
        Surf::HairGrey,
        Surf::WoodHaft,
        Surf::WoodPale,
        Surf::Wicker,
        Surf::Stone,
        Surf::StoneDark,
        Surf::Ember,
        Surf::EmberHousing,
        Surf::Crack,
    ];
}

/// One `StandardMaterial` per surface, built once and then shared by every box,
/// every slot and every swap. Cloned handles, so a hot-swap allocates geometry
/// only — never a new material.
#[derive(Clone, Resource)]
pub struct Palette {
    mats: Vec<Handle<StandardMaterial>>,
}

impl Palette {
    /// `silhouette = true` collapses everything to flat unlit black — the
    /// black-cutout test the design docs claim every character passes, run for
    /// real instead of asserted in prose.
    pub fn build(materials: &mut Assets<StandardMaterial>, silhouette: bool) -> Self {
        let mats = Surf::ALL
            .iter()
            .map(|s| {
                if silhouette {
                    materials.add(StandardMaterial {
                        base_color: Color::BLACK,
                        unlit: true,
                        ..default()
                    })
                } else {
                    let pr = s.props();
                    materials.add(StandardMaterial {
                        base_color: Color::srgb_u8(pr.rgb.0, pr.rgb.1, pr.rgb.2),
                        emissive: pr.emissive,
                        perceptual_roughness: pr.rough,
                        metallic: pr.metal,
                        reflectance: pr.refl,
                        ..default()
                    })
                }
            })
            .collect();
        Palette { mats }
    }

    pub fn get(&self, s: Surf) -> Handle<StandardMaterial> {
        self.mats[s.idx()].clone()
    }
}

// ---------------------------------------------------------------------------
// Slots
// ---------------------------------------------------------------------------

pub const SLOT_COUNT: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Slot {
    Head,
    Torso,
    Legs,
    Hands,
    Weapon,
    Back,
}

impl Slot {
    pub const ALL: [Slot; SLOT_COUNT] =
        [Slot::Head, Slot::Torso, Slot::Legs, Slot::Hands, Slot::Weapon, Slot::Back];

    pub fn idx(self) -> usize {
        match self {
            Slot::Head => 0,
            Slot::Torso => 1,
            Slot::Legs => 2,
            Slot::Hands => 3,
            Slot::Weapon => 4,
            Slot::Back => 5,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Slot::Head => "head",
            Slot::Torso => "torso",
            Slot::Legs => "legs",
            Slot::Hands => "hands",
            Slot::Weapon => "weapon",
            Slot::Back => "back",
        }
    }

    pub fn from_id(s: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|x| x.id() == s.trim().to_ascii_lowercase())
    }
}

/// Marks the container entity that holds one slot's boxes. Swapping a slot means
/// despawning the entity carrying this and spawning a fresh one — which is why
/// every part's geometry has to be a child of it, never of the root directly.
#[derive(Component, Clone, Copy, Debug)]
pub struct SlotNode(pub Slot);

// ---------------------------------------------------------------------------
// Parts
// ---------------------------------------------------------------------------

/// One wearable thing. `boxes` are authored in body space (see [`Bx`]).
pub struct Part {
    /// Stable id — this is what a save file or an item stack would store.
    pub id: &'static str,
    pub name: &'static str,
    pub slot: Slot,
    /// One line for the design doc / catalogue dump.
    pub note: &'static str,
    pub boxes: &'static [Bx],
}

// A part is identified by its `id`, not by its address: two `&'static Part` are
// the same item iff they name the same item. Hand-written rather than derived
// because deriving would demand `Bx: PartialEq` on a 26-variant surface table for
// no benefit — nobody ever asks whether two DIFFERENT parts happen to share
// geometry, only whether a slot still holds the thing it held last frame.
impl PartialEq for Part {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Part {}

impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.slot.id(), self.id)
    }
}

impl Part {
    /// The material classes this part actually puts on screen, in table order.
    /// Used by the catalogue dump and by the "is this really layered?" report.
    pub fn mats(&self) -> Vec<Mat> {
        let mut out: Vec<Mat> = Vec::new();
        for bx in self.boxes {
            let m = bx.surf.mat();
            if !out.contains(&m) {
                out.push(m);
            }
        }
        out
    }
}

/// A full set of worn parts. `None` = that slot is empty, and the base body has
/// to look deliberate underneath it — which is why the body carries a linen
/// underwrap rather than stopping at bare skin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Loadout {
    pub parts: [Option<&'static Part>; SLOT_COUNT],
}

impl Loadout {
    pub const EMPTY: Loadout = Loadout { parts: [None; SLOT_COUNT] };

    pub fn get(&self, slot: Slot) -> Option<&'static Part> {
        self.parts[slot.idx()]
    }

    pub fn with(mut self, part: &'static Part) -> Self {
        self.parts[part.slot.idx()] = Some(part);
        self
    }

    pub fn without(mut self, slot: Slot) -> Self {
        self.parts[slot.idx()] = None;
        self
    }

    /// `head=hood_travel torso=jerkin_leather …` — the one-line form printed next
    /// to every capture so a frame can be traced back to what was worn in it.
    pub fn describe(&self) -> String {
        Slot::ALL
            .iter()
            .map(|s| {
                format!(
                    "{}={}",
                    s.id(),
                    self.get(*s).map(|p| p.id).unwrap_or("-")
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn part_count(&self) -> usize {
        self.parts.iter().filter(|p| p.is_some()).count()
    }

    /// Boxes this loadout will actually put on screen — so it follows the sculpt
    /// lever. A count that ignored the lever would print the after number beside a
    /// before plate, which is exactly the sort of caption/pixel mismatch that
    /// makes a whole runlog worthless.
    pub fn box_count(&self) -> usize {
        self.parts.iter().flatten().map(|p| boxes_for(p).len()).sum()
    }
}

// ===========================================================================
// THE GEAR TABLES
//
// Everything below is authored against the Auren base body in `characters.rs`
// (feet at y=0, skull top at y=19.9, shoulder span ±5.9, waist ±3.0). A part that
// wants to sit on a different body has to be re-authored — which is the honest
// trade for being able to read a bracer's position without composing transforms.
// ===========================================================================

// --- HEAD ------------------------------------------------------------------

/// Villager. Barely more than a band — but it is asymmetric (the tail hangs on
/// one side only), so even this reads in a black cutout.
#[rustfmt::skip]
pub static KERCHIEF: Part = Part {
    id: "kerchief", name: "Field Kerchief", slot: Slot::Head,
    note: "cloth band + knot; the cheapest possible head slot, still asymmetric",
    boxes: &[
        b(-2.4, 18.9, -2.7,   2.4, 19.9,  2.1, Surf::ClothWalnut),   // brow band
        b(-2.5, 18.6,  1.5,   2.5, 20.0,  2.5, Surf::ClothWalnut),   // knot, at the back
        b( 1.1, 17.0,  1.9,   2.2, 18.9,  2.7, Surf::ClothWalnut),   // trailing tail (one side only)
        b(-2.45,19.6, -2.75,  2.45,19.95, 2.15, Surf::LeatherWorn),  // sun-bleached top edge
    ],
};

/// Adventurer. A raised hood — the single biggest silhouette change any slot in
/// this file makes. Built as a shell (back + two sides + crown + brow peak) with
/// the front left OPEN, so the face still reads under it instead of the head
/// becoming one closed lump.
#[rustfmt::skip]
pub static HOOD_TRAVEL: Part = Part {
    id: "hood_travel", name: "Traveller's Hood", slot: Slot::Head,
    note: "raised hood shell, face left open; the loudest silhouette in the set",
    boxes: &[
        b(-3.0, 16.0,  1.9,   3.0, 21.1,  3.0, Surf::ClothEspresso), // back
        b(-3.0, 16.0, -3.0,  -2.3, 21.1,  2.2, Surf::ClothEspresso), // side L
        b( 2.3, 16.0, -3.0,   3.0, 21.1,  2.2, Surf::ClothEspresso), // side R
        b(-3.0, 20.2, -3.0,   3.0, 21.1,  2.2, Surf::ClothEspresso), // crown
        b(-2.3, 19.7, -3.2,   2.3, 20.7, -2.6, Surf::ClothEspresso), // brow peak over the eyes
        b(-3.1, 15.3, -3.0,   3.1, 16.4,  3.1, Surf::ClothEspresso), // mantle at the neck
        b(-3.2, 14.5, -3.0,   3.2, 15.5,  3.2, Surf::LeatherStrap),  // hood collar — leather, not cloth
        b(-3.05,17.4, -3.05, -2.35,19.6,  2.25,Surf::LeatherWorn),   // rain-worn edge, left side only
        // ---- sculpt pass: cloth does not stand at right angles ---------------
        // The peak rakes forward and down over the brow, the sides fall INWARD to
        // the jaw, and the mantle flares out. Three diagonals: the hood stops
        // being a chimney with a notch cut in it.
        br(-2.4, 19.6, -3.55,  2.4, 20.6, -2.35, 34.0,0.0,  0.0, Surf::ClothEspresso), // peak, raked forward
        br(-3.05,16.6, -3.15, -2.35,20.4,  2.15,  0.0,0.0,  8.0, Surf::ClothEspresso), // side falls in, L
        br( 2.35,16.6, -3.15,  3.05,20.4,  2.15,  0.0,0.0, -8.0, Surf::ClothEspresso), // side falls in, R
        br(-3.3, 15.1, -3.15, -1.2, 16.5,  3.15,  0.0,0.0,-14.0, Surf::ClothEspresso), // mantle flare L
        br( 1.2, 15.1, -3.15,  3.3, 16.5,  3.15,  0.0,0.0, 14.0, Surf::ClothEspresso), // mantle flare R
        br(-2.9, 16.2,  2.6,   1.4, 20.6,  3.35, -9.0,0.0,  5.0, Surf::ClothEspresso), // the fold down the back
        br(-3.25,14.4, -3.05,  3.25,15.1,  3.25, 45.0,0.0,  0.0, Surf::LeatherStrap),  // collar roll
    ],
};

/// War. Steel, brass crest, and a real visor slit — the gap between the two front
/// plates, backed by a dark interior box so the opening reads as a void and not as
/// a hole into a lit skin block.
#[rustfmt::skip]
pub static HELM_GREAT: Part = Part {
    id: "helm_great", name: "Greathelm", slot: Slot::Head,
    boxes: &[
        b(-2.6, 16.2,  0.0,   2.6, 20.7,  2.4, Surf::SteelBright),   // rear shell
        b(-2.6, 16.2, -2.9,  -2.0, 20.7,  0.1, Surf::SteelBright),   // side L
        b( 2.0, 16.2, -2.9,   2.6, 20.7,  0.1, Surf::SteelBright),   // side R
        b(-2.1, 18.9, -2.9,   2.1, 20.7,  0.1, Surf::SteelBright),   // front, above the slit
        b(-2.1, 16.2, -2.9,   2.1, 18.2,  0.1, Surf::SteelBright),   // front, below the slit
        b(-2.0, 18.0, -2.5,   2.0, 19.1, -1.9, Surf::LeatherDark),   // the void behind the slit
        b(-2.8, 19.5, -3.0,   2.8, 20.3,  2.6, Surf::SteelDark),     // brow band
        b(-2.75,17.6, -3.0,  -2.05,19.4,  1.9, Surf::SteelDark),     // cheek plate L
        b( 2.05,17.6, -3.0,   2.75,19.4,  1.9, Surf::SteelDark),     // cheek plate R
        b(-0.5, 20.5, -2.8,   0.5, 22.1,  1.8, Surf::Brass),         // crest fin
        b(-0.4, 21.1, -2.4,   0.4, 22.7,  0.5, Surf::Brass),         // crest tip
        b(-2.7, 15.9, -3.0,   2.7, 16.9,  2.5, Surf::SteelDark),     // gorget ring
        b(-2.65,17.2, -3.05, -1.7, 18.0,  0.0, Surf::SteelRust),     // rust bloom on one cheek
        // ---- sculpt pass: chamfers + a raked crest --------------------------
        // A 45°-rolled slab along an edge is a bevel. It costs one box and it is
        // the difference between a helmet and a steel bucket: the key light lands
        // on the chamfer as a hard band that no flat face of the shell can make.
        br(-2.55,20.55,-2.85,  2.55,21.05, 2.35,  0.0,0.0,45.0, Surf::SteelBright), // skull chamfer
        br(-2.85,19.35,-2.95, -2.25,20.35, 2.45,  0.0,0.0,45.0, Surf::SteelDark),   // temple chamfer L
        br( 2.25,19.35,-2.95,  2.85,20.35, 2.45,  0.0,0.0,45.0, Surf::SteelDark),   // temple chamfer R
        br(-2.15,18.15,-3.02,  2.15,18.55,-2.55,  30.0,0.0,0.0, Surf::SteelDark),   // visor brow, raked
        br(-0.5, 21.9, -2.6,   0.5, 23.3,  0.9,  22.0,0.0, 0.0, Surf::Brass),       // crest swept back
        br(-2.0, 16.5, -3.05,  2.0, 17.1, -2.45,  0.0,0.0,45.0, Surf::SteelDark),   // chin chamfer
    ],
    note: "steel shell with a real visor slit; brass crest breaks the head outline",
};

// --- TORSO -----------------------------------------------------------------

/// Villager. Loose, cheap, and PATCHED — the patch is on the left rib only, which
/// is what stops the tunic reading as a symmetric box.
#[rustfmt::skip]
pub static TUNIC_LINEN: Part = Part {
    id: "tunic_linen", name: "Linen Work Tunic", slot: Slot::Torso,
    note: "loose cloth, one patch, sash knotted off-centre. Zero specular anywhere",
    boxes: &[
        b(-4.7, 10.0, -2.7,   4.7, 15.6,  2.4, Surf::ClothLinen),    // body, loose over the waist
        b(-4.9, 15.1, -2.9,   4.9, 16.1,  2.6, Surf::ClothLinen),    // shoulder
        b(-6.2, 13.5, -2.5,  -3.8, 15.9,  2.2, Surf::ClothLinen),    // sleeve L (short)
        b( 3.8, 13.5, -2.5,   6.2, 15.9,  2.2, Surf::ClothLinen),    // sleeve R
        b(-4.8,  9.5, -2.8,   4.8, 10.4,  2.5, Surf::ClothWalnut),   // hem band
        b(-4.25,11.4, -2.78, -2.6, 12.9, -2.42,Surf::ClothWalnut),   // the patch — left rib only
        b(-3.95,12.6, -2.85, -2.9, 12.95,-2.48,Surf::LeatherWorn),   // its stitching
        b(-3.6, 11.2, -2.85,  3.6, 12.1,  2.55,Surf::ClothWalnut),   // waist sash
        b(-3.7, 11.05,-3.0,  -1.2, 12.25,-2.55,Surf::ClothWalnut),   // sash knot, off-centre
        b(-2.2, 11.3, -3.05, -1.4, 13.6, -2.6, Surf::ClothWalnut),   // sash tail, hanging
        b( 2.6, 10.2, -2.9,   3.8, 11.4, -2.45,Surf::LeatherStrap),  // a single small pouch
        // ---- sculpt pass: a loose tunic hangs, it does not stack -------------
        br(-4.9,  9.4, -2.85, -1.6, 10.9,  2.55,  0.0,0.0,  9.0, Surf::ClothLinen),  // hem swings out, L
        br( 1.6,  9.4, -2.85,  4.9, 10.9,  2.55,  0.0,0.0, -9.0, Surf::ClothLinen),  // hem swings out, R
        br(-6.3, 13.3, -2.55, -3.7, 15.1,  2.25,  0.0,0.0, 16.0, Surf::ClothLinen),  // sleeve falls, L
        br( 3.7, 13.3, -2.55,  6.3, 15.1,  2.25,  0.0,0.0,-16.0, Surf::ClothLinen),  // sleeve falls, R
        br(-4.95,15.05,-2.95, -1.2, 16.15, 2.65,  0.0,0.0,-10.0, Surf::ClothLinen),  // shoulder follows the trap
        br( 1.2, 15.05,-2.95,  4.95,16.15, 2.65,  0.0,0.0, 10.0, Surf::ClothLinen),
        br(-2.3, 10.4, -3.1,  -1.3, 13.4, -2.6,   0.0,0.0, 12.0, Surf::ClothWalnut), // sash tail, swinging
    ],
};

/// Adventurer. The hero silhouette: fitted leather, ONE pauldron (left), a chest
/// strap with a brass buckle, and the ember pouch the bible reserves as the only
/// pre-reveal Shaper hint.
#[rustfmt::skip]
pub static JERKIN_LEATHER: Part = Part {
    id: "jerkin_leather", name: "Traveller's Jerkin", slot: Slot::Torso,
    note: "leather + brass + one steel-rimmed pauldron; carries the ember pouch",
    boxes: &[
        b(-4.6, 10.2, -2.6,   4.6, 15.8,  2.3, Surf::LeatherMid),    // jerkin body
        b(-4.8, 15.2, -2.8,   4.8, 16.2,  2.5, Surf::LeatherDark),   // collar
        b(-6.5, 13.4, -2.6,  -4.3, 16.1,  2.3, Surf::LeatherDark),   // pauldron — LEFT ONLY
        b(-6.6, 15.0, -2.7,  -4.2, 15.8,  2.4, Surf::SteelDark),     // its steel rim
        b(-6.55,13.2, -2.7,  -5.7, 15.1,  2.4, Surf::LeatherWorn),   // scuffed leading edge
        b(-1.4, 12.2, -2.9,   1.4, 16.0, -2.5, Surf::LeatherStrap),  // chest strap
        b(-1.9, 13.8, -3.02,  1.9, 14.7, -2.62,Surf::Brass),         // strap plate
        b( 1.2, 12.3, -2.9,   4.3, 13.3, -2.5, Surf::LeatherStrap),  // bandolier to the right hip
        b(-4.9, 10.9, -2.8,   4.9, 12.2,  2.5, Surf::LeatherDark),   // belt
        b(-1.1, 10.9, -3.0,   1.1, 12.3, -2.6, Surf::Brass),         // belt buckle
        b( 2.2, 10.3, -3.0,   3.7, 12.0, -2.4, Surf::LeatherWorn),   // hip pouch
        b( 2.4, 10.1, -3.08,  3.5, 10.9, -2.5, Surf::LeatherStrap),  // its flap
        b(-4.65,12.4, -2.86, -3.6, 13.3, -2.48,Surf::LeatherWorn),   // wear scuff over the ribs
        b( 3.9, 14.2, -2.72,  4.65,15.4, -2.4, Surf::LeatherWorn),   // second scuff, right shoulder
        // ---- the ember pouch: the ONE narrative emissive on this body --------
        b(-4.0, 10.2, -2.95, -2.4, 11.7, -2.4, Surf::EmberHousing),
        b(-3.7, 10.5, -3.1,  -2.7, 11.4, -2.9, Surf::Ember),
        // ---- sculpt pass: the jerkin follows the shoulder line ---------------
        br(-6.6, 15.3, -2.72, -4.2, 16.3,  2.42, 0.0,0.0, 20.0, Surf::LeatherDark),  // pauldron cap (L only)
        br(-6.65,13.1, -2.72, -5.5, 15.4,  2.42, 0.0,0.0, 20.0, Surf::LeatherWorn),  // its rolled leading edge
        br(-4.85,15.15,-2.85, -1.0, 16.35, 2.45, 0.0,0.0,-11.0, Surf::LeatherDark),  // collar, follows the trap L
        br( 1.0, 15.15,-2.85,  4.85,16.35, 2.45, 0.0,0.0, 11.0, Surf::LeatherDark),  // collar R
        br(-4.7, 10.0, -2.7,  -1.6, 11.4,  2.4,  0.0,0.0,  7.0, Surf::LeatherMid),   // skirt flare L
        br( 1.6, 10.0, -2.7,   4.7, 11.4,  2.4,  0.0,0.0, -7.0, Surf::LeatherMid),   // skirt flare R
        br(-1.5, 12.1, -2.98,  1.5, 12.7, -2.6,  0.0,0.0, 45.0, Surf::LeatherStrap), // strap edge, catching light
    ],
};

/// War. Steel plate with brass trim, a centre keel, rivets and REAL battle damage
/// (rust bloom + a strike scar), all of it asymmetric.
#[rustfmt::skip]
pub static CUIRASS_STEEL: Part = Part {
    id: "cuirass_steel", name: "Warplate Cuirass", slot: Slot::Torso,
    note: "steel + brass; two pauldrons, one of them battle-chipped",
    boxes: &[
        b(-4.9, 10.6, -2.9,   4.9, 15.9,  2.6, Surf::SteelBright),   // breastplate
        b(-5.0, 15.3, -3.0,   5.0, 16.4,  2.7, Surf::SteelDark),     // gorget
        b(-0.7, 11.0, -3.05,  0.7, 15.8, -2.55,Surf::SteelDark),     // centre keel
        b(-5.05,12.6, -2.98,  5.05,13.4,  2.68,Surf::Brass),         // trim, low
        b(-5.02,14.4, -2.96,  5.02,14.9,  2.66,Surf::Brass),         // trim, high
        b(-7.4, 13.5, -3.1,  -4.6, 16.4,  2.8, Surf::SteelBright),   // pauldron L
        b( 4.6, 13.5, -3.1,   7.4, 16.4,  2.8, Surf::SteelBright),   // pauldron R
        b(-7.5, 15.4, -3.15, -4.5, 16.2,  2.85,Surf::Brass),         // pauldron trim L
        b( 4.5, 15.4, -3.15,  7.5, 16.2,  2.85,Surf::Brass),         // pauldron trim R
        b(-7.55,13.3, -3.15, -6.8, 15.5,  2.85,Surf::SteelRust),     // battle-chipped edge — L only
        b(-5.0,  9.9, -3.0,   5.0, 10.8,  2.7, Surf::SteelDark),     // fauld
        b(-3.2,  8.9, -2.95,  3.2, 10.1,  2.65,Surf::LeatherStrap),  // leather under-skirt
        b(-4.65,11.5, -3.02, -3.4, 12.4, -2.6, Surf::SteelRust),     // rust bloom / dent
        b( 1.6, 14.0, -3.02,  2.9, 15.2, -2.6, Surf::SteelRust),     // sword-strike scar
        b(-4.3, 15.9, -3.05, -3.6, 16.5, -2.7, Surf::Brass),         // rivet L
        b( 3.6, 15.9, -3.05,  4.3, 16.5, -2.7, Surf::Brass),         // rivet R
        // ---- sculpt pass ----------------------------------------------------
        // Pauldrons get a rolled cap each, tilted the way a real one sheds a blow:
        // outer edge DOWN. That single diagonal per shoulder does more for "this
        // is armour" than the four trim bands under it.
        br(-7.5, 15.5, -3.15, -4.5, 16.6,  2.85, 0.0,0.0, 26.0, Surf::SteelBright), // pauldron cap L
        br( 4.5, 15.5, -3.15,  7.5, 16.6,  2.85, 0.0,0.0,-26.0, Surf::SteelBright), // pauldron cap R
        br(-7.6, 14.6, -3.18, -6.5, 15.7,  2.88, 0.0,0.0, 26.0, Surf::SteelDark),   // pauldron lame L
        br( 6.5, 14.6, -3.18,  7.6, 15.7,  2.88, 0.0,0.0,-26.0, Surf::SteelDark),   // pauldron lame R
        br(-4.95,15.85,-3.0,   4.95,16.35, 2.7,  0.0,0.0, 45.0, Surf::SteelBright), // gorget chamfer
        br(-4.95, 9.85,-2.95,  4.95,10.35, 2.65, 45.0,0.0,  0.0, Surf::SteelDark),  // fauld chamfer
        br(-0.75,10.9, -3.15,  0.75,15.9, -2.75, 0.0,0.0, 45.0, Surf::SteelBright), // keel ridge, edge-on
        br(-4.9, 12.4, -3.05, -3.9, 15.6,  2.6,  0.0,3.0,  6.0, Surf::SteelDark),   // rib swage L
        br( 3.9, 12.4, -3.05,  4.9, 15.6,  2.6,  0.0,-3.0,-6.0, Surf::SteelDark),   // rib swage R
    ],
};

// --- LEGS ------------------------------------------------------------------

#[rustfmt::skip]
pub static TROUSERS_WORK: Part = Part {
    id: "trousers_work", name: "Work Trousers", slot: Slot::Legs,
    note: "cloth, rolled cuffs, one knee patch, plain shoes",
    boxes: &[
        b(-3.75, 4.4, -2.05, -0.25,  9.9,  2.15, Surf::ClothWalnut),  // leg L
        b( 0.25, 4.4, -2.65,  3.75,  9.9,  1.55, Surf::ClothWalnut),  // leg R (forward foot)
        b(-3.85, 4.2, -2.15, -0.15,  5.0,  2.25, Surf::ClothEspresso),// rolled cuff L
        b( 0.15, 4.2, -2.75,  3.85,  5.0,  1.65, Surf::ClothEspresso),// rolled cuff R
        b(-3.9,  5.8, -2.2,  -2.5,   7.2,  0.2,  Surf::ClothEspresso),// knee patch — left only
        b(-3.95, 9.0, -2.25,  3.95,  9.9,  2.25, Surf::LeatherStrap), // rope belt
        // Shoes now reach the GROUND (y 0.0, was 0.9). The old pair floated a vx
        // above the sole and left bare skin showing under a fully-dressed villager
        // — invisible at 720p, obvious the moment the sheet got bigger.
        b(-3.6,  0.0, -2.45, -0.4,   4.6,  2.65, Surf::LeatherWorn),  // shoe L
        b( 0.4,  0.0, -3.65,  3.6,   4.6,  1.45, Surf::LeatherWorn),  // shoe R
        // ---- sculpt pass ----------------------------------------------------
        br(-3.5,  0.0, -2.6,  -0.5,   0.9,  1.1,   0.0, 7.0, 0.0, Surf::LeatherDark),  // toe cap L, toed out
        br( 0.5,  0.0, -3.8,   3.5,   0.9, -0.1,   0.0,-7.0, 0.0, Surf::LeatherDark),  // toe cap R
        br(-3.7,  4.15,-2.55, -0.3,   4.85, 2.75, 45.0, 0.0, 0.0, Surf::ClothEspresso),// cuff roll L
        br( 0.3,  4.15,-3.75,  3.7,   4.85, 1.55, 45.0, 0.0, 0.0, Surf::ClothEspresso),// cuff roll R
        br(-3.9,  8.6, -2.2,  -1.6,   9.9,  2.3,   0.0, 0.0, 6.0, Surf::ClothWalnut),  // cloth gathers at the belt, L
        br( 1.6,  8.6, -2.8,   3.9,   9.9,  1.7,   0.0, 0.0,-6.0, Surf::ClothWalnut),  // ... R
    ],
};

#[rustfmt::skip]
pub static BREECHES_TRAVEL: Part = Part {
    id: "breeches_travel", name: "Travel Breeches & Boots", slot: Slot::Legs,
    note: "leather over cloth, buckled boot straps, a knife sheathed on the right thigh",
    boxes: &[
        b(-3.75, 4.8, -2.05, -0.25,  9.9,  2.15, Surf::LeatherMid),   // breech L
        b( 0.25, 4.8, -2.65,  3.75,  9.9,  1.55, Surf::LeatherMid),   // breech R
        b(-3.9,  0.6, -2.6,  -0.25,  5.2,  2.8,  Surf::LeatherDark),  // tall boot L
        b( 0.25, 0.6, -3.8,   3.9,   5.2,  1.6,  Surf::LeatherDark),  // tall boot R
        b(-4.0,  4.6, -2.7,  -0.2,   5.4,  2.9,  Surf::LeatherWorn),  // boot cuff L
        b( 0.2,  4.6, -3.9,   4.0,   5.4,  1.7,  Surf::LeatherWorn),  // boot cuff R
        b(-4.0,  2.2, -2.7,  -0.2,   2.9,  2.9,  Surf::LeatherStrap), // boot strap L
        b( 0.2,  2.2, -3.9,   4.0,   2.9,  1.7,  Surf::LeatherStrap), // boot strap R
        b(-4.05, 2.25,-2.78, -3.2,   2.85,-2.15, Surf::Brass),        // buckle L
        b( 3.2,  2.25,-3.98,  4.05,  2.85,-3.35, Surf::Brass),        // buckle R
        b(-3.85, 6.6, -2.15, -0.3,   7.6,  2.25, Surf::LeatherStrap), // thigh wrap L
        b(-3.85, 3.3, -2.72, -2.9,   4.0,  2.92, Surf::LeatherWorn),  // scuffed toe wear, L only
        b( 3.6,  6.0, -2.85,  4.45,  8.6, -1.5,  Surf::LeatherDark),  // knife sheath, right thigh
        b( 3.7,  8.4, -2.75,  4.35,  9.7, -1.7,  Surf::SteelDark),    // its hilt
        // ---- sculpt pass: a boot cuff FLARES, it does not step ---------------
        br(-4.15, 4.5, -2.8,  -0.15,  5.5,  3.0,  0.0, 0.0,  9.0, Surf::LeatherWorn),  // cuff flare L
        br( 0.15, 4.5, -4.0,   4.15,  5.5,  1.8,  0.0, 0.0, -9.0, Surf::LeatherWorn),  // cuff flare R
        br(-3.95, 0.0, -2.75, -0.25,  1.5,  2.95, 0.0, 6.0,  0.0, Surf::LeatherDark),  // boot toe L, toed out
        br( 0.25, 0.0, -3.95,  3.95,  1.5,  1.75, 0.0,-6.0,  0.0, Surf::LeatherDark),  // boot toe R
        br(-3.9,  7.4, -2.2,  -0.3,   9.8,  2.3,  0.0, 0.0,  4.0, Surf::LeatherMid),   // thigh taper L
        br( 0.3,  7.4, -2.8,   3.9,   9.8,  1.7,  0.0, 0.0, -4.0, Surf::LeatherMid),   // thigh taper R
        br( 3.55, 5.6, -2.95,  4.5,   8.9, -1.35, 0.0, 0.0,-11.0, Surf::LeatherDark),  // sheath hangs at an angle
    ],
};

#[rustfmt::skip]
pub static GREAVES_PLATE: Part = Part {
    id: "greaves_plate", name: "Plate Greaves", slot: Slot::Legs,
    note: "articulated steel with brass trim and knee cops; scored on one shin",
    boxes: &[
        b(-3.95, 4.8, -2.25, -0.2,   9.9,  2.35, Surf::SteelBright),  // cuisse L
        b( 0.2,  4.8, -2.85,  3.95,  9.9,  1.75, Surf::SteelBright),  // cuisse R
        b(-4.1,  5.6, -2.4,  -0.1,   6.4,  2.5,  Surf::Brass),        // trim L
        b( 0.1,  5.6, -3.0,   4.1,   6.4,  1.9,  Surf::Brass),        // trim R
        b(-4.0,  4.2, -2.4,  -0.15,  5.6,  2.5,  Surf::SteelDark),    // knee cop L
        b( 0.15, 4.2, -3.0,   4.0,   5.6,  1.9,  Surf::SteelDark),    // knee cop R
        b(-3.85, 1.2, -2.25, -0.35,  4.4,  2.35, Surf::SteelBright),  // greave L
        b( 0.35, 1.2, -2.85,  3.85,  4.4,  1.75, Surf::SteelBright),  // greave R
        b(-3.95, 0.0, -2.65, -0.3,   1.6,  2.75, Surf::SteelDark),    // sabaton L
        b( 0.3,  0.0, -3.85,  3.95,  1.6,  1.55, Surf::SteelDark),    // sabaton R
        b(-3.97, 2.6, -2.3,  -3.15,  3.6,  2.4,  Surf::SteelRust),    // battle scoring, L only
        b(-4.0,  8.0, -2.3,  -0.2,   8.7,  2.4,  Surf::LeatherStrap), // securing strap L
        b( 0.2,  8.0, -2.9,   4.0,   8.7,  1.8,  Surf::LeatherStrap), // securing strap R
        // ---- sculpt pass: articulated plate = chamfers + a pointed cop -------
        br(-4.05, 9.45,-2.3,  -0.15, 10.0,  2.4,  45.0,0.0, 0.0, Surf::SteelBright), // cuisse top chamfer L
        br( 0.15, 9.45,-2.9,   4.05, 10.0,  1.8,  45.0,0.0, 0.0, Surf::SteelBright), // ... R
        br(-4.1,  4.3, -2.75, -0.1,   5.3,  1.4,  22.0,0.0, 0.0, Surf::SteelDark),   // knee cop, pointed L
        br( 0.1,  4.3, -3.35,  4.1,   5.3,  0.8,  22.0,0.0, 0.0, Surf::SteelDark),   // ... R
        br(-3.9,  1.15,-2.3,  -0.4,   1.75, 2.4,  45.0,0.0, 0.0, Surf::SteelBright), // greave chamfer L
        br( 0.4,  1.15,-2.9,   3.9,   1.75, 1.8,  45.0,0.0, 0.0, Surf::SteelBright), // ... R
        br(-4.0,  0.0, -2.85, -0.25,  1.2,  2.5,   0.0,6.0, 0.0, Surf::SteelDark),   // sabaton, toed out L
        br( 0.25, 0.0, -4.05,  4.0,   1.2,  1.3,   0.0,-6.0,0.0, Surf::SteelDark),   // ... R
    ],
};

// --- HANDS -----------------------------------------------------------------

#[rustfmt::skip]
pub static WRAPS_CLOTH: Part = Part {
    id: "wraps_cloth", name: "Cloth Hand Wraps", slot: Slot::Hands,
    note: "linen strips; the poorest hands in the game",
    boxes: &[
        b(-5.78, 8.0, -2.05, -3.92, 10.1,  1.65, Surf::ClothLinen),
        b( 3.92, 8.0, -2.05,  5.78, 10.1,  1.65, Surf::ClothLinen),
        b(-5.82, 9.4, -2.1,  -3.88,  9.85, 1.7,  Surf::ClothWalnut),  // tie L
        b( 3.88, 9.4, -2.1,   5.82,  9.85, 1.7,  Surf::ClothWalnut),  // tie R
    ],
};

#[rustfmt::skip]
pub static GLOVES_LEATHER: Part = Part {
    id: "gloves_leather", name: "Buckled Bracers", slot: Slot::Hands,
    note: "leather gloves + bracers with brass buckles; knuckle wear on one hand",
    boxes: &[
        b(-5.88, 6.3, -2.22, -3.82,  8.35, 1.55, Surf::LeatherDark),  // glove L
        b( 3.82, 6.3, -2.22,  5.88,  8.35, 1.55, Surf::LeatherDark),  // glove R
        b(-5.92, 8.2, -2.18, -3.78, 10.7,  1.62, Surf::LeatherMid),   // bracer L
        b( 3.78, 8.2, -2.18,  5.92, 10.7,  1.62, Surf::LeatherMid),   // bracer R
        b(-5.97, 9.2, -2.22, -3.73,  9.75, 1.68, Surf::LeatherStrap), // bracer strap L
        b( 3.73, 9.2, -2.22,  5.97,  9.75, 1.68, Surf::LeatherStrap), // bracer strap R
        b(-6.0,  9.25,-2.28, -5.2,   9.7, -1.5,  Surf::Brass),        // buckle L
        b( 5.2,  9.25,-2.28,  6.0,   9.7, -1.5,  Surf::Brass),        // buckle R
        b(-5.92, 6.9, -2.28, -4.9,   7.35, 1.5,  Surf::LeatherWorn),  // knuckle wear — L only
        // ---- sculpt pass: gloves follow the 6° the arms now hang at ----------
        br(-5.9,  6.35,-2.25, -3.8,   8.3,  1.6,  0.0,0.0,  6.0, Surf::LeatherDark), // glove shell L
        br( 3.8,  6.35,-2.25,  5.9,   8.3,  1.6,  0.0,0.0, -6.0, Surf::LeatherDark), // glove shell R
        br(-4.5,  6.85,-1.7,  -3.85,  8.0, -0.2,  0.0,0.0,-16.0, Surf::LeatherDark), // thumb cover L
        br( 3.85, 6.85,-1.7,   4.5,   8.0, -0.2,  0.0,0.0, 16.0, Surf::LeatherDark), // thumb cover R
        br(-6.0,  10.3,-2.2,  -3.7,  10.95, 1.65,45.0,0.0,  0.0, Surf::LeatherMid),  // bracer top roll L
        br( 3.7,  10.3,-2.2,   6.0,  10.95, 1.65,45.0,0.0,  0.0, Surf::LeatherMid),  // bracer top roll R
    ],
};

#[rustfmt::skip]
pub static GAUNTLETS_STEEL: Part = Part {
    id: "gauntlets_steel", name: "Plate Gauntlets", slot: Slot::Hands,
    note: "steel gauntlet + vambrace + elbow cop; brass trim, one rust streak",
    boxes: &[
        b(-6.0,  6.2, -2.3,  -3.7,   8.45, 1.65, Surf::SteelBright),  // gauntlet L
        b( 3.7,  6.2, -2.3,   6.0,   8.45, 1.65, Surf::SteelBright),  // gauntlet R
        b(-6.05, 7.6, -2.36, -3.65,  8.25, 1.7,  Surf::SteelDark),    // knuckle plate L
        b( 3.65, 7.6, -2.36,  6.05,  8.25, 1.7,  Surf::SteelDark),    // knuckle plate R
        b(-6.1,  8.3, -2.32, -3.6,  11.0,  1.72, Surf::SteelBright),  // vambrace L
        b( 3.6,  8.3, -2.32,  6.1,  11.0,  1.72, Surf::SteelBright),  // vambrace R
        b(-6.15, 9.6, -2.36, -3.55, 10.2,  1.76, Surf::Brass),        // trim L
        b( 3.55, 9.6, -2.36,  6.15, 10.2,  1.76, Surf::Brass),        // trim R
        b(-6.2, 10.8, -2.4,  -3.5,  12.1,  1.8,  Surf::SteelDark),    // elbow cop L
        b( 3.5, 10.8, -2.4,   6.2,  12.1,  1.8,  Surf::SteelDark),    // elbow cop R
        b(-6.13, 8.6, -2.36, -5.3,   9.2,  1.7,  Surf::SteelRust),    // rust streak — L only
        // ---- sculpt pass: lames, chamfers, and a pointed elbow ---------------
        br(-6.05, 6.3, -2.3,  -3.65,  8.4,  1.65, 0.0,0.0,  6.0, Surf::SteelBright), // gauntlet shell L
        br( 3.65, 6.3, -2.3,   6.05,  8.4,  1.65, 0.0,0.0, -6.0, Surf::SteelBright), // gauntlet shell R
        br(-6.12, 7.55,-2.38, -3.58,  8.15, 1.72,45.0,0.0,  0.0, Surf::SteelDark),   // knuckle chamfer L
        br( 3.58, 7.55,-2.38,  6.12,  8.15, 1.72,45.0,0.0,  0.0, Surf::SteelDark),   // knuckle chamfer R
        br(-6.2, 10.9, -2.45, -3.5,  12.3,  1.2, 24.0,0.0,  0.0, Surf::SteelDark),   // elbow cop, pointed L
        br( 3.5, 10.9, -2.45,  6.2,  12.3,  1.2, 24.0,0.0,  0.0, Surf::SteelDark),   // elbow cop, pointed R
        br(-6.18,10.5, -2.38, -3.52, 11.1,  1.74,45.0,0.0,  0.0, Surf::SteelBright), // vambrace chamfer L
        br( 3.52,10.5, -2.38,  6.18, 11.1,  1.74,45.0,0.0,  0.0, Surf::SteelBright), // vambrace chamfer R
    ],
};

// --- BACK ------------------------------------------------------------------

#[rustfmt::skip]
pub static PACK_HARVEST: Part = Part {
    id: "pack_harvest", name: "Harvest Basket", slot: Slot::Back,
    note: "wicker pack with stalks breaking the head line — a villager's outline",
    boxes: &[
        b(-3.0, 11.0,  2.0,   3.0, 16.3,  4.8, Surf::Wicker),         // basket
        b(-3.2, 15.7,  1.9,   3.2, 16.6,  5.0, Surf::WoodPale),       // rim
        b(-3.2, 12.9,  1.9,   3.2, 13.5,  5.0, Surf::WoodPale),       // band
        b(-3.45,13.0, -2.6,  -2.45,16.4,  2.4, Surf::LeatherStrap),   // shoulder strap L
        b( 2.45,13.0, -2.6,   3.45,16.4,  2.4, Surf::LeatherStrap),   // shoulder strap R
        b(-1.6, 16.3,  3.4,   1.6, 17.6,  4.3, Surf::WoodPale),       // a bundle of stalks
        b(-0.9, 17.4,  3.5,   0.9, 20.0,  4.1, Surf::WoodPale),       // ... sticking out past the head
        b( 1.0, 17.2,  3.6,   1.8, 19.2,  4.0, Surf::WoodPale),       // one stalk leaning off-axis
        // ---- sculpt pass: stalks lean, baskets taper ------------------------
        br(-1.4, 17.3,  3.4,  -0.5, 20.6,  4.2,  -8.0,0.0, 13.0, Surf::WoodPale),  // stalk leaning left
        br( 0.6, 17.3,  3.5,   1.5, 20.9,  4.3, -12.0,0.0,-17.0, Surf::WoodPale),  // stalk leaning right
        br(-0.3, 17.5,  3.3,   0.5, 21.3,  4.1,  -5.0,0.0,  3.0, Surf::WoodPale),  // the tall one
        br(-3.1, 10.8,  2.1,   3.1, 12.2,  4.6,   0.0,0.0,  0.0, Surf::Wicker),    // basket base
        br(-3.25,15.65, 1.85,  3.25,16.65, 5.05, 45.0,0.0,  0.0, Surf::WoodPale),  // rim roll
    ],
};

/// The bible's locked read-point for Auren, rebuilt as a removable part: a
/// half-cloak that leaves the outline on the −X side from any angle, plus the
/// bedroll + satchel that say "arrived from elsewhere, still travelling".
#[rustfmt::skip]
pub static CLOAK_HALF: Part = Part {
    id: "cloak_half", name: "Half-Cloak & Travel Roll", slot: Slot::Back,
    note: "THE Auren read-point: asymmetric cloak, frayed corners, bedroll, satchel",
    boxes: &[
        b(-6.6, 14.6, -3.2,   1.0, 16.7,  3.2, Surf::ClothEspresso),  // shoulder cape, off-centre
        b(-6.9,  7.4, -3.1,  -2.0, 15.0,  3.4, Surf::ClothEspresso),  // side panel past the hip
        b(-7.3,  3.2,  0.2,  -2.6,  7.8,  4.2, Surf::ClothEspresso),  // trailing flap
        b(-7.45, 3.0,  0.4,  -6.6,  4.1,  4.35,Surf::ClothWalnut),    // frayed corner (bleached)
        b(-4.2,  3.3,  3.6,  -2.8,  4.2,  4.3, Surf::ClothWalnut),    // second fray notch
        b(-2.4, 15.6, -3.4,   1.6, 16.9,  2.0, Surf::LeatherStrap),   // throat strap
        b(-1.2, 15.75,-3.5,   0.4, 16.75,-3.05,Surf::Brass),          // cloak clasp
        b(-2.8, 16.0,  2.2,   2.8, 17.4,  4.6, Surf::ClothLinen),     // bedroll
        b(-3.0, 15.9,  2.4,  -2.4, 17.5,  4.8, Surf::LeatherStrap),   // roll tie L
        b( 2.4, 15.9,  2.4,   3.0, 17.5,  4.8, Surf::LeatherStrap),   // roll tie R
        b(-2.2, 12.4,  2.0,   2.2, 16.0,  3.9, Surf::LeatherMid),     // satchel
        b(-2.4, 12.2,  1.9,   2.4, 13.1,  4.0, Surf::LeatherDark),    // satchel flap
        b(-0.6, 12.2,  3.88,  0.6, 13.2,  4.15,Surf::Brass),          // satchel buckle
        // ---- sculpt pass: THE loudest silhouette change in the whole file ----
        // A cloak is the one garment a viewer reads entirely from its outline, and
        // an axis-aligned cloak is a curtain. These four panels swing it out and
        // back at 18–34°, so the −X edge is a diagonal from shoulder to floor.
        br(-7.2, 12.4, -3.0,  -4.6, 16.4,  3.3,   0.0, 0.0, 18.0, Surf::ClothEspresso), // shoulder fall
        br(-7.6,  6.6, -2.6,  -4.4, 13.2,  3.6,   0.0, 6.0, 12.0, Surf::ClothEspresso), // hip flare
        br(-7.9,  2.4,  0.6,  -4.2,  8.2,  4.4,  10.0, 0.0, 22.0, Surf::ClothEspresso), // trailing sweep
        br(-6.4,  1.6,  1.6,  -3.4,  4.4,  4.6,  16.0, 0.0, 34.0, Surf::ClothEspresso), // the tip, off the ground
        br(-6.5,  1.5,  1.7,  -5.2,  2.9,  4.5,  16.0, 0.0, 34.0, Surf::ClothWalnut),   // bleached frayed tip
        br(-2.6, 15.4,  2.1,   2.6, 17.6,  4.7,  -8.0, 0.0,  4.0, Surf::ClothLinen),    // bedroll, tilted
    ],
};

#[rustfmt::skip]
pub static CAPE_BATTLE: Part = Part {
    id: "cape_battle", name: "Battle Cape", slot: Slot::Back,
    note: "floor-length crimson over a steel mantle bar; ember set in the clasp",
    boxes: &[
        b(-5.0, 15.0,  2.4,   5.0, 17.0,  3.4, Surf::ClothCrimson),   // cape shoulder
        b(-4.4,  1.9,  2.8,   4.4, 15.4,  3.8, Surf::ClothCrimson),   // cape body, floor-length
        b(-4.6,  1.5,  2.9,   4.6,  2.6,  3.9, Surf::ClothEspresso),  // hem band
        b(-4.7,  1.2,  2.95, -2.6,  2.4,  4.0, Surf::ClothEspresso),  // torn corner
        b( 3.2, 10.4,  3.4,   4.7, 15.0,  3.95,Surf::ClothCrimson),   // torn pennant tail
        b(-5.2, 15.6, -3.2,   5.2, 17.2,  3.2, Surf::SteelDark),      // mantle bar
        b(-2.0, 16.0, -3.4,   2.0, 17.2, -2.85,Surf::Brass),          // clasp plate
        b(-0.8, 16.2, -3.55,  0.8, 17.0, -3.25,Surf::Ember),          // the ember in the clasp
        // ---- sculpt pass: the cape is caught mid-move ------------------------
        br(-5.4,  2.6,  2.7,  -2.2, 15.2,  4.0,   0.0, 5.0, 11.0, Surf::ClothCrimson), // panel swung out, L
        br( 2.2,  2.6,  2.7,   5.4, 15.2,  4.0,   0.0,-5.0,-11.0, Surf::ClothCrimson), // panel swung out, R
        br(-4.8,  1.2,  3.0,   0.6,  3.6,  4.3,  14.0, 0.0,  6.0, Surf::ClothEspresso),// hem lifting
        br( 3.0,  9.6,  3.3,   5.2, 15.4,  4.1,   0.0, 0.0,-24.0, Surf::ClothCrimson), // pennant tail, flying
        br(-5.3, 15.5, -3.25,  5.3, 17.3,  3.25,  0.0, 0.0,  0.0, Surf::SteelDark),    // mantle bar
        br(-5.35,16.9, -3.2,   5.35,17.5,  3.2,  45.0, 0.0,  0.0, Surf::SteelBright),  // its chamfered top
    ],
};

// --- WEAPON ----------------------------------------------------------------
//
// Every weapon is authored around the RIGHT hand, whose centre on the Auren body
// is (x 4.85, y 7.3, z −0.35). Change the body's arm and every weapon moves —
// which is correct: they are held, not floating.

#[rustfmt::skip]
pub static SICKLE_FIELD: Part = Part {
    id: "sickle_field", name: "Field Sickle", slot: Slot::Weapon,
    note: "wood haft + rusted iron; a tool, not a weapon — the villager read",
    boxes: &[
        b( 4.4,  6.0, -0.95,  5.3,  10.5, -0.05, Surf::WoodHaft),     // haft
        b( 4.35, 7.6, -1.0,   5.35,  8.5,   0.0, Surf::LeatherStrap), // grip wrap
        b( 4.5, 10.2, -1.0,   5.2,  11.1,  1.4,  Surf::SteelRust),    // blade base
        b( 4.5, 10.6,  1.2,   5.2,  11.3,  3.6,  Surf::SteelRust),    // the curve
        b( 4.5,  9.5,  3.2,   5.2,  10.9,  4.4,  Surf::SteelRust),    // tip hooking down
        // ---- sculpt pass: a sickle is a CURVE, so give it three angles -------
        br( 4.52,10.15, -1.0,  5.18, 11.15,  1.5,  -22.0,0.0,0.0, Surf::SteelRust),  // blade leaves the haft
        br( 4.52,10.5,   1.3,  5.18, 11.35,  3.7,   14.0,0.0,0.0, Surf::SteelRust),  // the belly of the curve
        br( 4.55, 9.3,   3.1,  5.15, 10.8,   4.6,   42.0,0.0,0.0, Surf::SteelRust),  // tip hooks back down
        br( 4.42, 5.9,  -1.02, 5.28, 10.6,  -0.02,  0.0, 0.0,3.0, Surf::WoodHaft),   // haft, slightly canted
    ],
};

#[rustfmt::skip]
pub static SWORD_SHORT: Part = Part {
    id: "sword_short", name: "Traveller's Shortsword", slot: Slot::Weapon,
    note: "the bible's mundane blade: steel, leather grip, brass pommel, one edge nick",
    boxes: &[
        b( 4.4,  3.2, -0.95,  5.3,   7.3, -0.05, Surf::SteelBright),  // blade, point down
        b( 4.5,  2.5, -0.9,   5.2,   3.3, -0.1,  Surf::SteelBright),  // point taper
        b( 3.6,  7.25,-1.2,   6.1,   7.95, 0.2,  Surf::SteelDark),    // crossguard
        b( 4.45, 7.85,-1.0,   5.25,  9.6, -0.05, Surf::LeatherStrap), // leather-wrapped grip
        b( 4.35, 9.5, -1.05,  5.35, 10.25, 0.05, Surf::Brass),        // pommel
        b( 4.38, 4.6, -0.98,  5.32,  5.05,-0.02, Surf::SteelDark),    // a nick in the edge
        // ---- sculpt pass: a REAL edge ---------------------------------------
        // A square blade cross-section is a bar. Rolling a second, inset square
        // 45° about the blade's own long axis makes the section a diamond, so the
        // blade has two edges and a spine — and the key light runs down the spine
        // as one bright line, which is the entire visual read of "sword".
        br( 4.52, 3.1, -0.83,  5.18,  7.35,-0.17,  0.0,45.0,0.0, Surf::SteelBright), // diamond section
        br( 4.58, 2.35,-0.77,  5.12,  3.25,-0.23,  0.0,45.0,0.0, Surf::SteelBright), // point, same section
        br( 3.55, 7.2, -1.25,  6.15,  8.0,  0.25,  0.0, 0.0,4.0, Surf::SteelDark),   // crossguard, canted
        br( 3.5,  7.35,-1.15,  4.15,  7.9,  0.15,  0.0, 0.0,14.0,Surf::SteelDark),   // guard tip up, L
        br( 5.55, 7.35,-1.15,  6.2,   7.9,  0.15,  0.0, 0.0,-14.0,Surf::SteelDark),  // guard tip up, R
        br( 4.33, 9.45,-1.08,  5.37, 10.3,  0.08, 45.0, 0.0,0.0,  Surf::Brass),      // pommel, faceted
    ],
};

#[rustfmt::skip]
pub static HAMMER_WAR: Part = Part {
    id: "hammer_war", name: "Warhammer", slot: Slot::Weapon,
    note: "head stands a full block above the shoulder — reads as 'heavy' in a cutout",
    boxes: &[
        b( 4.4,  5.0, -1.0,   5.3,  13.7, -0.1,  Surf::WoodHaft),     // haft
        b( 4.35, 7.0, -1.05,  5.35,  9.4,  -0.05,Surf::LeatherStrap), // grip wrap
        b( 4.3,  4.5, -1.1,   5.4,   5.4,   0.0, Surf::SteelDark),    // butt cap
        b( 3.5, 13.2, -1.9,   6.2,  15.8,   0.9, Surf::SteelBright),  // hammer head
        b( 3.3, 13.6, -2.05,  3.62, 15.4,   1.05,Surf::SteelRust),    // chipped striking face
        b( 6.2, 14.0, -1.7,   7.5,  15.0,   0.7, Surf::SteelDark),    // back spike
        b( 3.4, 15.6, -1.9,   6.3,  16.15,  0.9, Surf::Brass),        // head trim
        b( 4.3, 15.9, -1.1,   5.4,  16.7,   0.0, Surf::SteelDark),    // top spike
        // ---- sculpt pass: a forged head has bevelled faces -------------------
        br( 3.45,15.65,-1.95,  6.25, 16.2,   0.95, 45.0,0.0, 0.0, Surf::SteelBright), // head top chamfer
        br( 3.45,12.9, -1.95,  6.25, 13.45,  0.95, 45.0,0.0, 0.0, Surf::SteelBright), // head bottom chamfer
        br( 6.15,13.9, -1.75,  7.7,  15.1,   0.75,  0.0,0.0,-16.0, Surf::SteelDark),  // back spike, raked down
        br( 4.25,15.8, -1.15,  5.45, 17.0,   0.05, 12.0,0.0,  6.0, Surf::SteelDark),  // top spike, canted
        br( 4.28, 4.4, -1.12,  5.42,  5.5,   0.02, 45.0,0.0,  0.0, Surf::SteelDark),  // butt cap, faceted
    ],
};

/// The lore weapon. Still no teal — the escalation ladder keeps that in reserve;
/// this is a warm coal on a stave, the same accent family as the ember pouch.
#[rustfmt::skip]
pub static STAVE_SHAPER: Part = Part {
    id: "stave_shaper", name: "Shaper's Stave", slot: Slot::Weapon,
    note: "the tallest weapon; ember crown, brass ferrules, zero teal on purpose",
    boxes: &[
        b( 4.45, 1.0, -0.95,  5.25, 17.6, -0.05, Surf::WoodHaft),     // stave
        b( 4.4,  7.0, -1.0,   5.3,   9.2,  0.0,  Surf::LeatherStrap), // grip wrap
        b( 4.35,17.4, -1.05,  5.35, 18.4,  0.05, Surf::EmberHousing), // crown housing
        b( 4.15,18.2, -1.25,  5.55, 19.5,  0.25, Surf::EmberHousing), // crown cage
        b( 4.45,18.6, -0.95,  5.25, 19.2, -0.05, Surf::Ember),        // the coal
        b( 4.38, 3.6, -0.98,  5.32,  4.25,-0.02, Surf::Brass),        // ferrule, low
        b( 4.38,12.0, -0.98,  5.32, 12.65,-0.02, Surf::Brass),        // ferrule, high
        // ---- sculpt pass: the crown is a CAGE, not a lump --------------------
        br( 4.1, 18.1, -1.3,   5.6,  19.6,  0.3,   0.0,45.0, 0.0, Surf::EmberHousing), // cage, turned 45°
        br( 4.3, 17.3, -1.1,   5.4,  18.5,  0.1,  45.0, 0.0, 0.0, Surf::EmberHousing), // housing collar
        br( 4.42,18.55,-0.98,  5.28, 19.25,-0.02, 45.0,45.0, 0.0, Surf::Ember),        // the coal, faceted
        br( 4.36, 3.55,-1.0,   5.34,  4.3,   0.0,  45.0, 0.0, 0.0, Surf::Brass),       // ferrule, faceted
        br( 4.36,11.95,-1.0,   5.34, 12.7,   0.0,  45.0, 0.0, 0.0, Surf::Brass),       // ferrule, faceted
    ],
};

/// Garren's spear, pulled out of his monolithic body so a SECOND character proves
/// the slot machinery is general and not an Auren special case.
#[rustfmt::skip]
pub static SPEAR_GUARD: Part = Part {
    id: "spear_guard", name: "Guard-Issue Spear", slot: Slot::Weapon,
    note: "the vertical line that names a guard at any distance",
    boxes: &[
        b( 6.3,  0.6, -3.5,   7.2, 25.6, -2.6, Surf::WoodHaft),
        b( 6.1, 25.6, -3.7,   7.4, 28.4, -2.4, Surf::HuskPlate),
        b( 6.1,  8.0, -3.7,   7.4,  9.2, -2.4, Surf::HuskPlateDark),
    ],
};

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// Every part in the game, in slot order. A future item system indexes this by
/// [`Part::id`]; the catalogue dump below is its serialised form.
pub static PARTS: &[&Part] = &[
    &KERCHIEF,
    &HOOD_TRAVEL,
    &HELM_GREAT,
    &TUNIC_LINEN,
    &JERKIN_LEATHER,
    &CUIRASS_STEEL,
    &TROUSERS_WORK,
    &BREECHES_TRAVEL,
    &GREAVES_PLATE,
    &WRAPS_CLOTH,
    &GLOVES_LEATHER,
    &GAUNTLETS_STEEL,
    &PACK_HARVEST,
    &CLOAK_HALF,
    &CAPE_BATTLE,
    &SICKLE_FIELD,
    &SWORD_SHORT,
    &HAMMER_WAR,
    &STAVE_SHAPER,
    &SPEAR_GUARD,
];

pub fn part(id: &str) -> Option<&'static Part> {
    PARTS.iter().copied().find(|p| p.id == id)
}

// ---------------------------------------------------------------------------
// The sculpt lever — one binary shoots before AND after
// ---------------------------------------------------------------------------

/// `VOXELFORGE_SCULPT=0` renders the **pre-sculpt** tables: every part truncated
/// to the boxes it had before the 2026-08-14 sculpt pass, and Auren's body falls
/// back to `AUREN_V2`.
///
/// This exists because the alternative — build the old code into one exe, build
/// the new code into another, shoot one plate with each — is exactly the
/// comparison this lane has been burned by before: two binaries can differ by
/// driver state, shader-cache warmth, a teammate's unrelated commit, or nothing at
/// all. One process, one camera, one sun, one frame counter, and the ONLY
/// difference between the two plates is how many boxes came out of the table.
pub fn sculpt_on() -> bool {
    !matches!(
        std::env::var("VOXELFORGE_SCULPT").ok().as_deref().map(str::trim),
        Some("0") | Some("off") | Some("false") | Some("no")
    )
}

/// How many of a part's boxes predate the sculpt pass.
///
/// Every sculpt-pass box was **appended**, so the old part is exactly the first
/// `presculpt_len(id)` entries — no second table to drift out of sync with the
/// first. The numbers are checked at runtime against `PARTS` by
/// [`assert_presculpt_prefixes`], because a hand-maintained count that silently
/// goes stale would quietly turn the before plate into a lie.
pub fn presculpt_len(id: &str) -> usize {
    match id {
        "kerchief" => 4,
        "hood_travel" => 8,
        "helm_great" => 13,
        "tunic_linen" => 11,
        "jerkin_leather" => 16,
        "cuirass_steel" => 16,
        "trousers_work" => 8,
        "breeches_travel" => 14,
        "greaves_plate" => 13,
        "wraps_cloth" => 4,
        "gloves_leather" => 9,
        "gauntlets_steel" => 11,
        "pack_harvest" => 8,
        "cloak_half" => 13,
        "cape_battle" => 8,
        "sickle_field" => 5,
        "sword_short" => 6,
        "hammer_war" => 8,
        "stave_shaper" => 7,
        "spear_guard" => 3,
        // An unknown id means a part was added without a count. Falling back to
        // "no sculpt boxes" would make the before/after pair show a difference
        // that is really a missing table entry, so fall back to the FULL list:
        // the pair then shows no change, which is a visible, honest failure.
        _ => usize::MAX,
    }
}

/// The boxes to draw for `p` under the current lever.
pub fn boxes_for(p: &'static Part) -> &'static [Bx] {
    if sculpt_on() {
        p.boxes
    } else {
        &p.boxes[..presculpt_len(p.id).min(p.boxes.len())]
    }
}

/// Prove every count above is a real prefix length and not a stale number.
///
/// Cheap enough to run at startup (20 parts, one comparison each) and it prints,
/// so the runlog beside a before/after pair carries the check that makes the pair
/// meaningful. Returns the number of parts whose count is out of range.
pub fn assert_presculpt_prefixes() -> usize {
    let mut bad = 0;
    for p in PARTS {
        let n = presculpt_len(p.id);
        if n == usize::MAX {
            println!("SCULPT_PREFIX part={} MISSING count — before plate will show it unchanged", p.id);
            bad += 1;
        } else if n > p.boxes.len() {
            println!(
                "SCULPT_PREFIX part={} STALE count={} > boxes={}",
                p.id,
                n,
                p.boxes.len()
            );
            bad += 1;
        }
    }
    let added: usize =
        PARTS.iter().map(|p| p.boxes.len().saturating_sub(presculpt_len(p.id).min(p.boxes.len()))).sum();
    let total: usize = PARTS.iter().map(|p| p.boxes.len()).sum();
    println!(
        "SCULPT_PREFIX parts={} boxes_total={} boxes_added_by_sculpt={} bad_counts={} sculpt_on={}",
        PARTS.len(),
        total,
        added,
        bad,
        sculpt_on()
    );
    bad
}

// ---------------------------------------------------------------------------
// Presets — three complete outfits
// ---------------------------------------------------------------------------

/// Edhari villager: cloth everywhere, a wicker pack, a farm tool. No metal on the
/// body at all except the rust on the sickle, so it lights completely flat.
pub fn villager() -> Loadout {
    Loadout::EMPTY
        .with(&KERCHIEF)
        .with(&TUNIC_LINEN)
        .with(&TROUSERS_WORK)
        .with(&WRAPS_CLOTH)
        .with(&PACK_HARVEST)
        .with(&SICKLE_FIELD)
}

/// The hero as Act I opens: leather, one pauldron, the half-cloak, a mundane
/// sword and the ember pouch. This is the default Auren.
pub fn adventurer() -> Loadout {
    Loadout::EMPTY
        .with(&HOOD_TRAVEL)
        .with(&JERKIN_LEATHER)
        .with(&BREECHES_TRAVEL)
        .with(&GLOVES_LEATHER)
        .with(&CLOAK_HALF)
        .with(&SWORD_SHORT)
}

/// Full plate. Every slot is metal except the cape — the maximum-contrast end of
/// the material ladder, and the proof that the same body can carry it.
pub fn warplate() -> Loadout {
    Loadout::EMPTY
        .with(&HELM_GREAT)
        .with(&CUIRASS_STEEL)
        .with(&GREAVES_PLATE)
        .with(&GAUNTLETS_STEEL)
        .with(&CAPE_BATTLE)
        .with(&HAMMER_WAR)
}

pub fn preset(name: &str) -> Option<(Loadout, &'static str)> {
    match name.trim().to_ascii_lowercase().as_str() {
        "bare" | "naked" | "body" => Some((Loadout::EMPTY, "bare")),
        "villager" | "village" => Some((villager(), "villager")),
        "adventurer" | "traveller" | "traveler" | "hero" => Some((adventurer(), "adventurer")),
        "warplate" | "war" | "plate" | "armour" | "armor" => Some((warplate(), "warplate")),
        _ => None,
    }
}

/// **The four-plate gear ladder.** Bare → villager → adventurer → warplate, i.e.
/// zero slots filled → six cloth → six leather/brass → six steel.
///
/// Ordered by material weight on purpose: read left to right, the sheet is a ramp
/// from "no specular anywhere" to "every slot glints", which is the claim the
/// material classes in this file exist to make.
pub fn gear_ladder() -> [(&'static str, Loadout); 4] {
    [
        ("bare", Loadout::EMPTY),
        ("villager", villager()),
        ("adventurer", adventurer()),
        ("warplate", warplate()),
    ]
}

/// The three outfits, in the order the swap proof walks them.
pub fn outfit_ladder() -> [(&'static str, Loadout); 3] {
    [("villager", villager()), ("adventurer", adventurer()), ("warplate", warplate())]
}

/// The three weapons the weapon-swap proof cycles through, on one body that never
/// changes anything else.
pub fn weapon_ladder() -> [(&'static str, &'static Part); 3] {
    [("sword", &SWORD_SHORT), ("hammer", &HAMMER_WAR), ("stave", &STAVE_SHAPER)]
}

// ---------------------------------------------------------------------------
// Spawn + hot-swap
// ---------------------------------------------------------------------------

/// What a character is currently wearing, and where each slot's node lives.
///
/// Holding the node entities here rather than walking `Children` is what makes
/// [`equip`] a two-line operation with no hierarchy query: despawn the stored
/// entity, spawn a new one, overwrite the slot.
#[derive(Component, Clone, Debug)]
pub struct Wardrobe {
    pub loadout: Loadout,
    pub nodes: [Option<Entity>; SLOT_COUNT],
}

impl Wardrobe {
    pub fn new(loadout: Loadout) -> Self {
        Wardrobe { loadout, nodes: [None; SLOT_COUNT] }
    }
}

/// Spawn one slot's geometry as a child of `root` and return the node entity.
/// `None` part ⇒ an empty node, so the slot still exists and can be filled later
/// without special-casing "was there anything there before?".
pub fn spawn_slot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    root: Entity,
    slot: Slot,
    part: Option<&'static Part>,
) -> Entity {
    let node = commands
        .spawn((
            Transform::IDENTITY,
            // Explicit, never inherited-by-luck: a node spawned without a
            // Visibility never gets a ViewVisibility and is silently never
            // extracted — the exact failure that made the sky dome "render" into
            // an empty frame for a day.
            Visibility::default(),
            SlotNode(slot),
            Name::new(match part {
                Some(p) => format!("slot:{}={}", slot.id(), p.id),
                None => format!("slot:{}=<empty>", slot.id()),
            }),
            ChildOf(root),
        ))
        .id();

    if let Some(p) = part {
        commands.entity(node).with_children(|c| {
            for bx in boxes_for(p) {
                let size = bx.size();
                c.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(pal.get(bx.surf)),
                    bx.transform(),
                    Visibility::default(),
                ));
            }
        });
    }
    node
}

/// **The hot-swap.** Replace whatever is in `slot` with `part`, touching nothing
/// else on the body: same root entity, same transform, same other five slots,
/// same materials.
///
/// Prints `EQUIP root=<id> slot=<slot> <old> -> <new> boxes=<n>` so a swap is
/// visible in a log without a debugger — that line is the evidence the CEO asked
/// for, and it carries the root entity id precisely so a reader can check the
/// three captures came off ONE body rather than three respawns.
pub fn equip(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    root: Entity,
    wardrobe: &mut Wardrobe,
    slot: Slot,
    part: Option<&'static Part>,
) {
    let old = wardrobe.loadout.get(slot).map(|p| p.id).unwrap_or("-");
    if let Some(node) = wardrobe.nodes[slot.idx()].take() {
        // Recursive since Bevy 0.16 — the node's boxes go with it.
        commands.entity(node).despawn();
    }
    let node = spawn_slot(commands, meshes, pal, root, slot, part);
    wardrobe.nodes[slot.idx()] = Some(node);
    wardrobe.loadout.parts[slot.idx()] = part;

    println!(
        "EQUIP root={:?} slot={} {} -> {} boxes={}",
        root,
        slot.id(),
        old,
        part.map(|p| p.id).unwrap_or("-"),
        part.map(|p| p.boxes.len()).unwrap_or(0)
    );
}

/// Swap every slot at once — an outfit change. Still one entity: this is a loop
/// over [`equip`], never a despawn of the root.
pub fn equip_loadout(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    root: Entity,
    wardrobe: &mut Wardrobe,
    loadout: Loadout,
) {
    for slot in Slot::ALL {
        equip(commands, meshes, pal, root, wardrobe, slot, loadout.get(slot));
    }
}

// ---------------------------------------------------------------------------
// Catalogue dump — the hook a future item/inventory system reads
// ---------------------------------------------------------------------------

/// Serialise the whole registry to JSON: every part, its slot, its box count, the
/// material classes it puts on screen, and the three presets.
///
/// Hand-rolled rather than serde-derived on purpose — this file must stay free of
/// anything the isolated shot binary doesn't already link, and the schema is small
/// enough that a `format!` is more readable than five `#[derive]`s.
pub fn catalog_json() -> String {
    let mut s = String::from("{\n  \"schema\": \"voxelforge.equipment/1\",\n  \"slots\": [");
    s.push_str(
        &Slot::ALL.iter().map(|x| format!("\"{}\"", x.id())).collect::<Vec<_>>().join(", "),
    );
    s.push_str("],\n  \"surfaces\": [\n");
    let surfs: Vec<String> = Surf::ALL
        .iter()
        .map(|x| {
            let pr = x.props();
            format!(
                "    {{ \"surf\": \"{:?}\", \"class\": \"{}\", \"rgb\": \"#{:02X}{:02X}{:02X}\", \
                 \"roughness\": {:.2}, \"metallic\": {:.2}, \"reflectance\": {:.2} }}",
                x,
                pr.mat.label(),
                pr.rgb.0,
                pr.rgb.1,
                pr.rgb.2,
                pr.rough,
                pr.metal,
                pr.refl
            )
        })
        .collect();
    s.push_str(&surfs.join(",\n"));
    s.push_str("\n  ],\n  \"parts\": [\n");
    let parts: Vec<String> = PARTS
        .iter()
        .map(|p| {
            format!(
                "    {{ \"id\": \"{}\", \"name\": \"{}\", \"slot\": \"{}\", \"boxes\": {}, \
                 \"materials\": [{}], \"note\": \"{}\" }}",
                p.id,
                p.name,
                p.slot.id(),
                p.boxes.len(),
                p.mats().iter().map(|m| format!("\"{}\"", m.label())).collect::<Vec<_>>().join(", "),
                p.note
            )
        })
        .collect();
    s.push_str(&parts.join(",\n"));
    s.push_str("\n  ],\n  \"presets\": [\n");
    let presets: Vec<String> = ["villager", "adventurer", "warplate"]
        .iter()
        .map(|n| {
            let (l, _) = preset(n).expect("preset name is one of the three literals above");
            format!(
                "    {{ \"id\": \"{}\", \"boxes\": {}, \"wear\": {{ {} }} }}",
                n,
                l.box_count(),
                Slot::ALL
                    .iter()
                    .map(|s| format!(
                        "\"{}\": {}",
                        s.id(),
                        l.get(*s).map(|p| format!("\"{}\"", p.id)).unwrap_or_else(|| "null".into())
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    s.push_str(&presets.join(",\n"));
    s.push_str("\n  ]\n}\n");
    s
}
