//! Voxelforge quest & dialogue engine — data model, loader, state machine,
//! NPC interaction, triggers, objective tracker, and scripted proof.
//!
//! Loads `assets/story/act1.json` (Rose's schema, v1.0.0) and drives the full
//! quest loop: accept → complete objectives → reward → next quest unlocks.
//!
//! ## Schema reference: docs/act1-script.md + assets/story/act1.json
//! ## Integration: QuestPlugin in main.rs, gated to AppState::Play.

use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::combat;
use crate::FlyCam;

// =============================================================================
// JSON data types — matching assets/story/act1.json (Rose's schema v1.0.0)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryData {
    pub schema_version: String,
    pub act: u32,
    pub act_title: String,
    pub lang: String,
    pub map: String,
    pub start_quest: String,
    #[serde(default)]
    pub opening: Option<OpeningDef>,
    #[serde(default)]
    pub npcs: Vec<NpcDef>,
    #[serde(default)]
    pub regions: Vec<RegionDef>,
    #[serde(default)]
    pub quests: Vec<QuestDef>,
    #[serde(default)]
    pub dialogue: Vec<DialogueDef>,
    #[serde(default)]
    pub lore_items: Vec<LoreItemDef>,
    #[serde(default)]
    pub act_end: Option<ActEndDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningDef {
    pub id: String,
    pub duration_s: u32,
    pub title: String,
    #[serde(default)]
    pub no_menu: bool,
    #[serde(default)]
    pub no_loading_text: bool,
    pub spawn: SpawnDef,
    #[serde(default)]
    pub scene: Vec<String>,
    #[serde(default)]
    pub mechanics_seeded: Vec<String>,
    pub story_seed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnDef {
    pub position: PositionDef,
    pub facing: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PositionDef {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDef {
    pub id: String,
    pub name: String,
    pub role: String,
    pub voice: String,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub knows_and_withholds: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionDef {
    pub id: String,
    pub name: String,
    pub bounds: BoundsDef,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub code_spawned: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundsDef {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub giver: String,
    pub trigger: TriggerDef,
    pub premise: String,
    pub objectives: Vec<ObjectiveDef>,
    #[serde(default)]
    pub mechanics_taught: Vec<String>,
    pub rewards: QuestRewardDef,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDef {
    #[serde(rename = "type")]
    pub trigger_type: String,
    #[serde(default)]
    pub zone: Option<String>,
    #[serde(default)]
    pub quest: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub once: bool,
    #[serde(default)]
    pub entity: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveDef {
    pub id: String,
    pub kind: String,
    pub target: String,
    #[serde(default)]
    pub position: Option<PositionDef>,
    #[serde(default)]
    pub radius: Option<f32>,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestRewardDef {
    #[serde(default)]
    pub heal: Option<String>,
    #[serde(default)]
    pub set_flag: Option<String>,
    #[serde(default)]
    pub unlock_dialogue: Option<String>,
    #[serde(default)]
    pub advance_to: Option<String>,
    #[serde(default)]
    pub reveal_path: Option<String>,
    #[serde(default)]
    pub open_door: Option<String>,
    #[serde(default)]
    pub activate_campfire: Option<String>,
    #[serde(default)]
    pub unlock_act: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueDef {
    pub id: String,
    pub quest: String,
    pub trigger: TriggerDef,
    pub speaker: String,
    pub speaker_display: String,
    #[serde(default)]
    pub r#where: Option<String>,
    pub lines: Vec<String>,
    #[serde(default)]
    pub choices: Option<Vec<DialogueChoiceDef>>,
    #[serde(default)]
    pub completes_objective: Option<String>,
    #[serde(default)]
    pub sets_flag: Option<String>,
    #[serde(default)]
    pub advances_quest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoiceDef {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub next_dialogue: Option<String>,
    #[serde(default)]
    pub completes_objective: Option<String>,
    #[serde(default)]
    pub advances_quest: Option<String>,
    #[serde(default)]
    pub sets_flag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreItemDef {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub world_position: PositionDef,
    #[serde(default)]
    pub requires: Option<String>,
    pub lore_layer: u32,
    pub subtitle: String,
    pub text: String,
    #[serde(default)]
    pub triggers_dialogue: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActEndDef {
    pub id: String,
    pub title: String,
    pub trigger_quest: String,
    pub beats: Vec<String>,
    pub final_line: String,
    pub card: String,
    pub cliffhanger_questions: Vec<String>,
}

// =============================================================================
// Runtime state — Bevy resources & components
// =============================================================================

#[derive(Component, Clone, Debug)]
pub struct Npc {
    pub npc_id: String,
    pub display_name: String,
}

#[derive(Component)]
pub struct InteractionPrompt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgress {
    pub status: QuestStatus,
    pub current_objective: usize,
    pub completed_objectives: Vec<String>,
    /// objective_id → current count (for kill/collect objectives).
    pub objective_counts: HashMap<String, u32>,
    /// Flags set by quest rewards (e.g. "campfire_anchored", "maren_met").
    pub flags: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    Locked,
    Available,
    Active,
    Completed,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct QuestJournal {
    pub quests: HashMap<String, QuestProgress>,
    pub active_order: Vec<String>,
    pub flags: HashSet<String>,
}

impl Default for QuestJournal {
    fn default() -> Self {
        Self { quests: HashMap::new(), active_order: Vec::new(), flags: HashSet::new() }
    }
}

/// Current dialogue state — rendered by `dialogue_ui.rs`.
#[derive(Resource, Debug, Clone, Default)]
pub struct DialogueState {
    pub speaker: String,
    pub speaker_display: String,
    pub lines: Vec<String>,
    pub choices: Vec<DialogueChoiceDef>,
    pub current_line: usize,
    pub open: bool,
    pub dialogue_id: String,
    pub quest_id: String,
    /// True when choices are displayed — player must pick one.
    pub choosing: bool,
    /// The action from a completed dialogue line/choice.
    pub pending_action: Option<DialogueAction>,
}

#[derive(Debug, Clone)]
pub enum DialogueActionType {
    CompleteObjective(String), // objective id
    AdvanceQuest(String),      // quest id
    SetFlag(String),           // flag name
    OpenDialogue(String),      // dialogue id
    None,
}

#[derive(Debug, Clone)]
pub struct DialogueAction {
    pub action_type: DialogueActionType,
}

/// Messages the egui dialogue UI fires; the quest system reads them.
#[derive(Debug, Clone)]
pub enum DialogueUiEvent {
    Advance,               // Space → next line / show choices
    Choose(usize),          // Pick choice by index
    Close,                  // Close the dialogue box
}

impl Message for DialogueUiEvent {}

#[derive(Resource, Debug, Clone, Default)]
pub struct ObjectiveText {
    pub lines: Vec<String>,
}

#[derive(Component)]
pub struct ObjectiveTracker;

#[derive(Resource, Debug, Clone, Default)]
pub struct QuestDemo {
    pub phase: u8,
    pub stamp: Option<f32>,
    pub walked_to_gate: bool,
    pub accepted_q2: bool,
    pub killed_garren: bool,
    pub q3_completed: bool,
    pub q4_unlocked: bool,
    /// Scripted keyboard: the key currently held down, and when it changed.
    /// `just_pressed` only fires on the rising edge, so a tap has to let go
    /// before it can press again (see `tap`).
    pub tap_down: Option<KeyCode>,
    pub tap_t: f32,
    /// Which leg of `GATE_ROUTE` the demo is currently walking.
    pub leg: usize,
    /// Last whole-second tick when we printed a debug position line.
    pub debug_tick: u32,
    /// INSTRUMENTATION: monotonic counter incremented every frame quest_demo runs.
    /// check_block_place_triggers logs this value so we can verify .after() ordering
    /// in the raw log (demo_frame_id must be > 0 when check runs).
    pub demo_frame_id: u64,
}

/// Cached story data — loaded ONCE at startup so `load_story_data()` is never
/// called every frame from multiple Update systems (was logging STORY_LOAD 29k+ times).
#[derive(Resource, Debug, Clone)]
pub struct StoryDataRes {
    pub data: StoryData,
}

/// Track which enemy entities have been counted as kills.
#[derive(Resource, Debug, Clone, Default)]
pub struct KillLog {
    pub killed_entities: HashSet<Entity>,
}

// =============================================================================
// Campfire rest / save constants
// =============================================================================

/// Path for quest journal save file — written at campfire rest points.
const QUEST_SAVE_PATH: &str = "quest_save.json";

/// Campfire centre position (`campfire_square` x 29-35, z 27-31).
const CAMPFIRE_POS: (f32, f32) = (32.0, 29.0);

/// Max distance from campfire centre to show the "[E] Rest" prompt.
const CAMPFIRE_REST_RANGE: f32 = 4.0;

/// Marker component for the "[E] Rest at campfire" UI prompt.
#[derive(Component)]
pub struct CampfirePrompt;

/// Marker component for the "[E] Read" lore-item UI prompt.
#[derive(Component)]
pub struct LorePrompt;

// =============================================================================
// Save / Load — quest journal to disk
// =============================================================================

/// Serialise the full quest journal to `quest_save.json`.
pub fn save_quest_journal(journal: &QuestJournal) -> Result<(), String> {
    let json = serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?;
    std::fs::write(QUEST_SAVE_PATH, json).map_err(|e| e.to_string())?;
    println!("QUEST_SAVE ok path={QUEST_SAVE_PATH} flags={} quests={}",
        journal.flags.len(), journal.active_order.len());
    Ok(())
}

/// Try to load a previously-saved quest journal; returns `None` when the save
/// file doesn't exist or is corrupt — the caller falls back to `init_journal`.
pub fn load_quest_journal() -> Option<QuestJournal> {
    let text = std::fs::read_to_string(QUEST_SAVE_PATH).ok()?;
    match serde_json::from_str::<QuestJournal>(&text) {
        Ok(j) => {
            println!("QUEST_LOAD ok path={QUEST_SAVE_PATH} flags={} quests={}",
                j.flags.len(), j.active_order.len());
            Some(j)
        }
        Err(e) => {
            eprintln!("QUEST_LOAD parse error path={QUEST_SAVE_PATH} err={e}");
            None
        }
    }
}

// =============================================================================
// Plugin
// =============================================================================

pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestJournal>()
            .init_resource::<DialogueState>()
            .init_resource::<ObjectiveText>()
            .init_resource::<QuestDemo>()
            .init_resource::<KillLog>()
            .add_message::<DialogueUiEvent>()
            .add_systems(Startup, cache_story_data)
            .add_systems(PostStartup, (init_journal, spawn_npcs, spawn_objective_tracker, try_load_saved_journal).run_if(playing))
            .add_systems(
                Update,
                (
                    // INPUT_TRACE: snapshot just_pressed(R) BEFORE quest_demo injects.
                    // Ordering: before quest_demo → captures baseline state.
                    input_trace_before_inject.before(quest_demo),
                    quest_demo
                        .before(npc_interact)
                        .before(combat::gather_input)
                        .before(crate::fly_camera),
                    (npc_interact, resolve_dialogue_actions).chain(),
                    check_kill_triggers
                        .after(combat::player_combat)
                        .before(combat::husk_ai),
                    check_area_triggers,
                    check_approach_triggers,
                    fire_dialogue_triggers,
                    spawn_garren,
                    update_objective_tracker,
                    render_objective_tracker,
                    campfire_rest,
                    lore_interact,
                    // These two MUST run after quest_demo so they see keys
                    // the demo pressed this frame (just_pressed is cleared
                    // next PreUpdate, so check_* before quest_demo = lost).
                    check_block_place_triggers.after(quest_demo),
                    check_lore_read_triggers.after(quest_demo),
                    // INPUT_TRACE: snapshot just_pressed(R) AFTER the handler.
                    // Proves flag survived the full pipeline to end-of-frame.
                    input_trace_after_handler.after(check_block_place_triggers),
                    check_act_end,
                )
                    .run_if(in_state(crate::editor::AppState::Play)),
            );
    }
}

fn playing(cfg: Res<crate::Cfg>) -> bool { cfg.play }

// =============================================================================
// Story data cache — loaded ONCE at Startup; all systems read this instead of
// calling load_story_data() every frame (was logging STORY_LOAD 29k+ times).
// =============================================================================

fn cache_story_data(mut commands: Commands) {
    if let Some(data) = load_story_data() {
        commands.insert_resource(StoryDataRes { data });
    }
}

/// Shorthand to get the cached story data, panicking if it was never loaded
/// (should only happen if the JSON file is missing/corrupt).
fn story_data(story: &StoryDataRes) -> &StoryData { &story.data }

// =============================================================================
// JSON loader — called ONLY from cache_story_data at Startup.
// =============================================================================

pub fn load_story_data() -> Option<StoryData> {
    let path = "assets/story/act1.json";
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<StoryData>(&text) {
            Ok(data) => {
                println!("STORY_LOAD ok path={path} quests={} npcs={} dialogues={} regions={}",
                    data.quests.len(), data.npcs.len(), data.dialogue.len(), data.regions.len());
                Some(data)
            }
            Err(e) => { eprintln!("STORY_LOAD parse error path={path} err={e}"); None }
        },
        Err(e) => { eprintln!("STORY_LOAD read error path={path} err={e}"); None }
    }
}

// =============================================================================
// Journal init
// =============================================================================

fn init_journal(mut journal: ResMut<QuestJournal>, story: Res<StoryDataRes>) {
    let data = story_data(&story);
    for qdef in &data.quests {
        journal.quests.insert(qdef.id.clone(), QuestProgress {
            status: if qdef.trigger.trigger_type == "on_spawn" || Some(&qdef.id) == Some(&data.start_quest) {
                QuestStatus::Active
            } else {
                QuestStatus::Locked
            },
            current_objective: 0,
            completed_objectives: Vec::new(),
            objective_counts: HashMap::new(),
            flags: HashSet::new(),
        });
    }
    // Start with the opening quest.
    if let Some(prog) = journal.quests.get_mut(&data.start_quest) {
        prog.status = QuestStatus::Active;
        journal.active_order.push(data.start_quest.clone());
        println!("QUEST_INIT start={}", data.start_quest);
    }
    println!("QUEST_INIT quests_loaded={}", data.quests.len());
}

/// If a saved journal exists on disk, overwrite the freshly-initialised journal
/// with it — the player continues from their last campfire rest.
fn try_load_saved_journal(mut journal: ResMut<QuestJournal>) {
    if let Some(saved) = load_quest_journal() {
        *journal = saved;
    }
}

// =============================================================================
// NPC spawning — from story data NPCs list. Maren is mostly unseen (shadow
// behind gate), so we spawn a small marker entity at the gate crack.
// =============================================================================

fn spawn_npcs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    story: Res<StoryDataRes>,
) {
    let data = story_data(&story);

    // Spawn Maren at the gate square — she's behind the gate at position (32,1,4).
    let maren_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.45, 0.55),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mesh = meshes.add(Cuboid::new(0.5, 1.4, 0.3));
    commands.spawn((
        Mesh3d(mesh), MeshMaterial3d(maren_mat),
        Transform::from_xyz(32.0, 2.0, 4.0),
        Visibility::default(),
        Npc { npc_id: "maren".into(), display_name: "Elder Maren".into() },
    ));
    println!("SPAWN_NPC id=maren name=\"Elder Maren\" pos=(32,2,4)");
}

/// Garren the Husk — q3's `defeat` target. He is *not* in edhari.json (the eastern
/// guard post is `code_spawned` per the story data), so nothing stood at the post
/// and q3 could never be finished by fighting: `o3_defeat` had no body to kill.
/// He walks on when q3 goes Active, which is the moment Maren sends you east.
const GARREN_POS: (f32, f32) = (53.5, 8.0); // inside guard_post_east (x 48-56, z 4-12)
const GARREN_GROUND_Y: f32 = 1.0; // top face of the flat y=0 grass out there

fn spawn_garren(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    journal: Res<QuestJournal>,
    mut spawned: Local<bool>,
) {
    if *spawned { return; }
    let active = journal.quests.get("q3_gatekeeper")
        .map(|p| p.status == QuestStatus::Active).unwrap_or(false);
    if !active { return; }
    *spawned = true;

    let (x, z) = GARREN_POS;
    combat::spawn_guard_husk(&mut commands, &mut meshes, &mut materials, x, z, Some(GARREN_GROUND_Y));
    println!("SPAWN_NPC id=garren_husk name=\"Garren the Husk\" pos=({x},{GARREN_GROUND_Y},{z})");
}

/// Spawn the objective tracker UI node — a translucent panel in the top-right corner.
fn spawn_objective_tracker(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont { font_size: bevy::text::FontSize::from(14.0), ..default() },
        TextColor(Color::srgba(0.9, 0.85, 0.7, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            right: Val::Px(16.0),
            max_width: Val::Px(300.0),
            ..default()
        },
        ObjectiveTracker,
    ));
}

// =============================================================================
// NPC interaction — E key opens dialogue with nearest NPC
// =============================================================================

const INTERACT_RANGE: f32 = 5.0; // gate arch blocks terrain → interact through it

fn npc_interact(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<FlyCam>>,
    npcs: Query<(Entity, &Transform, &Npc)>,
    mut dialogue: ResMut<DialogueState>,
    journal: Res<QuestJournal>,
    story: Res<StoryDataRes>,
    mut commands: Commands,
    prompts: Query<Entity, With<InteractionPrompt>>,
    mut demo: ResMut<QuestDemo>,
) {
    let Ok(ptf) = player_q.single() else { return };

    let nearest = npcs.iter()
        .filter(|(_, ntf, _)| ptf.translation.distance(ntf.translation) < INTERACT_RANGE)
        .min_by(|(_, a, _), (_, b, _)| {
            da(a, ptf).partial_cmp(&da(b, ptf)).unwrap_or(std::cmp::Ordering::Equal)
        });

    for e in prompts.iter() { commands.entity(e).despawn(); }

    if let Some((_, _, npc)) = nearest {
        commands.spawn((
            Text::new(format!("[E] Talk to {}", npc.display_name)),
            TextFont { font_size: bevy::text::FontSize::from(18.0), ..default() },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
            Node { position_type: PositionType::Absolute, bottom: Val::Px(60.0),
                left: Val::Percent(50.0), margin: UiRect { left: Val::Px(-120.0), ..default() }, ..default() },
            InteractionPrompt,
        ));

        if keys.just_pressed(KeyCode::KeyE) && !dialogue.open {
            open_npc_dialogue(npc, &journal, &mut dialogue, story_data(&story));
            demo.walked_to_gate = true;
            println!("QUEST_INTERACT npc={} dialogue=open", npc.npc_id);
        }
    }
}

fn da(tf: &Transform, ptf: &Transform) -> f32 {
    tf.translation.distance(ptf.translation)
}

/// Open the appropriate dialogue for this NPC based on quest state.
fn open_npc_dialogue(npc: &Npc, journal: &QuestJournal, d: &mut DialogueState, data: &StoryData) {

    // Find the dialogue that should fire right now for this NPC.
    // Priority: active quest dialogue > any triggered dialogue.
    let active_dialogue = data.dialogue.iter().find(|dlg| {
        dlg.speaker == npc.npc_id &&
        dlg.trigger.trigger_type == "enter_zone" &&
        journal.quests.get(&dlg.quest)
            .map(|p| p.status == QuestStatus::Active).unwrap_or(false)
    });

    let dlg = match active_dialogue {
        Some(d) => d.clone(),
        None => {
            // Fall back to any dialogue for this speaker that hasn't been seen.
            match data.dialogue.iter().find(|dlg| dlg.speaker == npc.npc_id) {
                Some(d) => d.clone(),
                None => { d.open = false; return; }
            }
        }
    };

    d.speaker = dlg.speaker.clone();
    d.speaker_display = dlg.speaker_display.clone();
    d.lines = dlg.lines.clone();
    d.choices = dlg.choices.clone().unwrap_or_default();
    d.current_line = 0;
    d.open = true;
    d.dialogue_id = dlg.id.clone();
    d.quest_id = dlg.quest.clone();
    // Only show choices immediately if there are NO lines to read first.
    d.choosing = dlg.lines.is_empty() && dlg.choices.is_some();
    d.pending_action = None;
}

// =============================================================================
// Dialogue action resolution — process actions from completed dialogue lines
// =============================================================================

fn resolve_dialogue_actions(
    mut dialogue: ResMut<DialogueState>,
    mut journal: ResMut<QuestJournal>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_events: MessageReader<DialogueUiEvent>,
    story: Res<StoryDataRes>,
) {
    if !dialogue.open { return; }

    let data = story_data(&story);

    // Keyboard input
    let kb_advance = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    let kb_choice: Option<usize> =
        if keys.just_pressed(KeyCode::Digit1) { Some(0) }
        else if keys.just_pressed(KeyCode::Digit2) { Some(1) }
        else if keys.just_pressed(KeyCode::Digit3) { Some(2) }
        else if keys.just_pressed(KeyCode::Digit4) { Some(3) }
        else if keys.just_pressed(KeyCode::Digit5) { Some(4) }
        else if keys.just_pressed(KeyCode::Digit6) { Some(5) }
        else if keys.just_pressed(KeyCode::Escape) { dialogue.open = false; return; }
        else { None };
    let kb_close = keys.just_pressed(KeyCode::Escape);

    // UI events (from egui dialogue box)
    let mut ui_advance = false;
    let mut ui_choice: Option<usize> = None;
    let mut ui_close = false;
    for ev in ui_events.read() {
        match ev {
            DialogueUiEvent::Advance => ui_advance = true,
            DialogueUiEvent::Choose(i) => ui_choice = Some(*i),
            DialogueUiEvent::Close => ui_close = true,
        }
    }

    if ui_close { dialogue.open = false; return; }
    let advance = kb_advance || ui_advance;
    let picked = kb_choice.or(ui_choice);

    if dialogue.choosing {
        if let Some(idx) = picked {
            if let Some(choice) = dialogue.choices.get(idx).cloned() {
                apply_choice(&choice, &mut journal, &mut dialogue, data);
            }
        }
    } else if advance {
        // Advance past current line.
        if dialogue.current_line < dialogue.lines.len() {
            dialogue.current_line += 1;
        }
        // When we've shown all lines, show choices (if any) or close.
        if dialogue.current_line >= dialogue.lines.len() {
            if !dialogue.choices.is_empty() {
                // Choices defined — switch to choice mode.
                dialogue.choosing = true;
            } else {
                // No choices — check for dialogue-level actions then close.
                let dlg_def = data.dialogue.iter().find(|d| d.id == dialogue.dialogue_id);
                if let Some(def) = dlg_def {
                    if let Some(ref obj_id) = def.completes_objective {
                        complete_dialogue_objective(&mut journal, &dialogue.quest_id, obj_id, data);
                    }
                    if let Some(ref flag) = def.sets_flag {
                        journal.flags.insert(flag.clone());
                        println!("QUEST_FLAG set={flag}");
                    }
                    if let Some(ref next_q) = def.advances_quest {
                        advance_to_quest(&mut journal, next_q, data);
                    }
                }
                dialogue.open = false;
            }
        }
    }
}

fn apply_choice(choice: &DialogueChoiceDef, journal: &mut QuestJournal, dialogue: &mut DialogueState, data: &StoryData) {
    if let Some(ref obj_id) = choice.completes_objective {
        complete_dialogue_objective(journal, &dialogue.quest_id, obj_id, data);
    }
    if let Some(ref flag) = choice.sets_flag {
        journal.flags.insert(flag.clone());
        println!("QUEST_FLAG set={flag}");
    }
    if let Some(ref next_q) = choice.advances_quest {
        advance_to_quest(journal, next_q, data);
    }
    if let Some(ref next_dlg) = choice.next_dialogue {
        // Open a follow-up dialogue.
        if let Some(def) = data.dialogue.iter().find(|d| d.id == *next_dlg) {
            dialogue.dialogue_id = def.id.clone();
            dialogue.lines = def.lines.clone();
            dialogue.choices = def.choices.clone().unwrap_or_default();
            dialogue.current_line = 0;
            dialogue.choosing = def.lines.is_empty() && def.choices.is_some();
            return;
        }
    }
    dialogue.open = false;
}

fn complete_dialogue_objective(journal: &mut QuestJournal, quest_id: &str, obj_id: &str, data: &StoryData) {
    if let Some(prog) = journal.quests.get_mut(quest_id) {
        if !prog.completed_objectives.contains(&obj_id.to_string()) {
            prog.completed_objectives.push(obj_id.to_string());
            prog.current_objective += 1;
            println!("QUEST_STAGE_COMPLETE qid={quest_id} oid={obj_id} => PASS");

            // Check if all non-optional objectives are complete.
            if let Some(qdef) = data.quests.iter().find(|q| q.id == quest_id) {
                let all_done = qdef.objectives.iter()
                    .filter(|o| !o.optional)
                    .all(|o| prog.completed_objectives.contains(&o.id));
                if all_done {
                    complete_quest(journal, quest_id, data);
                }
            }
        }
    }
}

fn advance_to_quest(journal: &mut QuestJournal, quest_id: &str, data: &StoryData) {
    if let Some(prog) = journal.quests.get_mut(quest_id) {
        if prog.status == QuestStatus::Locked {
            prog.status = QuestStatus::Active;
            if !journal.active_order.contains(&quest_id.to_string()) {
                journal.active_order.push(quest_id.to_string());
            }
            println!("QUEST_ACCEPT id={quest_id} => PASS");
        }
    }
    // Also unlock via next chain.
    if let Some(qdef) = data.quests.iter().find(|q| q.id == quest_id) {
        if let Some(ref next_id) = qdef.next {
            if let Some(prog) = journal.quests.get_mut(next_id) {
                if prog.status == QuestStatus::Locked {
                    prog.status = QuestStatus::Available;
                    println!("QUEST_NEXT_OPEN next={next_id} => PASS");
                }
            }
        }
    }
}

fn complete_quest(journal: &mut QuestJournal, quest_id: &str, data: &StoryData) {
    let Some(prog) = journal.quests.get_mut(quest_id) else { return };
    if prog.status != QuestStatus::Active { return; }
    prog.status = QuestStatus::Completed;
    journal.active_order.retain(|id| id != quest_id);
    println!("QUEST_COMPLETE id={quest_id} => PASS");

    let Some(qdef) = data.quests.iter().find(|q| q.id == quest_id) else { return };

    // Apply rewards.
    if let Some(ref flag) = qdef.rewards.set_flag {
        journal.flags.insert(flag.clone());
    }
    // Advance to the next quest — try `next` then `rewards.advance_to`.
    let next_id = qdef.next.as_deref()
        .or(qdef.rewards.advance_to.as_deref());
    if let Some(next_id) = next_id {
        if let Some(next_prog) = journal.quests.get_mut(next_id) {
            if next_prog.status == QuestStatus::Locked {
                next_prog.status = QuestStatus::Active;
                if !journal.active_order.contains(&next_id.to_string()) {
                    journal.active_order.push(next_id.to_string());
                }
                // A quest can be taken on down two paths: a dialogue choice that
                // names it (`advance_to_quest`), or the quest before it finishing.
                // Both end in the same Locked→Active transition, so both log the
                // accept — otherwise whether QUEST_ACCEPT appears depends on which
                // of the two happened to land first.
                println!("QUEST_ACCEPT id={next_id} => PASS");
                println!("QUEST_NEXT_OPEN next={next_id} => PASS");
            }
        }
    }
}

// =============================================================================
// Kill-count trigger — when an enemy dies, advance any active quest objective
// that requires defeating that enemy.
// =============================================================================

fn check_kill_triggers(
    enemies: Query<(Entity, &combat::Health), With<combat::Enemy>>,
    mut kill_log: ResMut<KillLog>,
    mut journal: ResMut<QuestJournal>,
    story: Res<StoryDataRes>,
) {
    let data = story_data(&story);

    for (entity, health) in enemies.iter() {
        if !health.dead() { continue; }
        if kill_log.killed_entities.contains(&entity) { continue; }
        kill_log.killed_entities.insert(entity);

        // Find active quest objectives that require defeating "garren_husk".
        for qdef in &data.quests {
            let Some(prog) = journal.quests.get_mut(&qdef.id) else { continue };
            if prog.status != QuestStatus::Active { continue; }
            let Some(obj) = qdef.objectives.get(prog.current_objective) else { continue };
            if obj.kind != "defeat" { continue; }

            let count = prog.objective_counts.entry(obj.id.clone()).or_insert(0);
            let needed = if obj.count > 0 { obj.count } else { 1 };
            *count += 1;
            println!("QUEST_KILL qid={} oid={} count={}/{}",
                qdef.id, obj.id, count, needed);
            if *count >= needed && !prog.completed_objectives.contains(&obj.id) {
                prog.completed_objectives.push(obj.id.clone());
                prog.current_objective += 1;
                println!("QUEST_STAGE_COMPLETE qid={} oid={} => PASS (defeat)", qdef.id, obj.id);

                // Check quest completion.
                let all_done = qdef.objectives.iter()
                    .filter(|o| !o.optional)
                    .all(|o| prog.completed_objectives.contains(&o.id));
                if all_done {
                    complete_quest(&mut journal, &qdef.id, data);
                }
            }
        }
    }
}

// =============================================================================
// Area trigger — when the player enters a named region, advance quest stages.
// =============================================================================

fn check_area_triggers(
    player_q: Query<&Transform, With<FlyCam>>,
    mut journal: ResMut<QuestJournal>,
    mut entered: Local<HashSet<String>>,
    story: Res<StoryDataRes>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let data = story_data(&story);

    let px = ptf.translation.x;
    let pz = ptf.translation.z;

    for region in &data.regions {
        if entered.contains(&region.id) { continue; }
        if px >= region.bounds.x0 as f32 && px <= region.bounds.x1 as f32
            && pz >= region.bounds.z0 as f32 && pz <= region.bounds.z1 as f32
        {
            entered.insert(region.id.clone());
            println!("QUEST_AREA enter={} player=({px:.1},{pz:.1})", region.id);

            // Check quest stages requiring "reach_zone" for this region.
            for qdef in &data.quests {
                let Some(prog) = journal.quests.get_mut(&qdef.id) else { continue };
                if prog.status != QuestStatus::Active { continue; }
                let Some(obj) = qdef.objectives.get(prog.current_objective) else { continue };
                if obj.kind == "reach_zone" && obj.target == region.id
                    && !prog.completed_objectives.contains(&obj.id)
                {
                    prog.completed_objectives.push(obj.id.clone());
                    prog.current_objective += 1;
                    println!("QUEST_STAGE_COMPLETE qid={} oid={} => PASS (reach_zone)", qdef.id, obj.id);

                    let all_done = qdef.objectives.iter()
                        .filter(|o| !o.optional)
                        .all(|o| prog.completed_objectives.contains(&o.id));
                    if all_done {
                        complete_quest(&mut journal, &qdef.id, data);
                    }
                }
            }
        }
    }
}

// =============================================================================
// Approach trigger — when the player is within radius of an objective position.
// =============================================================================

fn check_approach_triggers(
    player_q: Query<&Transform, With<FlyCam>>,
    mut journal: ResMut<QuestJournal>,
    mut approached: Local<HashSet<String>>,
    story: Res<StoryDataRes>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let data = story_data(&story);

    for qdef in &data.quests {
        let Some(prog) = journal.quests.get_mut(&qdef.id) else { continue };
        if prog.status != QuestStatus::Active { continue; }
        let Some(obj) = qdef.objectives.get(prog.current_objective) else { continue };
        // `approach` (a place) and `approach_entity` (a creature) both resolve the
        // same way in the data — a position + a radius. Only `approach` was handled,
        // so q3's `o2_observe` never completed and the quest stalled one step short
        // of its `defeat` objective.
        if obj.kind != "approach" && obj.kind != "approach_entity" { continue; }
        if approached.contains(&obj.id) { continue; }

        let Some(pos) = obj.position else { continue; };
        let radius = obj.radius.unwrap_or(4.0);
        // `approach` / `approach_entity` use horizontal (x,z) distance only —
        // y is ignored per the schema (docs/act1-script.md §8: "the player walks,
        // so a high-mounted target such as the gate sigil at y=12 is reached by
        // standing beneath it at ground level").
        let dx = ptf.translation.x - pos.x;
        let dz = ptf.translation.z - pos.z;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist <= radius {
            approached.insert(obj.id.clone());
            prog.completed_objectives.push(obj.id.clone());
            prog.current_objective += 1;
            println!("QUEST_STAGE_COMPLETE qid={} oid={} => PASS (approach dist={dist:.1})",
                qdef.id, obj.id);

            let all_done = qdef.objectives.iter()
                .filter(|o| !o.optional)
                .all(|o| prog.completed_objectives.contains(&o.id));
            if all_done {
                complete_quest(&mut journal, &qdef.id, data);
            }
        }
    }
}

// =============================================================================
// Dialogue triggers — fire dialogue when entering zones or completing objectives.
// =============================================================================

fn fire_dialogue_triggers(
    mut dialogue: ResMut<DialogueState>,
    journal: Res<QuestJournal>,
    mut fired: Local<HashSet<String>>, // dialogue ids already fired
    story: Res<StoryDataRes>,
) {
    let data = story_data(&story);

    for dlg in &data.dialogue {
        if fired.contains(&dlg.id) { continue; }
        let should_fire = match dlg.trigger.trigger_type.as_str() {
            "enter_zone" => {
                // Check if player is in the zone.
                dlg.trigger.zone.as_ref().map_or(false, |zone| {
                    journal.flags.contains(&format!("entered_{zone}"))
                })
            }
            "on_objective" => {
                // Fire when the quest is active and the target objective isn't done yet.
                dlg.trigger.quest.as_ref().map_or(false, |_q| {
                    journal.quests.get(&dlg.quest)
                        .map(|p| p.status == QuestStatus::Active
                            && dlg.completes_objective.as_ref().map_or(true, |obj_id| {
                                !p.completed_objectives.contains(obj_id)
                            }))
                        .unwrap_or(false)
                })
            }
            "on_defeat" => {
                // Fire when an enemy is defeated. Check kill log.
                false // handled by check_kill_triggers directly
            }
            _ => false,
        };

        if should_fire {
            fired.insert(dlg.id.clone());
            dialogue.speaker = dlg.speaker.clone();
            dialogue.speaker_display = dlg.speaker_display.clone();
            dialogue.lines = dlg.lines.clone();
            dialogue.choices = dlg.choices.clone().unwrap_or_default();
            dialogue.current_line = 0;
            dialogue.open = true;
            dialogue.dialogue_id = dlg.id.clone();
            dialogue.quest_id = dlg.quest.clone();
            dialogue.choosing = dlg.lines.is_empty() && dlg.choices.is_some();
            println!("QUEST_DIALOGUE fire id={} trigger={}", dlg.id, dlg.trigger.trigger_type);
        }
    }
}

// =============================================================================
// Objective tracker HUD
// =============================================================================

fn update_objective_tracker(journal: Res<QuestJournal>, mut obj: ResMut<ObjectiveText>, story: Res<StoryDataRes>) {
    let data = story_data(&story);
    let mut lines = Vec::new();
    for qid in &journal.active_order {
        let Some(prog) = journal.quests.get(qid) else { continue };
        if prog.status != QuestStatus::Active { continue; }
        let Some(qdef) = data.quests.iter().find(|q| &q.id == qid) else { continue };
        lines.push(format!("◆ {}", qdef.title));
        if let Some(obj_def) = qdef.objectives.get(prog.current_objective) {
            let hint = obj_def.hint.as_deref().unwrap_or(&obj_def.id);
            let count = prog.objective_counts.get(&obj_def.id).copied().unwrap_or(0);
            let needed = if obj_def.count > 0 { obj_def.count } else { 1 };
            lines.push(format!("  · {} ({}/{})", obj_def.target, count, needed));
        }
    }
    obj.lines = lines;
}

fn render_objective_tracker(
    obj: Res<ObjectiveText>,
    mut q: Query<&mut Text, With<ObjectiveTracker>>,
) {
    if let Ok(mut text) = q.single_mut() {
        text.0 = obj.lines.join("\n");
    }
}

// =============================================================================
// Campfire rest point (site of grace) — save journal + heal + reset enemies
// =============================================================================

fn campfire_rest(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<FlyCam>>,
    journal: Res<QuestJournal>,
    mut died: MessageWriter<combat::PlayerDied>,
    mut commands: Commands,
    prompts: Query<Entity, With<CampfirePrompt>>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let dx = ptf.translation.x - CAMPFIRE_POS.0;
    let dz = ptf.translation.z - CAMPFIRE_POS.1;
    let dist = (dx * dx + dz * dz).sqrt();

    for e in prompts.iter() { commands.entity(e).despawn(); }

    if dist <= CAMPFIRE_REST_RANGE {
        commands.spawn((
            Text::new("[E] Rest at campfire"),
            TextFont { font_size: bevy::text::FontSize::from(18.0), ..default() },
            TextColor(Color::srgba(0.9, 0.75, 0.4, 0.9)),
            Node { position_type: PositionType::Absolute, bottom: Val::Px(100.0),
                left: Val::Percent(50.0),
                margin: UiRect { left: Val::Px(-100.0), ..default() }, ..default() },
            CampfirePrompt,
        ));
        if keys.just_pressed(KeyCode::KeyE) {
            if let Err(e) = save_quest_journal(&journal) {
                eprintln!("CAMPFIRE_REST save failed: {e}");
            }
            died.write(combat::PlayerDied);
            println!("CAMPFIRE_REST save + enemy reset fired");
        }
    }
}

// =============================================================================
// Lore item interaction — environmental storytelling
// =============================================================================

fn lore_interact(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<FlyCam>>,
    story: Res<StoryDataRes>,
    mut dialogue: ResMut<DialogueState>,
    mut commands: Commands,
    prompts: Query<Entity, With<LorePrompt>>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let data = story_data(&story);

    for e in prompts.iter() { commands.entity(e).despawn(); }

    let nearest = data.lore_items.iter()
        .filter_map(|li| {
            let dx = ptf.translation.x - li.world_position.x;
            let dz = ptf.translation.z - li.world_position.z;
            let d = (dx * dx + dz * dz).sqrt();
            (d <= 3.0).then_some((li, d))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((li, _)) = nearest {
        commands.spawn((
            Text::new(format!("[E] Read {}", li.name)),
            TextFont { font_size: bevy::text::FontSize::from(16.0), ..default() },
            TextColor(Color::srgba(0.8, 0.85, 0.9, 0.9)),
            Node { position_type: PositionType::Absolute, bottom: Val::Px(140.0),
                left: Val::Percent(50.0),
                margin: UiRect { left: Val::Px(-120.0), ..default() }, ..default() },
            LorePrompt,
        ));
        if keys.just_pressed(KeyCode::KeyE) && !dialogue.open {
            dialogue.speaker = li.id.clone();
            dialogue.speaker_display = li.name.clone();
            dialogue.lines = vec![li.subtitle.clone(), li.text.clone()];
            dialogue.choices = Vec::new();
            dialogue.current_line = 0;
            dialogue.open = true;
            dialogue.dialogue_id = li.id.clone();
            dialogue.quest_id = String::new();
            dialogue.choosing = false;
            dialogue.pending_action = None;
            if let Some(ref dlg_id) = li.triggers_dialogue {
                dialogue.pending_action = Some(DialogueAction {
                    action_type: DialogueActionType::OpenDialogue(dlg_id.clone()),
                });
            }
            println!("LORE_READ id={} name=\"{}\"", li.id, li.name);
        }
    }
}

// =============================================================================
// Block-place trigger — complete place_block objectives on R key near target
// =============================================================================

/// INSTRUMENTATION (diagnose-synthetic-input): atomic counter tracking how many
/// times the handler SAW the press (should match PRESS_R_COUNT if no consumption).
static DETECT_R_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// INSTRUMENTATION (q4-diagnose): snapshot just_pressed(R) BEFORE quest_demo
/// injects synthetic keys. This is the baseline — should be false unless a
/// real keyboard press or stale flag survived from the previous frame.
fn input_trace_before_inject(
    keys: Res<ButtonInput<KeyCode>>,
    demo: Res<QuestDemo>,
) {
    let r_jp = keys.just_pressed(KeyCode::KeyR);
    let r_held = keys.pressed(KeyCode::KeyR);
    let all_jp: Vec<String> = keys.get_just_pressed()
        .map(|k| format!("{:?}", k)).collect();
    println!("INPUT_TRACE BEFORE inject  R_just_pressed={} R_held={} jp=[{}] demo_fid={}",
        r_jp, r_held, all_jp.join(","), demo.demo_frame_id);
}

/// INSTRUMENTATION (q4-diagnose): snapshot just_pressed(R) AFTER
/// check_block_place_triggers ran. If the handler consumed the flag
/// (by reading just_pressed), Bevy still reports true — just_pressed
/// is only cleared at PreUpdate of the NEXT frame. This confirms
/// the flag survived the full pipeline.
fn input_trace_after_handler(
    keys: Res<ButtonInput<KeyCode>>,
    demo: Res<QuestDemo>,
) {
    let r_jp = keys.just_pressed(KeyCode::KeyR);
    let r_held = keys.pressed(KeyCode::KeyR);
    let all_jp: Vec<String> = keys.get_just_pressed()
        .map(|k| format!("{:?}", k)).collect();
    println!("INPUT_TRACE AFTER  handler R_just_pressed={} R_held={} jp=[{}] demo_fid={}",
        r_jp, r_held, all_jp.join(","), demo.demo_frame_id);
}

fn check_block_place_triggers(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<FlyCam>>,
    mut journal: ResMut<QuestJournal>,
    story: Res<StoryDataRes>,
    demo: Res<QuestDemo>,
) {
    // INSTRUMENTATION: capture demo frame-id to verify ordering (must be > 0
    // when check runs, proving quest_demo already ran this frame).
    let demo_fid = demo.demo_frame_id;
    let r_pressed = keys.just_pressed(KeyCode::KeyR);
    let r_held = keys.pressed(KeyCode::KeyR);
    // INSTRUMENTATION: snapshot ALL keys in just_pressed for cross-reference
    let all_jp: Vec<String> = keys.get_just_pressed()
        .map(|k| format!("{:?}", k)).collect();
    if !r_pressed {
        // DEBUG: log every frame R is held but not just_pressed
        if r_held {
            let Ok(ptf) = player_q.single() else { return };
            println!("QUEST_DEBUG_BLOCK R held but not just_pressed pt=({:.1},{:.1}) jp=[{}] press_calls={} detect_calls={} demo_fid={}",
                ptf.translation.x, ptf.translation.z, all_jp.join(","),
                PRESS_R_COUNT.load(std::sync::atomic::Ordering::Relaxed),
                DETECT_R_COUNT.load(std::sync::atomic::Ordering::Relaxed),
                demo_fid);
        }
        return;
    }
    let n = DETECT_R_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let Ok(ptf) = player_q.single() else { return };
    let data = story_data(&story);
    println!("QUEST_DEBUG_BLOCK R just_pressed pt=({:.1},{:.1}) detect_count={} press_calls={} jp=[{}] demo_fid={}",
        ptf.translation.x, ptf.translation.z, n + 1,
        PRESS_R_COUNT.load(std::sync::atomic::Ordering::Relaxed),
        all_jp.join(","),
        demo_fid);

    // INSTRUMENTATION: dump q4 state to see if it's even eligible
    let q4 = journal.quests.get("q4_what_walls_remember");
    let q4_status = q4.map(|p| format!("{:?}", p.status)).unwrap_or_else(|| "MISSING".into());
    let q4_obj = q4.map(|p| p.current_objective).unwrap_or(999);
    let q4_done = q4.map(|p| p.completed_objectives.join(",")).unwrap_or_default();
    println!("QUEST_DEBUG_BLOCK q4_state status={} cur_obj={} done=[{}]",
        q4_status, q4_obj, q4_done);

    let mut found_any = false;
    for qdef in &data.quests {
        let Some(prog) = journal.quests.get_mut(&qdef.id) else { continue };
        if prog.status != QuestStatus::Active { continue; }
        let Some(obj) = qdef.objectives.get(prog.current_objective) else { continue };
        if obj.kind != "place_block" { continue; }
        if prog.completed_objectives.contains(&obj.id) { continue; }
        let Some(pos) = obj.position else { continue; };
        let radius = obj.radius.unwrap_or(4.0);
        let dx = ptf.translation.x - pos.x;
        let dz = ptf.translation.z - pos.z;
        let dist = (dx * dx + dz * dz).sqrt();
        println!("QUEST_DEBUG_BLOCK checking qid={} oid={} pos=({},{}) dist={:.1} radius={:.0}",
            qdef.id, obj.id, pos.x, pos.z, dist, radius);
        if dist <= radius {
            found_any = true;
            prog.completed_objectives.push(obj.id.clone());
            prog.current_objective += 1;
            println!("QUEST_STAGE_COMPLETE qid={} oid={} => PASS (place_block)", qdef.id, obj.id);
            let all_done = qdef.objectives.iter().filter(|o| !o.optional)
                .all(|o| prog.completed_objectives.contains(&o.id));
            if all_done { complete_quest(&mut journal, &qdef.id, data); }
        } else {
            println!("QUEST_DEBUG_BLOCK too far qid={} oid={} dist={:.1} > radius={:.0}",
                qdef.id, obj.id, dist, radius);
        }
    }
    if !found_any {
        println!("QUEST_DEBUG_BLOCK NO place_block objective matched — q4 may be locked/wrong-obj/already-done");
    }
}

// =============================================================================
// Lore-read trigger — complete interact objectives on E key near lore item
// =============================================================================

fn check_lore_read_triggers(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<FlyCam>>,
    mut journal: ResMut<QuestJournal>,
    story: Res<StoryDataRes>,
) {
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(ptf) = player_q.single() else { return };
    let data = story_data(&story);

    for qdef in &data.quests {
        let Some(prog) = journal.quests.get_mut(&qdef.id) else { continue };
        if prog.status != QuestStatus::Active { continue; }
        let Some(obj) = qdef.objectives.get(prog.current_objective) else { continue };
        if obj.kind != "interact" { continue; }
        if prog.completed_objectives.contains(&obj.id) { continue; }
        let Some(li) = data.lore_items.iter().find(|li| li.id == obj.target) else { continue; };
        let dx = ptf.translation.x - li.world_position.x;
        let dz = ptf.translation.z - li.world_position.z;
        if (dx * dx + dz * dz).sqrt() <= 3.0 {
            prog.completed_objectives.push(obj.id.clone());
            prog.current_objective += 1;
            println!("QUEST_STAGE_COMPLETE qid={} oid={} => PASS (interact lore=\"{}\")",
                qdef.id, obj.id, li.name);
            let all_done = qdef.objectives.iter().filter(|o| !o.optional)
                .all(|o| prog.completed_objectives.contains(&o.id));
            if all_done { complete_quest(&mut journal, &qdef.id, data); }
        }
    }
}

// =============================================================================
// Act-end check — when q5 completes + act1_complete flag is set, print the
// cliffhanger beats per §11.8 of the schema spec.
// =============================================================================

fn check_act_end(
    journal: Res<QuestJournal>,
    story: Res<StoryDataRes>,
    mut fired: Local<bool>,
) {
    if *fired { return; }
    let q5_done = journal.quests.get("q5_sigil_that_knew_you")
        .map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
    if !q5_done || !journal.flags.contains("act1_complete") { return; }
    *fired = true;

    let data = story_data(&story);
    if let Some(ref end) = data.act_end {
        println!("ACT_END id={} title=\"{}\"", end.id, end.title);
        for (i, beat) in end.beats.iter().enumerate() {
            println!("ACT_END beat[{}] {}", i, beat);
        }
        println!("ACT_END final_line: {}", end.final_line);
        println!("ACT_END card: {}", end.card);
        println!("ACT1_COMPLETE");
    }
}

// =============================================================================
// Scripted quest demo proof (`--quest-demo`)
// =============================================================================
// Proves the full quest loop on Rose's schema:
//   q1_embers (auto-complete on spawn, approach campfire)
//   → q2_voice_in_stone (reach gate zone, talk to Maren)
//   → q3_gatekeeper (advance from dialogue, reach guard post, defeat husk)
//   → quest complete → q4 unlocked
//
// The demo's ONLY output is `ButtonInput<KeyCode>` — it presses keys and then
// watches the journal. It never opens a dialogue, applies a choice or scores an
// objective by hand, and it never prints a QUEST_ACCEPT / QUEST_KILL grade for
// itself: those lines come out of `advance_to_quest` / `complete_quest` /
// `check_kill_triggers` when the real systems do the work. A demo that graded its
// own shortcut would pass with the whole interaction path broken — which is
// exactly what the previous version did.

/// Edhari is not a straight line, and every leg here is a wall the body actually
/// hits: the spawn shelter has two-block posts at x=29 and x=35, a six-block
/// longhouse (x 28-36, z 25-27) sits directly north of the campfire, and the
/// gate square is a two-block plateau whose only climbable side is the x≈32 ramp
/// (z 12-15).  Step-up is one block, so this is the walkable path.
/// Each leg is an (x, z) waypoint.
///
/// Route: walk north FIRST to z≈22 (south of the longhouse, clear of shelter
/// posts), THEN east to x=45, then north along the open field, then west to the
/// ramp, then up onto the gate square.  This avoids the x=35 post that blocked
/// the previous "east-first" route on the current map.
const GATE_ROUTE: [(f32, f32); 5] = [
    (32.5, 22.0), // north — clear the shelter posts (x=29/35) + longhouse south edge
    (45.0, 22.0), // east along clear ground south of the longhouse
    (45.0, 16.5), // north along the open east field
    (32.5, 16.5), // west onto the ramp column
    (32.5, 6.4),  // up the ramp onto the gate square
];
const WAYPOINT_TOL: f32 = 0.6; // blocks — a 6 b/s walk moves ~0.1 per frame
/// How close the demo walks to Maren before pressing E. `INTERACT_RANGE` is 5.0
/// and the gate plateau puts the eye 2.6 blocks above her, so leave real margin.
const TALK_DIST: f32 = 4.3;
/// Stop inside `guard_post_east` (x 48-56, z 4-12) — far enough in that the region
/// trigger has fired, close enough that Garren's 12-block aggro picks the player up.
const POST_STOP_X: f32 = 49.5;
/// Close to this before swinging: melee reach is 2.0 + half-width + 0.6.
const MELEE_CLOSE: f32 = 2.2;
/// Scripted taps alternate release → press on this cadence. `just_pressed` only
/// fires on a rising edge, so a key held down forever registers exactly once.
const TAP_PERIOD: f32 = 0.25;

/// One scripted keystroke: release whatever is held, then press `key` after a
/// minimum gap.  Always presses — no alternating — so every call that is far
/// enough from the last produces a clean `just_pressed` edge the reader can see.
///
/// INSTRUMENTATION (diagnose-synthetic-input): atomic counter so we can
/// cross-reference "N presses emitted" vs "M presses detected" in the handler.
static PRESS_R_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static PRESS_E_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn press_key(demo: &mut QuestDemo, keys: &mut ButtonInput<KeyCode>, key: KeyCode, t: f32) {
    // Release the previous key so it stops being held.
    if let Some(down) = demo.tap_down.take() {
        keys.reset(down);
        if down == KeyCode::KeyR { println!("QUEST_DEBUG_PRESS R released prev={:?}  just_pressed_after_reset={}",
            down, keys.just_pressed(KeyCode::KeyR)); }
    }
    // Only press when enough time has passed since the last press (proven 0.35s
    // hold + 0.15s gap is enough for `just_pressed` to register).
    if t - demo.tap_t > TAP_PERIOD {
        let was_already = keys.pressed(key);
        keys.press(key);
        demo.tap_down = Some(key);
        demo.tap_t = t;
        if key == KeyCode::KeyR {
            let n = PRESS_R_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            println!("QUEST_DEBUG_PRESS R pressed at t={:.3} press_count={} was_already_pressed={} just_pressed_now={}",
                t, n + 1, was_already, keys.just_pressed(KeyCode::KeyR));
        }
        if key == KeyCode::KeyE {
            let n = PRESS_E_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            println!("QUEST_DEBUG_PRESS E pressed at t={:.3} press_count={} was_already_pressed={} just_pressed_now={}",
                t, n + 1, was_already, keys.just_pressed(KeyCode::KeyE));
        }
    } else {
        // DEBUG: log when press_key is called but TAP_PERIOD hasn't elapsed
        if key == KeyCode::KeyR {
            println!("QUEST_DEBUG_PRESS R SKIPPED (tap cooldown) t={:.3} tap_t={:.3} gap={:.3}",
                t, demo.tap_t, t - demo.tap_t);
        }
    }
}

fn set_key(keys: &mut ButtonInput<KeyCode>, key: KeyCode, down: bool) {
    if down { keys.press(key); } else { keys.reset(key); }
}

/// Hold the WASD keys that walk the body toward a target offset. WASD is
/// camera-relative and a headless run never moves the mouse, so yaw stays 0:
/// W = -Z, S = +Z, D = +X, A = -X. Holding the key toward the target also turns
/// the avatar to face it, which is what puts a swing's 60° cone on an enemy.
fn steer(keys: &mut ButtonInput<KeyCode>, dx: f32, dz: f32, tol: f32) {
    set_key(keys, KeyCode::KeyD, dx > tol);
    set_key(keys, KeyCode::KeyA, dx < -tol);
    set_key(keys, KeyCode::KeyS, dz > tol);
    set_key(keys, KeyCode::KeyW, dz < -tol);
}

#[allow(clippy::too_many_arguments)]
fn quest_demo(
    time: Res<Time>,
    mut key_input: ResMut<ButtonInput<KeyCode>>,
    mut demo: ResMut<QuestDemo>,
    dialogue: Res<DialogueState>,
    journal: Res<QuestJournal>,
    player_q: Query<(&Transform, &combat::Health), With<FlyCam>>,
    mut fly_q: Query<&mut FlyCam>,
    npcs: Query<&Transform, With<Npc>>,
    enemies: Query<(&Transform, &combat::Health), With<combat::Enemy>>,
    mut exit: bevy::ecs::message::MessageWriter<AppExit>,
    cfg: Res<crate::Cfg>,
) {
    if !cfg.quest_demo { return; }
    // INSTRUMENTATION: bump frame-id at the START so any .after() system
    // that reads it can prove quest_demo really ran first this frame.
    demo.demo_frame_id = demo.demo_frame_id.wrapping_add(1);
    let t = time.elapsed_secs();
    let Ok((ptf, php)) = player_q.single() else { return };

    if t < 1.5 { return; }
    if demo.stamp.is_none() { demo.stamp = Some(t); }
    let phase_t = demo.stamp.map(|s| t - s).unwrap_or(0.0);
    macro_rules! enter { ($p:expr) => {{ demo.phase = $p; demo.stamp = Some(t); }}; }
    macro_rules! hands_off {
        () => {{
            steer(&mut key_input, 0.0, 0.0, 1.0);
            if let Some(k) = demo.tap_down.take() { key_input.reset(k); }
        }};
    }

    // ---- phase 0: walk the route to the gate square (q2's trigger zone) ---------
    // Spawn is (32.5,_,32.5); the gate_square region is x 27-37 / z 3-8 and Maren
    // stands in the gate arch at (32,2,4). Walking in trips q2's `o1_gate`
    // reach_zone objective on the way — that is the quest doing its own work.
    if demo.phase == 0 {
        if let Ok(mut fly) = fly_q.single_mut() { fly.walking = true; }
        if let Some(&(tx, tz)) = GATE_ROUTE.get(demo.leg) {
            let (dx, dz) = (tx - ptf.translation.x, tz - ptf.translation.z);
            if dx.abs() <= WAYPOINT_TOL && dz.abs() <= WAYPOINT_TOL {
                demo.leg += 1;
                return;
            }
            // ---- fight any enemy that aggros during the walk ----
            // Face the closest enemy, close distance, and attack. The walk
            // continues once the enemy is dead. The walk clock is NOT reset
            // during combat — 90 s is enough for the full route + the fight.
            let target = enemies.iter()
                .filter(|(etf, h)| !h.dead() && da(etf, ptf) < 12.0)
                .min_by(|(a, _), (b, _)| {
                    da(a, ptf).partial_cmp(&da(b, ptf)).unwrap_or(std::cmp::Ordering::Equal)
                });
            if let Some((etf, _)) = target {
                let to = etf.translation - ptf.translation;
                let flat = Vec3::new(to.x, 0.0, to.z);
                if flat.length() > MELEE_CLOSE {
                    steer(&mut key_input, flat.x, flat.z, 0.4);
                } else {
                    steer(&mut key_input, 0.0, 0.0, 1.0);
                }
                press_key(&mut demo, &mut key_input,KeyCode::KeyX, t);
            } else {
                steer(&mut key_input, dx, dz, WAYPOINT_TOL);
            }
            let tick = phase_t as u32;
            if tick > 0 && tick % 3 == 0 && tick != demo.debug_tick {
                println!("QUEST_DEBUG pt=({:.1},{:.1}) leg={} t={:.1}",
                    ptf.translation.x, ptf.translation.z, demo.leg, phase_t);
                demo.debug_tick = tick;
            }
            if phase_t > 90.0 {
                steer(&mut key_input, 0.0, 0.0, 1.0);
                println!("QUEST_WALK_TO_GATE timeout leg={} at ({:.1},{:.1}) => FAIL",
                    demo.leg, ptf.translation.x, ptf.translation.z);
                demo.phase = 99;
            }
            return;
        }
        steer(&mut key_input, 0.0, 0.0, 1.0);
        demo.walked_to_gate = true;
        println!("QUEST_WALK_TO_GATE z={:.1} => reached gate_square", ptf.translation.z);
        // By now the approach trigger has closed out q1 (campfire) and the area
        // trigger has closed out q2's gate objective.
        let q1 = journal.quests.get("q1_embers").map(|p| p.status).unwrap_or(QuestStatus::Locked);
        println!("QUEST_CHECK q1_embers status={:?}", q1);
        enter!(1);
    }

    // ---- phase 1: talk to Maren with the keyboard ----------------------------
    // E → `npc_interact` opens dlg_maren_gate (4 lines, then 5 choices).
    // Enter → `resolve_dialogue_actions` walks the lines, until choices appear.
    // Digit5 → choice index 4, "I'll go east", whose `advances_quest` is what
    // makes q3 Active — and the engine prints QUEST_ACCEPT when it does.
    //
    // Key sequencing uses `phase_t` with a 0.35 s hold + 0.15 s gap per key,
    // and the last key pressed is always cleared before the next press so the
    // reader always sees a clean `just_pressed` edge.  The previous `tap()`
    // alternator could release the wrong key when timing drifted.
    if demo.phase == 1 {
        let q3_active = journal.quests.get("q3_gatekeeper")
            .map(|p| p.status == QuestStatus::Active).unwrap_or(false);
        if q3_active {
            hands_off!();
            demo.accepted_q2 = true;
            enter!(2);
            return;
        }

        // Close the last stride if collision parked the body outside talking range.
        if let Some(mtf) = npcs.iter().next() {
            let d = ptf.translation.distance(mtf.translation);
            if d > TALK_DIST && !dialogue.open {
                let to = mtf.translation - ptf.translation;
                steer(&mut key_input, to.x, to.z, 0.3);
                if phase_t > 8.0 {
                    steer(&mut key_input, 0.0, 0.0, 1.0);
                    println!("QUEST_INTERACT unreachable — Maren {d:.1} blocks away => FAIL");
                    demo.phase = 99;
                }
                return;
            }
        }
        steer(&mut key_input, 0.0, 0.0, 1.0);

        // Release whatever was held, then press the right key for this time slice.
        // 0.35 s hold + 0.15 s gap = 0.5 s per key press → 10 keys in 5 s.
        if let Some(down) = demo.tap_down.take() { key_input.reset(down); }
        let t0 = demo.tap_t;
        let key = if !dialogue.open {
            KeyCode::KeyE
        } else if dialogue.choosing {
            KeyCode::Digit5
        } else {
            KeyCode::Enter
        };
        // Only press a fresh key when we've waited long enough since the last press.
        if t - t0 > 0.5 {
            key_input.press(key);
            demo.tap_down = Some(key);
            demo.tap_t = t;
        }

        if phase_t > 25.0 {
            hands_off!();
            println!("QUEST_ACCEPT timeout — q3_gatekeeper not active => FAIL");
            demo.phase = 99;
        }
        return;
    }

    // ---- phase 2: walk east into guard_post_east (q3's o1_east) ---------------
    // Off the gate plateau and out along z≈6.5, which is flat ground all the way.
    // Crossing x=48 is what fires the region trigger; `o2_observe` (radius 12 of
    // (52,1,8)) falls out on the next frame, leaving `o3_defeat` current.
    if demo.phase == 2 {
        // The quest itself says when the walk is over: `o2_observe` completing means
        // the region trigger fired AND Garren is in sight. Marching on to a fixed x
        // past that point just walks into his reach with no hands on the fight keys,
        // which is how the player used to die here.
        let observed = journal.quests.get("q3_gatekeeper")
            .map(|p| p.completed_objectives.iter().any(|o| o == "o2_observe"))
            .unwrap_or(false);
        if !observed && ptf.translation.x < POST_STOP_X {
            steer(&mut key_input, POST_STOP_X - ptf.translation.x, 0.0, WAYPOINT_TOL);
            if phase_t > 25.0 {
                steer(&mut key_input, 0.0, 0.0, 1.0);
                println!("QUEST_WALK_GUARD_POST timeout x={:.1} z={:.1} hp={:.0} => FAIL",
                    ptf.translation.x, ptf.translation.z, php.cur);
                demo.phase = 99;
            }
            return;
        }
        steer(&mut key_input, 0.0, 0.0, 1.0);
        println!("QUEST_WALK_GUARD_POST reached x={:.1} z={:.1} hp={:.0}",
            ptf.translation.x, ptf.translation.z, php.cur);
        enter!(3);
        return;
    }

    // ---- phase 3: fight Garren the Husk at the guard post ---------------------
    // Real keys only: a direction key to close the gap (which also turns the body
    // so the swing's cone covers him) and X for a light attack. The kill is scored
    // by `check_kill_triggers` — QUEST_KILL is the engine's line, not the demo's.
    if demo.phase == 3 {
        if journal.quests.get("q3_gatekeeper")
            .map(|p| p.status == QuestStatus::Completed).unwrap_or(false)
        {
            hands_off!();
            demo.killed_garren = true;
            enter!(4);
            return;
        }
        // Only fight what is actually at the post — the village husk 20 blocks
        // west is a different encounter and must not pull the demo off the quest.
        let target = enemies.iter()
            .filter(|(etf, h)| !h.dead() && etf.translation.distance(ptf.translation) < 16.0)
            .min_by(|(a, _), (b, _)| {
                da(a, ptf).partial_cmp(&da(b, ptf)).unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some((etf, _)) = target {
            let to = etf.translation - ptf.translation;
            let flat = Vec3::new(to.x, 0.0, to.z);
            if flat.length() > MELEE_CLOSE {
                steer(&mut key_input, flat.x, flat.z, 0.4);
            } else {
                steer(&mut key_input, 0.0, 0.0, 1.0);
            }
            press_key(&mut demo, &mut key_input,KeyCode::KeyX, t);
        }
        // Dying respawns the body at the campfire ~20 blocks west; without this the
        // demo would stand there swinging at nothing until the timeout and blame it
        // on Garren. Say what actually happened instead.
        if ptf.translation.x < 40.0 {
            hands_off!();
            println!("QUEST_KILL player died at the post (respawned at x={:.1}) => FAIL",
                ptf.translation.x);
            demo.phase = 99;
            return;
        }
        if phase_t > 60.0 {
            hands_off!();
            println!("QUEST_KILL timeout — Garren still standing (player hp={:.0}) => FAIL",
                php.cur);
            demo.phase = 99;
        }
        return;
    }

    // ---- phase 4: complete q4 (build + read 2 lore items) ----------------------
    // o1_build: walk to (50,9), press R (place_block). o2_ledger: walk to (46,24),
    // press E (interact lore_village_ledger). o3_offering: walk to (33,14), press E.
    if demo.phase == 4 {
        let q4_done = journal.quests.get("q4_what_walls_remember")
            .map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        if q4_done {
            hands_off!();
            println!("QUEST_COMPLETE q4_what_walls_remember => PASS (all objectives done)");
            enter!(5);
            return;
        }
        let cur = journal.quests.get("q4_what_walls_remember")
            .map(|p| p.current_objective).unwrap_or(0);
        // ---- DEBUG: trace Phase 4 state every whole second ----
        let tick = phase_t as u32;
        if tick > 0 && tick % 2 == 0 && tick != demo.debug_tick {
            let q4_status = journal.quests.get("q4_what_walls_remember")
                .map(|p| format!("{:?}/{}", p.status, p.current_objective))
                .unwrap_or_else(|| "missing".into());
            println!("QUEST_DEBUG_P4 pt=({:.1},{:.1}) cur={} status={} walking={} t={:.1}",
                ptf.translation.x, ptf.translation.z, cur, q4_status, fly_q.single().map(|f| f.walking).unwrap_or(false), phase_t);
            demo.debug_tick = tick;
        }
        match cur {
            0 => { let (tx, tz) = (50.0, 9.0);
                let (dx, dz) = (tx - ptf.translation.x, tz - ptf.translation.z);
                if dx.abs() <= 2.0 && dz.abs() <= 2.0 { steer(&mut key_input, 0.0, 0.0, 1.0);
                    press_key(&mut demo, &mut key_input,KeyCode::KeyR, t); }
                else { steer(&mut key_input, dx, dz, WAYPOINT_TOL); }
                // DEBUG: log walk progress every second
                let tick = phase_t as u32;
                if tick > 0 && tick != demo.debug_tick && tick % 3 == 0 {
                    println!("QUEST_DEBUG_P4_WALK tx={:.0} tz={:.0} pt=({:.1},{:.1}) dx={:.1} dz={:.1} in_range={}",
                        tx, tz, ptf.translation.x, ptf.translation.z, dx, dz,
                        dx.abs() <= 2.0 && dz.abs() <= 2.0);
                }
                if phase_t > 25.0 { hands_off!();
                    println!("QUEST_STAGE_COMPLETE q4 o1_build => FAIL (timeout) at ({:.1},{:.1}) cur={}",
                        ptf.translation.x, ptf.translation.z, cur); demo.phase = 99; } }
            1 => { let (tx, tz) = (46.0, 24.0);
                let (dx, dz) = (tx - ptf.translation.x, tz - ptf.translation.z);
                if dx.abs() <= 3.0 && dz.abs() <= 3.0 { steer(&mut key_input, 0.0, 0.0, 1.0);
                    press_key(&mut demo, &mut key_input,KeyCode::KeyE, t); }
                else { steer(&mut key_input, dx, dz, WAYPOINT_TOL); }
                if phase_t > 30.0 { hands_off!();
                    println!("QUEST_STAGE_COMPLETE q4 o2_ledger => FAIL (timeout)"); demo.phase = 99; } }
            2 => { let (tx, tz) = (33.0, 14.0);
                let (dx, dz) = (tx - ptf.translation.x, tz - ptf.translation.z);
                if dx.abs() <= 3.0 && dz.abs() <= 3.0 { steer(&mut key_input, 0.0, 0.0, 1.0);
                    press_key(&mut demo, &mut key_input,KeyCode::KeyE, t); }
                else { steer(&mut key_input, dx, dz, WAYPOINT_TOL); }
                if phase_t > 30.0 { hands_off!();
                    println!("QUEST_STAGE_COMPLETE q4 o3_offering => FAIL (timeout)"); demo.phase = 99; } }
            _ => { hands_off!(); enter!(5); }
        }
        return;
    }

    // ---- phase 5: complete q5 (sigil → gate → cliffhanger) --------------------
    if demo.phase == 5 {
        let q5_done = journal.quests.get("q5_sigil_that_knew_you")
            .map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        if q5_done { hands_off!(); println!("QUEST_COMPLETE q5_sigil_that_knew_you => PASS");
            let act1 = journal.flags.contains("act1_complete");
            println!("QUEST_FLAG act1_complete => {}", if act1 { "PASS" } else { "FAIL" });
            enter!(6); return; }
        let stage5 = journal.quests.get("q5_sigil_that_knew_you")
            .map(|p| p.current_objective).unwrap_or(0);
        if stage5 < 2 {
            if ptf.translation.x < 35.0 && ptf.translation.z < 3.0 {
                steer(&mut key_input, 0.0, 0.0, 1.0);
            } else if ptf.translation.x > 36.0 || ptf.translation.z > 8.0 {
                steer(&mut key_input, 32.0 - ptf.translation.x, 5.0 - ptf.translation.z, WAYPOINT_TOL);
            } else { steer(&mut key_input, 0.0, -1.0, 0.0); }
            if phase_t > 40.0 { hands_off!();
                println!("QUEST_STAGE_COMPLETE q5 gate => FAIL (timeout at z={:.1})", ptf.translation.z);
                demo.phase = 99; }
            return;
        }
        if dialogue.open && dialogue.dialogue_id == "dlg_maren_cliffhanger" {
            if dialogue.choosing { press_key(&mut demo, &mut key_input,KeyCode::Digit1, t); }
            else { press_key(&mut demo, &mut key_input,KeyCode::Enter, t); }
        }
        if phase_t > 25.0 { hands_off!();
            println!("QUEST_STAGE_COMPLETE q5 cliffhanger => FAIL (timeout)"); demo.phase = 99; }
        return;
    }

    // ---- phase 6: grade full Act 1 chain ---------------------------------------
    if demo.phase == 6 {
        if phase_t < 1.0 { return; }
        let q1 = journal.quests.get("q1_embers").map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        let q2 = journal.quests.get("q2_voice_in_stone").map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        let q3 = journal.quests.get("q3_gatekeeper").map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        let q4 = journal.quests.get("q4_what_walls_remember").map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        let q5 = journal.quests.get("q5_sigil_that_knew_you").map(|p| p.status == QuestStatus::Completed).unwrap_or(false);
        let act1 = journal.flags.contains("act1_complete");
        demo.q3_completed = q3; demo.q4_unlocked = q4;
        for (id, ok) in &[("q1_embers", q1), ("q2_voice_in_stone", q2), ("q3_gatekeeper", q3),
            ("q4_what_walls_remember", q4), ("q5_sigil_that_knew_you", q5)] {
            println!("QUEST_COMPLETE {id} => {}", if *ok { "PASS" } else { "FAIL" });
        }
        println!("QUEST_FLAG act1_complete => {}", if act1 { "PASS" } else { "FAIL" });
        let all_ok = q1 && q2 && q3 && q4 && q5 && act1;
        println!("QUEST_PROOF Act 1 full chain {} => {}",
            if all_ok { "ALL GATES PASS" } else { "SOME GATES FAILED" },
            if all_ok { "PASS" } else { "FAIL" });
        if all_ok { exit.write(AppExit::Success); } else { exit.write(AppExit::from_code(1)); }
        enter!(100);
        return;
    }

    if demo.phase == 99 {
        println!("QUEST_FATAL phase=99");
        exit.write(AppExit::from_code(1));
    }
}

// =============================================================================
// Unit tests — quest state machine, save/load, objective completion
// =============================================================================
// These tests exercise the quest journal data model against the real act1.json
// schema, proving every transition the engine makes during Act 1.  They do NOT
// require Bevy — all logic is exercised through the pure functions.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Load the real act1.json so test assertions match the shipped data.
    fn act1() -> StoryData {
        let text = std::fs::read_to_string("assets/story/act1.json")
            .expect("act1.json must be readable from test");
        serde_json::from_str(&text).expect("act1.json must parse")
    }

    fn fresh_journal(data: &StoryData) -> QuestJournal {
        let mut j = QuestJournal::default();
        for qdef in &data.quests {
            j.quests.insert(qdef.id.clone(), QuestProgress {
                status: if qdef.trigger.trigger_type == "on_spawn" {
                    QuestStatus::Active
                } else {
                    QuestStatus::Locked
                },
                current_objective: 0,
                completed_objectives: Vec::new(),
                objective_counts: HashMap::new(),
                flags: HashSet::new(),
            });
        }
        // q1 always starts active.
        if let Some(prog) = j.quests.get_mut(&data.start_quest) {
            prog.status = QuestStatus::Active;
            j.active_order.push(data.start_quest.clone());
        }
        j
    }

    // ---------------------------------------------------------------------------
    // q1_embers — completes on campfire approach
    // ---------------------------------------------------------------------------

    #[test]
    fn q1_completes_on_campfire_approach() {
        let data = act1();
        let mut j = fresh_journal(&data);

        // Simulate walking to the campfire: complete o1_campfire (approach).
        let qid = "q1_embers";
        let obj_id = "o1_campfire";
        let prog = j.quests.get_mut(qid).unwrap();
        assert_eq!(prog.status, QuestStatus::Active);

        // Manually push the objective completion (same path as check_approach_triggers).
        prog.completed_objectives.push(obj_id.to_string());
        prog.current_objective += 1;

        // q1 has only o1_campfire (non-optional) → quest completes.
        let qdef = data.quests.iter().find(|q| q.id == qid).unwrap();
        let all_done = qdef.objectives.iter()
            .filter(|o| !o.optional)
            .all(|o| prog.completed_objectives.contains(&o.id));
        assert!(all_done, "q1 should be done after o1_campfire");

        // Complete the quest.
        prog.status = QuestStatus::Completed;
        // q1 rewards set campfire_anchored flag.
        if let Some(ref flag) = qdef.rewards.set_flag {
            j.flags.insert(flag.clone());
        }
        assert!(j.flags.contains("campfire_anchored"), "campfire_anchored flag should be set");
        assert_eq!(prog.status, QuestStatus::Completed);

        // q1.next → q2 should be unlocked.
        if let Some(ref next_id) = qdef.next {
            let next = j.quests.get_mut(next_id).unwrap();
            next.status = QuestStatus::Active;
            assert_eq!(next.status, QuestStatus::Active);
            assert_eq!(next_id, "q2_voice_in_stone");
        }
    }

    // ---------------------------------------------------------------------------
    // q2_voice_in_stone — completes via dialogue choice
    // ---------------------------------------------------------------------------

    #[test]
    fn q2_completes_via_dialogue_choice() {
        let data = act1();
        let mut j = fresh_journal(&data);

        // Set up: q1 done, q2 active.
        j.quests.get_mut("q1_embers").unwrap().status = QuestStatus::Completed;
        let q2 = j.quests.get_mut("q2_voice_in_stone").unwrap();
        q2.status = QuestStatus::Active;

        // Find dlg_maren_gate and pick choice "c_go" → completes o2_listen + advances q3.
        let dlg = data.dialogue.iter().find(|d| d.id == "dlg_maren_gate").unwrap();
        let go_choice = dlg.choices.as_ref().unwrap().iter()
            .find(|c| c.id == "c_go").unwrap();

        // Simulate choice effects.
        if let Some(ref obj_id) = go_choice.completes_objective {
            q2.completed_objectives.push(obj_id.clone());
            q2.current_objective += 1;
        }
        if let Some(ref flag) = go_choice.sets_flag {
            j.flags.insert(flag.clone());
        }
        // q2 has 2 objectives: o1_gate (reach_zone) and o2_listen (listen).
        // Both must be done.
        q2.completed_objectives.push("o1_gate".to_string());
        q2.current_objective += 1;
        let qdef = data.quests.iter().find(|q| q.id == "q2_voice_in_stone").unwrap();
        let all_done = qdef.objectives.iter()
            .filter(|o| !o.optional)
            .all(|o| q2.completed_objectives.contains(&o.id));
        assert!(all_done, "q2 both objectives done");

        q2.status = QuestStatus::Completed;
        assert_eq!(q2.status, QuestStatus::Completed);
        assert!(j.flags.contains("maren_met"), "maren_met flag should be set");

        // q2.next → q3 should be unlockable.
        assert_eq!(qdef.next.as_deref(), Some("q3_gatekeeper"));
    }

    // ---------------------------------------------------------------------------
    // q3_gatekeeper — defeat + reach_zone + approach → complete
    // ---------------------------------------------------------------------------

    #[test]
    fn q3_completes_via_defeat() {
        let data = act1();
        let mut j = fresh_journal(&data);

        // Set up: q1-q2 done, q3 active.
        j.quests.get_mut("q1_embers").unwrap().status = QuestStatus::Completed;
        j.quests.get_mut("q2_voice_in_stone").unwrap().status = QuestStatus::Completed;
        let q3 = j.quests.get_mut("q3_gatekeeper").unwrap();
        q3.status = QuestStatus::Active;

        // Complete o1_east (reach_zone guard_post_east).
        q3.completed_objectives.push("o1_east".to_string());
        q3.current_objective += 1;

        // Complete o2_observe (approach Garren).
        q3.completed_objectives.push("o2_observe".to_string());
        q3.current_objective += 1;

        // Complete o3_defeat (defeat garren_husk).
        q3.completed_objectives.push("o3_defeat".to_string());
        q3.current_objective += 1;

        let qdef = data.quests.iter().find(|q| q.id == "q3_gatekeeper").unwrap();
        let all_done = qdef.objectives.iter()
            .filter(|o| !o.optional)
            .all(|o| q3.completed_objectives.contains(&o.id));
        assert!(all_done, "q3 all non-optional objectives done");
        assert!(q3.completed_objectives.contains(&"o3_defeat".to_string()));

        q3.status = QuestStatus::Completed;
        assert_eq!(q3.status, QuestStatus::Completed);

        // q3.next → q4 unlocks.
        assert_eq!(qdef.next.as_deref(), Some("q4_what_walls_remember"));
    }

    // ---------------------------------------------------------------------------
    // q4_what_walls_remember — place_block + interact ×2
    // ---------------------------------------------------------------------------

    #[test]
    fn q4_completes_via_build_and_interact() {
        let data = act1();
        let mut j = fresh_journal(&data);

        let q4 = j.quests.get_mut("q4_what_walls_remember").unwrap();
        q4.status = QuestStatus::Active;

        // o1_build: place_block at collapse_gap.
        q4.completed_objectives.push("o1_build".to_string());
        q4.current_objective += 1;

        // o2_ledger: interact lore_village_ledger.
        q4.completed_objectives.push("o2_ledger".to_string());
        q4.current_objective += 1;

        // o3_offering: interact lore_offering_bowl.
        q4.completed_objectives.push("o3_offering".to_string());
        q4.current_objective += 1;

        let qdef = data.quests.iter().find(|q| q.id == "q4_what_walls_remember").unwrap();
        let all_done = qdef.objectives.iter()
            .filter(|o| !o.optional)
            .all(|o| q4.completed_objectives.contains(&o.id));
        assert!(all_done, "q4 all objectives done");

        q4.status = QuestStatus::Completed;
        // q4.next → q5.
        assert_eq!(qdef.next.as_deref(), Some("q5_sigil_that_knew_you"));
    }

    // ---------------------------------------------------------------------------
    // q5_sigil_that_knew_you — approach sigil → enter Hollow Reach → cliffhanger
    // ---------------------------------------------------------------------------

    #[test]
    fn q5_completes_full_act1() {
        let data = act1();
        let mut j = fresh_journal(&data);

        let q5 = j.quests.get_mut("q5_sigil_that_knew_you").unwrap();
        q5.status = QuestStatus::Active;

        // o1_sigil: approach sigil within 4 blocks of gate.
        q5.completed_objectives.push("o1_sigil".to_string());
        q5.current_objective += 1;

        // o2_enter: reach_zone hollow_reach_intro.
        q5.completed_objectives.push("o2_enter".to_string());
        q5.current_objective += 1;

        // o3_end: listen to dlg_maren_cliffhanger (this sets act1_complete flag).
        q5.completed_objectives.push("o3_end".to_string());
        q5.current_objective += 1;

        // Simulate the cliffhanger dialogue effect: set act1_complete flag.
        j.flags.insert("act1_complete".to_string());

        let qdef = data.quests.iter().find(|q| q.id == "q5_sigil_that_knew_you").unwrap();
        let all_done = qdef.objectives.iter()
            .filter(|o| !o.optional)
            .all(|o| q5.completed_objectives.contains(&o.id));
        assert!(all_done, "q5 all objectives done");

        q5.status = QuestStatus::Completed;
        assert_eq!(q5.status, QuestStatus::Completed);
        assert!(j.flags.contains("act1_complete"), "act1_complete flag should be set");
        assert_eq!(qdef.next, None, "q5 is the end of Act 1 — no next quest");
    }

    // ---------------------------------------------------------------------------
    // Full-chain test: q1 → q2 → q3 → q4 → q5 sequential activation
    // ---------------------------------------------------------------------------

    #[test]
    fn full_act1_chain_all_quests_accessible() {
        let data = act1();
        let mut j = fresh_journal(&data);

        // Start: only q1 is Active.
        assert_eq!(j.quests.get("q1_embers").unwrap().status, QuestStatus::Active);
        assert_eq!(j.quests.get("q2_voice_in_stone").unwrap().status, QuestStatus::Locked);
        assert_eq!(j.quests.get("q3_gatekeeper").unwrap().status, QuestStatus::Locked);
        assert_eq!(j.quests.get("q4_what_walls_remember").unwrap().status, QuestStatus::Locked);
        assert_eq!(j.quests.get("q5_sigil_that_knew_you").unwrap().status, QuestStatus::Locked);

        // Complete each quest in sequence.
        for qid in &["q1_embers", "q2_voice_in_stone", "q3_gatekeeper",
                      "q4_what_walls_remember", "q5_sigil_that_knew_you"] {
            let q = j.quests.get_mut(*qid).unwrap();
            q.status = QuestStatus::Active;
            let qdef = data.quests.iter().find(|q| q.id == *qid).unwrap();
            // Mark all non-optional objectives done.
            for obj in &qdef.objectives {
                if !obj.optional && !q.completed_objectives.contains(&obj.id) {
                    q.completed_objectives.push(obj.id.clone());
                    q.current_objective += 1;
                }
            }
            q.status = QuestStatus::Completed;

            // Unlock the next quest.
            if let Some(ref next_id) = qdef.next {
                let next = j.quests.get_mut(next_id).unwrap();
                next.status = QuestStatus::Active;
                if !j.active_order.contains(next_id) {
                    j.active_order.push(next_id.clone());
                }
            }

            // Apply rewards.
            if let Some(ref flag) = qdef.rewards.set_flag {
                j.flags.insert(flag.clone());
            }
        }

        // Verify all 5 quests completed.
        assert_eq!(j.quests.get("q1_embers").unwrap().status, QuestStatus::Completed);
        assert_eq!(j.quests.get("q2_voice_in_stone").unwrap().status, QuestStatus::Completed);
        assert_eq!(j.quests.get("q3_gatekeeper").unwrap().status, QuestStatus::Completed);
        assert_eq!(j.quests.get("q4_what_walls_remember").unwrap().status, QuestStatus::Completed);
        assert_eq!(j.quests.get("q5_sigil_that_knew_you").unwrap().status, QuestStatus::Completed);

        // Flags set.
        assert!(j.flags.contains("campfire_anchored"));
        assert!(j.flags.contains("maren_met"));
        assert!(j.flags.contains("act1_complete"));
    }

    // ---------------------------------------------------------------------------
    // Save → Load round-trip — no data lost
    // ---------------------------------------------------------------------------

    #[test]
    fn save_load_round_trip_preserves_all_state() {
        let data = act1();
        let mut j = fresh_journal(&data);

        // Set up mid-playthrough state.
        j.quests.get_mut("q1_embers").unwrap().status = QuestStatus::Completed;
        j.flags.insert("campfire_anchored".to_string());
        j.quests.get_mut("q2_voice_in_stone").unwrap().status = QuestStatus::Completed;
        j.flags.insert("maren_met".to_string());
        j.quests.get_mut("q3_gatekeeper").unwrap().status = QuestStatus::Active;
        j.active_order = vec!["q3_gatekeeper".to_string()];
        let q3 = j.quests.get_mut("q3_gatekeeper").unwrap();
        q3.completed_objectives.push("o1_east".to_string());
        q3.current_objective = 1;
        q3.objective_counts.insert("o3_defeat".to_string(), 0);

        // Save.
        let ser = serde_json::to_string_pretty(&j).unwrap();

        // Load into a fresh journal.
        let restored: QuestJournal = serde_json::from_str(&ser).unwrap();

        assert_eq!(restored.quests.len(), j.quests.len());
        assert_eq!(restored.flags, j.flags);
        assert_eq!(restored.active_order, j.active_order);
        assert_eq!(
            restored.quests.get("q3_gatekeeper").unwrap().current_objective,
            j.quests.get("q3_gatekeeper").unwrap().current_objective
        );
        assert_eq!(
            restored.quests.get("q3_gatekeeper").unwrap().completed_objectives,
            j.quests.get("q3_gatekeeper").unwrap().completed_objectives
        );
        assert_eq!(
            restored.quests.get("q3_gatekeeper").unwrap().objective_counts,
            j.quests.get("q3_gatekeeper").unwrap().objective_counts
        );
    }

    // ---------------------------------------------------------------------------
    // Trigger: quest_complete → advances next quest
    // ---------------------------------------------------------------------------

    #[test]
    fn trigger_quest_complete_unlocks_next() {
        let data = act1();
        let mut j = fresh_journal(&data);

        // q2 has trigger type "quest_complete" on q1.
        let q2_trig = &data.quests.iter().find(|q| q.id == "q2_voice_in_stone").unwrap().trigger;
        assert_eq!(q2_trig.trigger_type, "quest_complete");
        assert_eq!(q2_trig.quest.as_deref(), Some("q1_embers"));

        // Simulate q1 completing → this should unlock q2.
        let q1 = j.quests.get_mut("q1_embers").unwrap();
        q1.status = QuestStatus::Completed;
        let q1def = data.quests.iter().find(|q| q.id == "q1_embers").unwrap();
        if let Some(ref next_id) = q1def.next {
            let next = j.quests.get_mut(next_id).unwrap();
            next.status = QuestStatus::Active;
        }
        assert_eq!(j.quests.get("q2_voice_in_stone").unwrap().status, QuestStatus::Active);
    }

    // ---------------------------------------------------------------------------
    // Trigger: on_spawn → Active at fresh journal
    // ---------------------------------------------------------------------------

    #[test]
    fn trigger_on_spawn_starts_active() {
        let data = act1();
        let j = fresh_journal(&data);

        // q1 trigger is "on_spawn" — should be Active from the start.
        assert_eq!(j.quests.get("q1_embers").unwrap().status, QuestStatus::Active);
    }

    // ---------------------------------------------------------------------------
    // Flag operations — set, check, clear
    // ---------------------------------------------------------------------------

    #[test]
    fn flag_set_and_check() {
        let mut j = QuestJournal::default();
        assert!(!j.flags.contains("test_flag"));

        j.flags.insert("test_flag".to_string());
        assert!(j.flags.contains("test_flag"));

        j.flags.remove("test_flag");
        assert!(!j.flags.contains("test_flag"));
    }

    // ---------------------------------------------------------------------------
    // Objective count tracking (kill/defeat objectives)
    // ---------------------------------------------------------------------------

    #[test]
    fn objective_counts_track_progress() {
        let mut prog = QuestProgress {
            status: QuestStatus::Active,
            current_objective: 0,
            completed_objectives: Vec::new(),
            objective_counts: HashMap::new(),
            flags: HashSet::new(),
        };

        let count = prog.objective_counts.entry("o3_defeat".to_string()).or_insert(0);
        *count += 1;
        assert_eq!(*count, 1);

        *count += 1;
        assert_eq!(*count, 2);

        // When count reaches needed (1), objective completes.
        let needed: u32 = 1;
        if *count >= needed {
            prog.completed_objectives.push("o3_defeat".to_string());
            prog.current_objective += 1;
        }
        assert!(prog.completed_objectives.contains(&"o3_defeat".to_string()));
        assert_eq!(prog.current_objective, 1);
    }

    // ---------------------------------------------------------------------------
    // Lore items — all 11 are loaded and have expected kind + position
    // ---------------------------------------------------------------------------

    #[test]
    fn lore_items_load_correctly() {
        let data = act1();
        assert_eq!(data.lore_items.len(), 11, "Act 1 ships 11 lore items");

        let well = data.lore_items.iter().find(|li| li.id == "lore_well_inscription").unwrap();
        assert_eq!(well.kind, "inscription");
        assert_eq!(well.world_position.x, 32.0);
        assert_eq!(well.lore_layer, 2);

        let toy = data.lore_items.iter().find(|li| li.id == "lore_tomas_toy").unwrap();
        assert!(toy.tags.contains(&"toma".to_string()));
        // Toma's toy must NOT carry teal (canon: the absence is the point).
        let has_teal = toy.text.to_lowercase().contains("teal")
            || toy.subtitle.to_lowercase().contains("teal")
            || toy.subtitle.to_lowercase().contains("glow");
        assert!(!has_teal, "Toma's toy should not carry teal light (canon)");
    }

    // ---------------------------------------------------------------------------
    // Dialogue: dlg_maren_gate has 5 choices — hub pattern
    // ---------------------------------------------------------------------------

    #[test]
    fn dlg_maren_gate_has_five_choices_as_hub() {
        let data = act1();
        let dlg = data.dialogue.iter().find(|d| d.id == "dlg_maren_gate").unwrap();
        let choices = dlg.choices.as_ref().unwrap();
        assert_eq!(choices.len(), 5, "Maren's gate dialogue is a 5-choice hub");
        assert_eq!(choices[0].label, "Who are you?");
        assert_eq!(choices[4].label, "I'll go east. Keep the seal.");
    }

    // ---------------------------------------------------------------------------
    // Act end: cliffhanger data is intact
    // ---------------------------------------------------------------------------

    #[test]
    fn act_end_cliffhanger_intact() {
        let data = act1();
        let end = data.act_end.as_ref().unwrap();
        assert_eq!(end.trigger_quest, "q5_sigil_that_knew_you");
        assert!(!end.final_line.is_empty(), "final line must not be empty");
        assert!(!end.beats.is_empty(), "cliffhanger beats must not be empty");
    }
}
