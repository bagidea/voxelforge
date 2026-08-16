//! Voxelforge audio — SFX + ambient + music system (Kevin, audio lane on loan
//! to yamamoto 2026-08-14 — see `docs/LANES.md`).
//!
//! ## Architecture
//! * **Buses** — four virtual buses (SFX / ambient / music / UI) implemented as
//!   volume multipliers in [`AudioSettings`]; master volume drives Bevy's
//!   `GlobalVolume`. `ui` is defined but not yet wired to a UI sound source —
//!   pre-existing, out of scope here.
//! * **Messages** — [`SfxEvent`] is a Bevy [`Message`]; systems in `combat.rs`
//!   write through [`MessageWriter`]`<SfxEvent>` and the audio layer reads them
//!   via [`MessageReader`]`<SfxEvent>` to spawn the matching sound.
//! * **Footsteps** — [`footstep_tracker`] watches the player transform and fires
//!   a `Footstep` event every ~0.45 s of horizontal travel, mapping the block
//!   underfoot to a surface type via [`surface_at`].
//! * **Ambient** — [`spawn_ambient`] places 3D looping sounds (wind, campfire,
//!   village murmur) near the player; they are despawned on scene reset via the
//!   shared `AppState` transitions. All three now loop (`PlaybackMode::Loop`) —
//!   the file/doc comments always claimed "looping" but the spawn code left
//!   `mode` at its `default()` of `PlaybackMode::Once`, so every ambient loop
//!   silently died after its ~2.5-3s clip finished once. Fixed here since the
//!   zone crossfade below is meaningless against a source that already stopped.
//! * **Zone crossfade** — [`track_ambient_zone`] reads the player position
//!   against [`crate::quest::StoryDataRes`]'s `regions` (Rose's map-authored
//!   zones, e.g. `campfire_square`, `village_road`, `hollow_reach_intro`) via
//!   the same [`crate::quest_rules::region_contains`] box test `quest.rs` uses
//!   for its own area triggers, and buckets the result into an [`AmbientZone`].
//!   [`crossfade_ambient`] lerps each of the 3 ambient tracks' mix weight
//!   toward that zone's target over ~1.7s and pushes the result to the live
//!   `AudioSink`/`SpatialAudioSink` — **not** `PlaybackSettings`, whose own doc
//!   says "changes ... will not be applied to already-playing audio" (this is
//!   also why `settings_menu.rs`'s `update_ambient_volumes` — monanisa's lane,
//!   not touched here — silently no-ops against a live loop; flagged for her,
//!   not fixed here). No story data (e.g. a non-story test scene) → [`AmbientZone::Wilds`].
//! * **Music** — [`spawn_music`] plays a single looping background track on its
//!   own bus, independent of SFX/ambient/UI, live-pushed to the sink each frame
//!   by [`update_music_volume`] so the mixer slider responds instantly instead
//!   of waiting for the track to restart. `AUDIO_MUSIC_THEME_PATH` points at a
//!   synthesized placeholder pad (see the constant's doc) — a real composed
//!   theme should replace it.
//! * **Zone crossfade (live)** — [`track_ambient_zone`] buckets the player's
//!   position into an [`AmbientZone`]; [`crossfade_ambient`] linearly moves
//!   [`ZoneAmbientMix`] toward that zone's [`zone_gains`] over ~1.7s and pushes
//!   the result straight to each ambient track's live `AudioSink` /
//!   `SpatialAudioSink` (not `PlaybackSettings` — see the note above on why
//!   that doesn't affect already-playing audio). This is the actual fix for
//!   the mixer sliders too: `settings_menu.rs`'s `update_ambient_volumes`
//!   writes `PlaybackSettings` and silently no-ops against a live loop, but
//!   this system re-applies live volume from [`AudioSettings`] every frame
//!   regardless, so dragging Master/Ambient/Music changes what you hear
//!   immediately without needing that file touched.
//! * **Weight (layering + jitter)** — [`play_sfx`] no longer plays one thin
//!   one-shot wav per hit. Swings layer a `swing_whoosh.wav` air-cut on top of
//!   the base swing; heavy hits/blocks layer a low `impact_thump.wav` for
//!   punch and an `impact_tail.wav` hang; every layer's volume and pitch
//!   (`PlaybackSettings::speed`) is jittered a few % per play via
//!   [`jitter_volume`]/[`jitter_speed`] so repeated hits don't sound like a
//!   stuck sample. See [`extra_layers`] for the per-event layer table.
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
//! | `EnemyGrowl`   | `audio/enemy_growl.wav`   |
//! | `PlayerHurt`   | `audio/player_hurt.wav`   |
//! | `PlayerDeath`  | `audio/player_death.wav`  |
//! | `PlayerRespawn`| `audio/player_respawn.wav`|
//! | (ambient)      | `audio/ambient_wind.wav`, `audio/ambient_campfire.wav`, `audio/ambient_village.wav` |
//! | (music)        | `audio/music_theme.wav` — synthesized placeholder pad, see [`AUDIO_MUSIC_THEME_PATH`] |
//! | (layers, see [`extra_layers`]) | `audio/swing_whoosh.wav`, `audio/impact_thump.wav`, `audio/impact_tail.wav`, `audio/hit_splatter.wav` |

use std::collections::HashMap;

use bevy::audio::{
    AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, GlobalVolume, PlaybackMode,
    PlaybackSettings, SpatialAudioSink, SpatialListener, Volume,
};
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::editor::AppState;
use crate::quest::StoryDataRes;
use crate::quest_rules::region_contains;
use crate::scene::Campsite;
use crate::FlyCam;
use voxelforge_sim::block::BlockId;
use voxelforge_sim::worldgen::{terrain_block, terrain_height};

/// Looping background-music track. Currently a synthesized ambient drone pad
/// (4-layer sine chord + slow tremolo, generated with ffmpeg — see
/// `assets/audio/CREDITS.md`), phase-exact so its 24s loop point is seamless.
/// It is a functional placeholder, not a composed theme — swap the file for a
/// real track without touching code, the path is the only contract.
const AUDIO_MUSIC_THEME_PATH: &str = "audio/music_theme.wav";

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Per-bus volume settings. Every sound's playback volume is computed as
/// `AudioSettings.master * AudioSettings.<bus>` so the player can lower SFX
/// without touching ambient, and vice-versa.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master: f32,
    pub sfx: f32,
    pub ambient: f32,
    pub ui: f32,
    /// Background-music bus. Independent of `sfx` — was previously conflated
    /// with `ambient` (see `settings_menu.rs`'s "Music / Ambient" slider,
    /// monanisa's lane; not renamed here since that's her file to update).
    #[serde(default = "default_music_volume")]
    pub music: f32,
}

fn default_music_volume() -> f32 {
    0.55
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { master: 0.8, sfx: 1.0, ambient: 0.6, ui: 1.0, music: default_music_volume() }
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
    /// Enemy threat/alert bark — not yet fired by any real system (enemy_ai.rs
    /// is rose's lane, see `docs/LANES.md`); routing + asset proven here via
    /// the scripted `audio_proof_main.rs` driver, same pattern as the combat
    /// events below. Real trigger point: rose's `Alert`/`Pursuit` state
    /// transition in `enemy_ai.rs`.
    EnemyGrowl {
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

/// A beauty-tour phase change request, written by `main.rs`'s tour timeline and
/// read here to override the position-derived ambient zone. A message (not a
/// resource) so the tour stays on the Bevy 0.19 message API like every other
/// cross-lane signal in this file — see [`beauty_tour_ambience`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeautyTourPhase {
    Noon,
    Cool,
    Night,
}

impl Message for BeautyTourPhase {}

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
pub struct AmbientEnts {
    pub wind: Option<Entity>,
    pub campfire: Option<Entity>,
    pub village: Option<Entity>,
}

/// The looping background-music entity, if one is currently spawned.
#[derive(Resource, Default)]
pub struct MusicEnt(pub Option<Entity>);

/// Coarse ambience buckets a map region resolves to. Drives which of the 3
/// ambient loops [`crossfade_ambient`] fades toward. `Wilds` is also the
/// fallback when there's no [`StoryDataRes`] at all (e.g. `voxelforge_perf`'s
/// bare scene) or the player stands outside every authored region.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AmbientZone {
    #[default]
    Wilds,
    Village,
    Campfire,
}

/// Target mix weight (wind, campfire, village) — each in `0.0..=1.0` — for a
/// given [`AmbientZone`]. Not a hard switch: [`crossfade_ambient`] lerps the
/// live mix toward these, so moving between zones fades rather than cuts.
fn zone_gains(zone: AmbientZone) -> (f32, f32, f32) {
    match zone {
        AmbientZone::Wilds => (1.0, 0.0, 0.0),
        AmbientZone::Village => (0.5, 0.0, 1.0),
        AmbientZone::Campfire => (0.3, 1.0, 0.2),
    }
}

/// Map a `docs`/`act1.json` region id (see [`StoryDataRes`]) to its
/// [`AmbientZone`] bucket. Region ids not listed here (future map content)
/// default to `Village` — most named, walkable regions read as "inhabited";
/// `Wilds` is reserved for the frontier/threshold regions and for standing
/// outside every region entirely.
fn zone_for_region(region_id: &str) -> AmbientZone {
    match region_id {
        "spawn_shelter" | "campfire_square" => AmbientZone::Campfire,
        "gate_square" | "guard_post_east" | "hollow_reach_intro" => AmbientZone::Wilds,
        _ => AmbientZone::Village,
    }
}

/// Which [`AmbientZone`] the player is currently in, tracked so
/// [`track_ambient_zone`] only prints `AUDIO_ZONE:` on actual change.
#[derive(Resource, Default)]
pub struct CurrentAmbientZone(pub AmbientZone);

/// Overrides the position-derived [`AmbientZone`] while a beauty tour is playing
/// (the tour camera flies where the player never stands, so following the player
/// body would keep the ambience pinned to the campsite). `None` ⇒ normal play,
/// [`track_ambient_zone`] buckets by player position exactly as before.
#[derive(Resource, Default)]
struct AmbientZoneOverride(pub Option<AmbientZone>);

/// Live crossfade state — current mix weight per ambient track, each
/// `0.0..=1.0`. Starts at all-zero so ambience fades *in* on entering Play
/// rather than popping at whatever the spawn zone's target is.
#[derive(Resource, Default)]
pub struct ZoneAmbientMix {
    pub wind: f32,
    pub campfire: f32,
    pub village: f32,
}

/// Diagnostic counter: how many times each sound file was spawned by the audio
/// system.  Written by [`play_sfx`] — the real sound system — so a grep for
/// `AUDIO_PLAY:` on stdout proves the game found its own sounds, not a harness.
#[derive(Resource, Default)]
pub struct SfxCounter {
    /// `"audio/player_hurt.wav"` → count
    pub plays: HashMap<String, u64>,
    /// Running total across all sounds.
    pub total: u64,
    /// Seconds since last summary dump (drives [`dump_sfx_summary`]).
    pub since_dump: f32,
}

impl SfxCounter {
    fn bump(&mut self, path: &str) {
        *self.plays.entry(path.to_string()).or_insert(0) += 1;
        self.total += 1;
    }
}

/// TEMPORARY proof resource — fires every [`SfxEvent`] variant after entering Play
/// so the real `play_sfx` system prints `AUDIO_PLAY:` for every audio path.
/// Only active when `VOXELFORGE_AUDIO_PROOF=1` — see [`AudioProofMode`].
/// REMOVE after the audio proof log shows every path count ≥1.
#[derive(Resource)]
struct SfxProof {
    frames: u32,
}

/// TEMPORARY — gates [`sfx_proof_driver`] and shortens the `dump_sfx_summary`
/// interval. Off by default so the debug SFX spam never ships in a normal
/// build; set `VOXELFORGE_AUDIO_PROOF=1` to enable for verification runs.
/// REMOVE alongside `SfxProof`/`sfx_proof_driver`.
#[derive(Resource)]
struct AudioProofMode(bool);

fn audio_proof_enabled(proof: Res<AudioProofMode>) -> bool {
    proof.0
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let proof_mode = std::env::var("VOXELFORGE_AUDIO_PROOF")
            .map(|v| v == "1")
            .unwrap_or(false);
        app
            .insert_resource(AudioSettings::default())
            .insert_resource(StepTracker::default())
            .insert_resource(AmbientEnts::default())
            .insert_resource(MusicEnt::default())
            .insert_resource(CurrentAmbientZone::default())
            .insert_resource(AmbientZoneOverride::default())
            .insert_resource(ZoneAmbientMix::default())
            .insert_resource(SfxCounter::default())
            .insert_resource(SfxProof { frames: 0 })
            .insert_resource(AudioProofMode(proof_mode))
            .add_message::<SfxEvent>()
            .add_message::<BeautyTourPhase>()
            .add_systems(Update, (
                attach_listener,
                sfx_proof_driver.run_if(audio_proof_enabled),
                play_sfx,
                footstep_tracker,
                update_volumes,
                beauty_tour_ambience,
                track_ambient_zone,
                crossfade_ambient,
                update_music_volume,
                dump_sfx_summary,
            ).run_if(in_state(AppState::Play)))
            .add_systems(OnEnter(AppState::Play), (spawn_ambient, spawn_music))
            .add_systems(OnExit(AppState::Play), (despawn_ambient, despawn_music));
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

/// Extra simultaneous layers spawned alongside the primary sound for a given
/// event: `(asset path, volume multiplier of the primary hit's volume, speed
/// offset from 1.0)`. This is what makes swings/hits read as *weighty*
/// instead of one thin one-shot wav — a swing gets an air-cut whoosh under
/// it, a heavy hit/block gets a low-end thump for punch plus a short tail for
/// hang, matching the brief's "wind-cut + impact + tail" layering. `HitLight`/
/// `HitHeavy` also layer `hit_splatter.wav` (wet impact burst — the "blood
/// splatter" cue) since both represent a weapon landing on an enemy's body,
/// unlike `HitBlock`/`HitParry` which hit a shield/weapon and stay dry.
/// Distinct combos per event (on top of each already having its own base wav)
/// is also how flesh/block/parry/heavy read as different weights, not just
/// different samples.
fn extra_layers(ev: &SfxEvent) -> &'static [(&'static str, f32, f32)] {
    match ev {
        SfxEvent::SwingLight { .. } => &[("audio/swing_whoosh.wav", 0.55, 0.15)],
        SfxEvent::SwingHeavy { .. } => &[("audio/swing_whoosh.wav", 0.75, -0.10)],
        SfxEvent::HitLight { .. } => &[
            ("audio/impact_tail.wav", 0.35, 0.10),
            ("audio/hit_splatter.wav", 0.45, 0.10),
        ],
        SfxEvent::HitHeavy { .. } => &[
            ("audio/impact_thump.wav", 0.9, -0.08),
            ("audio/impact_tail.wav", 0.55, -0.05),
            ("audio/hit_splatter.wav", 0.7, -0.05),
        ],
        SfxEvent::HitBlock { .. } => &[("audio/impact_thump.wav", 0.6, 0.05)],
        SfxEvent::HitParry { .. } => &[("audio/impact_tail.wav", 0.5, 0.35)],
        SfxEvent::EnemyDeath { .. } => &[("audio/impact_thump.wav", 0.7, -0.20)],
        _ => &[],
    }
}

/// ±10% random volume jitter so repeated hits don't sound like a stuck sample.
fn jitter_volume(base: f64) -> f64 {
    let j = (fastrand::f64() - 0.5) * 0.20;
    (base * (1.0 + j)).max(0.0)
}

/// ±8% random pitch/speed jitter around `base_speed` (a layer's own fixed
/// offset from [`extra_layers`], e.g. the thump running a bit deeper than 1x).
fn jitter_speed(base_speed: f32) -> f32 {
    let j = (fastrand::f32() - 0.5) * 0.16;
    (base_speed + j).max(0.1)
}

/// Spawn one audio entity — primary hit or an [`extra_layers`] layer — with
/// jittered volume/pitch, bump the proof counter, and print `AUDIO_PLAY:`.
fn spawn_sfx_layer(
    commands: &mut Commands,
    asset_server: &AssetServer,
    counter: &mut SfxCounter,
    path: &'static str,
    pos: Vec3,
    base_volume: f64,
    base_speed: f32,
    elapsed: f32,
) {
    let handle: Handle<AudioSource> = asset_server.load(path);
    let playback_volume = Volume::Linear(jitter_volume(base_volume) as f32);
    let speed = jitter_speed(base_speed);
    let mut entity = commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings {
            volume: playback_volume,
            speed,
            spatial: pos != Vec3::ZERO,
            ..default()
        },
    ));
    if pos != Vec3::ZERO {
        entity.insert(Transform::from_translation(pos));
    }
    // Diagnostic: the real audio system prints this line — grep for
    // `AUDIO_PLAY:` to prove the game found its own sounds. `t=`/`pos=` make
    // each line a self-contained (timestamp, cue, world position) evidence
    // triple instead of just a bare path.
    counter.bump(path);
    println!("AUDIO_PLAY:{path} t={elapsed:.2} pos=({:.1},{:.1},{:.1})", pos.x, pos.y, pos.z);
}

/// Read [`SfxEvent`] messages and spawn matching audio entities (primary +
/// weight layers from [`extra_layers`]).
#[allow(clippy::too_many_arguments)]
fn play_sfx(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
    mut events: MessageReader<SfxEvent>,
    mut counter: ResMut<SfxCounter>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
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
            SfxEvent::EnemyGrowl { position } => ("audio/enemy_growl.wav", *position, settings.sfx as f64),
            // Player-centric sounds play at the camera (non-spatial fallback = full volume).
            SfxEvent::PlayerHurt => ("audio/player_hurt.wav", Vec3::ZERO, settings.sfx as f64),
            SfxEvent::PlayerDeath => ("audio/player_death.wav", Vec3::ZERO, settings.sfx as f64),
            SfxEvent::PlayerRespawn => ("audio/player_respawn.wav", Vec3::ZERO, settings.sfx as f64),
        };
        let base_volume = vol * bus_vol;
        spawn_sfx_layer(&mut commands, &asset_server, &mut counter, path, pos, base_volume, 1.0, elapsed);
        for &(layer_path, layer_mult, speed_offset) in extra_layers(ev) {
            spawn_sfx_layer(
                &mut commands,
                &asset_server,
                &mut counter,
                layer_path,
                pos,
                base_volume * layer_mult as f64,
                1.0 + speed_offset,
                elapsed,
            );
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
pub(crate) fn update_volumes(settings: Res<AudioSettings>, mut global: ResMut<GlobalVolume>) {
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
    mut counter: ResMut<SfxCounter>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    let vol = (settings.master * settings.ambient) as f64;
    let player_pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // Wind — placed at the player (it's everywhere, no real position).
    let wind_path = "audio/ambient_wind.wav";
    let wind_h: Handle<AudioSource> = asset_server.load(wind_path);
    let wind = commands.spawn((
        AudioPlayer(wind_h),
        PlaybackSettings {
            volume: Volume::Linear((vol * 0.7) as f32),
            spatial: false, // wind is omnipresent — no positional attenuation
            ..default()
        },
    )).id();
    ents.wind = Some(wind);
    counter.bump(wind_path);
    println!("AUDIO_PLAY:{wind_path} t={elapsed:.2} pos=(omnipresent)");

    // Campfire — placed at the actual campfire position from the scene.
    let fire_pos = camp.map(|c| c.fire).unwrap_or(player_pos + Vec3::new(0.0, 0.0, -3.0));
    let fire_path = "audio/ambient_campfire.wav";
    let fire_h: Handle<AudioSource> = asset_server.load(fire_path);
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
    counter.bump(fire_path);
    println!("AUDIO_PLAY:{fire_path} t={elapsed:.2} pos=({:.1},{:.1},{:.1})", fire_pos.x, fire_pos.y, fire_pos.z);

    // Village murmur — distant, placed some distance from spawn.
    let village_pos = player_pos + Vec3::new(15.0, 0.0, -10.0);
    let village_path = "audio/ambient_village.wav";
    let village_h: Handle<AudioSource> = asset_server.load(village_path);
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
    counter.bump(village_path);
    println!("AUDIO_PLAY:{village_path} t={elapsed:.2} pos=({:.1},{:.1},{:.1})", village_pos.x, village_pos.y, village_pos.z);
}

/// Despawn all ambient sound entities when leaving Play.
fn despawn_ambient(mut commands: Commands, mut ents: ResMut<AmbientEnts>) {
    for opt in [ents.wind.take(), ents.campfire.take(), ents.village.take()] {
        if let Some(e) = opt {
            commands.entity(e).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Music
// ---------------------------------------------------------------------------

/// Spawn the looping background-music track on its own bus, independent of
/// SFX/ambient/UI.
fn spawn_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
    mut music: ResMut<MusicEnt>,
    mut counter: ResMut<SfxCounter>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    let vol = (settings.master * settings.music) as f64;
    let handle: Handle<AudioSource> = asset_server.load(AUDIO_MUSIC_THEME_PATH);
    let e = commands
        .spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                volume: Volume::Linear(vol as f32),
                mode: PlaybackMode::Loop,
                spatial: false, // music has no world position
                ..default()
            },
        ))
        .id();
    music.0 = Some(e);
    counter.bump(AUDIO_MUSIC_THEME_PATH);
    println!("AUDIO_PLAY:{AUDIO_MUSIC_THEME_PATH} t={elapsed:.2} pos=(non-spatial)");
}

/// Despawn the music track when leaving Play.
fn despawn_music(mut commands: Commands, mut music: ResMut<MusicEnt>) {
    if let Some(e) = music.0.take() {
        commands.entity(e).despawn();
    }
}

/// Live-push the music bus volume to the playing sink every frame, so
/// dragging the Music slider is heard immediately instead of only on the next
/// loop restart (a fresh `AudioPlayer` spawn would be the alternative, but
/// that pops/restarts the track — a live `AudioSink::set_volume` doesn't).
fn update_music_volume(
    settings: Res<AudioSettings>,
    music: Res<MusicEnt>,
    mut sinks: Query<&mut AudioSink>,
) {
    let Some(e) = music.0 else { return };
    let Ok(mut sink) = sinks.get_mut(e) else { return };
    sink.set_volume(Volume::Linear((settings.master * settings.music) as f32));
}

// ---------------------------------------------------------------------------
// Zone ambient crossfade
// ---------------------------------------------------------------------------

/// Read a [`BeautyTourPhase`] message and pin the ambient-zone override to the
/// matching [`AmbientZone`] until the next message: noon = open wind, cool =
/// village murmur, night = campfire crackle. The override is what lets the tour
/// camera fly away from the campsite without the ambience snapping back to the
/// player's position every frame.
fn beauty_tour_ambience(
    mut phases: MessageReader<BeautyTourPhase>,
    mut ovr: ResMut<AmbientZoneOverride>,
) {
    for phase in phases.read() {
        let zone = match phase {
            BeautyTourPhase::Noon => AmbientZone::Wilds,
            BeautyTourPhase::Cool => AmbientZone::Village,
            BeautyTourPhase::Night => AmbientZone::Campfire,
        };
        ovr.0 = Some(zone);
    }
}

/// Bucket the player's position into an [`AmbientZone`] via the same region
/// boxes `quest.rs` uses for its own triggers. No story data (e.g. a bare
/// test scene) or standing outside every region → [`AmbientZone::Wilds`].
/// Prints `AUDIO_ZONE:` only on actual change so this isn't spam every frame.
fn track_ambient_zone(
    story: Option<Res<StoryDataRes>>,
    player_q: Query<&Transform, With<FlyCam>>,
    ovr: Res<AmbientZoneOverride>,
    mut current: ResMut<CurrentAmbientZone>,
    time: Res<Time>,
) {
    let pos = player_q.single().ok().map(|t| t.translation);
    // A beauty tour drives the camera far from the player body; while it runs,
    // the tour's explicit zone wins over the position bucket below.
    let zone = if let Some(z) = ovr.0 {
        z
    } else {
        let Some(pos) = pos else { return };
        story
            .and_then(|s| {
                s.data
                    .regions
                    .iter()
                    .find(|r| {
                        region_contains(
                            r.bounds.x0 as f32,
                            r.bounds.z0 as f32,
                            r.bounds.x1 as f32,
                            r.bounds.z1 as f32,
                            pos.x,
                            pos.z,
                        )
                    })
                    .map(|r| zone_for_region(&r.id))
            })
            .unwrap_or(AmbientZone::Wilds)
    };
    if zone != current.0 {
        current.0 = zone;
        let p = pos.unwrap_or(Vec3::ZERO);
        println!(
            "AUDIO_ZONE:{zone:?} t={:.2} pos=({:.1},{:.1},{:.1})",
            time.elapsed_secs(), p.x, p.y, p.z
        );
    }
}

/// Move `cur` toward `target` at a constant rate so the full 0..1 sweep takes
/// `secs` seconds, regardless of frame rate.
fn move_toward(cur: f32, target: f32, dt: f32, secs: f32) -> f32 {
    let max_delta = dt / secs;
    let diff = target - cur;
    if diff.abs() <= max_delta {
        target
    } else {
        cur + max_delta * diff.signum()
    }
}

/// Apply a live volume to whichever sink kind an ambient entity has
/// (non-spatial wind → [`AudioSink`], positional campfire/village →
/// [`SpatialAudioSink`] — Bevy inserts exactly one depending on
/// `PlaybackSettings::spatial` at spawn time).
fn apply_ambient_volume(
    ent: Option<Entity>,
    vol: f64,
    sinks: &mut Query<&mut AudioSink>,
    spatial_sinks: &mut Query<&mut SpatialAudioSink>,
) {
    let Some(e) = ent else { return };
    if let Ok(mut sink) = sinks.get_mut(e) {
        sink.set_volume(Volume::Linear(vol as f32));
    } else if let Ok(mut sink) = spatial_sinks.get_mut(e) {
        sink.set_volume(Volume::Linear(vol as f32));
    }
}

const ZONE_CROSSFADE_SECS: f32 = 1.7;

/// Crossfade the 3 ambient loops toward the current zone's [`zone_gains`] over
/// ~1.7s and push the live volume to each track's sink every frame — this is
/// also what makes the Master/Ambient mixer sliders work against
/// already-playing ambience (see the module doc's "Zone crossfade (live)"
/// note on why `PlaybackSettings` alone doesn't).
#[allow(clippy::too_many_arguments)]
fn crossfade_ambient(
    time: Res<Time>,
    current: Res<CurrentAmbientZone>,
    mut mix: ResMut<ZoneAmbientMix>,
    settings: Res<AudioSettings>,
    ents: Res<AmbientEnts>,
    mut sinks: Query<&mut AudioSink>,
    mut spatial_sinks: Query<&mut SpatialAudioSink>,
) {
    let dt = time.delta_secs();
    let (target_wind, target_campfire, target_village) = zone_gains(current.0);
    mix.wind = move_toward(mix.wind, target_wind, dt, ZONE_CROSSFADE_SECS);
    mix.campfire = move_toward(mix.campfire, target_campfire, dt, ZONE_CROSSFADE_SECS);
    mix.village = move_toward(mix.village, target_village, dt, ZONE_CROSSFADE_SECS);

    let bus = (settings.master * settings.ambient) as f64;
    // Base per-track gains match the original spawn_ambient mix (wind 0.7,
    // campfire 0.8, village 0.5) so the zone-neutral (Wilds) mix sounds
    // identical to the old always-on-three-tracks behavior.
    apply_ambient_volume(ents.wind, bus * 0.7 * mix.wind as f64, &mut sinks, &mut spatial_sinks);
    apply_ambient_volume(ents.campfire, bus * 0.8 * mix.campfire as f64, &mut sinks, &mut spatial_sinks);
    apply_ambient_volume(ents.village, bus * 0.5 * mix.village as f64, &mut sinks, &mut spatial_sinks);
}

// ---------------------------------------------------------------------------
// TEMPORARY proof driver — fire every SFX variant so real systems print PASS
// ---------------------------------------------------------------------------

/// TEMPORARY: Fire every [`SfxEvent`] variant once per frame for the first ~2
/// seconds after entering Play, so `play_sfx` prints `AUDIO_PLAY:` for every
/// audio path and `dump_sfx_summary` reports all counts ≥1.
///
/// Does NOT print PASS — only the real `play_sfx` / `dump_sfx_summary` print.
/// REMOVE after the audio proof log confirms `AUDIO_SUMMARY:total=XX` with
/// every path ≥1 and no missing assets (asset+routing proven, gameplay trigger
/// ยังไม่ proven — trigger จริงต้องรอ sun ปิดบั๊ก synthetic input ก่อน).
fn sfx_proof_driver(
    mut proof: ResMut<SfxProof>,
    mut events: MessageWriter<SfxEvent>,
    player_q: Query<&Transform, With<FlyCam>>,
) {
    proof.frames += 1;
    // Fire for ~2 seconds (≈120 frames at 60fps), then go quiet.
    // That's long enough for the 5-second `dump_sfx_summary` to catch every path.
    if proof.frames > 120 {
        return;
    }
    let pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // ---- Footsteps: all 4 surfaces ----
    for &surface in &[
        FootstepSurface::Grass,
        FootstepSurface::Stone,
        FootstepSurface::Wood,
        FootstepSurface::Sand,
    ] {
        events.write(SfxEvent::Footstep { surface, position: pos });
    }

    // ---- Combat positional: 8 events ----
    events.write(SfxEvent::SwingLight { position: pos });
    events.write(SfxEvent::SwingHeavy { position: pos });
    events.write(SfxEvent::HitLight { position: pos });
    events.write(SfxEvent::HitHeavy { position: pos });
    events.write(SfxEvent::HitBlock { position: pos });
    events.write(SfxEvent::HitParry { position: pos });
    events.write(SfxEvent::EnemyDeath { position: pos });
    events.write(SfxEvent::EnemyGrowl { position: pos });

    // ---- Player events: 3 that need no position ----
    events.write(SfxEvent::PlayerHurt);
    events.write(SfxEvent::PlayerDeath);
    events.write(SfxEvent::PlayerRespawn);
}

// ---------------------------------------------------------------------------
// Diagnostic — periodic counter dump
// ---------------------------------------------------------------------------

/// Every 5 seconds (0.5s under `VOXELFORGE_AUDIO_PROOF=1` — short-lived demos
/// like `combat_demo` exit at t=4.0s, before a 5s cadence would ever flush),
/// print a summary of all SFX counters to stdout so a verification harness
/// can grep for `AUDIO_SUMMARY:` and confirm every sound file played ≥1 time.
fn dump_sfx_summary(
    time: Res<Time>,
    mut counter: ResMut<SfxCounter>,
    proof: Res<AudioProofMode>,
) {
    let interval = if proof.0 { 0.5 } else { 5.0 };
    counter.since_dump += time.delta_secs();
    if counter.since_dump < interval || counter.total == 0 {
        return;
    }
    counter.since_dump = 0.0;
    let mut keys: Vec<&String> = counter.plays.keys().collect();
    keys.sort();
    let detail: Vec<String> = keys.iter().map(|k| format!("{k}={}", counter.plays[*k])).collect();
    println!("AUDIO_SUMMARY:total={} | {}", counter.total, detail.join(" "));
}
