//! Block type — a single u8 wrapping the block ID.
//!
//! Using a newtype keeps the type system honest (no confusion with raw u8 counts)
//! while staying zero-cost. 0 == air, 1-255 == opaque terrain blocks.
//!
//! The palette is open-ended: new block types are additive (just append a const
//! here and add its colour to the client atlas).

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct BlockId(pub u8);

impl BlockId {
    // ---- original palette (IDs 0-4) ----
    pub const AIR:   Self = Self(0);
    pub const GRASS: Self = Self(1);
    pub const DIRT:  Self = Self(2);
    pub const STONE: Self = Self(3);
    pub const SAND:  Self = Self(4);

    // ---- expanded palette (IDs 5-15) ----
    pub const WOOD:        Self = Self(5);
    pub const LEAVES:      Self = Self(6);
    pub const SNOW:        Self = Self(7);
    pub const RED_SAND:    Self = Self(8);
    pub const CLAY:        Self = Self(9);
    pub const GRAVEL:      Self = Self(10);
    pub const COBBLESTONE: Self = Self(11);
    pub const OBSIDIAN:    Self = Self(12);
    pub const BRICK:       Self = Self(13);
    pub const MOSS:        Self = Self(14);
    pub const LIMESTONE:   Self = Self(15);
    pub const LAMP:        Self = Self(16);
    /// The first block that is solid but **not opaque** — see [`BlockId::is_opaque`].
    pub const GLASS:       Self = Self(17);
    /// The second solid-but-not-opaque block, and the only **liquid**: a river.
    ///
    /// Solid so a player stands on its surface (same contract as the pane — the
    /// mesher's opinion must not leak into collision), see-through so the sandy
    /// bed under it renders. The client mesher additionally drops its top face
    /// to 14/16 of the cell; that is presentation, not data, and lives in
    /// `client/src/voxel.rs`.
    pub const WATER:       Self = Self(18);
    /// Opaque structural metal — the one block in the palette whose material
    /// response is allowed a `metallic > 0`.
    pub const METAL:       Self = Self(19);

    // ---- shaped palette (IDs 20-24) ----
    //
    // The first blocks that do NOT fill their cell. Everything above is a cube;
    // these five wear a `"mode"` in `assets/textures/blocks/atlas.json` and the
    // client emits their real geometry (`client/src/block_shapes.rs`) instead of
    // six faces. Data-side that makes them exactly one thing: **not opaque** —
    // a half-height slab must not cull the face of the block beside it, or the
    // wall behind a fence post disappears.
    //
    // They stay `is_solid()`, so collision keeps treating the cell as full. That
    // is a deliberate, documented approximation and the honest one to make first:
    // a stair you can walk up (one cube step) reads right, while a stair you fall
    // through does not. Per-shape collision hulls are a separate pass.
    /// Wooden step — a bottom slab plus a back half-block, auto-facing the way
    /// it is climbed (see `block_shapes::stair_facing`).
    pub const STAIR_WOOD:  Self = Self(20);
    /// Stone half-block: the bottom half of the cell.
    pub const SLAB_STONE:  Self = Self(21);
    /// Fence post + rails, joined to whatever it stands next to.
    pub const FENCE_WOOD:  Self = Self(22);
    /// A thin sheet of glass, joined along whichever axis it continues on.
    pub const PANE_GLASS:  Self = Self(23);
    /// Two crossed alpha-masked quads — the placeable, deliberate cousin of the
    /// ambient scatter in `scene::scatter_foliage`.
    pub const PLANT_CROSS: Self = Self(24);

    /// All placeable (non-air) blocks in palette order — used by the client HUD
    /// cycle and the editor's pick row.
    pub const ALL_PLACEABLE: &[Self] = &[
        Self::GRASS,
        Self::DIRT,
        Self::STONE,
        Self::SAND,
        Self::WOOD,
        Self::LEAVES,
        Self::SNOW,
        Self::RED_SAND,
        Self::CLAY,
        Self::GRAVEL,
        Self::COBBLESTONE,
        Self::OBSIDIAN,
        Self::BRICK,
        Self::MOSS,
        Self::LIMESTONE,
        Self::LAMP,
        Self::GLASS,
        Self::WATER,
        Self::METAL,
        Self::STAIR_WOOD,
        Self::SLAB_STONE,
        Self::FENCE_WOOD,
        Self::PANE_GLASS,
        Self::PLANT_CROSS,
    ];

    /// Does this block render as something other than a cube?
    ///
    /// The one question the shaped palette adds, and the reason it is here rather
    /// than in the client: [`BlockId::is_opaque`] has to answer it too, and a
    /// second list of shaped ids on the client would be a contract that has to
    /// agree with this one.
    #[inline]
    pub fn is_shaped(self) -> bool {
        matches!(
            self,
            Self::STAIR_WOOD
                | Self::SLAB_STONE
                | Self::FENCE_WOOD
                | Self::PANE_GLASS
                | Self::PLANT_CROSS
        )
    }

    /// Does this block **occupy** its cell? Everything but air.
    ///
    /// This is the question collision, map saving, spawn clearance and the LOD
    /// downsample are actually asking. They asked [`BlockId::is_opaque`] until
    /// glass arrived, because the two answers used to be the same one.
    #[inline]
    pub fn is_solid(self) -> bool {
        self.0 != 0
    }

    /// Does this block **hide what is behind it**?
    ///
    /// The mesher's question, and only the mesher's: a face is culled when its
    /// neighbour is opaque, and ambient occlusion is cast by opaque blocks. A
    /// pane of glass is solid to a player and invisible to both — which is why
    /// this is no longer just "not air".
    ///
    /// A shaped block answers `false` for a different reason than glass does: not
    /// because you can see through it, but because it does not *fill* the cell.
    /// A slab that culled its neighbour's face would punch a hole in the floor
    /// beside it, and a fence post casting a full cell of AO would darken a whole
    /// square of ground.
    #[inline]
    pub fn is_opaque(self) -> bool {
        self.0 != 0
            && self.0 != Self::GLASS.0
            && self.0 != Self::WATER.0
            && !self.is_shaped()
    }

    /// Is this the liquid? Water is the only one; the question exists apart
    /// from "not opaque" because the client mesher treats a liquid column as
    /// having a *surface* (top face lowered, side faces shaved to match) while
    /// a pane keeps its full cell.
    #[inline]
    pub fn is_liquid(self) -> bool {
        self.0 == Self::WATER.0
    }

    /// Human-readable name (lowercase, no spaces) — used by map files and the HUD.
    pub fn name(self) -> &'static str {
        match self {
            Self::AIR         => "air",
            Self::GRASS       => "grass",
            Self::DIRT        => "dirt",
            Self::STONE       => "stone",
            Self::SAND        => "sand",
            Self::WOOD        => "wood",
            Self::LEAVES      => "leaves",
            Self::SNOW        => "snow",
            Self::RED_SAND    => "red_sand",
            Self::CLAY        => "clay",
            Self::GRAVEL      => "gravel",
            Self::COBBLESTONE => "cobblestone",
            Self::OBSIDIAN    => "obsidian",
            Self::BRICK       => "brick",
            Self::MOSS        => "moss",
            Self::LIMESTONE   => "limestone",
            Self::LAMP        => "lamp",
            Self::GLASS       => "glass",
            Self::WATER       => "water",
            Self::METAL       => "metal",
            // The shaped five. These names are also the `kinds` keys the client
            // atlas looks them up by (`voxel::atlas_kind` is `name()`), so the
            // manifest entry that gives a stair its `"mode": "stair"` is found
            // under exactly this string.
            Self::STAIR_WOOD  => "stair_wood",
            Self::SLAB_STONE  => "slab_stone",
            Self::FENCE_WOOD  => "fence_wood",
            Self::PANE_GLASS  => "pane_glass",
            Self::PLANT_CROSS => "plant_cross",
            _                 => "unknown",
        }
    }

    /// Base (unshaded) sRGB colour for the procedural atlas tile.
    /// Client reads this to build its atlas row for each block.
    ///
    /// Every value except `LAMP` is Monanisa's, from `docs/block-palette.md`
    /// §6.1 ("New hex") — the golden-hour pass over all 16 slots. They are
    /// *unshaded albedo* on purpose: `client/src/look.rs` adds the warm key and
    /// haze afterwards, so feeding this table a warm-lit pixel would double-count
    /// the grade. Do not re-tune them here; the hex belongs to the designer and
    /// `scripts/block_palette_audit.py` fails if this table drifts off her doc.
    pub fn base_color(self) -> [u8; 3] {
        match self {
            Self::AIR         => [0, 0, 0],
            Self::GRASS       => [91, 140, 70],     // #5b8c46
            Self::DIRT        => [107, 85, 64],     // #6b5540
            Self::STONE       => [143, 135, 118],   // #8f8776
            Self::SAND        => [214, 202, 148],
            Self::WOOD        => [156, 107, 58],    // #9c6b3a
            Self::LEAVES      => [58, 116, 54],     // #3a7436
            Self::SNOW        => [240, 236, 224],   // #f0ece0
            Self::RED_SAND    => [200, 130, 70],
            Self::CLAY        => [126, 150, 160],   // #7e96a0
            Self::GRAVEL      => [110, 100, 94],
            Self::COBBLESTONE => [140, 138, 120],   // #8c8a78
            Self::OBSIDIAN    => [26, 22, 32],      // #1a1620
            Self::BRICK       => [150, 90, 60],
            Self::MOSS        => [75, 110, 55],     // #4b6e37
            Self::LIMESTONE   => [222, 204, 168],   // #decca8
            // PROVISIONAL — the one block with no designer hex. `block-palette.md`
            // §4 was written when the lamp had no `BlockId` at all, so its only
            // lamp entry is a *glow gradient* for an unloaded bonus tile
            // (`#fff0ce`→`#ffb25a`), not a flat albedo for this table. This is the
            // value the client has actually been painting the lamp tile with
            // (`voxel.rs::tile_base` used to carry its own copy) — kept byte-exact
            // so unifying the two changed no pixel. Awaiting Monanisa; see
            // `docs/note-to-monanisa-lamp-albedo-gap-2026-08-14.md`.
            Self::LAMP        => [255, 196, 118],   // #ffc476
            // The pane's colour is the artist's `glass.png`, alpha and all — this
            // entry only exists so the block is never error-magenta on the
            // procedural fallback path (`VOXELFORGE_ATLAS_MODE=off`). A pale
            // cool tint, because that is what an untextured pane should read as.
            Self::GLASS       => [198, 222, 226],
            // Same status as the pane: Monanisa's `water.png` / `metal.png` are
            // the real colour, and these entries exist so the procedural
            // fallback path (`VOXELFORGE_ATLAS_MODE=off`) paints a believable
            // deep-water blue and a bright worked metal instead of magenta.
            // Water is the sunset-valley river blue-green, dark enough that a
            // 0.6-alpha material over it still reads as water; metal is a warm
            // unweathered steel bright enough to take the key.
            Self::WATER       => [38, 92, 118],    // #265c76
            Self::METAL       => [178, 174, 166],  // #b2aea6
            // The shaped five deliberately REUSE their material's designer hex
            // rather than inventing one: a wooden stair is wood, seen at a
            // different angle. Borrowing keeps `docs/block-palette.md` the one
            // place a colour is decided — a new hex here would be a second
            // palette nobody signed off. (Only the procedural fallback path,
            // `VOXELFORGE_ATLAS_MODE=off`, ever paints with these; the shipped
            // path takes its albedo from the `kinds` entry in atlas.json.)
            Self::STAIR_WOOD | Self::FENCE_WOOD => Self::WOOD.base_color(),
            Self::SLAB_STONE  => Self::STONE.base_color(),
            Self::PANE_GLASS  => Self::GLASS.base_color(),
            Self::PLANT_CROSS => Self::LEAVES.base_color(),
            _                 => [255, 0, 255], // error magenta
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The designer's shipped palette, transcribed from `docs/block-palette.md`
    /// §6.1 "New hex". Pinned here because the village's whole read comes off
    /// this table and nothing else in the build notices a one-digit edit: a
    /// wrong hex still compiles, still renders, and only shows up as "the
    /// village looks like coloured boxes again" in a screenshot days later.
    ///
    /// `lamp` is deliberately absent — it has no designer hex yet, and a test
    /// that pinned the placeholder would make the gap look decided.
    const DESIGNER_HEX: &[(BlockId, u32)] = &[
        (BlockId::GRASS, 0x5b8c46),
        (BlockId::DIRT, 0x6b5540),
        (BlockId::STONE, 0x8f8776),
        (BlockId::SAND, 0xd6ca94),
        (BlockId::WOOD, 0x9c6b3a),
        (BlockId::LEAVES, 0x3a7436),
        (BlockId::SNOW, 0xf0ece0),
        (BlockId::RED_SAND, 0xc88246),
        (BlockId::CLAY, 0x7e96a0),
        (BlockId::GRAVEL, 0x6e645e),
        (BlockId::COBBLESTONE, 0x8c8a78),
        (BlockId::OBSIDIAN, 0x1a1620),
        (BlockId::BRICK, 0x965a3c),
        (BlockId::MOSS, 0x4b6e37),
        (BlockId::LIMESTONE, 0xdecca8),
    ];

    #[test]
    fn base_color_matches_the_designer_palette() {
        for &(id, hex) in DESIGNER_HEX {
            let want = [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8];
            assert_eq!(
                id.base_color(),
                want,
                "{} drifted off docs/block-palette.md §6.1 (#{hex:06x})",
                id.name()
            );
        }
    }

    /// `is_solid` and `is_opaque` used to be the same function. They are not any
    /// more, and the split is load-bearing: collision asks the first, the mesher
    /// asks the second, and swapping them gives you either a world you fall
    /// through or a pane you cannot see past. Water joins glass in the
    /// solid-but-see-through pair — a river you stand on and see the bed of.
    ///
    /// The shaped five are the *second* reason the two answers differ, and it is
    /// not transparency: a slab occupies half a cell, so it must not cull the
    /// face of the block beside it. Which is why this test is written as "every
    /// solid-but-not-opaque block is one we can name a reason for" rather than
    /// re-listing the ids — the list is `is_shaped()` plus the two see-through
    /// blocks, and anything else reaching that state is the bug.
    #[test]
    fn solid_but_not_opaque_is_exactly_glass_water_and_the_shaped_blocks() {
        assert!(BlockId::GLASS.is_solid());
        assert!(!BlockId::GLASS.is_opaque());
        assert!(BlockId::WATER.is_solid());
        assert!(!BlockId::WATER.is_opaque());
        assert!(!BlockId::AIR.is_solid());
        assert!(!BlockId::AIR.is_opaque());
        for &id in BlockId::ALL_PLACEABLE {
            let excused = id == BlockId::GLASS || id == BlockId::WATER || id.is_shaped();
            assert!(id.is_solid(), "{} stopped occupying its cell", id.name());
            assert_eq!(
                id.is_opaque(),
                !excused,
                "{} changed meaning to the mesher",
                id.name()
            );
        }
    }

    /// A shaped block still fills its cell for collision. This is the compromise
    /// the shaped palette shipped with, written down so it is a decision and not
    /// a bug someone finds later: you stand ON a slab at full block height, and a
    /// fence is as wide as its cell. Per-shape hulls are a separate pass; the day
    /// they land, this test is the one that should change.
    #[test]
    fn shaped_blocks_are_still_solid_to_collision() {
        for &id in BlockId::ALL_PLACEABLE {
            if !id.is_shaped() {
                continue;
            }
            assert!(id.is_solid(), "{} became walk-through", id.name());
            assert!(!id.is_opaque(), "{} would cull its neighbour", id.name());
        }
        // ...and no cube quietly joined them.
        for &id in &[BlockId::STONE, BlockId::WOOD, BlockId::GLASS, BlockId::WATER] {
            assert!(!id.is_shaped(), "{} is not a shaped block", id.name());
        }
    }

    /// The liquid is a class of exactly one today, and the questions "is it the
    /// liquid" and "can you see through it" must not collapse into each other:
    /// the mesher lowers a liquid's surface and leaves a pane's cell whole.
    #[test]
    fn water_is_the_only_liquid_and_it_is_not_opaque() {
        assert!(BlockId::WATER.is_liquid());
        for &id in BlockId::ALL_PLACEABLE {
            assert_eq!(
                id.is_liquid(),
                id == BlockId::WATER,
                "{} has the wrong liquid answer",
                id.name()
            );
        }
        assert!(BlockId::WATER.is_liquid() && !BlockId::WATER.is_opaque());
        // Metal is an ordinary opaque solid that merely responds like a metal.
        assert!(BlockId::METAL.is_solid() && BlockId::METAL.is_opaque());
        assert!(!BlockId::METAL.is_liquid());
    }

    /// The magenta arm is the "somebody added a BlockId and forgot its colour"
    /// alarm. It must never be reachable from a block a player can place.
    #[test]
    fn every_placeable_block_is_named_and_coloured() {
        for &id in BlockId::ALL_PLACEABLE {
            assert_ne!(id.name(), "unknown", "block {} has no name", id.0);
            assert_ne!(
                id.base_color(),
                [255, 0, 255],
                "{} falls through to error magenta",
                id.name()
            );
        }
    }

    /// The palette is a dense 1..=N run with no gaps or repeats — the client
    /// atlas indexes tiles *by id*, so a hole would paint one block with its
    /// neighbour's tile and a duplicate would hide one entirely.
    #[test]
    fn placeable_ids_are_dense_and_unique() {
        for (i, &id) in BlockId::ALL_PLACEABLE.iter().enumerate() {
            assert_eq!(
                id.0 as usize,
                i + 1,
                "{} is out of palette order",
                id.name()
            );
        }
        assert_eq!(
            BlockId::AIR.0,
            0,
            "air must stay id 0 — it is implicit in maps"
        );
    }

    /// Every designer-owned entry is a *distinct* colour. Two blocks sharing a
    /// hex is the failure the doc's §6.1 caught by eye (dirt was nearly wood);
    /// it reads as one material and quietly costs the map a block type.
    #[test]
    fn designer_colours_are_distinct() {
        for (i, &(a, _)) in DESIGNER_HEX.iter().enumerate() {
            for &(b, _) in &DESIGNER_HEX[i + 1..] {
                assert_ne!(
                    a.base_color(),
                    b.base_color(),
                    "{} and {} are the same colour",
                    a.name(),
                    b.name()
                );
            }
        }
    }
}
