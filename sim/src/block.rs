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
    ];

    #[inline]
    pub fn is_opaque(self) -> bool {
        self.0 != 0
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
            _                 => "unknown",
        }
    }

    /// Base (unshaded) sRGB colour for the procedural atlas tile.
    /// Client reads this to build its atlas row for each block.
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
            _                 => [255, 0, 255], // error magenta
        }
    }
}
