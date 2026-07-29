//! Editor configuration — `EditorSettings` + `InputConfigPlugin`.
//!
//! ## Wiring (Poppy — add these two lines to `client/src/main.rs`)
//!
//! ```ignore
//! // 1. Module declaration (next to `mod combat; mod hero; …`):
//! mod editor_config;
//!
//! // 2. Plugin registration (inside `fn main()`, after `.add_plugins(DefaultPlugins…)`):
//! .add_plugins(editor_config::InputConfigPlugin);
//! ```
//!
//! ## API (Poppy / Shiba)
//!
//! ```ignore
//! // ---- Read keybindings ----
//! fn my_edit_system(
//!     keys: Res<ButtonInput<KeyCode>>,
//!     mouse: Res<ButtonInput<MouseButton>>,
//!     bindings: Res<editor_config::input_map::KeyBindings>,
//! ) {
//!     use editor_config::input_map::{action_just_pressed, EditorAction};
//!     if action_just_pressed(&keys, &mouse, &bindings, EditorAction::Save) {
//!         // quick-save
//!     }
//! }
//!
//! // ---- Read editor settings ----
//! fn my_gizmo_system(settings: Res<editor_config::EditorSettings>) {
//!     let snap = settings.grid_snap;
//!     let mode = settings.gizmo_mode;
//! }
//!
//! // ---- Save changed settings ----
//! editor_config::save_config(&bindings, &settings);
//! ```
//!
//! ## Config file format (`editor_config.json`, generated on first run)
//!
//! ```json
//! {
//!   "keybindings": {
//!     "map": {
//!       "Place": "MouseRight",
//!       "Break": "MouseLeft",
//!       "Save": "F5",
//!       …
//!     }
//!   },
//!   "settings": {
//!     "grid_snap": 1.0,
//!     "gizmo_mode": "Translate",
//!     "camera_sensitivity": 0.0025,
//!     "default_daytime": 12.0
//!   }
//! }
//! ```

// Include `input_map.rs` as a submodule so everything is reachable through one
// `mod editor_config;` declaration in main.rs.
#[path = "input_map.rs"]
pub mod input_map;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Re-export the input-map API so callers only need `use editor_config::…`.
#[allow(unused_imports)]
pub use input_map::{
    action_just_pressed, action_pressed, first_action_just_pressed, EditorAction, KeyBindings,
};

// =============================================================================
// EditorSettings — persistent editor preferences
// =============================================================================

/// Persistent editor settings loaded from `editor_config.json` on boot.
///
/// Saved automatically whenever a value changes (call `save_config()` after
/// mutating the resource).
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    /// Grid-snap step in blocks.  `1.0` snaps to integer block boundaries;
    /// `0.5` is half-block precision; `0.0` disables snapping entirely.
    pub grid_snap: f32,
    /// Active transform gizmo (translate / rotate / scale).
    pub gizmo_mode: GizmoMode,
    /// Mouse sensitivity multiplier for the orbit camera (scales the raw
    /// mouse-delta before it is applied to yaw/pitch).  The default `0.0025`
    /// matches the current hardcoded factor in `fly_camera`.
    pub camera_sensitivity: f32,
    /// Default time-of-day in hours (0–24) for the directional-light azimuth
    /// when no env override is set.  `12.0` = noon sun directly overhead.
    pub default_daytime: f32,
}

/// The active transform-gizmo mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GizmoMode {
    /// Move the selected region.
    Translate,
    /// Rotate the selected region.
    Rotate,
    /// Scale the selected region.
    Scale,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            grid_snap: 1.0,
            gizmo_mode: GizmoMode::Translate,
            camera_sensitivity: 0.0025,
            default_daytime: 12.0,
        }
    }
}

// =============================================================================
// Combined config file payload
// =============================================================================

/// The full contents of `editor_config.json` — keybindings + settings in one
/// file so users only have one config to manage.
#[derive(Serialize, Deserialize)]
struct EditorConfigFile {
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub settings: EditorSettings,
}

// =============================================================================
// Plugin
// =============================================================================

/// Bevy plugin that registers `KeyBindings` and `EditorSettings` as resources.
///
/// On native builds the plugin loads both from `editor_config.json` (creating a
/// default file if none exists).  On `wasm32` it inserts the defaults directly.
///
/// ```ignore
/// app.add_plugins(editor_config::InputConfigPlugin);
/// ```
pub struct InputConfigPlugin;

impl Plugin for InputConfigPlugin {
    fn build(&self, app: &mut App) {
        let (bindings, settings) = load_config();
        app.insert_resource(bindings).insert_resource(settings);
    }
}

// =============================================================================
// Load / save
// =============================================================================

const CONFIG_PATH: &str = "editor_config.json";

/// Public save entry-point.  Call this after mutating `KeyBindings` or
/// `EditorSettings` to persist the change to disk.  No-op on `wasm32`.
pub fn save_config(bindings: &KeyBindings, settings: &EditorSettings) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(e) = write_config_file(bindings, settings) {
            eprintln!("EDITOR_CONFIG save failed: {e}");
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (bindings, settings);
    }
}

/// Load the config from disk, falling back to defaults and writing a template
/// file so the user has something to edit.
#[cfg(not(target_arch = "wasm32"))]
fn load_config() -> (KeyBindings, EditorSettings) {
    match std::fs::read_to_string(CONFIG_PATH) {
        Ok(text) => match serde_json::from_str::<EditorConfigFile>(&text) {
            Ok(cfg) => {
                println!("EDITOR_CONFIG loaded {CONFIG_PATH} — {} actions, grid_snap={}",
                    cfg.keybindings.map.len(), cfg.settings.grid_snap);
                (cfg.keybindings, cfg.settings)
            }
            Err(e) => {
                eprintln!("EDITOR_CONFIG parse error (using defaults): {e}");
                (KeyBindings::default(), EditorSettings::default())
            }
        },
        Err(_) => {
            let (b, s) = (KeyBindings::default(), EditorSettings::default());
            let _ = write_config_file(&b, &s);
            (b, s)
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn load_config() -> (KeyBindings, EditorSettings) {
    (KeyBindings::default(), EditorSettings::default())
}

/// Write the full config to disk as pretty-printed JSON.
#[cfg(not(target_arch = "wasm32"))]
fn write_config_file(bindings: &KeyBindings, settings: &EditorSettings) -> Result<(), String> {
    let cfg = EditorConfigFile {
        keybindings: bindings.clone(),
        settings: settings.clone(),
    };
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(CONFIG_PATH, &json).map_err(|e| e.to_string())?;
    println!("EDITOR_CONFIG saved {CONFIG_PATH}");
    Ok(())
}

// =============================================================================
// Unit tests — round-trip save → load, defaults, settings serialisation
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // EditorSettings
    // ------------------------------------------------------------------

    #[test]
    fn settings_defaults_are_sensible() {
        let s = EditorSettings::default();
        assert!(s.grid_snap > 0.0, "grid_snap must be positive");
        assert!(s.camera_sensitivity > 0.0, "sensitivity must be positive");
        assert!(
            (0.0..=24.0).contains(&s.default_daytime),
            "daytime in 0-24 range"
        );
    }

    #[test]
    fn settings_round_trip_json() {
        let s = EditorSettings {
            grid_snap: 0.5,
            gizmo_mode: GizmoMode::Rotate,
            camera_sensitivity: 0.001,
            default_daytime: 18.0,
        };
        let json = serde_json::to_string_pretty(&s).expect("serialize");
        assert!(json.contains("\"grid_snap\""));
        assert!(json.contains("0.5"));
        assert!(json.contains("\"Rotate\""));

        let s2: EditorSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s.grid_snap, s2.grid_snap);
        assert_eq!(s.gizmo_mode, s2.gizmo_mode);
        assert_eq!(s.camera_sensitivity, s2.camera_sensitivity);
        assert_eq!(s.default_daytime, s2.default_daytime);
    }

    #[test]
    fn settings_partial_json_fills_defaults() {
        // Only grid_snap specified — everything else gets defaults.
        let json = r#"{"grid_snap":2.0}"#;
        let s: EditorSettings = serde_json::from_str(json).expect("parse");
        assert_eq!(s.grid_snap, 2.0);
        assert_eq!(s.gizmo_mode, GizmoMode::Translate); // default
        assert_eq!(s.camera_sensitivity, 0.0025); // default
    }

    // ------------------------------------------------------------------
    // Combined config file round-trip
    // ------------------------------------------------------------------

    #[test]
    fn config_file_round_trip() {
        let b = KeyBindings::defaults();
        let s = EditorSettings {
            grid_snap: 0.25,
            gizmo_mode: GizmoMode::Scale,
            camera_sensitivity: 0.004,
            default_daytime: 7.0,
        };

        let cfg = EditorConfigFile {
            keybindings: b.clone(),
            settings: s.clone(),
        };
        let json = serde_json::to_string_pretty(&cfg).expect("serialize");

        // The JSON must contain both sections.
        assert!(json.contains("\"keybindings\""));
        assert!(json.contains("\"settings\""));
        assert!(json.contains("\"grid_snap\""));
        assert!(json.contains("\"Place\""));
        assert!(json.contains("\"F5\""));

        let cfg2: EditorConfigFile = serde_json::from_str(&json).expect("deserialize");

        // Keybindings round-trip.
        for action in EditorAction::ALL {
            assert_eq!(
                cfg.keybindings.key(*action),
                cfg2.keybindings.key(*action),
                "keybinding mismatch on {:?}",
                action
            );
        }

        // Settings round-trip.
        assert_eq!(cfg.settings.grid_snap, cfg2.settings.grid_snap);
        assert_eq!(cfg.settings.gizmo_mode, cfg2.settings.gizmo_mode);
        assert_eq!(cfg.settings.camera_sensitivity, cfg2.settings.camera_sensitivity);
        assert_eq!(cfg.settings.default_daytime, cfg2.settings.default_daytime);
    }

    #[test]
    fn empty_config_file_uses_all_defaults() {
        let json = r#"{}"#;
        let cfg: EditorConfigFile = serde_json::from_str(json).expect("parse empty");
        // Keybindings fall back to defaults.
        let defs = KeyBindings::defaults();
        assert_eq!(cfg.keybindings.key(EditorAction::Place), defs.key(EditorAction::Place));
        assert_eq!(cfg.keybindings.key(EditorAction::Save), defs.key(EditorAction::Save));
        // Settings fall back to defaults.
        assert_eq!(cfg.settings.grid_snap, EditorSettings::default().grid_snap);
        assert_eq!(cfg.settings.gizmo_mode, EditorSettings::default().gizmo_mode);
    }

    #[test]
    fn config_file_missing_section_fills_defaults() {
        // Keybindings section only — settings should default.
        let json = r#"{"keybindings":{"map":{"Place":"KeyP"}}}"#;
        let cfg: EditorConfigFile = serde_json::from_str(json).expect("parse");
        assert_eq!(cfg.keybindings.key(EditorAction::Place), input_map::Key::KeyP);
        assert_eq!(cfg.settings.grid_snap, EditorSettings::default().grid_snap);

        // Settings section only — keybindings should default.
        let json = r#"{"settings":{"grid_snap":3.0}}"#;
        let cfg: EditorConfigFile = serde_json::from_str(json).expect("parse");
        assert_eq!(cfg.settings.grid_snap, 3.0);
        let defs = KeyBindings::defaults();
        assert_eq!(
            cfg.keybindings.key(EditorAction::Break),
            defs.key(EditorAction::Break)
        );
    }

    // ------------------------------------------------------------------
    // GizmoMode
    // ------------------------------------------------------------------

    #[test]
    fn gizmo_mode_serialises_as_string() {
        let json = serde_json::to_string(&GizmoMode::Translate).expect("serialize");
        assert_eq!(json, r#""Translate""#);
        let json = serde_json::to_string(&GizmoMode::Rotate).expect("serialize");
        assert_eq!(json, r#""Rotate""#);
        let json = serde_json::to_string(&GizmoMode::Scale).expect("serialize");
        assert_eq!(json, r#""Scale""#);
    }

    #[test]
    fn gizmo_mode_deserialises_from_string() {
        let m: GizmoMode = serde_json::from_str(r#""Translate""#).expect("parse");
        assert_eq!(m, GizmoMode::Translate);
        let m: GizmoMode = serde_json::from_str(r#""Rotate""#).expect("parse");
        assert_eq!(m, GizmoMode::Rotate);
        let m: GizmoMode = serde_json::from_str(r#""Scale""#).expect("parse");
        assert_eq!(m, GizmoMode::Scale);
    }
}
