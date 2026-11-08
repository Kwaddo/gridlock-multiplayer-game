use crate::core::GameConfig;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use crate::weapons::WeaponInventory;
use macroquad::prelude::*;

use super::{crosshair_settings, keybinds, panel_layout, sliders, SettingsScreen, SettingsTab};

impl SettingsScreen {
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
        let tabs = self.tabs();
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
