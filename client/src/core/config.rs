/*
    This file contains the game configuration, such as keybinds and settings.
    did this so we can easily have settings menu in the future where players can change these values.
*/

use macroquad::prelude::*;

/// How far every player sees, in world units: the camera's far plane and the
/// maze's draw distance. One value for everyone, and the same for walls and
/// players, so nobody sees players past walls that are not drawn.
pub const VIEW_DISTANCE: f32 = 100.0;

// A single input binding — can be a key OR a mouse button.
// Future: add a Controller(u16) variant for gamepad.
#[derive(Clone, Copy, PartialEq)]
pub enum InputBinding {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl InputBinding {
    pub fn is_down(&self) -> bool {
        match self {
            InputBinding::Key(k) => is_key_down(*k),
            InputBinding::Mouse(m) => is_mouse_button_down(*m),
        }
    }

    pub fn is_pressed(&self) -> bool {
        match self {
            InputBinding::Key(k) => is_key_pressed(*k),
            InputBinding::Mouse(m) => is_mouse_button_pressed(*m),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            InputBinding::Key(k) => format!("{:?}", k),
            InputBinding::Mouse(m) => match m {
                MouseButton::Left => "Mouse Left".to_string(),
                MouseButton::Right => "Mouse Right".to_string(),
                MouseButton::Middle => "Mouse Middle".to_string(),
                _ => format!("Mouse {:?}", m),
            },
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct GameConfig {
    // Mouse (UI value 10-100, default 60)
    pub sensitivity: f32,

    // Keybinds — all use InputBinding so any key or mouse button works
    pub bind_forward: InputBinding,
    pub bind_backward: InputBinding,
    pub bind_left: InputBinding,
    pub bind_right: InputBinding,
    pub bind_sprint: InputBinding,
    pub bind_crouch: InputBinding,
    pub crouch_toggle: bool, // if true, crouch is a toggle instead of hold
    pub slow_walk_toggle: bool, // if true, slow walk is toggle instead of hold
    pub prone_toggle: bool, // if true, prone is toggle instead of hold
    pub bind_jump: InputBinding,
    pub bind_settings: InputBinding,
    pub bind_reload: InputBinding,
    pub bind_fire: InputBinding,
    pub bind_aim: InputBinding,
    pub bind_weapon1: InputBinding,
    pub bind_weapon2: InputBinding,
    pub bind_weapon3: InputBinding,
    pub bind_weapon4: InputBinding,
    pub bind_weapon5: InputBinding,
    pub bind_taurus_toggle: InputBinding,
    pub bind_punch: InputBinding,
    pub bind_slow_walk: InputBinding,
    pub bind_prone: InputBinding,
    pub bind_fullscreen: InputBinding,
    pub bind_chat: InputBinding,

    // Camera (UI value 40-120, default 90)
    pub fov: f32,

    // GUI Scale (0=Auto, 1-4=fixed scale)
    pub gui_scale: u8,

    // Player
    pub standing_height: f32,
    pub crouch_height: f32,

    // Minimap (0=Small, 1=Medium, 2=Large)
    pub minimap_size: u8,
    // Minimap shape (0=Circle, 1=Square)
    pub minimap_shape: u8,
    // Audio
    pub mute_audio: bool,
    /// Master volume, 0..1.
    pub master_volume: f32,
    // Match timer override: None = use server/default, Some(seconds) = show this local timer
    pub match_time_override_secs: Option<u32>,
    // If true, hide the match timer (treat as infinite locally)
    pub hide_match_timer: bool,
    // Audio
}

impl GameConfig {
    /// Pause / back: the bound key, or Esc, which always works too (natively,
    /// and in browsers that hand it to the game; elsewhere losing the mouse
    /// to Esc is passed on as an Esc press, see web/gridlock_web.js).
    pub fn pause_pressed(&self) -> bool {
        self.bind_settings.is_pressed() || is_key_pressed(KeyCode::Escape)
    }

    pub fn defaults() -> Self {
        Self {
            // Mouse - UI value, converted to internal with raw_sensitivity()
            /*
                the reason for this is that before the normal sensitivity was 0.002 but its not an
                intuitive number, so i normalized it to be 60 by default which is an easier number to
                understand and adjust in the settings menu, but raw_sensitivity() will convert it back to the
            actual multiplier used in the game
            */
            sensitivity: 60.0,

            // Keybinds
            bind_forward: InputBinding::Key(KeyCode::W),
            bind_backward: InputBinding::Key(KeyCode::S),
            bind_left: InputBinding::Key(KeyCode::A),
            bind_right: InputBinding::Key(KeyCode::D),
            bind_sprint: InputBinding::Key(KeyCode::LeftShift),
            // C, not Ctrl: in a browser Ctrl+W (crouch-walking) closes the tab
            bind_crouch: InputBinding::Key(KeyCode::C),
            crouch_toggle: false, // default to hold
            slow_walk_toggle: false, // default to hold
            prone_toggle: false, // default to hold
            bind_jump: InputBinding::Key(KeyCode::Space),
            // P, not Esc: browsers keep Esc to release the mouse. Esc still
            // pauses as well (see `pause_pressed`).
            bind_settings: InputBinding::Key(KeyCode::P),
            bind_reload: InputBinding::Key(KeyCode::R),
            bind_fire: InputBinding::Mouse(MouseButton::Left),
            bind_aim: InputBinding::Mouse(MouseButton::Right),
            bind_weapon1: InputBinding::Key(KeyCode::Key1),
            bind_weapon2: InputBinding::Key(KeyCode::Key2),
            bind_weapon3: InputBinding::Key(KeyCode::Key3),
            bind_weapon4: InputBinding::Key(KeyCode::Key4),
            bind_weapon5: InputBinding::Key(KeyCode::Key5),
            bind_taurus_toggle: InputBinding::Key(KeyCode::V),
            bind_punch: InputBinding::Key(KeyCode::G),
            bind_slow_walk: InputBinding::Key(KeyCode::LeftAlt),
            bind_prone: InputBinding::Key(KeyCode::Z),
            bind_fullscreen: InputBinding::Key(KeyCode::F11),
            bind_chat: InputBinding::Key(KeyCode::T),

            // Camera - UI value, converted to internal with raw_fov()
            // this is the same as the sensitivity from earlier, just converting it to a more intuitive UI value while raw_fov() converts it back to the actual FOV used in the game
            fov: 90.0,

            // GUI Scale
            gui_scale: 0, // Auto

            // Player
            standing_height: 1.0,
            crouch_height: 0.5,

            // Minimap
            minimap_size: 1,  // Medium
            minimap_shape: 0, // Circle
            // Audio
            mute_audio: false,
            master_volume: 0.75,
            match_time_override_secs: None,
            hide_match_timer: false,
        }
    }

    pub fn new() -> Self {
        Self::defaults()
    }

    pub fn reset(&mut self) {
        *self = Self::defaults();
    }

    // Convert UI sensitivity (10-100) to the actual mouse multiplier
    // UI 60 = old 0.002
    pub fn raw_sensitivity(&self) -> f32 {
        self.sensitivity * (0.002 / 60.0)
    }

    // Convert UI FOV (40-120) to vertical FOV for camera
    // Uses the value directly clamped to reasonable range
    pub fn raw_fov(&self) -> f32 {
        self.fov.clamp(40.0, 120.0)
    }

    pub fn gui_scale_factor(&self) -> f32 {
        let effective = if self.gui_scale == 0 {
            Self::auto_gui_scale()
        } else {
            self.gui_scale
        };
        match effective {
            1 => 0.75,
            2 => 1.0,
            3 => 1.25,
            4 => 1.5,
            _ => 1.0,
        }
    }

    fn auto_gui_scale() -> u8 {
        let h = screen_height();
        if h >= 2160.0 {
            4
        } else if h >= 1440.0 {
            3
        } else if h >= 900.0 {
            2
        } else {
            1
        }
    }

    pub fn effective_gui_scale(&self) -> u8 {
        if self.gui_scale == 0 {
            Self::auto_gui_scale()
        } else {
            self.gui_scale
        }
    }

    // collect all bindings into a vec for duplicate detection
    pub fn all_bindings(&self) -> Vec<(&str, InputBinding)> {
        vec![
            ("Forward", self.bind_forward),
            ("Backward", self.bind_backward),
            ("Left", self.bind_left),
            ("Right", self.bind_right),
            ("Sprint", self.bind_sprint),
            ("Crouch", self.bind_crouch),
            ("Jump", self.bind_jump),
            ("Reload", self.bind_reload),
            ("Fire", self.bind_fire),
            ("Aim", self.bind_aim),
            ("Pause", self.bind_settings),
            ("Weapon 1", self.bind_weapon1),
            ("Weapon 2", self.bind_weapon2),
            ("Weapon 3", self.bind_weapon3),
            ("Weapon 4", self.bind_weapon4),
            ("Weapon 5", self.bind_weapon5),
            ("Taurus Toggle", self.bind_taurus_toggle),
            ("Punch", self.bind_punch),
            ("Slow Walk", self.bind_slow_walk),
            ("Prone", self.bind_prone),
            ("Fullscreen", self.bind_fullscreen),
            ("Chat", self.bind_chat),
        ]
    }
}
