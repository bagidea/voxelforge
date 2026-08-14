//! ISOLATED audio proof binary (yamamoto, audio lane) — drives the real,
//! unmodified `audio.rs` through a scripted scenario and prints its real
//! `AUDIO_PLAY:` / `AUDIO_ZONE:` / `AUDIO_SUMMARY:` diagnostics, so those logs
//! come from actual gameplay-shaped triggers (real movement → real
//! `footstep_tracker`, real zone crossing → real `track_ambient_zone` /
//! `crossfade_ambient`, real bus-slider mutation → real live `AudioSink`
//! push) instead of only the in-engine `sfx_proof_driver` (which is
//! synthetic by its own doc comment).
//!
//! WHY A SEPARATE BIN. `voxelforge`'s main bin does not compile right now —
//! two lanes that are *not* mine are mid-edit and broken: `main_menu.rs`
//! (`ctx.screen_rect()` doesn't exist on this bevy_egui version) and
//! `combat.rs` (a `match e.state` missing 4 of `HuskState`'s arms). Neither
//! is audio's to fix (docs/LANES.md). `quest.rs` and `scene.rs` — which own
//! the real `StoryDataRes`/`Campsite` types `audio.rs` reads — both `use
//! crate::combat`, so pulling either in unmodified drags the same breakage
//! into any bin that includes them. Same trick `shot_main.rs`/`perf_main.rs`
//! already use for this exact situation: `#[path]`-include the real
//! `audio.rs` verbatim, and supply the handful of crate-root items it reaches
//! for (`FlyCam`, `scene::Campsite`, `quest::StoryDataRes`,
//! `editor::AppState`, `quest_rules::region_contains`) as local shims shaped
//! exactly like the real ones. `audio.rs` itself is not edited or copied —
//! only its (currently blocked) dependencies are stood in for. Two of the
//! four — `editor::AppState` and `quest_rules::region_contains` — aren't even
//! shims: `quest_rules.rs` is `#[path]`-included verbatim (it has zero
//! `crate::` deps, confirmed clean) and `AppState`'s shim is a byte-for-byte
//! copy of the real enum in `editor.rs` (same variants/derives), so
//! `in_state(AppState::Play)` / `OnEnter`/`OnExit` type-check identically.
//!
//! What's real vs. scripted, honestly: footsteps, ambient spawn/despawn,
//! zone-crossfade, music spawn, and the live mixer push are *real* audio.rs
//! systems reacting to *real* transform movement and *real* resource
//! mutation. The combat `SfxEvent`s (swing/hit/parry/death/hurt/respawn) are
//! fired from a scripted `MessageWriter<SfxEvent>` here, the same call shape
//! `combat.rs` uses — not real combat.rs logic, because combat.rs can't
//! compile right now. That's the one thing this bin can't upgrade to "real
//! trigger" until Rose's match arm is fixed; everything else here is genuine.

use audio::{AudioSettings, SfxEvent};
use bevy::app::AppExit;
use bevy::asset::AssetPlugin;
use bevy::audio::AudioPlugin as BevyAudioPlugin;
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;

#[path = "audio.rs"]
mod audio;

#[path = "quest_rules.rs"]
mod quest_rules;

/// Shim for `editor::AppState` — same variants/derives as the real one in
/// `editor.rs`, so `audio.rs`'s unmodified `in_state`/`OnEnter`/`OnExit`
/// calls type-check against it.
pub mod editor {
    use bevy::prelude::*;

    #[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AppState {
        #[default]
        MainMenu,
        Editor,
        Play,
    }
}

/// Shim for `scene::Campsite` — `audio::spawn_ambient` only reads `.fire`.
pub mod scene {
    use bevy::prelude::*;

    #[derive(Resource)]
    pub struct Campsite {
        pub fire: Vec3,
    }
}

/// Shim for `quest::StoryDataRes` — same field shape `audio::track_ambient_zone`
/// walks (`data.regions[].{id, bounds.{x0,z0,x1,z1}}`), fed below with a small
/// fixture map instead of the real `act1.json`, so the *real*
/// `zone_for_region` / `region_contains` / crossfade code runs against
/// honest (if made-up) region data.
pub mod quest {
    #[derive(Clone, Copy)]
    pub struct BoundsDef {
        pub x0: i32,
        pub z0: i32,
        pub x1: i32,
        pub z1: i32,
    }
    pub struct RegionDef {
        pub id: String,
        pub bounds: BoundsDef,
    }
    pub struct StoryData {
        pub regions: Vec<RegionDef>,
    }
    #[derive(bevy::prelude::Resource)]
    pub struct StoryDataRes {
        pub data: StoryData,
    }
}

/// Shim for `crate::FlyCam` — `audio.rs` only ever uses it as a query marker
/// (`With<FlyCam>`), never reads a field, so an empty marker is enough.
#[derive(Component)]
pub(crate) struct FlyCam;

/// One leg of the scripted walk: move toward `to` at `speed` units/s, arriving
/// and holding there until `hold_until` (seconds since Play started).
struct WalkLeg {
    to: Vec3,
    speed: f32,
    hold_until: f32,
}

#[derive(Resource)]
struct ProofScript {
    elapsed: f32,
    legs: Vec<WalkLeg>,
    leg_idx: usize,
    fired_combat: bool,
    mutated_mixer: bool,
    done: bool,
}

fn main() -> AppExit {
    // Pin assets to the exe directory (same fix `main.rs`/`shot_main.rs` use
    // for bevy 0.19's `get_base_path()` CARGO_MANIFEST_DIR hijack).
    let exe_dir = std::env::current_exe()
        .expect("current exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    let asset_path = exe_dir.join("assets");

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin { file_path: asset_path.to_string_lossy().to_string(), ..default() },
        TransformPlugin,
        StatesPlugin,
        BevyAudioPlugin::default(),
    ))
    .init_state::<editor::AppState>()
    .insert_resource(quest::StoryDataRes {
        data: quest::StoryData {
            regions: vec![
                quest::RegionDef {
                    id: "village_road".into(),
                    bounds: quest::BoundsDef { x0: -12, z0: -12, x1: 12, z1: 12 },
                },
                quest::RegionDef {
                    id: "campfire_square".into(),
                    bounds: quest::BoundsDef { x0: 28, z0: 28, x1: 52, z1: 52 },
                },
                quest::RegionDef {
                    id: "gate_square".into(),
                    bounds: quest::BoundsDef { x0: -80, z0: -80, x1: -60, z1: -60 },
                },
            ],
        },
    })
    .insert_resource(scene::Campsite { fire: Vec3::new(40.0, 0.0, 40.0) })
    .insert_resource(ProofScript {
        elapsed: 0.0,
        legs: vec![
            // Start well outside every region → AmbientZone::Wilds by default.
            WalkLeg { to: Vec3::new(-100.0, 0.0, -100.0), speed: 60.0, hold_until: 1.0 },
            // Walk into the village region → AmbientZone::Village.
            WalkLeg { to: Vec3::ZERO, speed: 30.0, hold_until: 4.5 },
            // Walk into the campfire region → AmbientZone::Campfire.
            WalkLeg { to: Vec3::new(40.0, 0.0, 40.0), speed: 20.0, hold_until: 8.5 },
        ],
        leg_idx: 0,
        fired_combat: false,
        mutated_mixer: false,
        done: false,
    })
    .add_plugins(audio::AudioPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, run_script);

    println!("AUDIO_PROOF_START");
    app.run()
}

fn setup(mut commands: Commands, mut next: ResMut<NextState<editor::AppState>>) {
    commands.spawn((FlyCam, Transform::from_translation(Vec3::new(-100.0, 0.0, -100.0))));
    next.set(editor::AppState::Play);
    println!("AUDIO_PROOF: entered Play — spawn_ambient + spawn_music should fire");
}

#[allow(clippy::too_many_arguments)]
fn run_script(
    time: Res<Time>,
    mut script: ResMut<ProofScript>,
    mut player_q: Query<&mut Transform, With<FlyCam>>,
    mut sfx: MessageWriter<SfxEvent>,
    mut settings: ResMut<AudioSettings>,
    zone: Res<audio::CurrentAmbientZone>,
    mut exit: MessageWriter<AppExit>,
) {
    if script.done {
        return;
    }
    let dt = time.delta_secs();
    script.elapsed += dt;

    // --- scripted walk: real movement drives real footstep_tracker + real
    // zone crossing drives real track_ambient_zone/crossfade_ambient ---
    if let Ok(mut tf) = player_q.single_mut() {
        if script.leg_idx < script.legs.len() {
            let leg_speed = script.legs[script.leg_idx].speed;
            let target = script.legs[script.leg_idx].to;
            let to_target = target - tf.translation;
            let dist = to_target.length();
            let step = leg_speed * dt;
            if dist <= step || dist < 0.001 {
                tf.translation = target;
            } else {
                tf.translation += to_target / dist * step;
            }
        }
        if script.leg_idx < script.legs.len()
            && script.elapsed >= script.legs[script.leg_idx].hold_until
        {
            script.leg_idx += 1;
        }
    }

    // --- combat SFX proof (scripted trigger — see file doc header) ---
    if !script.fired_combat && script.elapsed >= 9.0 {
        script.fired_combat = true;
        println!("AUDIO_PROOF: firing scripted combat SfxEvents (combat.rs can't compile right now — see file doc)");
        let pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
        sfx.write(SfxEvent::SwingLight { position: pos });
        sfx.write(SfxEvent::SwingHeavy { position: pos });
        sfx.write(SfxEvent::HitLight { position: pos });
        sfx.write(SfxEvent::HitHeavy { position: pos });
        sfx.write(SfxEvent::HitBlock { position: pos });
        sfx.write(SfxEvent::HitParry { position: pos });
        sfx.write(SfxEvent::EnemyDeath { position: pos });
        sfx.write(SfxEvent::PlayerHurt);
        sfx.write(SfxEvent::PlayerDeath);
        sfx.write(SfxEvent::PlayerRespawn);
    }

    // --- mixer live-push proof: mutate AudioSettings mid-run like the
    // settings menu would, prove the change is audible (sinks re-pushed
    // every frame by update_music_volume / crossfade_ambient) ---
    if !script.mutated_mixer && script.elapsed >= 10.5 {
        script.mutated_mixer = true;
        println!(
            "AUDIO_PROOF: mixer before mutate — master={:.2} music={:.2} ambient={:.2} sfx={:.2}",
            settings.master, settings.music, settings.ambient, settings.sfx
        );
        settings.music = 0.15;
        settings.ambient = 0.95;
        settings.master = 0.5;
        println!(
            "AUDIO_PROOF: mixer after mutate  — master={:.2} music={:.2} ambient={:.2} sfx={:.2} (live sinks should reflect this next frame)",
            settings.master, settings.music, settings.ambient, settings.sfx
        );
    }

    if script.elapsed >= 12.5 && !script.done {
        script.done = true;
        println!("AUDIO_PROOF: final zone={:?}", zone.0);
        println!("AUDIO_PROOF_DONE");
        exit.write(AppExit::Success);
    }
}
