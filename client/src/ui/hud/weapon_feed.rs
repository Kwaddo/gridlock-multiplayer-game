use macroquad::prelude::*;
use std::collections::HashMap;

use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::OtherPlayer;
use crate::weapons::stats::WeaponId;
use crate::weapons::WeaponInventory;

pub(super) fn draw_weapon_and_kill_feed(
    weapons: &WeaponInventory,
    assets: &HashMap<String, Texture2D>,
    _other_players: &[OtherPlayer],
    kill_feed: &[crate::screens::playing::KillFeedEntry],
    local_player_id: u32,
    _local_username: &str,
) {
    let weapon = weapons.active();
    let stats = weapon.stats();

    if weapon.id != WeaponId::Knife {
        let ammo_text = if weapon.is_reloading {
            "RELOADING...".to_string()
        } else {
            format!("{} / {}", weapon.ammo, weapon.reserve)
        };

        let ammo_w = measure_game_text(&ammo_text, 24);
        draw_text_ex(
            &ammo_text,
            screen_width() - ammo_w - 20.0,
            screen_height() - 60.0,
            text_params(
                24,
                if weapon.ammo == 0 && !weapon.is_reloading {
                    RED
                } else {
                    WHITE
                },
            ),
        );
    }

    let name_w = measure_game_text(stats.name, 16);
    let name_x = screen_width() - name_w - 20.0;
    let name_y = screen_height() - 80.0;
    draw_text_ex(
        stats.name,
        name_x,
        name_y,
        text_params(16, Color::from_rgba(180, 180, 180, 255)),
    );

    // indicate when a server-mode lock is active (e.g. Gun Game)
    if weapons.locked_slot.is_some() {
        let lock_text = "(locked)";
        draw_text_ex(
            lock_text,
            name_x + name_w + 8.0,
            name_y,
            text_params(12, Color::from_rgba(255, 140, 140, 200)),
        );
    }

    let icon_w = stats.icon_width;
    let icon_h = stats.icon_height;
    let icon_x = name_x - icon_w - 8.0;
    let icon_y = name_y - icon_h + 4.0;

    let kick_y = 0.0;

    if let Some(tex) = assets.get(stats.icon_asset_key) {
        draw_texture_ex(
            tex,
            icon_x,
            icon_y + kick_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(icon_w, icon_h)),
                flip_x: stats.icon_flip_x,
                ..Default::default()
            },
        );
    } else {
        draw_rectangle(
            icon_x,
            icon_y + kick_y,
            icon_w,
            icon_h,
            Color::from_rgba(80, 80, 80, 150),
        );
        draw_rectangle_lines(icon_x, icon_y + kick_y, icon_w, icon_h, 1.0, GRAY);
    }

    let feed_x = 12.0; // kill-feed top-left
    let mut feed_y = 20.0;
    for entry in kill_feed.iter().rev().take(5) {
        let alpha = ((entry.timer / 5.0).min(1.0) * 255.0) as u8;
        let text_color = Color::from_rgba(255, 255, 255, alpha);
        let accent_color = Color::from_rgba(255, 107, 53, alpha); // orange accent
        let weapon_color = Color::from_rgba(200, 200, 210, alpha);

        // use pre-resolved names stored at creation time (survives player removal)
        let killer_name = entry.killer_name.as_str();
        let victim_name = entry.victim_name.as_str();

        // highlight the local player's name in orange
        let killer_color = if entry.killer_id == local_player_id {
            accent_color
        } else {
            text_color
        };
        let victim_color = if entry.victim_id == local_player_id {
            accent_color
        } else {
            text_color
        };

        // measure full width for background pill (name [weapon icon/text] name)
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

        // killer name
        draw_text_ex(killer_name, x_offset, feed_y, text_params(14, killer_color));
        x_offset += measure_game_text(killer_name, 14) + 4.0;

        // weapon icon or weapon name in brackets
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

        // victim name
        draw_text_ex(victim_name, x_offset, feed_y, text_params(14, victim_color));

        feed_y += 22.0;
    }
}
