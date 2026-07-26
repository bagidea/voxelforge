//! Deterministic terrain heightmap — same function used by both client mesher
//! and server to ensure identical world state without syncing every block.

use crate::chunk::CHUNK_SIZE;

/// Returns the terrain height (world Y) at the given world-space XZ position.
///
/// The formula is intentionally simple (trig noise only, no fastnoise) so the
/// server can regenerate any chunk without a noise library dep. Upgrade to a
/// proper multi-octave noise once the protocol serialises chunk payloads.
pub fn terrain_height(wx: f32, wz: f32) -> i32 {
    let h = 14.0
        + 7.0 * (wx * 0.08).sin() * (wz * 0.06).cos()
        + 3.0 * ((wx + wz) * 0.15).sin();
    (h.round() as i32).clamp(1, CHUNK_SIZE - 2)
}
