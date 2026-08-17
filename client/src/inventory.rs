//! Inventory — the player's item bag: a fixed-slot list of [`ItemStack`]s plus the
//! hotbar selection, wired to Play-mode dig/place and potion use.
//!
//! ## Keys (all Play-only)
//! **B** dig the aimed block into the bag · **N** place the selected block ·
//! **H** use the selected item (health potion → heal) · **I** toggle the bag ·
//! **1-9** pick a hotbar slot.
//!
//! Dig/place are deliberately NOT on the mouse: Left/Right are combat's attack
//! and block in `combat.rs`, and the editor already owns L/R place/break in
//! Editor state. So Play gets its own keys and shares nothing with combat.
//!
//! The bag + hotbar are drawn in `EguiPrimaryContextPass` (like `main_menu.rs` /
//! `settings_menu.rs`); drawing them anywhere else panics with "no fonts".

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use serde::{Deserialize, Serialize};
use voxelforge_sim::block::BlockId;

use crate::combat::Health;
use crate::editor::AppState;
use crate::main_menu::main_menu_closed;
use crate::settings_menu::settings_closed;
use crate::{get_world_voxel, raycast_voxel, set_world_voxel, FlyCam, OrbitCam, World};

/// Total inventory slots (the bag). The first [`HOTBAR_SLOTS`] are the hotbar.
pub const INVENTORY_SLOTS: usize = 20;
/// Hotbar width — the number of slots shown across the bottom of the screen.
pub const HOTBAR_SLOTS: usize = 9;
/// How much health one potion restores (capped at `Health::max`).
pub const POTION_HEAL: f32 = 40.0;
/// Play-mode build reach in voxels — far shorter than the editor's 200, roughly
/// an arm's length from the third-person camera.
const BUILD_REACH: f32 = 6.0;

// Walnut/espresso palette, the same values `settings_menu::theme` and
// `main_menu.rs` use. `theme` is private there, so this module carries its own
// copy (the codebase's convention: every egui surface owns its constants).
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(30, 20, 15);
const BORDER: egui::Color32 = egui::Color32::from_rgb(58, 39, 22);
const WIDGET_IDLE: egui::Color32 = egui::Color32::from_rgb(58, 39, 22);
const ACCENT_AMBER: egui::Color32 = egui::Color32::from_rgb(244, 184, 96);
const TEXT_CREAM: egui::Color32 = egui::Color32::from_rgb(232, 216, 184);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(178, 160, 138);

// ---------------------------------------------------------------------------
// Item model
// ---------------------------------------------------------------------------

/// What a stack holds. Blocks are bound to `sim/src/block.rs`; the two item kinds
/// are gameplay items (a consumable and a quest key) that have no voxel form.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ItemKind {
    /// A placeable world block, indexed by its [`BlockId`].
    Block(BlockId),
    /// Restores [`POTION_HEAL`] health when used.
    HealthPotion,
    /// A quest key item, identified by its quest/objective id.
    QuestKey(String),
}

impl ItemKind {
    /// Human-readable label for the hotbar / bag. Blocks borrow [`BlockId::name`].
    pub fn name(&self) -> String {
        match self {
            ItemKind::Block(b) => b.name().to_string(),
            ItemKind::HealthPotion => "health potion".to_string(),
            ItemKind::QuestKey(id) => format!("quest key: {id}"),
        }
    }

    /// How many of this item fit in a single stack.
    pub fn max_stack(&self) -> u32 {
        match self {
            ItemKind::Block(_) => 999,
            ItemKind::HealthPotion => 16,
            ItemKind::QuestKey(_) => 1,
        }
    }

    /// The block this item places, if it is a block.
    pub fn as_block(&self) -> Option<BlockId> {
        match self {
            ItemKind::Block(b) => Some(*b),
            _ => None,
        }
    }

    /// Serialise to the string wire format. `sim` has no serde (and `BlockId`
    /// doesn't derive it), so `ItemKind` serialises by hand instead of forcing a
    /// serde dependency into the dependency-free sim crate.
    fn to_wire(&self) -> String {
        match self {
            ItemKind::Block(b) => format!("block:{}", b.0),
            ItemKind::HealthPotion => "health_potion".to_string(),
            ItemKind::QuestKey(id) => format!("quest_key:{id}"),
        }
    }

    fn from_wire(wire: &str) -> Option<Self> {
        if let Some(rest) = wire.strip_prefix("block:") {
            return rest.parse::<u8>().ok().map(|id| ItemKind::Block(BlockId(id)));
        }
        if wire == "health_potion" {
            return Some(ItemKind::HealthPotion);
        }
        if let Some(id) = wire.strip_prefix("quest_key:") {
            return Some(ItemKind::QuestKey(id.to_string()));
        }
        None
    }
}

impl Serialize for ItemKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_wire())
    }
}

impl<'de> Deserialize<'de> for ItemKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = String::deserialize(d)?;
        Self::from_wire(&wire)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown item kind `{wire}`")))
    }
}

/// A stack of identical items in one slot.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ItemStack {
    pub kind: ItemKind,
    pub count: u32,
}

/// The player's bag. A fixed-size `Vec` so slot indices are stable; `None` is an
/// empty slot. `selected` is the hotbar index into the first [`HOTBAR_SLOTS`].
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub selected: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: vec![None; INVENTORY_SLOTS],
            selected: 0,
        }
    }
}

impl Inventory {
    /// Reconcile the slot vector with [`INVENTORY_SLOTS`] and clamp `selected`.
    /// A save from an older build (fewer slots, or a missing inventory field)
    /// must not leave the UI indexing out of bounds.
    pub fn normalize(&mut self) {
        self.slots.resize(INVENTORY_SLOTS, None);
        if self.slots.len() > INVENTORY_SLOTS {
            self.slots.truncate(INVENTORY_SLOTS);
        }
        if self.selected >= HOTBAR_SLOTS {
            self.selected = 0;
        }
    }

    /// Add `count` of `kind`, topping up existing partial stacks first, then
    /// filling empty slots. Returns how many could NOT be added (0 = all fit).
    pub fn add(&mut self, kind: ItemKind, count: u32) -> u32 {
        if count == 0 {
            return 0;
        }
        let max = kind.max_stack();
        let mut remaining = count;

        // 1) top up existing partial stacks of the same kind
        for slot in self.slots.iter_mut().flatten() {
            if slot.kind == kind && slot.count < max {
                let take = (max - slot.count).min(remaining);
                slot.count += take;
                remaining -= take;
                if remaining == 0 {
                    return 0;
                }
            }
        }
        // 2) fill empty slots
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                let take = max.min(remaining);
                *slot = Some(ItemStack {
                    kind: kind.clone(),
                    count: take,
                });
                remaining -= take;
                if remaining == 0 {
                    return 0;
                }
            }
        }
        remaining
    }

    /// Remove up to `count` of `kind` across slots. Returns how many were
    /// actually removed (may be < `count` if the bag holds fewer).
    pub fn remove(&mut self, kind: &ItemKind, count: u32) -> u32 {
        let mut remaining = count;
        for slot in self.slots.iter_mut() {
            let Some(stack) = slot else { continue };
            if stack.kind != *kind {
                continue;
            }
            let take = stack.count.min(remaining);
            stack.count -= take;
            remaining -= take;
            if stack.count == 0 {
                *slot = None;
            }
            if remaining == 0 {
                break;
            }
        }
        count - remaining
    }

    /// Total count of `kind` across all slots.
    pub fn count(&self, kind: &ItemKind) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.kind == *kind)
            .map(|s| s.count)
            .sum()
    }

    /// Split `amount` off the stack in slot `from` into another slot that can
    /// take it (a same-kind partial stack, else an empty slot). Returns true when
    /// at least one item moved. The source is never emptied (`amount` < its count).
    ///
    /// Reserved for a split-stack UI gesture (right now the bag is read-only);
    /// the unit tests below are the only callers, so it's allow(dead_code) until
    /// that gesture lands.
    #[allow(dead_code)]
    pub fn split(&mut self, from: usize, amount: u32) -> bool {
        let Some(src) = self.slots.get(from).and_then(|s| s.as_ref()) else {
            return false;
        };
        if amount == 0 || amount >= src.count {
            return false;
        }
        let kind = src.kind.clone();
        let max = kind.max_stack();

        let mut dest = None;
        for (i, slot) in self.slots.iter().enumerate() {
            if i == from {
                continue;
            }
            match slot {
                None => {
                    dest = Some(i);
                    break;
                }
                Some(s) if s.kind == kind && s.count < max => {
                    dest = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let Some(d) = dest else { return false };

        let dest_room = match &self.slots[d] {
            None => max,
            Some(s) => max - s.count,
        };
        let take = amount.min(dest_room);
        // `take <= amount < src.count`, so the source never empties.
        self.slots[from].as_mut().unwrap().count -= take;
        match &mut self.slots[d] {
            None => self.slots[d] = Some(ItemStack { kind, count: take }),
            Some(s) => s.count += take,
        }
        true
    }

    /// The stack in the selected hotbar slot, if any.
    pub fn selected_stack(&self) -> Option<&ItemStack> {
        self.slots.get(self.selected).and_then(|s| s.as_ref())
    }

    /// Take `count` off the selected slot. Returns the kind removed if the slot
    /// held at least `count`; `None` otherwise (and the slot is left untouched).
    pub fn take_selected(&mut self, count: u32) -> Option<ItemKind> {
        let stack = self.slots.get_mut(self.selected)?.as_mut()?;
        if stack.count < count {
            return None;
        }
        stack.count -= count;
        let kind = stack.kind.clone();
        if stack.count == 0 {
            self.slots[self.selected] = None;
        }
        Some(kind)
    }
}

/// Whether the bag panel is open (I toggles). Public so `main.rs` can gate
/// `fly_camera` off while the bag is up — the cursor is freed then, and letting
/// the camera keep reading the mouse would fight the panel.
#[derive(Resource, Default)]
pub struct InventoryOpen {
    pub open: bool,
}

/// Run condition: the bag panel is closed (so dig/place/use can fire).
pub(crate) fn inventory_closed(open: Res<InventoryOpen>) -> bool {
    !open.open
}

// ---------------------------------------------------------------------------
// Input systems
// ---------------------------------------------------------------------------

fn hotbar_select(keys: Res<ButtonInput<KeyCode>>, mut inv: ResMut<Inventory>) {
    const DIGITS: [KeyCode; HOTBAR_SLOTS] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    for (i, kc) in DIGITS.into_iter().enumerate() {
        if keys.just_pressed(kc) {
            inv.selected = i;
        }
    }
}

fn toggle_bag(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut open: ResMut<InventoryOpen>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::KeyI) {
        return;
    }
    open.open = !open.open;
    let Ok(mut cursor) = cursors.single_mut() else { return };
    if open.open {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if *state.get() == AppState::Play {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Leaving Play closes the bag so a leftover `InventoryOpen` can't gate
/// `fly_camera` off in the Editor (the bag only toggles while in Play).
fn close_bag_on_exit(mut open: ResMut<InventoryOpen>) {
    open.open = false;
}

/// B = dig the aimed block into the bag, N = place the selected block. Both go
/// through the same `raycast_voxel` / `set_world_voxel` loop the editor uses, so
/// a dig/place re-meshes exactly one chunk and respects the same placement rules.
fn dig_place(
    keys: Res<ButtonInput<KeyCode>>,
    cam_q: Query<&Transform, With<OrbitCam>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
    mut inv: ResMut<Inventory>,
) {
    let dig = keys.just_pressed(KeyCode::KeyB);
    let place = keys.just_pressed(KeyCode::KeyN);
    if !dig && !place {
        return;
    }
    let Ok(tf) = cam_q.single() else { return };
    let Some(hit) = raycast_voxel(&world, tf.translation, *tf.forward(), BUILD_REACH) else {
        return;
    };

    if dig {
        // A hit already guarantees a solid block in a loaded chunk, so the write
        // will succeed; only credit the bag when it actually did.
        let block = get_world_voxel(&world, hit.voxel);
        if block == BlockId::AIR {
            return;
        }
        if set_world_voxel(&mut world, hit.voxel, BlockId::AIR, &mut meshes, &mut commands) {
            inv.add(ItemKind::Block(block), 1);
        }
    } else {
        // Place the selected block into the empty cell before the hit. Only
        // consume the item if a chunk accepted it — placing into an unloaded
        // neighbour fails and must not eat the stack.
        let kind = match inv.selected_stack().map(|s| s.kind.clone()) {
            Some(ItemKind::Block(b)) => ItemKind::Block(b),
            _ => return,
        };
        if set_world_voxel(&mut world, hit.prev, kind.as_block().unwrap(), &mut meshes, &mut commands)
        {
            inv.remove(&kind, 1);
        }
    }
}

/// H = use the selected item. Only a health potion is consumable right now; it
/// heals [`POTION_HEAL`] (capped at `Health::max`) and is never wasted at full HP.
fn use_selected(
    keys: Res<ButtonInput<KeyCode>>,
    mut inv: ResMut<Inventory>,
    mut player_q: Query<&mut Health, With<FlyCam>>,
) {
    if !keys.just_pressed(KeyCode::KeyH) {
        return;
    }
    let is_potion = matches!(inv.selected_stack(), Some(s) if s.kind == ItemKind::HealthPotion);
    if !is_potion {
        return;
    }
    let Ok(mut hp) = player_q.single_mut() else { return };
    if hp.cur >= hp.max {
        return;
    }
    if inv.take_selected(1).is_some() {
        hp.cur = (hp.cur + POTION_HEAL).min(hp.max);
    }
}

// ---------------------------------------------------------------------------
// UI (drawn in EguiPrimaryContextPass)
// ---------------------------------------------------------------------------

/// Compact label for a hotbar slot — block names like "cobblestone" would clip a
/// 44px tile, so long names truncate.
fn short_name(kind: &ItemKind) -> String {
    let name = kind.name();
    if name.chars().count() > 8 {
        let mut s: String = name.chars().take(8).collect();
        s.push('…');
        s
    } else {
        name
    }
}

fn hotbar_ui(mut contexts: EguiContexts, inv: Res<Inventory>) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let screen = ctx.content_rect();
    let slot = 44.0;
    let gap = 4.0;
    let total = HOTBAR_SLOTS as f32 * slot + (HOTBAR_SLOTS as f32 - 1.0) * gap;
    let x0 = (screen.width() - total) * 0.5;
    let y0 = screen.height() - slot - 20.0;

    egui::Area::new(egui::Id::new("voxelforge_hotbar"))
        .fixed_pos(egui::pos2(x0, y0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for i in 0..HOTBAR_SLOTS {
                    let (label, count) = match &inv.slots[i] {
                        Some(s) => (short_name(&s.kind), Some(s.count)),
                        None => (String::new(), None),
                    };
                    let selected = i == inv.selected;
                    let fill = if selected { ACCENT_AMBER } else { PANEL_BG };
                    let stroke_w = if selected { 2.0 } else { 1.0 };
                    egui::Frame::NONE
                        .fill(fill)
                        .stroke(egui::Stroke::new(stroke_w, BORDER))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(slot, slot));
                            ui.vertical_centered(|ui| {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(label).size(10.0).color(TEXT_CREAM),
                                );
                                if let Some(c) = count {
                                    ui.label(
                                        egui::RichText::new(format!("{c}"))
                                            .size(10.0)
                                            .color(TEXT_DIM),
                                    );
                                }
                            });
                        });
                    if i + 1 < HOTBAR_SLOTS {
                        ui.add_space(gap);
                    }
                }
            });
        });
}

fn bag_ui(mut contexts: EguiContexts, open: Res<InventoryOpen>, inv: Res<Inventory>) {
    if !open.open {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let frame = egui::Frame::window(&ctx.style_of(ctx.theme()))
        .fill(PANEL_BG)
        .stroke(egui::Stroke::new(1.0, BORDER));
    egui::Window::new("Inventory")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_width(340.0);
            ui.label(
                egui::RichText::new("Backpack — I close · 1-9 hotbar · B dig · N place · H use")
                    .size(12.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(8.0);
            egui::Grid::new("voxelforge_inventory_grid")
                .num_columns(5)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (i, slot) in inv.slots.iter().enumerate() {
                        let selected = i == inv.selected;
                        let fill = if selected { ACCENT_AMBER } else { WIDGET_IDLE };
                        let (label, count) = match slot {
                            Some(s) => (s.kind.name(), format!("{}", s.count)),
                            None => ("—".to_string(), String::new()),
                        };
                        egui::Frame::NONE
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                if selected { 2.0 } else { 1.0 },
                                BORDER,
                            ))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(60.0, 42.0));
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new(label).size(11.0).color(TEXT_CREAM),
                                    );
                                    if !count.is_empty() {
                                        ui.label(
                                            egui::RichText::new(count)
                                                .size(11.0)
                                                .color(TEXT_DIM),
                                        );
                                    }
                                });
                            });
                        if (i + 1) % 5 == 0 {
                            ui.end_row();
                        }
                    }
                });
            // Footer: how much of the selected kind the bag holds in total.
            if let Some(s) = inv.selected_stack() {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("{} — {} total", s.kind.name(), inv.count(&s.kind)))
                        .size(12.0)
                        .color(TEXT_DIM),
                );
            }
        });
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        // Same idempotent guard as main_menu/settings — self-sufficient regardless
        // of plugin order.
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }
        app.init_resource::<Inventory>()
            .init_resource::<InventoryOpen>()
            .add_systems(OnExit(AppState::Play), close_bag_on_exit)
            // Slot select + bag toggle keep working while the bag is open (that's
            // how you close it again), so they only gate on Play + menus.
            .add_systems(
                Update,
                (hotbar_select, toggle_bag)
                    .run_if(in_state(AppState::Play))
                    .run_if(main_menu_closed)
                    .run_if(settings_closed),
            )
            // World edits and item use only make sense with the bag closed (the
            // cursor is free then, and B/N/H must not fire while clicking the panel).
            .add_systems(
                Update,
                (dig_place, use_selected)
                    .run_if(in_state(AppState::Play))
                    .run_if(main_menu_closed)
                    .run_if(settings_closed)
                    .run_if(inventory_closed),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (hotbar_ui, bag_ui)
                    .run_if(in_state(AppState::Play))
                    .run_if(main_menu_closed),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(id: u8) -> ItemKind {
        ItemKind::Block(BlockId(id))
    }

    #[test]
    fn add_stacks_onto_partial_then_new_slots() {
        let mut inv = Inventory::default();
        assert_eq!(inv.add(block(1), 5), 0);
        assert_eq!(inv.add(block(1), 5), 0);
        assert_eq!(inv.count(&block(1)), 10);
        // one slot holds both, no second slot consumed
        assert_eq!(inv.slots[0], Some(ItemStack { kind: block(1), count: 10 }));
        assert!(inv.slots[1].is_none());
    }

    #[test]
    fn add_respects_max_stack_and_reports_leftover() {
        let mut inv = Inventory::default();
        // blocks stack to 999
        let leftover = inv.add(block(1), 999 + 5);
        assert_eq!(leftover, 5);
        assert_eq!(inv.count(&block(1)), 999);
    }

    #[test]
    fn add_full_inventory_rejects_rest() {
        let mut inv = Inventory::default();
        let per_slot = ItemKind::HealthPotion.max_stack(); // 16
        let total = per_slot * INVENTORY_SLOTS as u32;
        assert_eq!(inv.add(ItemKind::HealthPotion, total), 0);
        assert_eq!(inv.count(&ItemKind::HealthPotion), total);
        assert_eq!(inv.add(ItemKind::HealthPotion, 1), 1);
    }

    #[test]
    fn remove_takes_across_slots_and_reports_removed() {
        let mut inv = Inventory::default();
        inv.add(block(2), 999);
        inv.add(block(2), 3); // second slot
        let removed = inv.remove(&block(2), 1000);
        assert_eq!(removed, 1000);
        assert_eq!(inv.count(&block(2)), 2);
    }

    #[test]
    fn split_moves_to_new_slot_and_never_empties_source() {
        let mut inv = Inventory::default();
        inv.add(block(1), 20);
        assert!(inv.split(0, 7));
        assert_eq!(inv.slots[0].as_ref().unwrap().count, 13);
        assert_eq!(inv.slots[1].as_ref().unwrap().count, 7);
    }

    #[test]
    fn split_rejects_zero_too_large_or_empty_source() {
        let mut inv = Inventory::default();
        inv.add(block(1), 5);
        assert!(!inv.split(0, 5)); // amount == count
        assert!(!inv.split(0, 0)); // zero
        assert!(!inv.split(3, 2)); // empty source
        assert_eq!(inv.slots[0].as_ref().unwrap().count, 5);
    }

    #[test]
    fn take_selected_consumes_and_clears_empty() {
        let mut inv = Inventory::default();
        inv.add(ItemKind::HealthPotion, 3); // lands in slot 0
        inv.selected = 0;
        assert_eq!(inv.take_selected(1), Some(ItemKind::HealthPotion));
        assert_eq!(inv.slots[0].as_ref().unwrap().count, 2);
        assert_eq!(inv.take_selected(2), Some(ItemKind::HealthPotion));
        assert!(inv.slots[0].is_none());
    }

    #[test]
    fn item_kind_wire_round_trips() {
        for kind in [
            ItemKind::Block(BlockId::STONE),
            ItemKind::Block(BlockId::GLASS),
            ItemKind::HealthPotion,
            ItemKind::QuestKey("q4_what_walls_remember".to_string()),
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: ItemKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn inventory_save_round_trips_and_normalizes() {
        let mut inv = Inventory::default();
        inv.add(ItemKind::Block(BlockId::DIRT), 7);
        inv.add(ItemKind::HealthPotion, 2);
        inv.selected = 3;
        let json = serde_json::to_string(&inv).unwrap();
        let mut back: Inventory = serde_json::from_str(&json).unwrap();
        back.normalize();
        assert_eq!(back.slots.len(), INVENTORY_SLOTS);
        assert_eq!(back.count(&ItemKind::Block(BlockId::DIRT)), 7);
        assert_eq!(back.count(&ItemKind::HealthPotion), 2);
        assert_eq!(back.selected, 3);
    }
}
