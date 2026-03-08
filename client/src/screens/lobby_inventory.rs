/*
    Full-screen loadout screen with weapon stat bars and 3D model preview.
    Left panel: slot selection + scrollable weapon list by category.
    Right panel: weapon name, stat bars, and interactive 3D weapon viewer.
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
const STAT_BAR_BG: Color = Color::new(0.06, 0.07, 0.13, 1.0);

const SLOT_LABELS: [&str; LOADOUT_SLOTS] = ["PRIMARY", "SECONDARY"];

/// Which weapon categories are valid for each slot.
fn allowed_categories(slot: usize) -> &'static [WeaponCategory] {
    match slot {
        0 => &[WeaponCategory::Shotgun, WeaponCategory::AssaultRifle, WeaponCategory::Sniper, WeaponCategory::Smg],
        1 => &[WeaponCategory::Pistol],
        _ => &[],
    }
}

struct StatBar {
    label: &'static str,
    /// Normalized 0.0..1.0
    value: f32,
}

const STAT_SEGMENTS: u32 = 10;

fn compute_stat_bars(weapon_id: WeaponId) -> Vec<StatBar> {
    let s = weapon_id.stats();
    let eff_damage = s.damage * s.pellet_count as f32;

    vec![
        StatBar { label: "DAMAGE",    value: ((eff_damage - 15.0) / (120.0 - 15.0)).clamp(0.0, 1.0) },
        StatBar { label: "FIRE RATE", value: ((s.fire_rate - 1.0) / (15.0 - 1.0)).clamp(0.0, 1.0) },
        StatBar { label: "ACCURACY",  value: (1.0 - (s.hip_spread - 0.01) / (0.08 - 0.01)).clamp(0.0, 1.0) },
        StatBar { label: "RANGE",     value: ((s.bullet_range - 15.0) / (100.0 - 15.0)).clamp(0.0, 1.0) },
        StatBar { label: "MOBILITY",  value: (1.0 - (s.ads_time - 0.10) / (0.50 - 0.10)).clamp(0.0, 1.0) },
        StatBar { label: "MAGAZINE",  value: ((s.magazine_size as f32 - 5.0) / (50.0 - 5.0)).clamp(0.0, 1.0) },
    ]
}

pub struct LobbyInventoryScreen {
    pub selected_slot: Option<usize>,
    pub scroll_offset: f32,
    pub hovered_weapon: Option<WeaponId>,
}

impl LobbyInventoryScreen {
    pub fn new() -> Self {
        Self {
            selected_slot: None,
            scroll_offset: 0.0,
            hovered_weapon: None,
        }
    }

    /// Returns true if the user wants to go back.
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

        // layout dimensions
        let left_w = sw * 0.40;
        let pad = 20.0;

        // back button
        let back_w = 100.0;
        let back_h = 34.0;
        let back_x = pad;
        let back_y = 12.0;
        if in_rect(mouse, back_x, back_y, back_w, back_h) && clicked {
            return true;
        }

        // slots row
        let slots_y = 60.0;
        let slot_w = (left_w - pad * 2.0 - 8.0) / LOADOUT_SLOTS as f32;
        let slot_h = 52.0;

        for i in 0..LOADOUT_SLOTS {
            let sx = pad + (slot_w + 8.0) * i as f32;
            if in_rect(mouse, sx, slots_y, slot_w, slot_h) && clicked {
                self.selected_slot = if self.selected_slot == Some(i) {
                    None
                } else {
                    Some(i)
                };
                self.scroll_offset = 0.0;
            }
        }

        // weapon list
        let hint_y = slots_y + slot_h + 14.0;
        let list_y = hint_y + 20.0;
        let list_h = sh - list_y - 40.0;
        let row_h = 32.0;
        let header_h = 28.0;

        if let Some(slot) = self.selected_slot {
            let allowed = allowed_categories(slot);
            let mut y = list_y - self.scroll_offset;
            for cat in &WeaponCategory::ALL {
                y += header_h;
                if !allowed.contains(cat) {
                    y += WeaponId::all_in(*cat).len() as f32 * row_h;
                    continue;
                }
                let weapons = WeaponId::all_in(*cat);
                for weapon_id in weapons {
                    if y + row_h > list_y && y < list_y + list_h {
                        let row_x = pad;
                        let row_w = left_w - pad * 2.0;
                        if in_rect(mouse, row_x, y, row_w, row_h) {
                            // hover sets preview
                            self.hovered_weapon = Some(weapon_id);
                            if clicked {
                                loadout.assign(slot, weapon_id);
                                self.selected_slot = None;
                            }
                        }
                    }
                    y += row_h;
                }
            }
        }

        // clamp scroll
        let total_content = self.calc_total_height();
        let visible_h = list_h;
        let max_scroll = (total_content - visible_h).max(0.0);
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        false
    }

    pub fn draw(&self, loadout: &Loadout) {
        let mouse = mouse_position();
        let sw = screen_width();
        let sh = screen_height();
        let pad = 20.0;

        // full-screen dark background
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.05, 0.06, 0.12, 0.95));

        let left_w = sw * 0.40;
        let right_x = left_w;
        let right_w = sw - left_w;

        // left panel background
        draw_rectangle(0.0, 0.0, left_w, sh, PANEL_BG);
        draw_rectangle(left_w - 1.0, 0.0, 1.0, sh, PANEL_BORDER);

        // title
        let title = "LOADOUT";
        let tw = measure_game_text(title, 22);
        draw_text_ex(title, (left_w - tw) / 2.0, 38.0, text_params(22, WHITE));

        // back button
        let back_w = 100.0;
        let back_h = 34.0;
        let back_x = pad;
        let back_y = 12.0;
        let back_hover = in_rect(mouse, back_x, back_y, back_w, back_h);
        draw_rectangle(
            back_x, back_y, back_w, back_h,
            if back_hover { ACCENT_HOVER } else { Color::new(0.15, 0.17, 0.28, 1.0) },
        );
        let bt = "< BACK";
        let btw = measure_game_text(bt, 15);
        draw_text_ex(bt, back_x + (back_w - btw) / 2.0, back_y + 23.0, text_params(15, WHITE));

        // slots row
        let slots_y = 60.0;
        let slot_w = (left_w - pad * 2.0 - 8.0) / LOADOUT_SLOTS as f32;
        let slot_h = 52.0;

        for i in 0..LOADOUT_SLOTS {
            let sx = pad + (slot_w + 8.0) * i as f32;
            let is_selected = self.selected_slot == Some(i);
            let hover = in_rect(mouse, sx, slots_y, slot_w, slot_h);

            let bg = if is_selected { SLOT_SELECTED } else if hover { ROW_HOVER } else { SLOT_BG };
            draw_rectangle(sx, slots_y, slot_w, slot_h, bg);
            draw_rectangle_lines(
                sx, slots_y, slot_w, slot_h,
                if is_selected { 2.0 } else { 1.0 },
                if is_selected { ACCENT } else { PANEL_BORDER },
            );

            let label = SLOT_LABELS[i];
            let lw = measure_game_text(label, 11);
            draw_text_ex(label, sx + (slot_w - lw) / 2.0, slots_y + 16.0, text_params(11, TEXT_DIM));

            let name = loadout.slots[i].display_name();
            let nw = measure_game_text(name, 13);
            draw_text_ex(name, sx + (slot_w - nw) / 2.0, slots_y + 36.0, text_params(13, WHITE));
        }

        // hint
        let hint = if let Some(slot) = self.selected_slot {
            if slot == 0 { "Select a Shotgun, Assault Rifle, SMG, or Sniper" } else { "Select a Pistol" }
        } else {
            "Click a slot above to change its weapon"
        };
        let hint_y = slots_y + slot_h + 14.0;
        let hw = measure_game_text(hint, 12);
        draw_text_ex(hint, (left_w - hw) / 2.0, hint_y, text_params(12, TEXT_DIM));

        // weapon list
        let list_y = hint_y + 20.0;
        let list_h = sh - list_y - 40.0;
        let row_h = 32.0;
        let header_h = 28.0;

        let mut y = list_y - self.scroll_offset;
        let loadout_set: Vec<WeaponId> = loadout.slots.to_vec();
        let active_allowed: Option<&[WeaponCategory]> =
            self.selected_slot.map(|s| allowed_categories(s));

        for cat in &WeaponCategory::ALL {
            let weapons = WeaponId::all_in(*cat);
            let cat_allowed = active_allowed.map_or(true, |allowed| allowed.contains(cat));

            // category header
            if y + header_h > list_y && y < list_y + list_h && y >= list_y {
                let header_color = if cat_allowed || active_allowed.is_none() {
                    CATEGORY_HEADER
                } else {
                    Color::new(0.40, 0.35, 0.20, 1.0)
                };
                draw_text_ex(cat.display_name(), pad + 4.0, y + 20.0, text_params(14, header_color));
                draw_rectangle(
                    pad, y + header_h - 2.0, left_w - pad * 2.0, 1.0,
                    Color::new(0.85, 0.65, 0.30, if cat_allowed { 0.3 } else { 0.1 }),
                );
            }
            y += header_h;

            for weapon_id in weapons {
                if y + row_h > list_y && y < list_y + list_h {
                    let row_x = pad;
                    let row_w = left_w - pad * 2.0;
                    let can_click = cat_allowed && self.selected_slot.is_some();
                    let hover = in_rect(mouse, row_x, y, row_w, row_h) && can_click;
                    let in_loadout = loadout_set.contains(&weapon_id);
                    let is_hovered_weapon = self.hovered_weapon == Some(weapon_id);

                    let bg = if is_hovered_weapon && can_click {
                        Color::new(0.14, 0.16, 0.28, 1.0)
                    } else if hover {
                        ROW_HOVER
                    } else if in_loadout && cat_allowed {
                        Color::new(0.10, 0.12, 0.22, 1.0)
                    } else {
                        ROW_BG
                    };
                    draw_rectangle(row_x, y, row_w, row_h, bg);

                    let name: &str = weapon_id.display_name();
                    let text_color = if !cat_allowed && active_allowed.is_some() {
                        Color::new(0.35, 0.35, 0.42, 1.0)
                    } else if in_loadout {
                        ACCENT
                    } else {
                        TEXT_LABEL
                    };
                    draw_text_ex(name, row_x + 12.0, y + 21.0, text_params(14, text_color));

                    if let Some(slot_idx) = loadout.slots.iter().position(|&w| w == weapon_id) {
                        let tag = format!("[{}]", SLOT_LABELS[slot_idx]);
                        let tag_w = measure_game_text(&tag, 12);
                        draw_text_ex(&tag, row_x + row_w - tag_w - 8.0, y + 20.0, text_params(12, TEXT_DIM));
                    }
                }
                y += row_h;
            }
        }

        // footer hint
        let footer = "ESC to go back";
        let fw = measure_game_text(footer, 12);
        draw_text_ex(footer, (left_w - fw) / 2.0, sh - 12.0, text_params(12, TEXT_DIM));

        // ========== RIGHT PANEL ==========
        if let Some(weapon_id) = self.hovered_weapon {
            let stats = weapon_id.stats();

            // weapon name
            let wname = stats.name;
            let wnw = measure_game_text(wname, 28);
            draw_text_ex(
                wname,
                right_x + (right_w - wnw) / 2.0,
                50.0,
                text_params(28, WHITE),
            );

            // category label
            let cat_label = weapon_id.category().display_name();
            let clw = measure_game_text(cat_label, 14);
            draw_text_ex(
                cat_label,
                right_x + (right_w - clw) / 2.0,
                72.0,
                text_params(14, TEXT_DIM),
            );

            // stat bars (10 discrete segments)
            let bars = compute_stat_bars(weapon_id);
            let bar_start_y = 100.0;
            let bar_h = 24.0;
            let bar_gap = 6.0;
            let label_w = 100.0;
            let seg_gap = 3.0;
            let bar_x = right_x + 30.0;
            let bar_area_w = right_w - 60.0 - label_w;
            let seg_w = (bar_area_w - seg_gap * (STAT_SEGMENTS as f32 - 1.0)) / STAT_SEGMENTS as f32;

            for (i, bar) in bars.iter().enumerate() {
                let by = bar_start_y + i as f32 * (bar_h + bar_gap);

                // label
                draw_text_ex(bar.label, bar_x, by + 17.0, text_params(12, TEXT_DIM));

                // filled segment count (round to nearest)
                let filled = (bar.value * STAT_SEGMENTS as f32).round() as u32;

                for seg in 0..STAT_SEGMENTS {
                    let sx = bar_x + label_w + seg as f32 * (seg_w + seg_gap);
                    let color = if seg < filled { ACCENT } else { STAT_BAR_BG };
                    draw_rectangle(sx, by + 5.0, seg_w, bar_h - 10.0, color);
                }
            }

        } else {
            // no weapon selected — placeholder
            let placeholder = "Select a weapon to preview";
            let pw = measure_game_text(placeholder, 16);
            draw_text_ex(
                placeholder,
                right_x + (right_w - pw) / 2.0,
                sh / 2.0,
                text_params(16, TEXT_DIM),
            );
        }
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
