//! Block type — a single u8 wrapping the block ID.
//!
//! Using a newtype keeps the type system honest (no confusion with raw u8 counts)
//! while staying zero-cost. We keep the palette small and open-ended: 0 == air,
//! 1-255 == opaque terrain blocks.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BlockId(pub u8);

impl BlockId {
    pub const AIR: Self = Self(0);
    pub const GRASS: Self = Self(1);
    pub const DIRT: Self = Self(2);
    pub const STONE: Self = Self(3);
    pub const SAND: Self = Self(4);

    #[inline]
    pub fn is_opaque(self) -> bool {
        self.0 != 0
    }
}
