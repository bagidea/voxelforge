//! Save / Load — the entrance's real "pick up where you left off" seam.
//!
//! One file, `savegame.json`, holds the player's position, heading, HP, stamina and
//! inventory (the item bag from `inventory.rs`). Quest progress travels through
//! Sun's existing [`quest::save_quest_journal`] /
//! [`quest::load_quest_journal`] API (writes `quest_save.json`) — this module never
//! touches `quest.rs`'s internals, it only calls its public save/load hooks and its
//! public story loader to re-seed a fresh journal for **New Game**.
//!
//! The inventory is saved as a whole [`Inventory`] (slots + hotbar selection), so a
//! Continue restores the exact bag the player saved. New Game starts empty.
//!
//! ## Keybind
//! **F6** quick-saves in Play (`quick_save`). F5/F9 are already taken by the editor's
//! map save/load (`editor_controls` in main.rs), so F6 is the free slot.

use std::collections::{HashMap, HashSet};

use bevy::input::ButtonInput;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    combat::{self, Health, Stamina},
    editor::AppState,
    inventory::Inventory,
    player_tuning as pt,
    quest::{self, QuestJournal, QuestProgress, QuestStatus},
    scene::Campsite,
    FlyCam, OrbitCam,
};

/// Where the player's own save lives (position/heading/HP/stamina). Quest progress
/// is `quest_save.json`, owned by `quest.rs` — the two are written together so a
/// "Continue" restores the whole session.
pub const SAVEGAME_PATH: &str = "savegame.json";

/// The quest save file `quest.rs` reads/writes (its `QUEST_SAVE_PATH` is private, so
/// this literal must stay in sync with `quest.rs:425`).
const QUEST_SAVE_PATH: &str = "quest_save.json";

/// Everything this module persists for one session. Kept deliberately small: the
/// heavy quest state lives in Sun's `quest_save.json`, not here.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveGame {
    pub version: u32,
    /// Avatar eye position (the same convention `move_body`/`FlyCam` use).
    pub position: [f32; 3],
    /// Avatar facing yaw (radians, around +Y).
    pub yaw: f32,
    pub hp: f32,
    pub stamina: f32,
    /// The player's bag (slots + hotbar selection). `#[serde(default)]` so a
    /// savegame from before inventory existed still loads (empty bag).
    #[serde(default)]
    pub inventory: Inventory,
}

impl Default for SaveGame {
    fn default() -> Self {
        Self {
            version: 1,
            position: [0.0, 0.0, 0.0],
            yaw: 0.0,
            hp: 100.0,
            stamina: 100.0,
            inventory: Inventory::default(),
        }
    }
}

/// Is there a save to continue from? Gates the menu's "Continue" button.
pub fn save_exists() -> bool {
    std::path::Path::new(SAVEGAME_PATH).exists()
}

/// The last session `write_save` flushed to disk, kept so the demo overlay can
/// print the *saved* values (not live drift) in the "before load" frame — which
/// is what makes it match the "after load" frame Continue restores, digit for digit.
#[derive(Resource, Default, Clone)]
pub struct LastSaveSnapshot {
    pub save: Option<SaveGame>,
    /// Quest flags sorted for a stable, copy-pasteable on-screen readout.
    pub flags: Vec<String>,
}

/// Serialise the current session to `savegame.json` **and** flush the quest journal
/// through Sun's API, so the two files always describe the same moment.
pub fn write_save(
    player: &Query<(&Transform, &FlyCam, &Health, &Stamina), Without<OrbitCam>>,
    journal: &QuestJournal,
    last: &mut LastSaveSnapshot,
    inv: &Inventory,
) -> Result<SaveGame, String> {
    let (tf, fly, hp, stam) = player.single().map_err(|e| e.to_string())?;
    let save = SaveGame {
        version: 1,
        position: [tf.translation.x, tf.translation.y, tf.translation.z],
        yaw: fly.face_yaw,
        hp: hp.cur,
        stamina: stam.cur,
        inventory: inv.clone(),
    };
    let json = serde_json::to_string_pretty(&save).map_err(|e| e.to_string())?;
    std::fs::write(SAVEGAME_PATH, json).map_err(|e| e.to_string())?;
    quest::save_quest_journal(journal)?;
    // Snapshot the exact bytes we wrote so the on-screen proof shows the save,
    // not whatever the avatar drifted to a frame later.
    let mut flags: Vec<String> = journal.flags.iter().cloned().collect();
    flags.sort();
    *last = LastSaveSnapshot {
        save: Some(save.clone()),
        flags,
    };
    println!("GAME_SAVE flags={:?}", journal.flags);
    println!(
        "GAME_SAVE ok path={SAVEGAME_PATH} pos=({:.2},{:.2},{:.2}) yaw={:.2} hp={:.0} stamina={:.0}",
        save.position[0], save.position[1], save.position[2], save.yaw, save.hp, save.stamina
    );
    Ok(save)
}

/// Read `savegame.json`; `None` when missing or corrupt.
pub fn read_save() -> Option<SaveGame> {
    let text = std::fs::read_to_string(SAVEGAME_PATH).ok()?;
    match serde_json::from_str::<SaveGame>(&text) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("GAME_LOAD parse error path={SAVEGAME_PATH} err={e}");
            None
        }
    }
}

/// Delete both save files (New Game starts from a clean slate).
fn delete_saves() {
    let _ = std::fs::remove_file(SAVEGAME_PATH);
    let _ = std::fs::remove_file(QUEST_SAVE_PATH);
    println!("GAME_NEW cleared {SAVEGAME_PATH} + {QUEST_SAVE_PATH}");
}

/// Re-seed a fresh journal from the story data, mirroring `quest::init_journal`
/// (which is private to `quest.rs`) via its public `load_story_data` + public types.
/// Each quest starts Active if `on_spawn`-triggered (or the opening quest), else
/// Locked; the opening quest is made Active and queued first in `active_order`.
fn seed_fresh_journal() -> QuestJournal {
    let mut journal = QuestJournal::default();
    let Some(data) = quest::load_story_data() else {
        eprintln!("GAME_NEW story data missing — quest journal left empty");
        return journal;
    };
    for qdef in &data.quests {
        journal.quests.insert(
            qdef.id.clone(),
            QuestProgress {
                status: if qdef.trigger.trigger_type == "on_spawn"
                    || Some(&qdef.id) == Some(&data.start_quest)
                {
                    QuestStatus::Active
                } else {
                    QuestStatus::Locked
                },
                current_objective: 0,
                completed_objectives: Vec::new(),
                objective_counts: HashMap::new(),
                flags: HashSet::new(),
            },
        );
    }
    if let Some(prog) = journal.quests.get_mut(&data.start_quest) {
        prog.status = QuestStatus::Active;
        journal.active_order.push(data.start_quest.clone());
        println!("GAME_NEW quest start={}", data.start_quest);
    }
    journal
}

/// **New Game**: wipe both saves, reset the quest journal to a fresh seed, and put
/// the player back at the campsite with full HP/stamina. Called by the menu button
/// (and the scripted proof) exactly once per selection.
pub fn start_new_game(
    player: &mut Query<
        (&mut Transform, &mut FlyCam, &mut Health, &mut Stamina),
        Without<OrbitCam>,
    >,
    cam: &mut Query<(&mut Transform, &mut OrbitCam)>,
    camp: Option<&Campsite>,
    journal: &mut QuestJournal,
    inv: &mut Inventory,
) {
    delete_saves();
    *journal = seed_fresh_journal();
    // New Game starts with an empty bag.
    *inv = Inventory::default();

    if let Ok((mut tf, mut fly, mut hp, mut stam)) = player.single_mut() {
        if let Some(camp) = camp {
            tf.translation = camp.eye;
            tf.rotation = Quat::from_axis_angle(Vec3::Y, camp.yaw);
            fly.face_yaw = camp.yaw;
        }
        fly.walking = true;
        fly.grounded = true;
        fly.vel = Vec3::ZERO;
        fly.hvel = Vec3::ZERO;
        fly.coyote_timer = 0.0;
        fly.jump_buffer = 0.0;
        *hp = Health::new(combat::HP_PLAYER);
        *stam = Stamina::full();
        println!(
            "GAME_NEW ok spawn=({:.2},{:.2},{:.2}) hp={:.0} stamina={:.0}",
            tf.translation.x, tf.translation.y, tf.translation.z, hp.cur, stam.cur
        );
    }
    if let Ok((mut ctf, mut orbit)) = cam.single_mut() {
        if let Some(camp) = camp {
            let rot = Quat::from_axis_angle(Vec3::Y, camp.yaw);
            ctf.translation = camp.eye + Vec3::Y * pt::PIVOT_UP + (rot * Vec3::Z) * pt::BOOM_DIST;
            ctf.rotation = rot;
            orbit.smooth_pos = ctf.translation;
            orbit.smooth_rot = rot;
        }
        orbit.yaw = camp.map(|c| c.yaw).unwrap_or_default();
        orbit.pitch = 0.0;
        orbit.dist = pt::BOOM_DIST;
        orbit.want_dist = pt::BOOM_DIST;
        orbit.pos_vel = Vec3::ZERO;
        orbit.fov = pt::CAM_BASE_FOV;
    }
}

/// **Continue**: apply a saved session to the live player. Returns `true` when a
/// save existed and was applied (the caller transitions to Play); `false` when
/// there is nothing to continue from (button is disabled in that case anyway).
pub fn continue_game(
    player: &mut Query<
        (&mut Transform, &mut FlyCam, &mut Health, &mut Stamina),
        Without<OrbitCam>,
    >,
    cam: &mut Query<(&mut Transform, &mut OrbitCam)>,
    journal: &mut QuestJournal,
    inv: &mut Inventory,
) -> bool {
    let Some(save) = read_save() else {
        println!("GAME_LOAD none — no {SAVEGAME_PATH}");
        return false;
    };
    let Ok((mut tf, mut fly, mut hp, mut stam)) = player.single_mut() else {
        return false;
    };
    tf.translation = Vec3::new(save.position[0], save.position[1], save.position[2]);
    tf.rotation = Quat::from_axis_angle(Vec3::Y, save.yaw);
    fly.face_yaw = save.yaw;
    fly.walking = true;
    fly.grounded = true;
    fly.vel = Vec3::ZERO;
    fly.hvel = Vec3::ZERO;
    fly.coyote_timer = 0.0;
    fly.jump_buffer = 0.0;
    hp.cur = save.hp.clamp(0.0, hp.max);
    stam.cur = save.stamina.clamp(0.0, combat::STAMINA_MAX);
    stam.delay = 0.0;
    stam.exhausted = 0.0;
    stam.penalty = false;
    // Restore the bag from the same moment — `write_save` flushed it into the
    // same file, so Continue must read it back. `normalize` re-pads the slot vec
    // if an older save had fewer slots.
    *inv = save.inventory.clone();
    inv.normalize();
    // Restore the quest journal from the same moment — `write_save` flushed both
    // files together, so Continue must read both (savegame.json + quest_save.json).
    if let Some(saved) = quest::load_quest_journal() {
        *journal = saved;
    }
    // Reset camera spring state so the lens doesn't interpolate from wherever it
    // was in the menu into the loaded position.
    if let Ok((mut ctf, mut orbit)) = cam.single_mut() {
        let rot = Quat::from_axis_angle(Vec3::Y, save.yaw);
        ctf.translation = tf.translation + Vec3::Y * pt::PIVOT_UP + (rot * Vec3::Z) * pt::BOOM_DIST;
        ctf.rotation = rot;
        orbit.yaw = save.yaw;
        orbit.pitch = 0.0;
        orbit.dist = pt::BOOM_DIST;
        orbit.want_dist = pt::BOOM_DIST;
        orbit.smooth_pos = ctf.translation;
        orbit.smooth_rot = rot;
        orbit.pos_vel = Vec3::ZERO;
        orbit.fov = pt::CAM_BASE_FOV;
    }
    println!("GAME_LOAD flags={:?}", journal.flags);
    println!(
        "GAME_LOAD ok pos=({:.2},{:.2},{:.2}) yaw={:.2} hp={:.0} stamina={:.0}",
        save.position[0], save.position[1], save.position[2], save.yaw, save.hp, save.stamina
    );
    true
}

/// **F6** quick-save while playing. F5/F9 belong to the editor's map save/load, so
/// F6 is the game-save slot (surfaced in the HUD line in main.rs).
fn quick_save(
    keys: Res<ButtonInput<KeyCode>>,
    player: Query<(&Transform, &FlyCam, &Health, &Stamina), Without<OrbitCam>>,
    journal: Res<QuestJournal>,
    inventory: Res<Inventory>,
    mut last: ResMut<LastSaveSnapshot>,
) {
    if keys.just_pressed(KeyCode::F6) {
        if let Err(e) = write_save(&player, &journal, &mut last, &inventory) {
            eprintln!("GAME_SAVE FAIL {e}");
        }
    }
}

/// Registers the F6 quick-save (Play only).
pub struct SaveGamePlugin;

impl Plugin for SaveGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastSaveSnapshot>()
            .add_systems(Update, quick_save.run_if(in_state(AppState::Play)));
    }
}
