pub mod crosshair_settings;
pub mod fov;
mod keybinds;
pub mod sensitivity;
mod sliders;

use crate::core::GameConfig;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair_presets::CrosshairPresetManager;
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

    pub fn update(
        &mut self,
        config: &mut GameConfig,
        weapons: &mut WeaponInventory,
        presets: &mut CrosshairPresetManager,
    ) -> bool {
        let s = config.gui_scale_factor();
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let held = is_mouse_button_down(MouseButton::Left);

        // Handle crosshair save prompt (fullscreen modal, blocks everything)
        if self.active_tab == SettingsTab::Crosshair && self.crosshair_state.show_save_prompt {
            if let Some(should_leave) = crosshair_settings::update_crosshair_save_prompt(
                &mut self.crosshair_state,
                weapons,
                presets,
            ) {
                if should_leave {
                    if let Some(tab_id) = self.crosshair_state.pending_tab.take() {
                        self.active_tab = match tab_id {
                            0 => SettingsTab::General,
                            1 => SettingsTab::Keybinds,
                            _ => SettingsTab::General,
                        };
                        return false;
                    }
                    return true;
                }
                if let Some(tab_id) = self.crosshair_state.pending_tab.take() {
                    self.active_tab = match tab_id {
                        0 => SettingsTab::General,
                        1 => SettingsTab::Keybinds,
                        _ => SettingsTab::General,
                    };
                }
            }
            return false;
        }

        // Handle unsaved changes prompt
        if self.show_unsaved_prompt {
            return self.update_unsaved_prompt(config, mouse, clicked, s);
        }

        // ESC tries to go back
        if is_key_pressed(KeyCode::Escape) {
            if self.active_tab == SettingsTab::Crosshair {
                if self.try_leave_crosshair(weapons, None) {
                    return true;
                }
                return false;
            }
            if self.has_unsaved_changes(config) {
                self.show_unsaved_prompt = true;
                return false;
            }
            return true;
        }

        // Tab buttons
        let tab_y = 80.0 * s;
        let tab_w = 120.0 * s;
        let tab_h = 36.0 * s;
        let tab_gap = 10.0 * s;
        let tabs = [
            (SettingsTab::General, "General"),
            (SettingsTab::Keybinds, "Keybinds"),
            (SettingsTab::Crosshair, "Crosshair"),
        ];
        let total_tabs_w =
            tab_w * tabs.len() as f32 + tab_gap * (tabs.len() as f32 - 1.0);
        let tabs_start_x = (screen_width() - total_tabs_w) / 2.0;

        for (i, (tab, _)) in tabs.iter().enumerate() {
            let tab_x = tabs_start_x + (tab_w + tab_gap) * (i as f32);
            if clicked
                && mouse.0 >= tab_x
                && mouse.0 <= tab_x + tab_w
                && mouse.1 >= tab_y
                && mouse.1 <= tab_y + tab_h
            {
                if self.active_tab != *tab {
                    if self.active_tab == SettingsTab::Crosshair {
                        let pending = match *tab {
                            SettingsTab::General => 0u8,
                            SettingsTab::Keybinds => 1u8,
                            SettingsTab::Crosshair => 2u8,
                        };
                        if self.try_leave_crosshair(weapons, Some(pending)) {
                            // No changes, switch directly
                            self.active_tab = *tab;
                        }
                        break;
                    }
                    if *tab == SettingsTab::Crosshair {
                        self.crosshair_state.reset_visit();
                    }
                    self.active_tab = *tab;
                }
            }
        }

        let content_top = tab_y + tab_h + 15.0 * s;

        // Snapshot crosshairs on first frame of crosshair tab
        if self.active_tab == SettingsTab::Crosshair
            && self.crosshair_state.saved_crosshairs.is_none()
        {
            self.crosshair_state.snapshot_crosshairs(weapons);
        }

        let (panel_x, panel_w, panel_h) = panel_layout(s, content_top);

        match self.active_tab {
            SettingsTab::General => {
                let slider_x = panel_x + 130.0 * s;
                let slider_w = panel_w - 250.0 * s;
                let btn_size = 25.0 * s;

                sliders::update_sliders(
                    config,
                    slider_x,
                    slider_w,
                    btn_size,
                    content_top,
                    mouse,
                    clicked,
                    held,
                    s,
                );
            }
            SettingsTab::Keybinds => {
                keybinds::update_keybinds(
                    config,
                    &mut self.rebinding,
                    panel_x,
                    content_top,
                    panel_w,
                    mouse,
                    clicked,
                    s,
                );
            }
            SettingsTab::Crosshair => {
                crosshair_settings::update_crosshair_settings(
                    &mut self.crosshair_state,
                    weapons,
                    presets,
                    panel_x,
                    content_top,
                    panel_w,
                    panel_h,
                    mouse,
                    clicked,
                    held,
                    s,
                );
            }
        }

        // Back button
        let back_y = screen_height() - 60.0 * s;
        let back_w = 120.0 * s;
        let back_h = 36.0 * s;
        let back_x = (screen_width() - back_w) / 2.0;
        if clicked
            && mouse.0 >= back_x
            && mouse.0 <= back_x + back_w
            && mouse.1 >= back_y
            && mouse.1 <= back_y + back_h
        {
            if self.active_tab == SettingsTab::Crosshair {
                if self.try_leave_crosshair(weapons, None) {
                    return true;
                }
            } else if self.has_unsaved_changes(config) {
                self.show_unsaved_prompt = true;
            } else {
                return true;
            }
        }

        // Save button
        let save_w = 120.0 * s;
        let save_x = back_x + back_w + 15.0 * s;
        let has_dupes = Self::has_duplicates(config);
        if clicked
            && !has_dupes
            && mouse.0 >= save_x
            && mouse.0 <= save_x + save_w
            && mouse.1 >= back_y
            && mouse.1 <= back_y + back_h
        {
            if self.active_tab == SettingsTab::Crosshair {
                self.crosshair_state.show_save_prompt = true;
                self.crosshair_state.save_prompt_is_leave = false;
                self.crosshair_state.pending_tab = None;
            } else {
                config.save();
                self.saved_config = Some(config.clone());
            }
        }

        // Reset button
        let reset_w = 120.0 * s;
        let reset_x = back_x - reset_w - 15.0 * s;
        if clicked
            && mouse.0 >= reset_x
            && mouse.0 <= reset_x + reset_w
            && mouse.1 >= back_y
            && mouse.1 <= back_y + back_h
        {
            config.reset();
        }

        false
    }

    fn update_unsaved_prompt(
        &mut self,
        config: &mut GameConfig,
        mouse: (f32, f32),
        clicked: bool,
        s: f32,
    ) -> bool {
        let pw = 350.0 * s;
        let ph = 150.0 * s;
        let px = (screen_width() - pw) / 2.0;
        let py = (screen_height() - ph) / 2.0;

        // Draw prompt overlay
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 150),
        );
        draw_rectangle(px, py, pw, ph, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle_lines(px, py, pw, ph, 2.0, WHITE);

        let msg_fs = (20.0 * s) as u16;
        let msg = "You have unsaved changes!";
        let msg_w = measure_game_text(msg, msg_fs);
        draw_text_ex(
            msg,
            px + (pw - msg_w) / 2.0,
            py + 35.0 * s,
            text_params(msg_fs, YELLOW),
        );

        let btn_w = 140.0 * s;
        let btn_h = 35.0 * s;
        let leave_x = px + 20.0 * s;
        let btn_y = py + ph - btn_h - 20.0 * s;
        let btn_fs = (18.0 * s) as u16;

        // "Leave" button
        draw_rectangle(
            leave_x,
            btn_y,
            btn_w,
            btn_h,
            Color::from_rgba(80, 30, 30, 255),
        );
        draw_rectangle_lines(leave_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let lt = "Leave";
        let ltw = measure_game_text(lt, btn_fs);
        draw_text_ex(
            lt,
            leave_x + (btn_w - ltw) / 2.0,
            btn_y + btn_h * 0.68,
            text_params(btn_fs, WHITE),
        );

        // "Save & Leave" button
        let save_x = px + pw - btn_w - 20.0 * s;
        let has_dupes = Self::has_duplicates(config);
        let save_color = if has_dupes {
            Color::from_rgba(50, 50, 50, 255)
        } else {
            Color::from_rgba(30, 80, 30, 255)
        };
        draw_rectangle(save_x, btn_y, btn_w, btn_h, save_color);
        draw_rectangle_lines(save_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let st = if has_dupes {
            "Fix duplicates"
        } else {
            "Save & Leave"
        };
        let stw = measure_game_text(st, btn_fs);
        draw_text_ex(
            st,
            save_x + (btn_w - stw) / 2.0,
            btn_y + btn_h * 0.68,
            text_params(btn_fs, if has_dupes { GRAY } else { WHITE }),
        );

        if clicked {
            if mouse.0 >= leave_x
                && mouse.0 <= leave_x + btn_w
                && mouse.1 >= btn_y
                && mouse.1 <= btn_y + btn_h
            {
                if let Some(saved) = &self.saved_config {
                    *config = saved.clone();
                }
                self.show_unsaved_prompt = false;
                return true;
            }
            if !has_dupes
                && mouse.0 >= save_x
                && mouse.0 <= save_x + btn_w
                && mouse.1 >= btn_y
                && mouse.1 <= btn_y + btn_h
            {
                config.save();
                self.saved_config = Some(config.clone());
                self.show_unsaved_prompt = false;
                return true;
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            self.show_unsaved_prompt = false;
        }

        false
    }

    fn draw_header(&self, s: f32) {
        let title_fs = (28.0 * s) as u16;
        let title = "SETTINGS";
        let title_w = measure_game_text(title, title_fs);
        draw_text_ex(
            title,
            (screen_width() - title_w) / 2.0,
            45.0 * s,
            text_params(title_fs, WHITE),
        );

        let tab_y = 80.0 * s;
        let tab_w = 120.0 * s;
        let tab_h = 36.0 * s;
        let tab_gap = 10.0 * s;
        let tab_fs = (16.0 * s) as u16;
        let tabs = [
            (SettingsTab::General, "General"),
            (SettingsTab::Keybinds, "Keybinds"),
            (SettingsTab::Crosshair, "Crosshair"),
        ];
        let total_tabs_w =
            tab_w * tabs.len() as f32 + tab_gap * (tabs.len() as f32 - 1.0);
        let tabs_start_x = (screen_width() - total_tabs_w) / 2.0;

        for (i, (tab, name)) in tabs.iter().enumerate() {
            let tab_x = tabs_start_x + (tab_w + tab_gap) * (i as f32);
            let tab_color = if *tab == self.active_tab {
                Color::from_rgba(80, 150, 220, 255)
            } else {
                Color::from_rgba(40, 40, 40, 255)
            };
            draw_rectangle(tab_x, tab_y, tab_w, tab_h, tab_color);
            draw_rectangle_lines(tab_x, tab_y, tab_w, tab_h, 2.0, WHITE);
            let text_w = measure_game_text(name, tab_fs);
            draw_text_ex(
                name,
                tab_x + (tab_w - text_w) / 2.0,
                tab_y + tab_h * 0.65,
                text_params(tab_fs, WHITE),
            );
        }
    }

    pub fn draw(
        &self,
        config: &GameConfig,
        weapons: &WeaponInventory,
        presets: &CrosshairPresetManager,
        assets: &std::collections::HashMap<String, Texture2D>,
    ) {
        let s = config.gui_scale_factor();

        // Full-screen dark overlay matching pause menu style
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 180),
        );

        self.draw_header(s);

        let tab_h = 36.0 * s;
        let content_top = 80.0 * s + tab_h + 15.0 * s;
        let (panel_x, panel_w, panel_h) = panel_layout(s, content_top);

        match self.active_tab {
            SettingsTab::General => {
                draw_rectangle(
                    panel_x,
                    content_top,
                    panel_w,
                    panel_h,
                    Color::from_rgba(30, 30, 30, 240),
                );
                draw_rectangle_lines(panel_x, content_top, panel_w, panel_h, 2.0, WHITE);

                let section_fs = (20.0 * s) as u16;
                let section_title = "General";
                let section_w = measure_game_text(section_title, section_fs);
                draw_text_ex(
                    section_title,
                    panel_x + (panel_w - section_w) / 2.0,
                    content_top + 30.0 * s,
                    text_params(section_fs, YELLOW),
                );

                let slider_x = panel_x + 130.0 * s;
                let slider_w = panel_w - 250.0 * s;
                let btn_size = 25.0 * s;
                let label_x = panel_x + 20.0 * s;

                sliders::draw_sliders(
                    config, label_x, slider_x, slider_w, btn_size, content_top, s,
                );
            }
            SettingsTab::Keybinds => {
                keybinds::draw_keybinds(
                    config,
                    &self.rebinding,
                    panel_x,
                    panel_w,
                    content_top,
                    panel_h,
                    s,
                );
            }
            SettingsTab::Crosshair => {
                // Draw background for crosshair panel
                draw_rectangle(
                    panel_x,
                    content_top,
                    panel_w,
                    panel_h,
                    Color::from_rgba(30, 30, 30, 240),
                );

                crosshair_settings::draw_crosshair_settings(
                    &self.crosshair_state,
                    weapons,
                    presets,
                    panel_x,
                    content_top,
                    panel_w,
                    panel_h,
                    assets,
                    mouse_position(),
                    s,
                );

                // Clip overflow above and below the panel (full-screen width)
                let clip_color = Color::from_rgba(0, 0, 0, 180);
                draw_rectangle(0.0, 0.0, screen_width(), content_top, clip_color);
                let panel_bottom = content_top + panel_h;
                draw_rectangle(
                    0.0,
                    panel_bottom,
                    screen_width(),
                    screen_height() - panel_bottom,
                    clip_color,
                );

                // Redraw title and tabs over the top mask
                self.draw_header(s);

                // Redraw panel border
                draw_rectangle_lines(panel_x, content_top, panel_w, panel_h, 2.0, WHITE);
            }
        }

        // Bottom buttons
        self.draw_bottom_buttons(config, s);

        // Draw save prompt overlay on top of everything
        if self.active_tab == SettingsTab::Crosshair && self.crosshair_state.show_save_prompt {
            crosshair_settings::draw_crosshair_save_prompt(&self.crosshair_state, presets);
        }
    }

    fn draw_bottom_buttons(&self, config: &GameConfig, s: f32) {
        let back_y = screen_height() - 60.0 * s;
        let back_w = 120.0 * s;
        let back_h = 36.0 * s;
        let back_x = (screen_width() - back_w) / 2.0;
        let btn_fs = (18.0 * s) as u16;

        // Back button
        draw_rectangle(
            back_x,
            back_y,
            back_w,
            back_h,
            Color::from_rgba(50, 50, 50, 255),
        );
        draw_rectangle_lines(back_x, back_y, back_w, back_h, 1.0, WHITE);
        let back_tw = measure_game_text("Back", btn_fs);
        draw_text_ex(
            "Back",
            back_x + (back_w - back_tw) / 2.0,
            back_y + back_h * 0.65,
            text_params(btn_fs, WHITE),
        );

        // Reset button
        let reset_x = back_x - back_w - 15.0 * s;
        draw_rectangle(
            reset_x,
            back_y,
            back_w,
            back_h,
            Color::from_rgba(80, 60, 20, 255),
        );
        draw_rectangle_lines(reset_x, back_y, back_w, back_h, 1.0, WHITE);
        let reset_tw = measure_game_text("Reset", btn_fs);
        draw_text_ex(
            "Reset",
            reset_x + (back_w - reset_tw) / 2.0,
            back_y + back_h * 0.65,
            text_params(btn_fs, WHITE),
        );

        // Save button
        let save_x = back_x + back_w + 15.0 * s;
        let has_dupes = Self::has_duplicates(config);
        let has_changes = self.has_unsaved_changes(config);
        let save_color = if has_dupes {
            Color::from_rgba(50, 50, 50, 255)
        } else if has_changes {
            Color::from_rgba(30, 100, 30, 255)
        } else {
            Color::from_rgba(40, 60, 40, 255)
        };
        draw_rectangle(save_x, back_y, back_w, back_h, save_color);
        draw_rectangle_lines(
            save_x,
            back_y,
            back_w,
            back_h,
            1.0,
            if has_changes && !has_dupes {
                GREEN
            } else {
                WHITE
            },
        );
        let save_label = if has_dupes { "Fix dupes" } else { "Save" };
        let save_tw = measure_game_text(save_label, btn_fs);
        draw_text_ex(
            save_label,
            save_x + (back_w - save_tw) / 2.0,
            back_y + back_h * 0.65,
            text_params(btn_fs, if has_dupes { GRAY } else { WHITE }),
        );
    }
}
