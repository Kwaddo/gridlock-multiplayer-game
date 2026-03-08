use macroquad::prelude::*;
use std::collections::HashMap;

use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::OtherPlayer;
use crate::weapons::stats::WeaponId;
use crate::weapons::WeaponInventory;

/// Standalone kill feed renderer — can be called independently (e.g. on death screen).
pub fn draw_kill_feed(
    kill_feed: &[crate::screens::playing::KillFeedEntry],
    assets: &HashMap<String, Texture2D>,
    local_player_id: u32,
    local_team: u8,
) {
    let feed_x = 12.0;
    let mut feed_y = 20.0;
    for entry in kill_feed.iter().rev().take(5) {
        let alpha = ((entry.timer / 5.0).min(1.0) * 255.0) as u8;
        let weapon_color = Color::from_rgba(200, 200, 210, alpha);

        let killer_name = entry.killer_name.as_str();
        let victim_name = entry.victim_name.as_str();

        // color: local player = orange, same team = blue, enemy = red, neutral = white
        let killer_color = if entry.killer_id == local_player_id {
            Color::from_rgba(255, 107, 53, alpha)
        } else if local_team != 0 && entry.killer_team == local_team {
            Color::from_rgba(120, 170, 230, alpha)
        } else if local_team != 0 && entry.killer_team != 0 {
            Color::from_rgba(230, 120, 120, alpha)
        } else {
            Color::from_rgba(255, 255, 255, alpha)
        };
        let victim_color = if entry.victim_id == local_player_id {
            Color::from_rgba(255, 107, 53, alpha)
        } else if local_team != 0 && entry.victim_team == local_team {
            Color::from_rgba(120, 170, 230, alpha)
        } else if local_team != 0 && entry.victim_team != 0 {
            Color::from_rgba(230, 120, 120, alpha)
        } else {
            Color::from_rgba(255, 255, 255, alpha)
        };

        let weapon_display = format!("[{}]", entry.weapon);
        let full_text = format!("{}  {}  {}", killer_name, weapon_display, victim_name);
        let full_w = measure_game_text(&full_text, 14);
        draw_rectangle(
            feed_x - 4.0,
            feed_y - 14.0,
            full_w + 8.0,
            20.0,
            Color::from_rgba(8, 10, 16, (alpha as f32 * 0.6) as u8),
        );

        let mut x_offset = feed_x;

        draw_text_ex(killer_name, x_offset, feed_y, text_params(14, killer_color));
        x_offset += measure_game_text(killer_name, 14) + 4.0;

        let weapon_icon_key = format!("{}_icon", entry.weapon.to_lowercase());
        if let Some(icon_tex) = assets.get(&weapon_icon_key) {
            draw_texture_ex(
                icon_tex,
                x_offset,
                feed_y - 12.0,
                Color::from_rgba(255, 255, 255, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(16.0, 16.0)),
                    ..Default::default()
                },
            );
            x_offset += 20.0;
        } else {
            draw_text_ex(&weapon_display, x_offset, feed_y, text_params(14, weapon_color));
            x_offset += measure_game_text(&weapon_display, 14) + 4.0;
        }

        draw_text_ex(victim_name, x_offset, feed_y, text_params(14, victim_color));

        feed_y += 22.0;
    }
}

pub(super) fn draw_weapon_and_kill_feed(
    weapons: &WeaponInventory,
    assets: &HashMap<String, Texture2D>,
    _other_players: &[OtherPlayer],
    kill_feed: &[crate::screens::playing::KillFeedEntry],
    local_player_id: u32,
    _local_username: &str,
    local_team: u8,
    config: &crate::core::GameConfig,
) {
    // --- Valorant-style 3-slot weapon display (bottom-right) ---
    let hud_scale = 1.5;
    let slot_width = 180.0 * hud_scale;
    let slot_height = 46.0 * hud_scale;
    let slot_gap = 3.0 * hud_scale;
    let margin_right = 16.0 * hud_scale;
    let margin_bottom = 20.0 * hud_scale;

    // Slots: Primary (0), Secondary (1), Knife
    let slots: [(&crate::weapons::WeaponState, &str, bool); 3] = [
        (&weapons.weapons[0], "1", !weapons.knife_active && weapons.active_slot == 0),
        (&weapons.weapons[1], "2", !weapons.knife_active && weapons.active_slot == 1),
        (&weapons.knife, "X", weapons.knife_active),
    ];

    let base_x = screen_width() - slot_width - margin_right;
    let total_height = 3.0 * slot_height + 2.0 * slot_gap;
    let base_y = screen_height() - total_height - margin_bottom;

    for (i, (weapon, key_label, is_active)) in slots.iter().enumerate() {
        let y = base_y + i as f32 * (slot_height + slot_gap);
        let alpha = if *is_active { 255u8 } else { 120u8 };
        let bg_alpha = if *is_active { 100u8 } else { 35u8 };

        // slot background
        draw_rectangle(
            base_x, y, slot_width, slot_height,
            Color::from_rgba(12, 14, 24, bg_alpha),
        );
        // active slot border
        if *is_active {
            draw_rectangle_lines(
                base_x, y, slot_width, slot_height, 2.0,
                Color::from_rgba(255, 107, 53, 200),
            );
        }

        // key label (left side)
        draw_text_ex(
            key_label,
            base_x + 12.0,
            y + slot_height * 0.65,
            text_params(22, Color::from_rgba(150, 150, 160, alpha)),
        );

        // weapon name
        let stats = weapon.stats();
        draw_text_ex(
            stats.name,
            base_x + 44.0,
            y + 32.0,
            text_params(26, Color::from_rgba(200, 200, 210, alpha)),
        );

        if *is_active && weapon.id == WeaponId::TaurusRaging {
            let mode_text = if weapon.taurus_shotgun_mode {
                "SHOTGUN"
            } else {
                "REVOLVER"
            };
            draw_text_ex(
                mode_text,
                base_x + 44.0,
                y + 56.0,
                text_params(22, Color::from_rgba(230, 210, 150, alpha)),
            );

            let key_name = config.bind_taurus_toggle.display_name();
            let toggle_hint = format!("Press '{}' to toggle", key_name);
            draw_text_ex(
                &toggle_hint,
                base_x + 44.0,
                y + slot_height - 8.0,
                text_params(20, Color::from_rgba(190, 190, 200, alpha)),
            );
        }

        // ammo (skip for knife)
        if weapon.id != WeaponId::Knife {
            let ammo_text = if weapon.is_reloading {
                "RELOADING".to_string()
            } else {
                format!("{} / {}", weapon.ammo, weapon.reserve)
            };
            let ammo_color = if weapon.ammo == 0 && !weapon.is_reloading {
                Color::from_rgba(255, 80, 80, alpha)
            } else {
                Color::from_rgba(180, 180, 190, alpha)
            };
            draw_text_ex(
                &ammo_text,
                if *is_active && weapon.id == WeaponId::TaurusRaging {
                    base_x + 236.0
                } else {
                    base_x + 44.0
                },
                y + slot_height - 8.0,
                text_params(24, ammo_color),
            );
        }

        // weapon icon (right side)
        let icon_size = 64.0;
        let icon_x = base_x + slot_width - icon_size - 16.0;
        let icon_y_pos = y + (slot_height - icon_size) * 0.5;
        if let Some(tex) = assets.get(stats.icon_asset_key) {
            draw_texture_ex(
                tex,
                icon_x,
                icon_y_pos,
                Color::from_rgba(255, 255, 255, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(icon_size, icon_size)),
                    flip_x: stats.icon_flip_x,
                    ..Default::default()
                },
            );
        }
    }

    // indicate locked slot (Gun Game)
    if weapons.locked_slot.is_some() {
        let lock_y = base_y - 16.0;
        draw_text_ex(
            "(locked)",
            base_x + slot_width - measure_game_text("(locked)", 11) - 4.0,
            lock_y,
            text_params(11, Color::from_rgba(255, 140, 140, 180)),
        );
    }

    draw_kill_feed(kill_feed, assets, local_player_id, local_team);
}
