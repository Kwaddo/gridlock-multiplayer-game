//! Keeps the settings screen's choices between runs: General (sensitivity,
//! FOV, UI scale, minimap, audio) and Key bindings. Stored as JSON through
//! `platform::storage` (a file natively, localStorage in the browser).
//!
//! Only fields present and valid in the stored JSON are applied, so older or
//! hand-edited files never break a launch: anything else keeps its default.

use std::collections::BTreeMap;

use macroquad::prelude::{KeyCode, MouseButton};
use serde::{Deserialize, Serialize};

use super::config::{GameConfig, InputBinding};
use crate::settings::fov::{FOV_MAX, FOV_MIN};
use crate::settings::sensitivity::{SENS_MAX, SENS_MIN};

const STORAGE_KEY: &str = "settings.json";

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Saved {
    sensitivity: Option<f32>,
    fov: Option<f32>,
    gui_scale: Option<u8>,
    minimap_size: Option<u8>,
    minimap_shape: Option<u8>,
    /// Stored as `audio_muted`: the older `mute_audio` key held the old
    /// default (muted) with no way to change it, so it is ignored.
    audio_muted: Option<bool>,
    master_volume: Option<f32>,
    crouch_toggle: Option<bool>,
    slow_walk_toggle: Option<bool>,
    prone_toggle: Option<bool>,
    /// binding field name (e.g. "forward") -> "Key:W" or "Mouse:Right"
    binds: BTreeMap<String, String>,
}

impl Saved {
    /// Each field on its own, so one wrong value only loses that one setting.
    fn lenient(value: &serde_json::Value) -> Self {
        fn field<T: serde::de::DeserializeOwned>(value: &serde_json::Value, name: &str) -> Option<T> {
            serde_json::from_value(value.get(name)?.clone()).ok()
        }
        let binds = value
            .get("binds")
            .and_then(|b| b.as_object())
            .map(|b| {
                b.iter()
                    .filter_map(|(name, v)| Some((name.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Saved {
            sensitivity: field(value, "sensitivity"),
            fov: field(value, "fov"),
            gui_scale: field(value, "gui_scale"),
            minimap_size: field(value, "minimap_size"),
            minimap_shape: field(value, "minimap_shape"),
            audio_muted: field(value, "audio_muted"),
            master_volume: field(value, "master_volume"),
            crouch_toggle: field(value, "crouch_toggle"),
            slow_walk_toggle: field(value, "slow_walk_toggle"),
            prone_toggle: field(value, "prone_toggle"),
            binds,
        }
    }
}

/// Every binding in the settings screen, by the name it is stored under.
macro_rules! bindings {
    ($($name:literal => $field:ident),* $(,)?) => {
        fn binds_of(c: &GameConfig) -> BTreeMap<String, String> {
            [$(($name, &c.$field)),*]
                .into_iter()
                .map(|(name, b)| (name.to_string(), binding_to_string(b)))
                .collect()
        }

        fn apply_binds(c: &mut GameConfig, binds: &BTreeMap<String, String>) {
            $(
                if let Some(b) = binds.get($name).and_then(|s| binding_from_str(s)) {
                    c.$field = b;
                }
            )*
        }
    };
}

bindings! {
    "forward" => bind_forward,
    "backward" => bind_backward,
    "left" => bind_left,
    "right" => bind_right,
    "sprint" => bind_sprint,
    "crouch" => bind_crouch,
    "jump" => bind_jump,
    "settings" => bind_settings,
    "reload" => bind_reload,
    "fire" => bind_fire,
    "aim" => bind_aim,
    "taurus_toggle" => bind_taurus_toggle,
    "punch" => bind_punch,
    "slow_walk" => bind_slow_walk,
    "prone" => bind_prone,
    "fullscreen" => bind_fullscreen,
    "chat" => bind_chat,
    "weapon1" => bind_weapon1,
    "weapon2" => bind_weapon2,
    "weapon3" => bind_weapon3,
    "weapon4" => bind_weapon4,
    "weapon5" => bind_weapon5,
}

impl GameConfig {
    /// Defaults, overridden by whatever was saved last time.
    pub fn load() -> Self {
        let mut config = GameConfig::new();
        let saved = platform::storage::load(STORAGE_KEY)
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .map(|value| Saved::lenient(&value))
            .unwrap_or_default();

        let finite = |v: Option<f32>| v.filter(|v| v.is_finite());
        if let Some(v) = finite(saved.sensitivity) {
            config.sensitivity = v.clamp(SENS_MIN, SENS_MAX);
        }
        if let Some(v) = finite(saved.fov) {
            config.fov = v.clamp(FOV_MIN, FOV_MAX);
        }
        if let Some(v) = saved.gui_scale {
            config.gui_scale = v;
        }
        if let Some(v) = saved.minimap_size {
            config.minimap_size = v;
        }
        if let Some(v) = saved.minimap_shape {
            config.minimap_shape = v;
        }
        if let Some(v) = saved.audio_muted {
            config.mute_audio = v;
        }
        if let Some(v) = finite(saved.master_volume) {
            config.master_volume = v.clamp(0.0, 1.0);
        }
        if let Some(v) = saved.crouch_toggle {
            config.crouch_toggle = v;
        }
        if let Some(v) = saved.slow_walk_toggle {
            config.slow_walk_toggle = v;
        }
        if let Some(v) = saved.prone_toggle {
            config.prone_toggle = v;
        }
        apply_binds(&mut config, &saved.binds);
        config
    }

    /// Store the current settings; called when the player presses Save.
    pub fn save(&self) {
        let saved = Saved {
            sensitivity: Some(self.sensitivity),
            fov: Some(self.fov),
            gui_scale: Some(self.gui_scale),
            minimap_size: Some(self.minimap_size),
            minimap_shape: Some(self.minimap_shape),
            audio_muted: Some(self.mute_audio),
            master_volume: Some(self.master_volume),
            crouch_toggle: Some(self.crouch_toggle),
            slow_walk_toggle: Some(self.slow_walk_toggle),
            prone_toggle: Some(self.prone_toggle),
            binds: binds_of(self),
        };
        if let Ok(json) = serde_json::to_string_pretty(&saved) {
            if !platform::storage::save(STORAGE_KEY, &json) {
                println!("Could not save settings");
            }
        }
    }
}

fn binding_to_string(binding: &InputBinding) -> String {
    match binding {
        InputBinding::Key(k) => format!("Key:{:?}", k),
        InputBinding::Mouse(m) => format!("Mouse:{:?}", m),
    }
}

fn binding_from_str(s: &str) -> Option<InputBinding> {
    let (kind, name) = s.split_once(':')?;
    match kind {
        "Key" => ALL_KEYS.iter().find(|k| format!("{:?}", k) == name).map(|k| InputBinding::Key(*k)),
        "Mouse" => [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
            .into_iter()
            .find(|m| format!("{:?}", m) == name)
            .map(InputBinding::Mouse),
        _ => None,
    }
}

/// Every `KeyCode` variant (generated from miniquad's `event.rs`), so stored
/// names map back to keys without trusting raw numbers.
const ALL_KEYS: &[KeyCode] = &[
    KeyCode::Space,
    KeyCode::Apostrophe,
    KeyCode::Comma,
    KeyCode::Minus,
    KeyCode::Period,
    KeyCode::Slash,
    KeyCode::Key0,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
    KeyCode::Semicolon,
    KeyCode::Equal,
    KeyCode::A,
    KeyCode::B,
    KeyCode::C,
    KeyCode::D,
    KeyCode::E,
    KeyCode::F,
    KeyCode::G,
    KeyCode::H,
    KeyCode::I,
    KeyCode::J,
    KeyCode::K,
    KeyCode::L,
    KeyCode::M,
    KeyCode::N,
    KeyCode::O,
    KeyCode::P,
    KeyCode::Q,
    KeyCode::R,
    KeyCode::S,
    KeyCode::T,
    KeyCode::U,
    KeyCode::V,
    KeyCode::W,
    KeyCode::X,
    KeyCode::Y,
    KeyCode::Z,
    KeyCode::LeftBracket,
    KeyCode::Backslash,
    KeyCode::RightBracket,
    KeyCode::GraveAccent,
    KeyCode::World1,
    KeyCode::World2,
    KeyCode::Escape,
    KeyCode::Enter,
    KeyCode::Tab,
    KeyCode::Backspace,
    KeyCode::Insert,
    KeyCode::Delete,
    KeyCode::Right,
    KeyCode::Left,
    KeyCode::Down,
    KeyCode::Up,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::CapsLock,
    KeyCode::ScrollLock,
    KeyCode::NumLock,
    KeyCode::PrintScreen,
    KeyCode::Pause,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
    KeyCode::F13,
    KeyCode::F14,
    KeyCode::F15,
    KeyCode::F16,
    KeyCode::F17,
    KeyCode::F18,
    KeyCode::F19,
    KeyCode::F20,
    KeyCode::F21,
    KeyCode::F22,
    KeyCode::F23,
    KeyCode::F24,
    KeyCode::F25,
    KeyCode::Kp0,
    KeyCode::Kp1,
    KeyCode::Kp2,
    KeyCode::Kp3,
    KeyCode::Kp4,
    KeyCode::Kp5,
    KeyCode::Kp6,
    KeyCode::Kp7,
    KeyCode::Kp8,
    KeyCode::Kp9,
    KeyCode::KpDecimal,
    KeyCode::KpDivide,
    KeyCode::KpMultiply,
    KeyCode::KpSubtract,
    KeyCode::KpAdd,
    KeyCode::KpEnter,
    KeyCode::KpEqual,
    KeyCode::LeftShift,
    KeyCode::LeftControl,
    KeyCode::LeftAlt,
    KeyCode::LeftSuper,
    KeyCode::RightShift,
    KeyCode::RightControl,
    KeyCode::RightAlt,
    KeyCode::RightSuper,
    KeyCode::Menu,
    KeyCode::Back,
    KeyCode::Unknown,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_round_trip_by_name() {
        for b in [
            InputBinding::Key(KeyCode::W),
            InputBinding::Key(KeyCode::LeftControl),
            InputBinding::Key(KeyCode::F11),
            InputBinding::Mouse(MouseButton::Right),
        ] {
            assert!(binding_from_str(&binding_to_string(&b)) == Some(b));
        }
        assert!(binding_from_str("Key:NotAKey").is_none());
        assert!(binding_from_str("garbage").is_none());
    }

    #[test]
    fn one_bad_field_keeps_the_rest() {
        let value = serde_json::json!({
            "sensitivity": 2.5,
            "fov": "wide",
            "binds": { "forward": "Key:Up", "jump": 7 }
        });
        let saved = Saved::lenient(&value);
        assert_eq!(saved.sensitivity, Some(2.5));
        assert_eq!(saved.fov, None);
        assert_eq!(saved.binds.get("forward").map(String::as_str), Some("Key:Up"));
        assert!(!saved.binds.contains_key("jump"));
    }

    #[test]
    fn every_binding_is_stored() {
        let config = GameConfig::new();
        let mut restored = GameConfig::new();
        restored.bind_forward = InputBinding::Key(KeyCode::Up);
        apply_binds(&mut restored, &binds_of(&config));
        assert!(restored == config);
    }
}
