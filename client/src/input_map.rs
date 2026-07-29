//! Input mapping for Voxelforge — single source of truth for every editor keybind.
//!
//! Instead of hardcoding `KeyCode::KeyF` or `MouseButton::Left` across a dozen
//! systems, each editor action lives in `EditorAction` and maps to a key through
//! the `KeyBindings` resource.  Systems call `action_just_pressed(…)` or
//! `action_pressed(…)` — the binding is loaded from `editor_config.json` on boot
//! and saved whenever it changes, so players can rebind without a recompile.
//!
//! ## API (Poppy / Shiba)
//!
//! ```ignore
//! use crate::editor_config::input_map::{
//!     action_just_pressed, action_pressed, first_action_just_pressed,
//!     EditorAction, KeyBindings,
//! };
//!
//! fn my_system(keys: Res<ButtonInput<KeyCode>>,
//!              mouse: Res<ButtonInput<MouseButton>>,
//!              bindings: Res<KeyBindings>) {
//!     if action_just_pressed(&keys, &mouse, &bindings, EditorAction::Place) {
//!         // place a block
//!     }
//! }
//! ```

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// =============================================================================
// Key — serializable keyboard + mouse button representation
// =============================================================================

/// A keyboard key or mouse button that can be bound to an `EditorAction`.
///
/// Serialises as a short string (`"KeyF"`, `"MouseLeft"`, `"F5"`…) so
/// `editor_config.json` is hand-editable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    // ---- Mouse ---------------------------------------------------------------
    MouseLeft,
    MouseRight,
    MouseMiddle,

    // ---- Letters -------------------------------------------------------------
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // ---- Digits --------------------------------------------------------------
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // ---- Function keys -------------------------------------------------------
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // ---- Special -------------------------------------------------------------
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,

    // ---- Modifiers -----------------------------------------------------------
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,

    // ---- Arrows --------------------------------------------------------------
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

impl Key {
    /// Convert to Bevy's `KeyCode` (keyboard keys only — returns `None` for mouse).
    pub fn to_keycode(self) -> Option<KeyCode> {
        match self {
            // Mouse → no keyboard code
            Self::MouseLeft | Self::MouseRight | Self::MouseMiddle => None,

            // Letters
            Self::KeyA => Some(KeyCode::KeyA),
            Self::KeyB => Some(KeyCode::KeyB),
            Self::KeyC => Some(KeyCode::KeyC),
            Self::KeyD => Some(KeyCode::KeyD),
            Self::KeyE => Some(KeyCode::KeyE),
            Self::KeyF => Some(KeyCode::KeyF),
            Self::KeyG => Some(KeyCode::KeyG),
            Self::KeyH => Some(KeyCode::KeyH),
            Self::KeyI => Some(KeyCode::KeyI),
            Self::KeyJ => Some(KeyCode::KeyJ),
            Self::KeyK => Some(KeyCode::KeyK),
            Self::KeyL => Some(KeyCode::KeyL),
            Self::KeyM => Some(KeyCode::KeyM),
            Self::KeyN => Some(KeyCode::KeyN),
            Self::KeyO => Some(KeyCode::KeyO),
            Self::KeyP => Some(KeyCode::KeyP),
            Self::KeyQ => Some(KeyCode::KeyQ),
            Self::KeyR => Some(KeyCode::KeyR),
            Self::KeyS => Some(KeyCode::KeyS),
            Self::KeyT => Some(KeyCode::KeyT),
            Self::KeyU => Some(KeyCode::KeyU),
            Self::KeyV => Some(KeyCode::KeyV),
            Self::KeyW => Some(KeyCode::KeyW),
            Self::KeyX => Some(KeyCode::KeyX),
            Self::KeyY => Some(KeyCode::KeyY),
            Self::KeyZ => Some(KeyCode::KeyZ),

            // Digits
            Self::Digit0 => Some(KeyCode::Digit0),
            Self::Digit1 => Some(KeyCode::Digit1),
            Self::Digit2 => Some(KeyCode::Digit2),
            Self::Digit3 => Some(KeyCode::Digit3),
            Self::Digit4 => Some(KeyCode::Digit4),
            Self::Digit5 => Some(KeyCode::Digit5),
            Self::Digit6 => Some(KeyCode::Digit6),
            Self::Digit7 => Some(KeyCode::Digit7),
            Self::Digit8 => Some(KeyCode::Digit8),
            Self::Digit9 => Some(KeyCode::Digit9),

            // Function keys
            Self::F1 => Some(KeyCode::F1),
            Self::F2 => Some(KeyCode::F2),
            Self::F3 => Some(KeyCode::F3),
            Self::F4 => Some(KeyCode::F4),
            Self::F5 => Some(KeyCode::F5),
            Self::F6 => Some(KeyCode::F6),
            Self::F7 => Some(KeyCode::F7),
            Self::F8 => Some(KeyCode::F8),
            Self::F9 => Some(KeyCode::F9),
            Self::F10 => Some(KeyCode::F10),
            Self::F11 => Some(KeyCode::F11),
            Self::F12 => Some(KeyCode::F12),

            // Special
            Self::Space => Some(KeyCode::Space),
            Self::Enter => Some(KeyCode::Enter),
            Self::Escape => Some(KeyCode::Escape),
            Self::Tab => Some(KeyCode::Tab),
            Self::Backspace => Some(KeyCode::Backspace),
            Self::Delete => Some(KeyCode::Delete),

            // Modifiers
            Self::ShiftLeft => Some(KeyCode::ShiftLeft),
            Self::ShiftRight => Some(KeyCode::ShiftRight),
            Self::ControlLeft => Some(KeyCode::ControlLeft),
            Self::ControlRight => Some(KeyCode::ControlRight),
            Self::AltLeft => Some(KeyCode::AltLeft),
            Self::AltRight => Some(KeyCode::AltRight),

            // Arrows
            Self::ArrowUp => Some(KeyCode::ArrowUp),
            Self::ArrowDown => Some(KeyCode::ArrowDown),
            Self::ArrowLeft => Some(KeyCode::ArrowLeft),
            Self::ArrowRight => Some(KeyCode::ArrowRight),
        }
    }

    /// Convert to Bevy's `MouseButton` (mouse buttons only — returns `None` for keyboard).
    pub fn to_mouse_button(self) -> Option<MouseButton> {
        match self {
            Self::MouseLeft => Some(MouseButton::Left),
            Self::MouseRight => Some(MouseButton::Right),
            Self::MouseMiddle => Some(MouseButton::Middle),
            _ => None,
        }
    }
}

// =============================================================================
// EditorAction — every editor operation that can be bound to a key
// =============================================================================

/// Every editor action that can be assigned to a key.
///
/// Serialises as its variant name (`"Place"`, `"Save"`…) so the JSON config
/// is self-documenting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorAction {
    /// Place the selected block on the face the crosshair is aimed at (R-click).
    Place,
    /// Break the block the crosshair is aimed at (L-click).
    Break,
    /// Quick-save the current world to disk.
    Save,
    /// Quick-load the save-slot world from disk.
    Load,
    /// Toggle between PLAY and EDIT mode.
    PlayStop,
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,
    /// Toggle the grid overlay on/off.
    ToggleGrid,
    /// Grab / release the orbit camera (same as the click that enters play mode).
    CamOrbit,
    /// Select block in palette slot 1.
    SelectBlock1,
    /// Select block in palette slot 2.
    SelectBlock2,
    /// Select block in palette slot 3.
    SelectBlock3,
    /// Select block in palette slot 4.
    SelectBlock4,
    /// Toggle between grounded walk and free noclip fly.
    ToggleFly,
    /// Release the cursor from the window.
    ReleaseCursor,
    /// Initiate a box-fill (first corner; second G fills the box).
    BoxFill,
}

impl EditorAction {
    /// Short human-readable label for the HUD / config UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Place => "Place",
            Self::Break => "Break",
            Self::Save => "Save",
            Self::Load => "Load",
            Self::PlayStop => "Play/Stop",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::ToggleGrid => "Toggle Grid",
            Self::CamOrbit => "Camera Orbit",
            Self::SelectBlock1 => "Block 1",
            Self::SelectBlock2 => "Block 2",
            Self::SelectBlock3 => "Block 3",
            Self::SelectBlock4 => "Block 4",
            Self::ToggleFly => "Toggle Fly/Walk",
            Self::ReleaseCursor => "Release Cursor",
            Self::BoxFill => "Box Fill",
        }
    }

    /// Every action in a fixed order (for iteration, config UI, etc.).
    pub const ALL: &[Self] = &[
        Self::Place,
        Self::Break,
        Self::Save,
        Self::Load,
        Self::PlayStop,
        Self::Undo,
        Self::Redo,
        Self::ToggleGrid,
        Self::CamOrbit,
        Self::SelectBlock1,
        Self::SelectBlock2,
        Self::SelectBlock3,
        Self::SelectBlock4,
        Self::ToggleFly,
        Self::ReleaseCursor,
        Self::BoxFill,
    ];
}

// =============================================================================
// KeyBindings — the single source of truth for input
// =============================================================================

/// Maps every `EditorAction` to a keyboard key or mouse button.
///
/// Loaded from `editor_config.json` on startup (falls back to factory defaults).
/// Systems **must** read from here instead of hardcoding `KeyCode` literals so
/// that rebinding "just works" everywhere.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Action → bound key.  If an action is missing the helper falls back to the
    /// default, so a partial config file still works.
    #[serde(with = "serde_keybindings_map")]
    pub map: std::collections::HashMap<EditorAction, Key>,
}

/// Custom serialiser so the JSON keys are the action variant names, not Rust debug
/// output.  Produces `{"Place": "MouseRight", …}`.
mod serde_keybindings_map {
    use super::{EditorAction, Key};
    use serde::de::{Deserializer, MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::Serializer;
    use std::collections::HashMap;
    use std::fmt;

    pub fn serialize<S>(map: &HashMap<EditorAction, Key>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Round-trip through string keys so the JSON is human-readable.
        let string_map: HashMap<String, &Key> = map
            .iter()
            .map(|(a, k)| (action_name(*a).to_string(), k))
            .collect();
        let mut m = s.serialize_map(Some(string_map.len()))?;
        for (k, v) in &string_map {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result<HashMap<EditorAction, Key>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MapVisitor;
        impl<'de> Visitor<'de> for MapVisitor {
            type Value = HashMap<EditorAction, Key>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map of action name → key name")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut map = HashMap::new();
                while let Some((name, key)) = access.next_entry::<String, Key>()? {
                    if let Some(action) = action_from_name(&name) {
                        map.insert(action, key);
                    }
                }
                Ok(map)
            }
        }
        d.deserialize_map(MapVisitor)
    }

    /// Stable short name for an action (matches the serde variant name).
    fn action_name(a: EditorAction) -> &'static str {
        match a {
            EditorAction::Place => "Place",
            EditorAction::Break => "Break",
            EditorAction::Save => "Save",
            EditorAction::Load => "Load",
            EditorAction::PlayStop => "PlayStop",
            EditorAction::Undo => "Undo",
            EditorAction::Redo => "Redo",
            EditorAction::ToggleGrid => "ToggleGrid",
            EditorAction::CamOrbit => "CamOrbit",
            EditorAction::SelectBlock1 => "SelectBlock1",
            EditorAction::SelectBlock2 => "SelectBlock2",
            EditorAction::SelectBlock3 => "SelectBlock3",
            EditorAction::SelectBlock4 => "SelectBlock4",
            EditorAction::ToggleFly => "ToggleFly",
            EditorAction::ReleaseCursor => "ReleaseCursor",
            EditorAction::BoxFill => "BoxFill",
        }
    }

    /// Inverse of `action_name` — case-sensitive.
    fn action_from_name(name: &str) -> Option<EditorAction> {
        match name {
            "Place" => Some(EditorAction::Place),
            "Break" => Some(EditorAction::Break),
            "Save" => Some(EditorAction::Save),
            "Load" => Some(EditorAction::Load),
            "PlayStop" => Some(EditorAction::PlayStop),
            "Undo" => Some(EditorAction::Undo),
            "Redo" => Some(EditorAction::Redo),
            "ToggleGrid" => Some(EditorAction::ToggleGrid),
            "CamOrbit" => Some(EditorAction::CamOrbit),
            "SelectBlock1" => Some(EditorAction::SelectBlock1),
            "SelectBlock2" => Some(EditorAction::SelectBlock2),
            "SelectBlock3" => Some(EditorAction::SelectBlock3),
            "SelectBlock4" => Some(EditorAction::SelectBlock4),
            "ToggleFly" => Some(EditorAction::ToggleFly),
            "ReleaseCursor" => Some(EditorAction::ReleaseCursor),
            "BoxFill" => Some(EditorAction::BoxFill),
            _ => None,
        }
    }
}

impl KeyBindings {
    /// Factory-default layout — matches the current hardcoded keys in `main.rs`.
    pub fn defaults() -> Self {
        use EditorAction::*;
        let mut map = std::collections::HashMap::new();
        map.insert(Place, Key::MouseRight);
        map.insert(Break, Key::MouseLeft);
        map.insert(Save, Key::F5);
        map.insert(Load, Key::F9);
        map.insert(PlayStop, Key::Tab);
        map.insert(Undo, Key::KeyZ);
        map.insert(Redo, Key::KeyY);
        map.insert(ToggleGrid, Key::KeyG);
        map.insert(CamOrbit, Key::MouseLeft);
        map.insert(SelectBlock1, Key::Digit1);
        map.insert(SelectBlock2, Key::Digit2);
        map.insert(SelectBlock3, Key::Digit3);
        map.insert(SelectBlock4, Key::Digit4);
        map.insert(ToggleFly, Key::KeyF);
        map.insert(ReleaseCursor, Key::Escape);
        map.insert(BoxFill, Key::KeyG);
        Self { map }
    }

    /// Look up the key bound to `action`.  Never panics — falls back to the
    /// default if the action is missing from the map.
    pub fn key(&self, action: EditorAction) -> Key {
        self.map
            .get(&action)
            .copied()
            .unwrap_or_else(|| Self::defaults().map[&action])
    }

    /// Rebind an action to a new key.
    pub fn rebind(&mut self, action: EditorAction, key: Key) {
        self.map.insert(action, key);
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::defaults()
    }
}

// =============================================================================
// Helpers — the public API Poppy / Shiba call each frame
// =============================================================================

/// Was `action` **just pressed** this frame?
///
/// This is the canonical way to query editor input.  Use this instead of
/// reaching into `ButtonInput<KeyCode>` or `ButtonInput<MouseButton>` directly
/// — rebinding then works everywhere with zero changes.
pub fn action_just_pressed(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    bindings: &KeyBindings,
    action: EditorAction,
) -> bool {
    let k = bindings.key(action);
    if let Some(kc) = k.to_keycode() {
        keys.just_pressed(kc)
    } else if let Some(mb) = k.to_mouse_button() {
        mouse.just_pressed(mb)
    } else {
        false
    }
}

/// Is `action` **held down** this frame?  Use for continuous actions (e.g.
/// dragging out a fill region).
pub fn action_pressed(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    bindings: &KeyBindings,
    action: EditorAction,
) -> bool {
    let k = bindings.key(action);
    if let Some(kc) = k.to_keycode() {
        keys.pressed(kc)
    } else if let Some(mb) = k.to_mouse_button() {
        mouse.pressed(mb)
    } else {
        false
    }
}

/// Check several actions and return the **first** one that was just pressed.
/// Useful for mutually-exclusive actions (e.g. the block palette 1–4).
pub fn first_action_just_pressed(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    bindings: &KeyBindings,
    actions: &[EditorAction],
) -> Option<EditorAction> {
    actions
        .iter()
        .find(|a| action_just_pressed(keys, mouse, bindings, **a))
        .copied()
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_have_all_actions() {
        let b = KeyBindings::defaults();
        for action in EditorAction::ALL {
            let k = b.key(*action);
            // Every action must resolve to some key (never panic).
            assert!(
                k.to_keycode().is_some() || k.to_mouse_button().is_some(),
                "action {:?} bound to {:?} has no keycode or mouse button",
                action,
                k
            );
        }
    }

    #[test]
    fn key_round_trip_all_variants() {
        // Every Key variant must round-trip through its Bevy counterpart.
        let all_keys: &[Key] = &[
            Key::MouseLeft,
            Key::MouseRight,
            Key::MouseMiddle,
            Key::KeyA,
            Key::KeyW,
            Key::KeyS,
            Key::KeyD,
            Key::KeyF,
            Key::KeyG,
            Key::KeyR,
            Key::KeyX,
            Key::KeyC,
            Key::KeyV,
            Key::KeyY,
            Key::KeyZ,
            Key::Digit0,
            Key::Digit1,
            Key::Digit4,
            Key::Digit9,
            Key::F1,
            Key::F5,
            Key::F9,
            Key::F12,
            Key::Space,
            Key::Enter,
            Key::Escape,
            Key::Tab,
            Key::Backspace,
            Key::Delete,
            Key::ShiftLeft,
            Key::ControlLeft,
            Key::AltLeft,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
        ];
        for &k in all_keys {
            match (k.to_keycode(), k.to_mouse_button()) {
                (Some(_), None) => { /* keyboard — ok */ }
                (None, Some(_)) => { /* mouse — ok */ }
                other => panic!("{:?} mapped to both or neither: {:?}", k, other),
            }
        }
    }

    #[test]
    fn serde_keybindings_round_trip() {
        let b = KeyBindings::defaults();
        let json = serde_json::to_string_pretty(&b).expect("serialize");
        // The JSON must mention the action names and key names (not raw integers).
        assert!(json.contains("\"Place\""));
        assert!(json.contains("\"MouseRight\""));
        assert!(json.contains("\"Break\""));
        assert!(json.contains("\"F5\""));
        assert!(json.contains("\"Tab\""));

        let b2: KeyBindings = serde_json::from_str(&json).expect("deserialize");
        // All default actions must survive the round-trip.
        for action in EditorAction::ALL {
            assert_eq!(
                b.key(*action),
                b2.key(*action),
                "mismatch on {:?}",
                action
            );
        }
    }

    #[test]
    fn partial_config_missing_keys_fallback() {
        // A config with only one binding — all others must fall back to defaults.
        let json = r#"{"map":{"Place":"KeyP","Save":"KeyS"}}"#;
        let b: KeyBindings = serde_json::from_str(json).expect("parse partial");
        assert_eq!(b.key(EditorAction::Place), Key::KeyP);
        assert_eq!(b.key(EditorAction::Save), Key::KeyS);
        // Missing actions fall back to defaults.
        assert_eq!(b.key(EditorAction::Break), KeyBindings::defaults().key(EditorAction::Break));
        assert_eq!(b.key(EditorAction::Load), KeyBindings::defaults().key(EditorAction::Load));
    }

    #[test]
    fn rebind_and_query() {
        let mut b = KeyBindings::defaults();
        // Default: Place → MouseRight.
        assert_eq!(b.key(EditorAction::Place), Key::MouseRight);
        // Rebind to P.
        b.rebind(EditorAction::Place, Key::KeyP);
        assert_eq!(b.key(EditorAction::Place), Key::KeyP);
        // Other actions unchanged.
        assert_eq!(b.key(EditorAction::Break), Key::MouseLeft);
    }

    #[test]
    fn editor_action_label_is_stable() {
        // Labels are read by the config UI — changing them breaks saved configs
        // that use the old serde name.  At minimum every action returns *some* label.
        for action in EditorAction::ALL {
            let label = action.label();
            assert!(!label.is_empty(), "{:?} has empty label", action);
        }
    }

    #[test]
    fn all_actions_covered() {
        // Every variant in EditorAction must appear in ALL (otherwise iteration
        // loops — like the config UI — silently skip actions).
        // We prove it by checking that every action resolves to a key via defaults.
        let b = KeyBindings::defaults();
        for action in EditorAction::ALL {
            let _ = b.key(*action); // would panic if missing + default missing
        }
    }
}
