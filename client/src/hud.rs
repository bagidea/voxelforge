//! Combat HUD — Option A "Corner Minimal" (`docs/hud-design.md`).
//!
//! Replaces the plain-text HP/ST readout + generic red/green bars that used to
//! live in `combat::spawn_combat_hud` with the real design: bottom-left HP/
//! stamina bars on an espresso plaque, four-corner lock-on brackets, and a
//! fading top-center quest banner — all in the warm walnut/amber palette
//! (`docs/look-bible.md` §4), zero teal spent.
//!
//! This file is a standalone lane: it only *reads* `combat::{Health, Stamina,
//! LockOn}` and `quest::ObjectiveText`, which are already public resources /
//! components those lanes expose. Nothing here writes to `combat.rs` or
//! `quest.rs`.

use bevy::prelude::*;

use crate::combat::{Health, LockOn, Stamina, STAMINA_MAX};
use crate::quest::ObjectiveText;
use crate::FlyCam;

// ===========================================================================
// Design tokens — docs/hud-design.md §2 (hex values transcribed verbatim)
// ===========================================================================

const PANEL_BG: Color = Color::srgba_u8(0x1A, 0x12, 0x0D, 224); // #1A120D @ ~88%
const PANEL_BORDER: Color = Color::srgb_u8(0x3A, 0x27, 0x16);
const HP_FILL: Color = Color::srgb_u8(0x96, 0x36, 0x2C);
const HP_HILITE: Color = Color::srgba_u8(0xC4, 0x58, 0x3A, 210);
const STAMINA_FILL: Color = Color::srgb_u8(0xE8, 0xA9, 0x4E);
const STAMINA_HILITE: Color = Color::srgba_u8(0xFF, 0xD9, 0x8A, 210);
const TEXT_CREAM: Color = Color::srgb_u8(0xE8, 0xD8, 0xB8);
const ACCENT_AMBER: Color = Color::srgb_u8(0xF4, 0xB8, 0x60);
const SEGMENT_LINE: Color = Color::srgba_u8(0x3A, 0x27, 0x16, 179); // ~70%

// ===========================================================================
// Layout — docs/hud-design.md §3 ("Proposed" column)
// ===========================================================================

const BAR_W: f32 = 220.0;
const BAR_H: f32 = 14.0;
const BAR_GAP: f32 = 4.0;
const LEFT: f32 = 28.0;
const BOTTOM: f32 = 28.0;
const NUM_GAP: f32 = 10.0;

const RETICLE_SIZE: f32 = 40.0;
const RETICLE_TICK_LEN: f32 = 12.0;
const RETICLE_TICK_W: f32 = 2.0;

// docs/hud-design.md §4 ("States")
const IDLE_FADE_DELAY: f32 = 3.0;
const IDLE_OPACITY: f32 = 0.35;
const FADE_LERP_SPEED: f32 = 4.0;
const LOW_HP_FRAC: f32 = 0.25;
const PULSE_PERIOD: f32 = 0.6;

// ===========================================================================
// Components / resources
// ===========================================================================

/// Shared marker on every top-level HUD entity this file spawns, so
/// `main.rs`'s `despawn_encounter` can tear the whole HUD down with one query
/// (each despawn is recursive, so plaque/fill/hilite/ticks all go with it).
#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
struct HpTrack;
#[derive(Component)]
struct HpFill;
#[derive(Component)]
struct HpNumber;

#[derive(Component)]
struct StTrack;
#[derive(Component)]
struct StFill;
#[derive(Component)]
struct StNumber;

#[derive(Component)]
struct LockReticleRoot;
#[derive(Component)]
struct LockBracketPart;

#[derive(Component)]
struct QuestBannerRoot;
#[derive(Component)]
struct QuestBannerText;

/// Idle-fade + low-HP pulse timers (§4). Reset on damage / stamina spend /
/// lock-on so the HUD only fades during genuinely idle play, per the CEO
/// brief's "โผล่เมื่อจำเป็น จางเมื่อไม่ใช้".
#[derive(Resource)]
struct HudTiming {
    idle_t: f32,
    fade: f32,
    pulse_t: f32,
    last_hp: f32,
    last_stam: f32,
}

impl Default for HudTiming {
    fn default() -> Self {
        Self { idle_t: 0.0, fade: 1.0, pulse_t: 0.0, last_hp: 0.0, last_stam: 0.0 }
    }
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudTiming>().add_systems(
            Update,
            (
                hud_timing,
                hud_hp_bar,
                hud_stamina_bar,
                hud_lock_reticle,
                hud_quest_banner,
            )
                .chain()
                .run_if(in_state(crate::editor::AppState::Play)),
        );
    }
}

// ===========================================================================
// Spawn
// ===========================================================================

/// Spawn the Option A HUD. Called once from `main.rs`'s `spawn_encounter`
/// (`OnEnter(AppState::Play)`), same spot the old `combat::spawn_combat_hud`
/// used to be called from.
pub fn spawn_hud(commands: &mut Commands) {
    spawn_bar(
        commands,
        BOTTOM,
        HP_FILL,
        HP_HILITE,
        HpTrack,
        HpFill,
        HpNumber,
        "100",
    );
    spawn_bar(
        commands,
        BOTTOM + BAR_H + BAR_GAP,
        STAMINA_FILL,
        STAMINA_HILITE,
        StTrack,
        StFill,
        StNumber,
        "100",
    );
    spawn_lock_reticle(commands);
    spawn_quest_banner(commands);
}

/// One plaque + fill + hilite + 25/50/75% segment ticks + numeric readout.
/// `bottom` is the plaque's distance from the screen bottom edge.
#[allow(clippy::too_many_arguments)]
fn spawn_bar(
    commands: &mut Commands,
    bottom: f32,
    fill_color: Color,
    hilite_color: Color,
    track_marker: impl Component,
    fill_marker: impl Component,
    number_marker: impl Component,
    initial_number: &str,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(bottom),
                left: Val::Px(LEFT),
                width: Val::Px(BAR_W),
                height: Val::Px(BAR_H),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(PANEL_BORDER),
            HudRoot,
            track_marker,
        ))
        .with_children(|track| {
            track
                .spawn((
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(fill_color),
                    fill_marker,
                ))
                .with_children(|fill| {
                    // 1px top hilite — the "carved inlay" read, not a gloss gradient.
                    fill.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(0.0),
                            left: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            ..default()
                        },
                        BackgroundColor(hilite_color),
                    ));
                });
            // Segment ticks every 25% (a "segmented" soulslike read, not a smooth gauge).
            for pct in [25.0, 50.0, 75.0] {
                track.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Percent(pct),
                        width: Val::Px(1.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(SEGMENT_LINE),
                ));
            }
        });

    commands.spawn((
        Text::new(initial_number),
        TextFont { font_size: bevy::text::FontSize::from(16.0), ..default() },
        TextColor(TEXT_CREAM),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(bottom - 1.0),
            left: Val::Px(LEFT + BAR_W + NUM_GAP),
            ..default()
        },
        HudRoot,
        number_marker,
    ));
}

/// Four-corner lock-on brackets, centred on screen. `lock_on_camera`
/// (combat.rs) already snaps the camera to face the locked target, so a
/// fixed screen-center reticle reads as "on target" without this file
/// needing its own camera → viewport projection — the same simplification
/// the old single-glyph reticle used.
fn spawn_lock_reticle(commands: &mut Commands) {
    let half = RETICLE_SIZE / 2.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(50.0),
                left: Val::Percent(50.0),
                width: Val::Px(RETICLE_SIZE),
                height: Val::Px(RETICLE_SIZE),
                margin: UiRect { left: Val::Px(-half), top: Val::Px(-half), ..default() },
                ..default()
            },
            Visibility::Hidden,
            HudRoot,
            LockReticleRoot,
        ))
        .with_children(|r| {
            for (top, bottom, left, right) in [
                (Some(0.0), None, Some(0.0), None),       // top-left
                (Some(0.0), None, None, Some(0.0)),       // top-right
                (None, Some(0.0), Some(0.0), None),       // bottom-left
                (None, Some(0.0), None, Some(0.0)),       // bottom-right
            ] {
                let mut horiz = Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(RETICLE_TICK_LEN),
                    height: Val::Px(RETICLE_TICK_W),
                    ..default()
                };
                let mut vert = Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(RETICLE_TICK_W),
                    height: Val::Px(RETICLE_TICK_LEN),
                    ..default()
                };
                if let Some(t) = top {
                    horiz.top = Val::Px(t);
                    vert.top = Val::Px(t);
                }
                if let Some(b) = bottom {
                    horiz.bottom = Val::Px(b);
                    vert.bottom = Val::Px(b);
                }
                if let Some(l) = left {
                    horiz.left = Val::Px(l);
                    vert.left = Val::Px(l);
                }
                if let Some(rr) = right {
                    horiz.right = Val::Px(rr);
                    vert.right = Val::Px(rr);
                }
                r.spawn((horiz, BackgroundColor(ACCENT_AMBER), LockBracketPart));
                r.spawn((vert, BackgroundColor(ACCENT_AMBER), LockBracketPart));
            }
        });
}

/// Top-center fading banner for the live quest/objective line. Reads
/// `quest::ObjectiveText` (already public) — only visible while it has a line.
fn spawn_quest_banner(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            HudRoot,
            QuestBannerRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    column_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(PANEL_BORDER),
            ))
            .with_children(|plaque| {
                plaque.spawn((
                    Node { width: Val::Px(4.0), height: Val::Px(14.0), ..default() },
                    BackgroundColor(ACCENT_AMBER),
                ));
                plaque.spawn((
                    Text::new(""),
                    TextFont { font_size: bevy::text::FontSize::from(16.0), ..default() },
                    TextColor(TEXT_CREAM),
                    QuestBannerText,
                ));
            });
        });
}

// ===========================================================================
// Systems
// ===========================================================================

/// Advance the idle-fade + low-HP pulse clocks (§4). Runs before the bar/
/// reticle/banner systems so they all read the same frame's `fade`.
fn hud_timing(
    time: Res<Time>,
    lock: Res<LockOn>,
    player_q: Query<(&Health, &Stamina), With<FlyCam>>,
    mut timing: ResMut<HudTiming>,
) {
    let dt = time.delta_secs();
    timing.pulse_t += dt;

    if let Ok((hp, stam)) = player_q.single() {
        let damaged = hp.cur < timing.last_hp - 0.01;
        let spent = stam.cur < timing.last_stam - 0.01;
        timing.last_hp = hp.cur;
        timing.last_stam = stam.cur;
        if damaged || spent || lock.target.is_some() {
            timing.idle_t = 0.0;
        } else {
            timing.idle_t += dt;
        }
    }

    let target = if timing.idle_t >= IDLE_FADE_DELAY { IDLE_OPACITY } else { 1.0 };
    let t = (dt * FADE_LERP_SPEED).min(1.0);
    timing.fade += (target - timing.fade) * t;
}

fn hud_hp_bar(
    timing: Res<HudTiming>,
    player_q: Query<&Health, With<FlyCam>>,
    mut track_q: Query<&mut BackgroundColor, (With<HpTrack>, Without<HpFill>)>,
    mut fill_q: Query<(&mut Node, &mut BackgroundColor), (With<HpFill>, Without<HpTrack>)>,
    mut num_q: Query<(&mut Text, &mut TextColor), With<HpNumber>>,
) {
    let Ok(hp) = player_q.single() else { return };
    let frac = (hp.cur / hp.max).clamp(0.0, 1.0);

    // Low-HP pulse toward accent-amber and back over ~0.6s (§4), in-palette
    // instead of an off-brand red-alert flash.
    let color = if frac <= LOW_HP_FRAC {
        let s = (timing.pulse_t / PULSE_PERIOD * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        HP_FILL.mix(&ACCENT_AMBER, s)
    } else {
        HP_FILL
    };

    if let Ok(mut bg) = track_q.single_mut() {
        bg.0 = PANEL_BG.with_alpha(PANEL_BG.alpha() * timing.fade);
    }
    if let Ok((mut node, mut bg)) = fill_q.single_mut() {
        node.width = Val::Percent(frac * 100.0);
        bg.0 = color.with_alpha(timing.fade);
    }
    if let Ok((mut text, mut tc)) = num_q.single_mut() {
        text.0 = format!("{:.0}", hp.cur.round());
        tc.0 = TEXT_CREAM.with_alpha(timing.fade);
    }
}

fn hud_stamina_bar(
    timing: Res<HudTiming>,
    player_q: Query<&Stamina, With<FlyCam>>,
    mut track_q: Query<&mut BorderColor, (With<StTrack>, Without<StFill>)>,
    mut track_bg_q: Query<&mut BackgroundColor, (With<StTrack>, Without<StFill>)>,
    mut fill_q: Query<(&mut Node, &mut BackgroundColor), (With<StFill>, Without<StTrack>)>,
    mut num_q: Query<(&mut Text, &mut TextColor), With<StNumber>>,
) {
    let Ok(stam) = player_q.single() else { return };
    let frac = (stam.cur / STAMINA_MAX).clamp(0.0, 1.0);

    // Exhausted (§4, EXHAUST_LOCK / combat-tuning.md §1): border reads
    // accent-amber for as long as the lock-out lasts.
    if let Ok(mut border) = track_q.single_mut() {
        let c = if stam.exhausted > 0.0 { ACCENT_AMBER } else { PANEL_BORDER };
        *border = BorderColor::all(c.with_alpha(c.alpha() * timing.fade));
    }
    if let Ok(mut bg) = track_bg_q.single_mut() {
        bg.0 = PANEL_BG.with_alpha(PANEL_BG.alpha() * timing.fade);
    }
    if let Ok((mut node, mut bg)) = fill_q.single_mut() {
        node.width = Val::Percent(frac * 100.0);
        bg.0 = STAMINA_FILL.with_alpha(timing.fade);
    }
    if let Ok((mut text, mut tc)) = num_q.single_mut() {
        text.0 = format!("{:.0}", stam.cur.round());
        tc.0 = TEXT_CREAM.with_alpha(timing.fade);
    }
}

/// Toggle the four-corner brackets on `LockOn.target` (combat.rs, already
/// public). Locked is itself an "Active" trigger (see `hud_timing`), so the
/// brackets are always at full opacity whenever they're visible — no fade
/// wiring needed here.
fn hud_lock_reticle(
    lock: Res<LockOn>,
    mut root_q: Query<&mut Visibility, With<LockReticleRoot>>,
) {
    let Ok(mut vis) = root_q.single_mut() else { return };
    *vis = if lock.target.is_some() { Visibility::Visible } else { Visibility::Hidden };
}

/// Show the top-center banner only while `quest::ObjectiveText` has a line —
/// "only exists while a line is live" (§1, Option A).
fn hud_quest_banner(
    objective: Res<ObjectiveText>,
    mut root_q: Query<&mut Visibility, With<QuestBannerRoot>>,
    mut text_q: Query<&mut Text, With<QuestBannerText>>,
) {
    let Ok(mut vis) = root_q.single_mut() else { return };
    let live = objective.lines.first().is_some_and(|l| !l.is_empty());
    *vis = if live { Visibility::Visible } else { Visibility::Hidden };
    if live {
        if let Ok(mut text) = text_q.single_mut() {
            text.0 = objective.lines[0].clone();
        }
    }
}
