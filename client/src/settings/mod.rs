mod sliders;
mod keybinds;
pub mod sensitivity;
pub mod fov;
pub mod crosshair_settings;

use macroquad::prelude::*;
use crate::core::GameConfig;
use crate::ui::fonts::{text_params, measure_game_text};
use crate::weapons::WeaponInventory;

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

    pub fn update(&mut self, config: &mut GameConfig, weapons: &mut WeaponInventory) -> bool {
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let held = is_mouse_button_down(MouseButton::Left);

        // Handle unsaved changes prompt
        if self.show_unsaved_prompt {
            return self.update_unsaved_prompt(config, mouse, clicked);
        }

        // ESC tries to go back
        if is_key_pressed(KeyCode::Escape) {
            if self.has_unsaved_changes(config) {
                self.show_unsaved_prompt = true;
                return false;
            }
            return true;
        }

        // Tab buttons
        let tab_y = 80.0;
        let tab_w = 150.0;
        let tab_h = 40.0;
        let tab_gap = 20.0;
        let tabs = [(SettingsTab::General, "General"), (SettingsTab::Keybinds, "Keybinds"), (SettingsTab::Crosshair, "Crosshair")];

        for (i, (tab, _)) in tabs.iter().enumerate() {
            let tab_x = 20.0 + (tab_w + tab_gap) * (i as f32);
            if clicked && mouse.0 >= tab_x && mouse.0 <= tab_x + tab_w &&
               mouse.1 >= tab_y && mouse.1 <= tab_y + tab_h
            {
                if self.active_tab != *tab && *tab == SettingsTab::Crosshair {
                    self.crosshair_state.reset_visit();
                }
                self.active_tab = *tab;
            }
        }

        let content_top = tab_y + tab_h + 20.0;

        match self.active_tab {
            SettingsTab::General => {
                let gap = 20.0;
                let panel_w = screen_width() - gap * 2.0;
                let slider_x = gap + 160.0;
                let slider_w = panel_w - 220.0;
                let btn_size = 25.0;

                sliders::update_sliders(config, slider_x, slider_w, btn_size, content_top, mouse, clicked, held);
            }
            SettingsTab::Keybinds => {
                let gap = 20.0;
                let panel_w = screen_width() - gap * 2.0;

                keybinds::update_keybinds(config, &mut self.rebinding, gap, content_top, panel_w, mouse, clicked);
            }
            SettingsTab::Crosshair => {
                let gap = 20.0;
                let panel_w = screen_width() - gap * 2.0;
                crosshair_settings::update_crosshair_settings(
                    &mut self.crosshair_state,
                    weapons,
                    gap,
                    content_top,
                    panel_w,
                    screen_height() - content_top - 100.0,
                    mouse,
                    clicked,
                    held,
                );
            }
        }

        // Back button
        let back_y = screen_height() - 60.0;
        let back_w = 150.0;
        let back_x = (screen_width() - back_w) / 2.0;
        if clicked
            && mouse.0 >= back_x && mouse.0 <= back_x + back_w
            && mouse.1 >= back_y && mouse.1 <= back_y + 40.0
        {
            if self.has_unsaved_changes(config) {
                self.show_unsaved_prompt = true;
            } else {
                return true;
            }
        }

        // Save button
        let save_w = 150.0;
        let save_x = back_x + back_w + 20.0;
        let has_dupes = Self::has_duplicates(config);
        if clicked && !has_dupes
            && mouse.0 >= save_x && mouse.0 <= save_x + save_w
            && mouse.1 >= back_y && mouse.1 <= back_y + 40.0
        {
            self.saved_config = Some(config.clone());
        }

        // Reset button
        let reset_w = 150.0;
        let reset_x = back_x - reset_w - 20.0;
        if clicked
            && mouse.0 >= reset_x && mouse.0 <= reset_x + reset_w
            && mouse.1 >= back_y && mouse.1 <= back_y + 40.0
        {
            config.reset();
        }

        false
    }

    fn update_unsaved_prompt(&mut self, config: &mut GameConfig, mouse: (f32, f32), clicked: bool) -> bool {
        let pw = 350.0;
        let ph = 150.0;
        let px = (screen_width() - pw) / 2.0;
        let py = (screen_height() - ph) / 2.0;

        // Draw prompt overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));
        draw_rectangle(px, py, pw, ph, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle_lines(px, py, pw, ph, 2.0, WHITE);

        let msg = "You have unsaved changes!";
        let msg_w = measure_game_text(msg, 20);
        draw_text_ex(msg, px + (pw - msg_w) / 2.0, py + 35.0, text_params(20, YELLOW));

        let btn_w = 150.0;
        let btn_h = 35.0;
        let leave_x = px + 20.0;
        let btn_y = py + ph - btn_h - 20.0;

        // "Leave" button
        draw_rectangle(leave_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
        draw_rectangle_lines(leave_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let lt = "Leave";
        let ltw = measure_game_text(lt, 18);
        draw_text_ex(lt, leave_x + (btn_w - ltw) / 2.0, btn_y + 24.0, text_params(18, WHITE));

        // "Save & Leave" button
        let save_x = px + pw - btn_w - 20.0;
        let has_dupes = Self::has_duplicates(config);
        let save_color = if has_dupes {
            Color::from_rgba(50, 50, 50, 255)
        } else {
            Color::from_rgba(30, 80, 30, 255)
        };
        draw_rectangle(save_x, btn_y, btn_w, btn_h, save_color);
        draw_rectangle_lines(save_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let st = if has_dupes { "Fix duplicates" } else { "Save & Leave" };
        let stw = measure_game_text(st, 18);
        draw_text_ex(
            st,
            save_x + (btn_w - stw) / 2.0,
            btn_y + 24.0,
            text_params(18, if has_dupes { GRAY } else { WHITE }),
        );

        if clicked {
            // Leave without saving
            if mouse.0 >= leave_x && mouse.0 <= leave_x + btn_w
                && mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
            {
                if let Some(saved) = &self.saved_config {
                    *config = saved.clone();
                }
                self.show_unsaved_prompt = false;
                return true;
            }
            // Save & Leave
            if !has_dupes
                && mouse.0 >= save_x && mouse.0 <= save_x + btn_w
                && mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
            {
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

    pub fn draw(&self, config: &GameConfig, weapons: &WeaponInventory, assets: &std::collections::HashMap<String, Texture2D>) {
        // Title
        let title = "SETTINGS";
        let title_w = measure_game_text(title, 30);
        draw_text_ex(title, (screen_width() - title_w) / 2.0, 50.0, text_params(30, WHITE));

        // Tab buttons
        let tab_y = 80.0;
        let tab_w = 150.0;
        let tab_h = 40.0;
        let tab_gap = 20.0;
        let tabs = [(SettingsTab::General, "General"), (SettingsTab::Keybinds, "Keybinds"), (SettingsTab::Crosshair, "Crosshair")];
        let mouse = mouse_position();

        for (i, (tab, name)) in tabs.iter().enumerate() {
            let tab_x = 20.0 + (tab_w + tab_gap) * (i as f32);

            let tab_color = if *tab == self.active_tab {
                Color::from_rgba(80, 150, 220, 255)
            } else {
                Color::from_rgba(40, 40, 40, 255)
            };

            draw_rectangle(tab_x, tab_y, tab_w, tab_h, tab_color);
            draw_rectangle_lines(tab_x, tab_y, tab_w, tab_h, 1.0, GRAY);

            let text_w = measure_game_text(name, 18);
            draw_text_ex(
                name,
                tab_x + (tab_w - text_w) / 2.0,
                tab_y + 26.0,
                text_params(18, WHITE),
            );
        }

        let content_top = tab_y + tab_h + 20.0;

        match self.active_tab {
            SettingsTab::General => {
                let gap = 20.0;
                let panel_w = screen_width() - gap * 2.0;
                let panel_h = screen_height() - content_top - 70.0;

                draw_rectangle(gap, content_top, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
                draw_rectangle_lines(gap, content_top, panel_w, panel_h, 1.0, GRAY);

                let section_title = "General";
                let section_w = measure_game_text(section_title, 22);
                draw_text_ex(
                    section_title,
                    gap + (panel_w - section_w) / 2.0,
                    content_top + 30.0,
                    text_params(22, YELLOW),
                );

                let slider_x = gap + 160.0;
                let slider_w = panel_w - 220.0;
                let btn_size = 25.0;
                let label_x = gap + 20.0;

                sliders::draw_sliders(config, label_x, slider_x, slider_w, btn_size, content_top);
            }
            SettingsTab::Keybinds => {
                let gap = 20.0;
                let panel_w = screen_width() - gap * 2.0;
                let panel_h = screen_height() - content_top - 70.0;

                keybinds::draw_keybinds(config, &self.rebinding, gap, panel_w, content_top, panel_h);
            }
            SettingsTab::Crosshair => {
                let gap = 20.0;
                let panel_w = screen_width() - gap * 2.0;
                crosshair_settings::draw_crosshair_settings(
                    &self.crosshair_state,
                    weapons,
                    gap,
                    content_top,
                    panel_w,
                    screen_height() - content_top - 100.0,
                    assets,
                    mouse,
                );
            }
        }

        // Bottom buttons
        self.draw_bottom_buttons(config);
    }

    fn draw_bottom_buttons(&self, config: &GameConfig) {
        let back_y = screen_height() - 60.0;
        let back_w = 150.0;
        let back_x = (screen_width() - back_w) / 2.0;

        // Back button
        draw_rectangle(back_x, back_y, back_w, 40.0, Color::from_rgba(50, 50, 50, 255));
        draw_rectangle_lines(back_x, back_y, back_w, 40.0, 1.0, WHITE);
        let back_tw = measure_game_text("Back", 20);
        draw_text_ex("Back", back_x + (back_w - back_tw) / 2.0, back_y + 26.0, text_params(20, WHITE));

        // Reset button
        let reset_x = back_x - back_w - 20.0;
        draw_rectangle(reset_x, back_y, back_w, 40.0, Color::from_rgba(80, 60, 20, 255));
        draw_rectangle_lines(reset_x, back_y, back_w, 40.0, 1.0, WHITE);
        let reset_tw = measure_game_text("Reset", 20);
        draw_text_ex("Reset", reset_x + (back_w - reset_tw) / 2.0, back_y + 26.0, text_params(20, WHITE));

        // Save button
        let save_x = back_x + back_w + 20.0;
        let has_dupes = Self::has_duplicates(config);
        let has_changes = self.has_unsaved_changes(config);
        let save_color = if has_dupes {
            Color::from_rgba(50, 50, 50, 255)
        } else if has_changes {
            Color::from_rgba(30, 100, 30, 255)
        } else {
            Color::from_rgba(40, 60, 40, 255)
        };
        draw_rectangle(save_x, back_y, back_w, 40.0, save_color);
        draw_rectangle_lines(
            save_x, back_y, back_w, 40.0, 1.0,
            if has_changes && !has_dupes { GREEN } else { WHITE },
        );
        let save_label = if has_dupes { "Fix dupes" } else { "Save" };
        let save_tw = measure_game_text(save_label, 20);
        draw_text_ex(
            save_label,
            save_x + (back_w - save_tw) / 2.0,
            back_y + 26.0,
            text_params(20, if has_dupes { GRAY } else { WHITE }),
        );
    }
}
