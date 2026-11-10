use crate::core::GameConfig;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use crate::weapons::WeaponInventory;
use macroquad::prelude::*;

use super::{crosshair_settings, keybinds, panel_layout, sliders, SettingsScreen, SettingsTab};

impl SettingsScreen {
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
        let tabs = self.tabs();
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
}
