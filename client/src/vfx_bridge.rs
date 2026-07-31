//! VFX bridge — the ONE file that connects `vfx.rs` to the real game.
//!
//! `vfx.rs` is deliberately crate-independent (it `use`s nothing but `bevy`), so it
//! can be built and photographed inside the isolated `voxelforge_shot` bin. That
//! purity means it also knows nothing about combat — hence this file.
//!
//! The point of the bridge is that **it needs no edit in anyone else's lane**. The
//! combat and scene lanes already broadcast everything the VFX layer needs, they
//! just broadcast it for the *audio* lane:
//!
//! | already exists (not mine)   | who writes it | what the bridge turns it into |
//! |-----------------------------|---------------|-------------------------------|
//! | `SfxEvent::HitLight/HitHeavy` | `combat::player_combat` | `vfx::Impact` (burst + flash + hit-stop + cam kick) |
//! | `SfxEvent::HitParry/HitBlock` | `combat::enemy_attack`  | `vfx::Impact` with the parry palette |
//! | `SfxEvent::PlayerHurt`        | `combat::enemy_attack`  | `vfx::Impact` at the player, red flavour |
//! | `SfxEvent::EnemyDeath`        | `combat::husk_ai`       | `vfx::Unravel` (death dissolve) |
//! | `Campsite` resource           | `scene::setup_scene`    | a `vfx::CampfireVfx` emitter at `camp.fire` |
//! | `HuskArm` component           | `combat::spawn_guard_husk` | a `vfx::SwingTrail` on the husk's blade |
//!
//! `MessageReader` cursors are per-reader, so consuming `SfxEvent` here does NOT
//! steal it from `audio.rs` — both lanes see every message.
//!
//! If the combat lane later wants finer control it can write `vfx::Impact` directly
//! (one line, see `vfx.rs` docs) and delete the corresponding arm below.

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::audio::SfxEvent;
use crate::combat::{Enemy, HuskArm, HuskState, PlayerCombat};
use crate::scene::Campsite;
use crate::vfx::{CampfireVfx, HitFlavor, Impact, SwingTrail, Unravel};

/// Half-extents of a Guard Husk's torso+head silhouette. Mirrors the cuboids in
/// `combat::spawn_guard_husk` (0.9 × 1.4 × 0.6 torso, head at y≈2.0) so the
/// dissolve fills the volume the body actually occupied.
const HUSK_HALF: Vec3 = Vec3::new(0.45, 1.15, 0.30);
/// Centre of that silhouette above the husk's feet.
const HUSK_CENTRE_Y: f32 = 1.15;
/// The husk's armour colour — must match the `armor` material in `spawn_guard_husk`
/// or the blocks that come apart look like a different enemy.
const HUSK_TINT: Color = Color::srgb(0.32, 0.34, 0.40);
/// Chest height on the player, so a blow that lands on them flashes at the torso
/// rather than at their feet.
const PLAYER_CHEST: f32 = -0.2;
/// Half-extents of the player's silhouette, for the red wrap flash.
const PLAYER_HALF: Vec3 = Vec3::new(0.40, 0.90, 0.40);
/// The husk blade's cross-section — matches the `arm` cuboid (0.28 × 1.0 × 0.28).
const HUSK_BLADE_HALF: Vec3 = Vec3::new(0.14, 0.50, 0.14);

/// Add with one line next to `VfxPlugin`:
/// `.add_plugins((vfx::VfxPlugin, vfx_bridge::VfxBridgePlugin))`.
pub struct VfxBridgePlugin;

impl Plugin for VfxBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                sfx_to_vfx,
                light_the_campfire,
                attach_husk_trail,
                drive_husk_trail,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// ① ③ ④ ⑥ — impacts, hit flash, hit-stop, camera kick
// ---------------------------------------------------------------------------

/// Translate the combat lane's existing audio cues into VFX messages.
///
/// `power` follows `combat-design.md` §5.1: a light hit is 1.0, heavy/charged 1.8,
/// a parry is its own flavour and does not scale.
fn sfx_to_vfx(
    mut cues: MessageReader<SfxEvent>,
    mut impacts: MessageWriter<Impact>,
    mut unravels: MessageWriter<Unravel>,
    player: Query<&Transform, With<PlayerCombat>>,
) {
    // The player transform is the only thing the bridge needs to derive a direction
    // (SfxEvent carries a position but no vector). If there is no player yet — the
    // hero-shot bin, or the first frame — impacts still fire, just without recoil.
    let player_pos = player.iter().next().map(|tf| tf.translation);

    for cue in cues.read() {
        match *cue {
            SfxEvent::HitLight { position } => {
                impacts.write(husk_hit(position, player_pos, 1.0));
            }
            SfxEvent::HitHeavy { position } => {
                impacts.write(husk_hit(position, player_pos, 1.8));
            }
            SfxEvent::HitParry { position } => {
                impacts.write(Impact {
                    pos: position,
                    dir: away_from(player_pos, position),
                    power: 1.2,
                    flavor: HitFlavor::Parry,
                    // Nothing broke on a parry — ring only, no wrap flash.
                    body_half: None,
                });
            }
            SfxEvent::HitBlock { position } => {
                // A blocked blow is a dull scuff: same sparks, much smaller, and the
                // shield ate the flash.
                impacts.write(Impact {
                    pos: position,
                    dir: away_from(player_pos, position),
                    power: 0.5,
                    flavor: HitFlavor::Parry,
                    body_half: None,
                });
            }
            SfxEvent::PlayerHurt => {
                let Some(p) = player_pos else { continue };
                impacts.write(Impact {
                    pos: p + Vec3::Y * PLAYER_CHEST,
                    // The blow travels *into* the player; with no attacker position
                    // in the cue, straight down-camera reads correctly.
                    dir: Vec3::NEG_Z,
                    power: 1.4,
                    flavor: HitFlavor::Player,
                    body_half: Some(PLAYER_HALF),
                });
            }
            SfxEvent::EnemyDeath { position } => {
                unravels.write(Unravel {
                    pos: position + Vec3::Y * HUSK_CENTRE_Y,
                    half: HUSK_HALF,
                    tint: HUSK_TINT,
                });
            }
            // Footsteps, swings and the player's own death/respawn have no VFX of
            // their own — the swing is covered by the trail, the death fade by
            // `scene.rs`.
            _ => {}
        }
    }
}

/// Build the `Impact` for a blow the player landed on a husk.
fn husk_hit(position: Vec3, player_pos: Option<Vec3>, power: f32) -> Impact {
    Impact {
        pos: position,
        dir: away_from(player_pos, position),
        power,
        flavor: HitFlavor::Husk,
        body_half: Some(HUSK_HALF),
    }
}

/// Direction from the player to the point of contact — the way debris should fly
/// and the way the camera should recoil. Falls back to "up" so a missing player
/// never produces a zero vector (which would make debris stand still).
fn away_from(from: Option<Vec3>, to: Vec3) -> Vec3 {
    from.map(|f| (to - f).normalize_or_zero())
        .filter(|v| *v != Vec3::ZERO)
        .unwrap_or(Vec3::Y)
}

// ---------------------------------------------------------------------------
// ⑤ — the campfire
// ---------------------------------------------------------------------------

/// Give the scene lane's campfire its coal bed, embers and flicker light.
///
/// It spawns a *separate* emitter entity at `camp.fire` rather than adding the
/// component to `scene.rs`'s own fire entity — that keeps the marker (`scene::Flame`)
/// private to its lane and means the two never fight over the same transform.
fn light_the_campfire(
    mut commands: Commands,
    camp: Option<Res<Campsite>>,
    mut lit: Local<bool>,
) {
    let Some(camp) = camp else { return };
    // `Campsite` is re-inserted on respawn; only ever add one emitter.
    if *lit {
        return;
    }
    commands.spawn((
        Transform::from_translation(camp.fire),
        Visibility::default(),
        // The tuned defaults from vfx.rs (0.55 m pit, 240 klm flicker) — the bridge
        // has no reason to second-guess the look pass.
        CampfireVfx::default(),
    ));
    *lit = true;
    println!(
        "VFX campfire lit at ({:.1},{:.1},{:.1})",
        camp.fire.x, camp.fire.y, camp.fire.z
    );
}

// ---------------------------------------------------------------------------
// ② — the weapon trail
// ---------------------------------------------------------------------------

/// Every husk arm that spawns gets a (cold) trail emitter.
fn attach_husk_trail(
    mut commands: Commands,
    arms: Query<Entity, (With<HuskArm>, Without<SwingTrail>)>,
) {
    for arm in &arms {
        commands.entity(arm).insert(SwingTrail {
            hot: false,
            half: HUSK_BLADE_HALF,
            accum: 0.0,
        });
    }
}

/// The one line the combat lane would otherwise own: the ribbon is hot exactly
/// while the blade is in an *active* swing — not during the telegraph (which is
/// honest wind-up, no hitbox) and not during recovery.
fn drive_husk_trail(
    mut arms: Query<(&ChildOf, &mut SwingTrail)>,
    enemies: Query<&Enemy>,
) {
    for (parent, mut trail) in &mut arms {
        let hot = enemies
            .get(parent.parent())
            .map(|e| matches!(e.state, HuskState::Swing1 | HuskState::Swing2))
            .unwrap_or(false);
        trail.hot = hot;
    }
}
