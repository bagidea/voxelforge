//! Voxelforge audio — SFX + ambient system (Kevin).
//!
//! ## Architecture
//! * **Buses** — three virtual buses (SFX / ambient / UI) implemented as volume
//!   multipliers in [`AudioSettings`]; master volume drives Bevy's `GlobalVolume`.
//! * **Messages** — [`SfxEvent`] is a Bevy [`Message`]; systems in `combat.rs`
//!   write through [`MessageWriter`]`<SfxEvent>` and the audio layer reads them
//!   via [`MessageReader`]`<SfxEvent>` to spawn the matching sound.
//! * **Footsteps** — [`footstep_tracker`] watches the player transform and fires
//!   a `Footstep` event every ~0.45 s of horizontal travel, mapping the block
//!   underfoot to a surface type via [`surface_at`].
//! * **Ambient** — [`ambient_spawner`] places 3D looping sounds (wind, campfire,
//!   village murmur) near the player; they are despawned on scene reset via the
//!   shared `AppState` transitions.
//! * **Positional** — all SFX and ambient use Bevy's spatial audio when given a
//!   position; a [`SpatialListener`] is attached to the camera for pan + roll-off.
//!
//! ## File ↔ event map
//! | Event variant | Asset path |
//! |---|---|
//! | `Footstep { surface: Grass }` | `audio/footstep_grass.wav` |
//! | `Footstep { surface: Stone }` | `audio/footstep_stone.wav` |
//! | `Footstep { surface: Wood }`  | `audio/footstep_wood.wav`  |
//! | `Footstep { surface: Sand }`  | `audio/footstep_sand.wav`  |
//! | `SwingLight`   | `audio/swing_light.wav`   |
//! | `SwingHeavy`   | `audio/swing_heavy.wav`   |
//! | `HitLight`     | `audio/hit_light.wav`     |
//! | `HitHeavy`     | `audio/hit_heavy.wav`     |
//! | `HitBlock`     | `audio/hit_block.wav`     |
//! | `HitParry`     | `audio/hit_parry.wav`     |
//! | `EnemyDeath`   | `audio/enemy_death.wav`   |
//! | `PlayerHurt`   | `audio/player_hurt.wav`   |
//! | `PlayerDeath`  | `audio/player_death.wav`  |
//! | `PlayerRespawn`| `audio/player_respawn.wav`|
//! | (ambient)      | `audio/ambient_wind.wav` etc. |

use bevy::audio::{AudioPlayer, AudioSource, GlobalVolume, PlaybackSettings, SpatialListener, Volume};
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::editor::AppState;
use crate::scene::Campsite;
use crate::FlyCam;
use voxelforge_sim::block::BlockId;
use voxelforge_sim::worldgen::{terrain_block, terrain_height};

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Per-bus volume settings. Every sound's playback volume is computed as
/// `AudioSettings.master * AudioSettings.<bus>` so the player can lower SFX
/// without touching ambient, and vice-versa.
#[derive(Resource, Clone)]
pub struct AudioSettings {
    pub master: f32,
    pub sfx: f32,
    pub ambient: f32,
    pub ui: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { master: 0.8, sfx: 1.0, ambient: 0.6, ui: 1.0 }
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// Surface material under the player's feet — maps to the matching footstep WAV.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FootstepSurface {
    Grass,
    Stone,
    Wood,
    Sand,
}

/// Every sound the game can trigger.  Systems in `combat.rs` write these as
/// Bevy messages; the audio layer reads them via [`MessageReader`]`<SfxEvent>`
/// and spawns the matching audio entity.
#[derive(Clone, Debug)]
pub enum SfxEvent {
    /// A footstep on the given surface at the given world position.
    Footstep {
        surface: FootstepSurface,
        position: Vec3,
    },
    /// Light-attack weapon whoosh.
    SwingLight {
        position: Vec3,
    },
    /// Heavy-attack weapon whoosh.
    SwingHeavy {
        position: Vec3,
    },
    /// Light hit landed on an enemy.
    HitLight {
        position: Vec3,
    },
    /// Heavy/charged hit landed on an enemy.
    HitHeavy {
        position: Vec3,
    },
    /// Hit absorbed by a block.
    HitBlock {
        position: Vec3,
    },
    /// Successful parry.
    HitParry {
        position: Vec3,
    },
    /// Enemy death crumble.
    EnemyDeath {
        position: Vec3,
    },
    /// Player took damage.
    PlayerHurt,
    /// Player died.
    PlayerDeath,
    /// Player respawned at campfire.
    PlayerRespawn,
}

impl Message for SfxEvent {}

// ---------------------------------------------------------------------------
// Helper — block ID → footstep surface
// ---------------------------------------------------------------------------

/// Map the voxel block underneath the player to a footstep material.
pub fn surface_at(wx: f32, _wy: f32, wz: f32) -> FootstepSurface {
    let h = terrain_height(wx, wz) as i32;
    let block = terrain_block(wx, wz, h, h); // query the surface block
    match block {
        BlockId::GRASS | BlockId::DIRT | BlockId::MOSS => FootstepSurface::Grass,
        BlockId::STONE | BlockId::COBBLESTONE | BlockId::OBSIDIAN | BlockId::BRICK
        | BlockId::LIMESTONE | BlockId::CLAY | BlockId::GRAVEL => FootstepSurface::Stone,
        BlockId::WOOD => FootstepSurface::Wood,
        BlockId::SAND | BlockId::RED_SAND | BlockId::SNOW => FootstepSurface::Sand,
        _ => FootstepSurface::Grass, // AIR / unknown → default grass
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Tracks player movement to fire footsteps on distance travelled.
#[derive(Resource, Default)]
struct StepTracker {
    last_pos: Option<Vec3>,
    accum: f32,
}

/// Ambient sound entities so we can despawn them when leaving Play.
#[derive(Resource, Default)]
struct AmbientEnts {
    wind: Option<Entity>,
    campfire: Option<Entity>,
    village: Option<Entity>,
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(AudioSettings::default())
            .insert_resource(StepTracker::default())
            .insert_resource(AmbientEnts::default())
            .add_message::<SfxEvent>()
            .add_systems(Update, (
                attach_listener,
                play_sfx,
                footstep_tracker,
                update_volumes,
            ).run_if(in_state(AppState::Play)))
            .add_systems(OnEnter(AppState::Play), spawn_ambient)
            .add_systems(OnExit(AppState::Play), despawn_ambient);
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Attach a [`SpatialListener`] to the player camera so 3D sounds pan / roll off
/// correctly.  Runs every frame (cheap — just a marker component insert).
fn attach_listener(
    cam_q: Query<Entity, (With<FlyCam>, Without<SpatialListener>)>,
    mut commands: Commands,
) {
    for e in cam_q.iter() {
        commands.entity(e).insert(SpatialListener::default());
    }
}

/// Read [`SfxEvent`] messages and spawn matching audio entities.
#[allow(clippy::too_many_arguments)]
fn play_sfx(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
    mut events: MessageReader<SfxEvent>,
) {
    let vol = (settings.master * settings.sfx) as f64;
    for ev in events.read() {
        let (path, pos, bus_vol) = match ev {
            SfxEvent::Footstep { surface, position } => {
                let p = match surface {
                    FootstepSurface::Grass => "audio/footstep_grass.wav",
                    FootstepSurface::Stone => "audio/footstep_stone.wav",
                    FootstepSurface::Wood  => "audio/footstep_wood.wav",
                    FootstepSurface::Sand  => "audio/footstep_sand.wav",
                };
                (p, *position, settings.sfx as f64)
            }
            SfxEvent::SwingLight { position } => ("audio/swing_light.wav", *position, settings.sfx as f64),
            SfxEvent::SwingHeavy { position } => ("audio/swing_heavy.wav", *position, settings.sfx as f64),
            SfxEvent::HitLight { position } => ("audio/hit_light.wav", *position, settings.sfx as f64),
            SfxEvent::HitHeavy { position } => ("audio/hit_heavy.wav", *position, settings.sfx as f64),
            SfxEvent::HitBlock { position } => ("audio/hit_block.wav", *position, settings.sfx as f64),
            SfxEvent::HitParry { position } => ("audio/hit_parry.wav", *position, settings.sfx as f64),
            SfxEvent::EnemyDeath { position } => ("audio/enemy_death.wav", *position, settings.sfx as f64),
            // Player-centric sounds play at the camera (non-spatial fallback = full volume).
            SfxEvent::PlayerHurt => ("audio/player_hurt.wav", Vec3::ZERO, settings.sfx as f64),
            SfxEvent::PlayerDeath => ("audio/player_death.wav", Vec3::ZERO, settings.sfx as f64),
            SfxEvent::PlayerRespawn => ("audio/player_respawn.wav", Vec3::ZERO, settings.sfx as f64),
        };
        let handle: Handle<AudioSource> = asset_server.load(path);
        let playback_volume = Volume::Linear((vol * bus_vol) as f32);
        let mut entity = commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                volume: playback_volume,
                spatial: pos != Vec3::ZERO,
                ..default()
            },
        ));
        if pos != Vec3::ZERO {
            entity.insert(Transform::from_translation(pos));
        }
    }
}

/// Fire a [`SfxEvent::Footstep`] every ~0.45 s of horizontal movement,
/// picking the surface type from the block under the player's feet.
fn footstep_tracker(
    player_q: Query<&Transform, With<FlyCam>>,
    mut tracker: ResMut<StepTracker>,
    mut events: MessageWriter<SfxEvent>,
) {
    const STEP_INTERVAL: f32 = 0.45; // seconds between footstep sounds
    let Ok(tf) = player_q.single() else { return };
    let cur = tf.translation;
    if let Some(last) = tracker.last_pos {
        let horiz = Vec3::new(cur.x - last.x, 0.0, cur.z - last.z).length();
        tracker.accum += horiz;
        while tracker.accum >= STEP_INTERVAL {
            tracker.accum -= STEP_INTERVAL;
            let surface = surface_at(cur.x, cur.y - 1.0, cur.z);
            events.write(SfxEvent::Footstep {
                surface,
                position: cur,
            });
        }
    }
    tracker.last_pos = Some(cur);
}

/// Push [`GlobalVolume`] from [`AudioSettings::master`].
fn update_volumes(settings: Res<AudioSettings>, mut global: ResMut<GlobalVolume>) {
    global.volume = Volume::Linear(settings.master);
}

// ---------------------------------------------------------------------------
// Ambient
// ---------------------------------------------------------------------------

/// Spawn loopable 3D ambient sounds.  Positions track the world, not the player
/// — wind is heard everywhere, campfire is local.
fn spawn_ambient(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
    mut ents: ResMut<AmbientEnts>,
    camp: Option<Res<Campsite>>,
    player_q: Query<&Transform, With<FlyCam>>,
) {
    let vol = (settings.master * settings.ambient) as f64;
    let player_pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // Wind — placed at the player (it's everywhere, no real position).
    let wind_h: Handle<AudioSource> = asset_server.load("audio/ambient_wind.wav");
    let wind = commands.spawn((
        AudioPlayer(wind_h),
        PlaybackSettings {
            volume: Volume::Linear((vol * 0.7) as f32),
            spatial: false, // wind is omnipresent — no positional attenuation
            ..default()
        },
    )).id();
    ents.wind = Some(wind);

    // Campfire — placed at the actual campfire position from the scene.
    let fire_pos = camp.map(|c| c.fire).unwrap_or(player_pos + Vec3::new(0.0, 0.0, -3.0));
    let fire_h: Handle<AudioSource> = asset_server.load("audio/ambient_campfire.wav");
    let fire = commands.spawn((
        AudioPlayer(fire_h),
        PlaybackSettings {
            volume: Volume::Linear((vol * 0.8) as f32),
            spatial: true,
            ..default()
        },
        Transform::from_translation(fire_pos),
    )).id();
    ents.campfire = Some(fire);

    // Village murmur — distant, placed some distance from spawn.
    let village_pos = player_pos + Vec3::new(15.0, 0.0, -10.0);
    let village_h: Handle<AudioSource> = asset_server.load("audio/ambient_village.wav");
    let village = commands.spawn((
        AudioPlayer(village_h),
        PlaybackSettings {
            volume: Volume::Linear((vol * 0.5) as f32),
            spatial: true,
            ..default()
        },
        Transform::from_translation(village_pos),
    )).id();
    ents.village = Some(village);
}

/// Despawn all ambient sound entities when leaving Play.
fn despawn_ambient(mut commands: Commands, mut ents: ResMut<AmbientEnts>) {
    for opt in [ents.wind.take(), ents.campfire.take(), ents.village.take()] {
        if let Some(e) = opt {
            commands.entity(e).despawn();
        }
    }
}
