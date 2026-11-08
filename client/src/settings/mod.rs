pub mod crosshair_settings;
mod draw;
pub mod fov;
mod keybinds;
pub mod sensitivity;
mod sliders;
mod update;

use crate::core::GameConfig;
use crate::weapons::WeaponInventory;
use macroquad::prelude::*;

#[derive(Copy, Clone, PartialEq)]
pub enum SettingsTab {
    General,
    Keybinds,
    Crosshair,
}

pub struct SettingsScreen {
    pub rebinding: Option<String>,
    pub saved_config: Option<GameConfig>,
    pub show_unsaved_prompt: bool,
    pub active_tab: SettingsTab,
    pub crosshair_state: crosshair_settings::CrosshairSettingsPreview,
}

/// Compute centered, width-capped panel dimensions.
fn panel_layout(s: f32, content_top: f32) -> (f32, f32, f32) {
    let max_w = 800.0 * s;
    let panel_w = (screen_width() - 40.0 * s).min(max_w);
    let panel_x = (screen_width() - panel_w) / 2.0;
    let panel_h = screen_height() - content_top - 70.0 * s;
    (panel_x, panel_w, panel_h)
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            rebinding: None,
            saved_config: None,
            show_unsaved_prompt: false,
            active_tab: SettingsTab::General,
            crosshair_state: crosshair_settings::CrosshairSettingsPreview::new(),
        }
    }

    pub fn enter(&mut self, config: &GameConfig) {
        self.saved_config = Some(config.clone());
        self.show_unsaved_prompt = false;
        self.crosshair_state.reset_visit();
    }

    fn has_duplicates(config: &GameConfig) -> bool {
        let bindings = config.all_bindings();
        for i in 0..bindings.len() {
            for j in (i + 1)..bindings.len() {
                if bindings[i].1 == bindings[j].1 {
                    return true;
                }
            }
        }
        false
    }

    fn has_unsaved_changes(&self, config: &GameConfig) -> bool {
        match &self.saved_config {
            Some(saved) => saved != config,
            None => false,
        }
    }

    /// Try to leave the crosshair tab. If there are unsaved changes, show
    /// the save prompt and return false. Otherwise return true (caller can navigate).
    fn try_leave_crosshair(
        &mut self,
        weapons: &WeaponInventory,
        pending_tab: Option<u8>,
    ) -> bool {
        if self.crosshair_state.has_crosshair_changes(weapons) {
            self.crosshair_state.show_save_prompt = true;
            self.crosshair_state.save_prompt_is_leave = true;
            self.crosshair_state.pending_tab = pending_tab;
            false
        } else {
            true
        }
    }
}
