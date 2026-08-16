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
//! | `anim::RigWeapon` component   | `anim::build_rig` (this lane) | a `vfx::SwingTrail` riding the real blade |
//! | `anim::AnimSwing` message     | `anim.rs` (this lane) | drives that trail's `hot` flag on Contact |
//!
//! `MessageReader` cursors are per-reader, so consuming `SfxEvent` here does NOT
//! steal it from `audio.rs` — both lanes see every message.
//!
//! If the combat lane later wants finer control it can write `vfx::Impact` directly
//! (one line, see `vfx.rs` docs) and delete the corresponding arm below.

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::anim::{
    Actor, AnimDodge, AnimFootstep, AnimLand, AnimSwing, DodgePhase, RigWeapon, SwingPhase,
};
use crate::audio::SfxEvent;
use crate::combat::{Enemy, ImpactEvent, ImpactWeight, PlayerCombat};
use crate::scene::Campsite;
use crate::vfx::{CampfireVfx, FootDust, HitFlavor, Impact, SwingTrail, Unravel};
use crate::EYE_HEIGHT;

/// Half-extents of the Bone Sentinel's silhouette. Mirrors the authored boxes in
/// `enemies.rs::SENTINEL` (via `combat::spawn_guard_husk`): legs from y=0, helm
/// top at 21/8 ≈ 2.6 blocks, horn tip 22.2/8 ≈ 2.8, cuirass ±5/8 = 0.625 wide,
/// pauldrons reaching ±6.8/8 ≈ 0.85 — so the dissolve fills the volume the body
/// actually occupied.
const HUSK_HALF: Vec3 = Vec3::new(0.80, 1.40, 0.47);
/// Centre of that silhouette above the husk's feet.
const HUSK_CENTRE_Y: f32 = 1.40;
/// The Sentinel's armour colour — must match `Tone::Iron` (#23262B) in
/// `enemies.rs` or the blocks that come apart look like a different enemy.
const HUSK_TINT: Color = Color::srgb(0.137, 0.149, 0.169);
/// Chest height on the player, so a blow that lands on them flashes at the torso
/// rather than at their feet.
const PLAYER_CHEST: f32 = -0.2;
/// Half-extents of the player's silhouette, for the red wrap flash.
const PLAYER_HALF: Vec3 = Vec3::new(0.40, 0.90, 0.40);
/// Height above the enemy's `feet` root that actually reads as the point of
/// contact — the Sentinel's cuirass spans y ≈ 1.08–2.03 and the eye-slits sit at
/// ≈ 2.3, so a blade meeting its chest connects around 1.4.
/// `SfxEvent::HitLight/HitHeavy/HitParry/HitBlock` all carry the husk's *root*
/// transform (feet), so without this offset every spark/debris/flash burst was
/// drawing centred on the ground, half of it clipped underground — see
/// `contact_point` below.
const HUSK_CONTACT_Y: f32 = 1.40;
/// The husk blade's cross-section — matches the `weapon` cuboid `anim.rs` builds
/// for `Actor::Husk` (0.10 × 1.05 × 0.26).
const HUSK_BLADE_HALF: Vec3 = Vec3::new(0.05, 0.53, 0.13);
/// The player blade's cross-section — matches `anim.rs`'s `Actor::Player` weapon
/// cuboid (0.07 × 0.86 × 0.14).
const PLAYER_BLADE_HALF: Vec3 = Vec3::new(0.035, 0.43, 0.07);

/// Add with one line next to `VfxPlugin`:
/// `.add_plugins((vfx::VfxPlugin, vfx_bridge::VfxBridgePlugin))`.
pub struct VfxBridgePlugin;

impl Plugin for VfxBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                sfx_to_vfx,
                critical_impact_flash,
                light_the_campfire,
                attach_rig_weapon_trail,
                drive_rig_weapon_trail,
                footstep_dust,
                dodge_dust,
                landing_dust,
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
                    pos: contact_point(position, player_pos),
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
                    pos: contact_point(position, player_pos),
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
        pos: position + Vec3::Y * HUSK_CONTACT_Y,
        dir: away_from(player_pos, position),
        power,
        flavor: HitFlavor::Husk,
        body_half: Some(HUSK_HALF),
    }
}

/// Where a blow actually lands, given only the husk's root position (its feet —
/// `SfxEvent` carries no attacker/target distinction and no per-hit height).
/// `HitParry`/`HitBlock` fire at the player's guard, not the husk's body, so this
/// leans most of the way toward the player rather than sitting on the husk's
/// silhouette the way `husk_hit`'s `HUSK_CONTACT_Y` does.
fn contact_point(husk_root: Vec3, player_pos: Option<Vec3>) -> Vec3 {
    let husk_chest = husk_root + Vec3::Y * HUSK_CONTACT_Y;
    match player_pos {
        Some(p) => husk_chest.lerp(p + Vec3::Y * PLAYER_CHEST, 0.7),
        None => husk_chest,
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
// riposte / poise-break / charged — the "loudest hit there is"
// ---------------------------------------------------------------------------

/// A riposte, a poise break or a charged swing is `combat.rs`'s own
/// `ImpactWeight::Critical` — its own doc calls it "the loudest thing that can
/// happen in a soulslike trade". `ImpactEvent` already carries that
/// classification and is explicitly documented as a `vfx.rs` hook, but nothing
/// in the crate was reading it, so a riposte looked exactly like a plain heavy
/// hit. This layers the parry's own instant ring (see `vfx.rs::on_impact`) on
/// top of whatever the normal `sfx_to_vfx` burst already drew, so a critical
/// reads as unmistakably bigger than a plain swing in the same frame it lands.
fn critical_impact_flash(
    mut hits: MessageReader<ImpactEvent>,
    mut impacts: MessageWriter<Impact>,
    player: Query<Entity, With<PlayerCombat>>,
) {
    let player_e = player.iter().next();
    for hit in hits.read() {
        if hit.weight != ImpactWeight::Critical {
            continue;
        }
        // `ImpactEvent::pos` is the TARGET's own root, and the two actors use
        // different root conventions (husk root = feet, player root = eye —
        // see `husk_hit`/`SfxEvent::PlayerHurt` above), so which offset applies
        // depends on who got hit, not who swung.
        let pos = if Some(hit.target) == player_e {
            hit.pos + Vec3::Y * PLAYER_CHEST
        } else {
            hit.pos + Vec3::Y * HUSK_CONTACT_Y
        };
        impacts.write(Impact {
            pos,
            dir: hit.dir,
            power: 2.6,
            flavor: HitFlavor::Parry,
            body_half: None,
        });
    }
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

/// Every rig's weapon (player or husk — `anim::RigWeapon` is on both) gets a
/// (cold) trail emitter, sized to that actor's actual blade cross-section.
///
/// This rides the entity `anim.rs` really draws, not a proxy: the husk used to
/// get its trail from `combat::HuskArm`, a placeholder box `attach_rigs` hides
/// the instant a rig lands on that actor — the trail was tracing an invisible
/// entity's own, slightly different swing arc rather than the blade on screen.
fn attach_rig_weapon_trail(
    mut commands: Commands,
    weapons: Query<(Entity, &RigWeapon), Without<SwingTrail>>,
) {
    for (weapon, rig_weapon) in &weapons {
        let half = match rig_weapon.actor {
            Actor::Player => PLAYER_BLADE_HALF,
            Actor::Husk => HUSK_BLADE_HALF,
        };
        commands.entity(weapon).insert(SwingTrail { hot: false, half, accum: 0.0 });
        // Proof marker for docs/anim-events.md's checklist — fires once per weapon
        // entity (the `Without<SwingTrail>` filter drops it out of this query the
        // very next frame), never per-frame.
        println!("VFX_RIG_WEAPON_TRAIL attach actor={:?} entity={weapon:?}", rig_weapon.actor);
    }
}

/// Light the trail up on the frame the blade actually goes fast (`SwingPhase::
/// Contact` — the same window `combat.rs`'s hitbox is open on, per
/// `docs/anim-events.md`) and douse it once the swing starts recovering. Windup
/// is the telegraph — slow and deliberate on purpose — so it stays cold.
///
/// `AnimSwing` carries an `Actor`, not a specific entity (one rig per actor kind
/// is the only case this game spawns today — see `docs/LANES.md`'s Guard Husk
/// notes), so this drives every weapon of that actor kind together.
fn drive_rig_weapon_trail(
    mut swings: MessageReader<AnimSwing>,
    mut weapons: Query<(&RigWeapon, &mut SwingTrail)>,
) {
    for s in swings.read() {
        let hot = matches!(s.phase, SwingPhase::Contact);
        for (rig_weapon, mut trail) in &mut weapons {
            if rig_weapon.actor == s.actor {
                trail.hot = hot;
                // Proof marker for docs/anim-events.md's checklist — `AnimSwing` is
                // already edge-detected (fires once per phase entry in anim.rs), so
                // this is once per swing phase, not per-frame.
                println!("VFX_SWING_TRAIL hot={hot} actor={:?} phase={:?}", s.actor, s.phase);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ⑦ — footstep + dodge ground dust
// ---------------------------------------------------------------------------
//
// `anim.rs` has fired `AnimFootstep` (twice a stride) and `AnimDodge` (i-frame
// edges) since the animation-event stream was built, but nothing in the crate
// ever read either — a walk cycle and a dodge roll kicked up nothing. Neither
// message carries a position (only which `Actor`, per docs/anim-events.md),
// so — same limitation `drive_rig_weapon_trail` already lives with — this
// reads whichever live actor(s) of that kind exist right now.

/// Player root is eye-height; the Husk's is already its feet (see `anim.rs`'s
/// `build_rig` — same two offsets it uses for the rig root).
fn feet_of(tf: &Transform, actor: Actor) -> Vec3 {
    match actor {
        Actor::Player => tf.translation - Vec3::Y * EYE_HEIGHT,
        Actor::Husk => tf.translation,
    }
}

fn footstep_dust(
    mut steps: MessageReader<AnimFootstep>,
    mut dust: MessageWriter<FootDust>,
    player: Query<&Transform, With<PlayerCombat>>,
    enemies: Query<&Transform, With<Enemy>>,
) {
    for s in steps.read() {
        match s.actor {
            Actor::Player => {
                for tf in &player {
                    dust.write(FootDust { pos: feet_of(tf, Actor::Player), power: 1.0 });
                }
            }
            Actor::Husk => {
                for tf in &enemies {
                    // Bigger and heavier — a Husk's footfall should read as more
                    // ground contact than the player's.
                    dust.write(FootDust { pos: feet_of(tf, Actor::Husk), power: 1.5 });
                }
            }
        }
    }
}

/// A roll kicks up more dust than a footstep — reuses [`FootDust`] at a higher
/// `power` rather than a second effect. Fires on `IframeStart` only (the roll's
/// launch), not `IframeEnd`, so it reads as "pushed off from here", not two
/// puffs bookending the same roll.
fn dodge_dust(
    mut dodges: MessageReader<AnimDodge>,
    mut dust: MessageWriter<FootDust>,
    player: Query<&Transform, With<PlayerCombat>>,
    enemies: Query<&Transform, With<Enemy>>,
) {
    for d in dodges.read() {
        if d.phase != DodgePhase::IframeStart {
            continue;
        }
        match d.actor {
            Actor::Player => {
                for tf in &player {
                    dust.write(FootDust { pos: feet_of(tf, Actor::Player), power: 2.2 });
                }
            }
            Actor::Husk => {
                for tf in &enemies {
                    dust.write(FootDust { pos: feet_of(tf, Actor::Husk), power: 2.2 });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ⑧ — landing dust (a fall hitting the ground, not a mid-stride footfall)
// ---------------------------------------------------------------------------
//
// `anim.rs` has driven a landing-squash POSE off a hard fall (`rig.land`/
// `rig.land_amp`) since that system was built, but until `AnimLand` (added
// alongside this system) nothing told the VFX lane the moment it happened —
// a body dropping onto the ground kicked up nothing. `amp` mirrors the same
// `|fall speed| / LAND_FULL` the squash pose uses, so the dust and the pose
// read the same impact strength instead of two different numbers for one
// beat. Scaled well past a footstep (`power` 1.0) and a dodge kick-off (2.2)
// so even a light stumble reads as ground contact, not another footfall.
fn landing_dust(
    mut lands: MessageReader<AnimLand>,
    mut dust: MessageWriter<FootDust>,
    player: Query<&Transform, With<PlayerCombat>>,
    enemies: Query<&Transform, With<Enemy>>,
) {
    for l in lands.read() {
        let power = 1.4 + l.amp.clamp(0.0, 1.0) * 2.0;
        match l.actor {
            Actor::Player => {
                for tf in &player {
                    dust.write(FootDust { pos: feet_of(tf, Actor::Player), power });
                }
            }
            Actor::Husk => {
                for tf in &enemies {
                    dust.write(FootDust { pos: feet_of(tf, Actor::Husk), power });
                }
            }
        }
    }
}
