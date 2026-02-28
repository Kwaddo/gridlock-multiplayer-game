/*
    Module in the lobby screen where players can select weapons to have in their
    inventory, its cause we have so many weapons that we need a better way to manage them.
    It will also show weapon description and stats in the future
*/

use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::loadout::{Loadout, LOADOUT_SLOTS};
use crate::weapons::stats::{WeaponCategory, WeaponId};
use macroquad::prelude::*;

// colors matching the login screen style
const PANEL_BG: Color = Color::new(0.09, 0.11, 0.19, 0.95);
const PANEL_BORDER: Color = Color::new(0.20, 0.24, 0.40, 0.7);
const ACCENT: Color = Color::new(1.0, 0.42, 0.21, 1.0);
const ACCENT_HOVER: Color = Color::new(1.0, 0.55, 0.38, 1.0);
const TEXT_DIM: Color = Color::new(0.50, 0.52, 0.62, 1.0);
const TEXT_LABEL: Color = Color::new(0.72, 0.74, 0.84, 1.0);
const SLOT_BG: Color = Color::new(0.06, 0.07, 0.13, 1.0);
const SLOT_SELECTED: Color = Color::new(1.0, 0.42, 0.21, 0.25);
const ROW_BG: Color = Color::new(0.07, 0.08, 0.15, 1.0);
const ROW_HOVER: Color = Color::new(0.12, 0.14, 0.24, 1.0);
const CATEGORY_HEADER: Color = Color::new(0.85, 0.65, 0.30, 1.0);

const SLOT_LABELS: [&str; LOADOUT_SLOTS] = ["PRIMARY", "SECONDARY"];

/// Which weapon categories are valid for each slot.
fn allowed_categories(slot: usize) -> &'static [WeaponCategory] {
    match slot {
        0 => &[WeaponCategory::Shotgun, WeaponCategory::AssaultRifle, WeaponCategory::Sniper],
        1 => &[WeaponCategory::Pistol],
        _ => &[],
    }
}

pub struct LobbyInventoryScreen {
    pub selected_slot: Option<usize>, // which slot (0-1) the player clicked to assign into
    pub scroll_offset: f32,
}

impl LobbyInventoryScreen {
    pub fn new() -> Self {
        Self {
            selected_slot: None,
            scroll_offset: 0.0,
        }
    }

    // returns true if the user wants to go back to login.
    pub fn update(&mut self, loadout: &mut Loadout) -> bool {
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let sw = screen_width();
        let sh = screen_height();

        // scroll
        let (_, wheel_y) = mouse_wheel();
        self.scroll_offset = (self.scroll_offset - wheel_y * 30.0).max(0.0);

        // ESC to go back
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }

        // --- layout ---
        let panel_w = 600.0_f32.min(sw * 0.9);
        let panel_h = sh * 0.8;
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = (sh - panel_h) / 2.0;

        // --- back button ---
        let back_w = 100.0;
        let back_h = 34.0;
        let back_x = panel_x + 12.0;
        let back_y = panel_y + 8.0;
        let back_hover = in_rect(mouse, back_x, back_y, back_w, back_h);
        if back_hover && clicked {
            return true;
        }

        // --- slots row (clickable) ---
        let slots_y = panel_y + 54.0;
        let slot_w = (panel_w - 40.0 - (LOADOUT_SLOTS as f32 - 1.0) * 8.0) / LOADOUT_SLOTS as f32;
        let slot_h = 52.0;

        for i in 0..LOADOUT_SLOTS {
            let sx = panel_x + 20.0 + (slot_w + 8.0) * i as f32;
            if in_rect(mouse, sx, slots_y, slot_w, slot_h) && clicked {
                self.selected_slot = if self.selected_slot == Some(i) {
                    None // deselect
                } else {
                    Some(i)
                };
                // reset scroll when switching slots so valid weapons are visible
                self.scroll_offset = 0.0;
            }
        }

        // --- weapon list (clickable) ---
        let list_y = slots_y + slot_h + 16.0;
        let list_h = panel_y + panel_h - list_y - 12.0;
        let row_h = 32.0;
        let header_h = 28.0;

        // only process clicks if a slot is selected
        if let Some(slot) = self.selected_slot {
            let allowed = allowed_categories(slot);
            let mut y = list_y - self.scroll_offset;
            for cat in &WeaponCategory::ALL {
                y += header_h; // skip header
                if !allowed.contains(cat) {
                    // skip weapons in this category (they're dimmed/unclickable)
                    y += WeaponId::all_in(*cat).len() as f32 * row_h;
                    continue;
                }
                let weapons = WeaponId::all_in(*cat);
                for weapon_id in weapons {
                    if y + row_h > list_y && y < list_y + list_h {
                        let row_x = panel_x + 16.0;
                        let row_w = panel_w - 32.0;
                        if in_rect(mouse, row_x, y, row_w, row_h) && clicked {
                            loadout.assign(slot, weapon_id);
                            self.selected_slot = None; // deselect after assigning
                        }
                    }
                    y += row_h;
                }
            }
        }

        // clamp scroll
        let total_content = self.calc_total_height();
        let list_y_start = slots_y + slot_h + 16.0;
        let visible_h = panel_y + panel_h - list_y_start - 12.0;
        let max_scroll = (total_content - visible_h).max(0.0);
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        false
    }

    pub fn draw(&self, loadout: &Loadout) {
        let mouse = mouse_position();
        let sw = screen_width();
        let sh = screen_height();

        // dim background
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));

        let panel_w = 600.0_f32.min(sw * 0.9);
        let panel_h = sh * 0.8;
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = (sh - panel_h) / 2.0;

        // panel
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        // title
        let title = "LOADOUT";
        let tw = measure_game_text(title, 22);
        draw_text_ex(
            title,
            panel_x + (panel_w - tw) / 2.0,
            panel_y + 32.0,
            text_params(22, WHITE),
        );

        // back button
        let back_w = 100.0;
        let back_h = 34.0;
        let back_x = panel_x + 12.0;
        let back_y = panel_y + 8.0;
        let back_hover = in_rect(mouse, back_x, back_y, back_w, back_h);
        draw_rectangle(
            back_x,
            back_y,
            back_w,
            back_h,
            if back_hover {
                ACCENT_HOVER
            } else {
                Color::new(0.15, 0.17, 0.28, 1.0)
            },
        );
        let bt = "< BACK";
        let btw = measure_game_text(bt, 15);
        draw_text_ex(
            bt,
            back_x + (back_w - btw) / 2.0,
            back_y + 23.0,
            text_params(15, WHITE),
        );

        // --- slots row ---
        let slots_y = panel_y + 54.0;
        let slot_w = (panel_w - 40.0 - (LOADOUT_SLOTS as f32 - 1.0) * 8.0) / LOADOUT_SLOTS as f32;
        let slot_h = 52.0;

        for i in 0..LOADOUT_SLOTS {
            let sx = panel_x + 20.0 + (slot_w + 8.0) * i as f32;
            let is_selected = self.selected_slot == Some(i);
            let hover = in_rect(mouse, sx, slots_y, slot_w, slot_h);

            let bg = if is_selected {
                SLOT_SELECTED
            } else if hover {
                ROW_HOVER
            } else {
                SLOT_BG
            };
            draw_rectangle(sx, slots_y, slot_w, slot_h, bg);
            draw_rectangle_lines(
                sx,
                slots_y,
                slot_w,
                slot_h,
                if is_selected { 2.0 } else { 1.0 },
                if is_selected { ACCENT } else { PANEL_BORDER },
            );

            // slot label (PRIMARY / SECONDARY)
            let label = SLOT_LABELS[i];
            let lw = measure_game_text(label, 11);
            draw_text_ex(label, sx + (slot_w - lw) / 2.0, slots_y + 16.0, text_params(11, TEXT_DIM));

            // weapon name
            let name = loadout.slots[i].display_name();
            let nw = measure_game_text(name, 13);
            let name_x = sx + (slot_w - nw) / 2.0;
            draw_text_ex(name, name_x, slots_y + 36.0, text_params(13, WHITE));
        }

        // hint
        let hint = if let Some(slot) = self.selected_slot {
            if slot == 0 {
                "Select a Shotgun, Assault Rifle, or Sniper"
            } else {
                "Select a Pistol"
            }
        } else {
            "Click a slot above to change its weapon"
        };
        let hw = measure_game_text(hint, 12);
        draw_text_ex(
            hint,
            panel_x + (panel_w - hw) / 2.0,
            slots_y + slot_h + 12.0,
            text_params(12, TEXT_DIM),
        );

        // --- weapon list ---
        let list_y = slots_y + slot_h + 16.0;
        let list_h = panel_y + panel_h - list_y - 12.0;
        let row_h = 32.0;
        let header_h = 28.0;

        // clip to list area using scissor
        // macroquad doesn't have scissor, so we just skip drawing outside bounds
        let mut y = list_y - self.scroll_offset;

        // track which weapons are currently in the loadout for highlighting
        let loadout_set: Vec<WeaponId> = loadout.slots.to_vec();

        // which categories are valid for the currently selected slot (None = show all dimmed)
        let active_allowed: Option<&[WeaponCategory]> =
            self.selected_slot.map(|s| allowed_categories(s));

        for cat in &WeaponCategory::ALL {
            let weapons = WeaponId::all_in(*cat);
            let cat_allowed = active_allowed.map_or(true, |allowed| allowed.contains(cat));

            // category header
            if y + header_h > list_y && y < list_y + list_h {
                let display_y = y.max(list_y);
                if display_y == y {
                    let header_color = if cat_allowed || active_allowed.is_none() {
                        CATEGORY_HEADER
                    } else {
                        Color::new(0.40, 0.35, 0.20, 1.0) // dimmed
                    };
                    draw_text_ex(
                        cat.display_name(),
                        panel_x + 20.0,
                        y + 20.0,
                        text_params(14, header_color),
                    );
                    draw_rectangle(
                        panel_x + 20.0,
                        y + header_h - 2.0,
                        panel_w - 40.0,
                        1.0,
                        Color::new(0.85, 0.65, 0.30, if cat_allowed { 0.3 } else { 0.1 }),
                    );
                }
            }
            y += header_h;

            for weapon_id in weapons {
                if y + row_h > list_y && y < list_y + list_h {
                    let row_x = panel_x + 16.0;
                    let row_w = panel_w - 32.0;
                    let can_click = cat_allowed && self.selected_slot.is_some();
                    let hover = in_rect(mouse, row_x, y, row_w, row_h) && can_click;
                    let in_loadout = loadout_set.contains(&weapon_id);

                    let bg = if hover {
                        ROW_HOVER
                    } else if in_loadout && cat_allowed {
                        Color::new(0.10, 0.12, 0.22, 1.0)
                    } else {
                        ROW_BG
                    };
                    draw_rectangle(row_x, y, row_w, row_h, bg);

                    let name: &str = weapon_id.display_name();
                    let text_color = if !cat_allowed && active_allowed.is_some() {
                        Color::new(0.35, 0.35, 0.42, 1.0) // dimmed / unavailable
                    } else if in_loadout {
                        ACCENT
                    } else {
                        TEXT_LABEL
                    };
                    draw_text_ex(name, row_x + 12.0, y + 21.0, text_params(14, text_color));

                    // show which slot it's in
                    if let Some(slot_idx) = loadout.slots.iter().position(|&w| w == weapon_id) {
                        let tag = format!("[{}]", SLOT_LABELS[slot_idx]);
                        let tag_w = measure_game_text(&tag, 12);
                        draw_text_ex(
                            &tag,
                            row_x + row_w - tag_w - 8.0,
                            y + 20.0,
                            text_params(12, TEXT_DIM),
                        );
                    }
                }
                y += row_h;
            }
        }

        // footer hint
        let footer = "ESC to go back";
        let fw = measure_game_text(footer, 12);
        draw_text_ex(
            footer,
            panel_x + (panel_w - fw) / 2.0,
            panel_y + panel_h - 4.0,
            text_params(12, TEXT_DIM),
        );
    }

    fn calc_total_height(&self) -> f32 {
        let row_h = 32.0;
        let header_h = 28.0;
        let mut total = 0.0;
        for cat in &WeaponCategory::ALL {
            total += header_h;
            total += WeaponId::all_in(*cat).len() as f32 * row_h;
        }
        total
    }
}

fn in_rect(mouse: (f32, f32), x: f32, y: f32, w: f32, h: f32) -> bool {
    mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h
}
