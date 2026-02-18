use macroquad::prelude::*;
use std::collections::HashMap;

use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::OtherPlayer;
use crate::weapons::WeaponInventory;

pub(super) fn draw_weapon_and_kill_feed(
    weapons: &WeaponInventory,
    assets: &HashMap<String, Texture2D>,
    other_players: &[OtherPlayer],
    kill_feed: &[crate::screens::playing::KillFeedEntry],
    local_player_id: u32,
    local_username: &str,
) {
    let weapon = weapons.active();
    let stats = weapon.stats();

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

    let name_w = measure_game_text(stats.name, 16);
    let name_x = screen_width() - name_w - 20.0;
    let name_y = screen_height() - 80.0;
    draw_text_ex(
        stats.name,
        name_x,
        name_y,
        text_params(16, Color::from_rgba(180, 180, 180, 255)),
    );

    let icon_w = stats.icon_width;
    let icon_h = stats.icon_height;
    let icon_x = name_x - icon_w - 8.0;
    let icon_y = name_y - icon_h + 4.0;

    let kick_y = if weapon.muzzle_flash_timer > 0.0 {
        (weapon.muzzle_flash_timer / 0.08) * 6.0
    } else {
        0.0
    };

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

    if weapon.muzzle_flash_timer > 0.0 {
        let t = weapon.muzzle_flash_timer / 0.08;
        let alpha = (t * 255.0) as u8;
        let flash_x = if stats.icon_flip_x {
            icon_x + 6.0
        } else {
            icon_x + icon_w - 6.0
        };
        let flash_y = icon_y + kick_y + 5.0;

        draw_circle(
            flash_x,
            flash_y,
            14.0 * t,
            Color::from_rgba(255, 180, 50, alpha / 2),
        );
        draw_circle(
            flash_x,
            flash_y,
            6.0 * t,
            Color::from_rgba(255, 240, 180, alpha),
        );
        for i in 0..4 {
            let angle = (i as f32) * 0.8 + t * 3.0;
            let len = 10.0 * t;
            draw_line(
                flash_x,
                flash_y,
                flash_x + angle.cos() * len,
                flash_y + angle.sin() * len,
                1.5,
                Color::from_rgba(255, 200, 80, alpha),
            );
        }
    }

    let feed_x = screen_width() - 400.0;
    let mut feed_y = 20.0;
    for entry in kill_feed.iter().rev().take(5) {
        let alpha = ((entry.timer / 5.0).min(1.0) * 255.0) as u8;
        let text_color = Color::from_rgba(255, 255, 255, alpha);

        let killer_name = if entry.killer_id == local_player_id {
            local_username
        } else {
            other_players
                .iter()
                .find(|p| p.id == entry.killer_id)
                .map(|p| p.username.as_str())
                .unwrap_or("Player")
        };
        let victim_name = if entry.victim_id == local_player_id {
            local_username
        } else {
            other_players
                .iter()
                .find(|p| p.id == entry.victim_id)
                .map(|p| p.username.as_str())
                .unwrap_or("Player")
        };

        let mut x_offset = feed_x;

        draw_text_ex(killer_name, x_offset, feed_y, text_params(14, text_color));
        x_offset += measure_game_text(killer_name, 14) + 5.0;

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
        } else {
            let weapon_text = format!("[{}]", entry.weapon);
            draw_text_ex(&weapon_text, x_offset, feed_y, text_params(12, text_color));
        }
        x_offset += 20.0;

        draw_text_ex(victim_name, x_offset, feed_y, text_params(14, text_color));
        feed_y += 20.0;
    }
}
