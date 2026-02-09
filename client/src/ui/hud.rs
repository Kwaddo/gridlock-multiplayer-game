use macroquad::prelude::*;
use std::collections::HashMap;

use crate::physics::movement::MovementState;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::OtherPlayer;
use crate::weapons::WeaponInventory;
use crate::weapons::stats::CrosshairStyle;

pub fn draw_hud(
    username: &str,
    player_id: u32,
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    pitch: f32,
    movement_state: &MovementState,
    other_players: &[OtherPlayer],
    weapons: &WeaponInventory,
    assets: &HashMap<String, Texture2D>,
) {
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;

    let weapon = weapons.active();
    let stats = weapon.stats();

    // ── DYNAMIC CROSSHAIR ────────────────────────────────────────────
    // gap expands with bloom + ads_progress shrinks it
    let base_spread_px = stats.crosshair_base_size;
    let bloom_px = weapon.bloom * 600.0; // scale bloom radians → pixels
    let ads_shrink = weapon.ads_progress * (base_spread_px * 0.6); // ADS tightens crosshair
    let gap = (base_spread_px + bloom_px - ads_shrink).max(2.0);
    let arm_len = gap + 6.0; // length of each crosshair arm
    let thickness = stats.crosshair_thickness;

    // flash white briefly on shot
    let xhair_color = if weapon.muzzle_flash_timer > 0.04 {
        Color::from_rgba(255, 255, 200, 255)
    } else {
        WHITE
    };

    match stats.crosshair_style {
        CrosshairStyle::Cross => {
            draw_line(cx - arm_len, cy, cx - gap, cy, thickness, xhair_color);
            draw_line(cx + gap, cy, cx + arm_len, cy, thickness, xhair_color);
            draw_line(cx, cy - arm_len, cx, cy - gap, thickness, xhair_color);
            draw_line(cx, cy + gap, cx, cy + arm_len, thickness, xhair_color);
        }
        CrosshairStyle::CircleDots => {
            let radius = gap + 8.0;
            draw_circle_lines(cx, cy, radius, thickness, xhair_color);
            let dot_count = stats.pellet_count.min(12) as usize;
            for i in 0..dot_count {
                let angle = (i as f32 / dot_count as f32) * std::f32::consts::TAU;
                let dot_r = gap * 0.6;
                draw_circle(cx + angle.cos() * dot_r, cy + angle.sin() * dot_r, 2.0, xhair_color);
            }
        }
        CrosshairStyle::ScopeCross => {
            let scope_r = 30.0 * (1.0 - weapon.ads_progress * 0.3);
            draw_circle_lines(cx, cy, scope_r, 1.0, Color::from_rgba(200, 200, 200, 120));
            draw_line(cx - arm_len, cy, cx - 1.0, cy, 1.0, xhair_color);
            draw_line(cx + 1.0, cy, cx + arm_len, cy, 1.0, xhair_color);
            draw_line(cx, cy - arm_len, cx, cy - 1.0, 1.0, xhair_color);
            draw_line(cx, cy + 1.0, cx, cy + arm_len, 1.0, xhair_color);
        }
    }

    // ── HUD TEXT ─────────────────────────────────────────────────────
    draw_text_ex("Multiplayer FPS", 20.0, 20.0, text_params(30, WHITE));
    draw_text_ex(
        &format!("Player: {} (ID: {})", username, player_id),
        20.0,
        50.0,
        text_params(20, WHITE),
    );
    draw_text_ex(
        &format!(
            "Pos: {:.2}, {:.2}, {:.2}",
            position.x, position.y, position.z
        ),
        20.0,
        70.0,
        text_params(20, WHITE),
    );

    // ── STAMINA BAR ──────────────────────────────────────────────────
    let bar_width = 200.0;
    let bar_height = 8.0;
    let bar_x = screen_width() - bar_width - 20.0;
    let bar_y = screen_height() - 40.0;
    draw_rectangle(
        bar_x,
        bar_y,
        bar_width,
        bar_height,
        Color::from_rgba(50, 50, 50, 255),
    );
    let fill = movement_state.stamina / 100.0;
    let bar_color = if movement_state.stamina < 20.0 {
        RED
    } else {
        Color::from_rgba(80, 180, 80, 255)
    };
    draw_rectangle(bar_x, bar_y, bar_width * fill, bar_height, bar_color);
    draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 1.0, GRAY);

    // ── CROUCH INDICATOR ─────────────────────────────────────────────
    if movement_state.is_crouching {
        draw_text_ex(
            "CROUCHING",
            20.0,
            screen_height() - 20.0,
            text_params(16, Color::from_rgba(150, 150, 150, 255)),
        );
    }

    // ── NAMETAGS ─────────────────────────────────────────────────────
    for p in other_players {
        let head_pos = vec3(p.x, p.y + 1.8, p.z);
        let rel = head_pos - position;
        let dot = rel.dot(forward);
        if dot > 0.1 {
            let right_dot = rel.dot(right);
            let up_dot = rel.y - position.y + pitch.sin() * dot;
            let fov_scale = screen_height() / (2.0 * ((45.0_f32).to_radians() / 2.0).tan());
            let screen_x = screen_width() / 2.0 + (right_dot / dot) * fov_scale;
            let screen_y = screen_height() / 2.0 - (up_dot / dot) * fov_scale;
            let name_w = measure_game_text(&p.username, 16);
            draw_text_ex(
                &p.username,
                screen_x - name_w / 2.0,
                screen_y,
                text_params(16, p.color),
            );
        }
    }

    // ── PLAYER LIST ──────────────────────────────────────────────────
    draw_text_ex(
        &format!("Players online: {}", other_players.len() + 1),
        20.0,
        100.0,
        text_params(20, YELLOW),
    );
    for (i, p) in other_players.iter().enumerate() {
        draw_text_ex(
            &format!("   {} (ID: {})", p.username, p.id),
            20.0,
            120.0 + (i as f32) * 20.0,
            text_params(18, p.color),
        );
    }

    // ── WEAPON INFO (sprite + name + ammo) ──────────────────────────
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
        text_params(24, if weapon.ammo == 0 && !weapon.is_reloading { RED } else { WHITE }),
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

    // weapon sprite: sits to the left of the weapon name
    let icon_w = stats.icon_width;
    let icon_h = stats.icon_height;
    let icon_x = name_x - icon_w - 8.0;
    let icon_y = name_y - icon_h + 4.0; // align bottom of icon with text baseline

    // recoil kick on fire
    let kick_y = if weapon.muzzle_flash_timer > 0.0 {
        (weapon.muzzle_flash_timer / 0.08) * 6.0
    } else {
        0.0
    };

    if let Some(tex) = assets.get(stats.icon_asset_key) {
        draw_texture_ex(tex, icon_x, icon_y + kick_y, WHITE, DrawTextureParams {
            dest_size: Some(vec2(icon_w, icon_h)),
            flip_x: stats.icon_flip_x,
            ..Default::default()
        });
    } else {
        draw_rectangle(icon_x, icon_y + kick_y, icon_w, icon_h, Color::from_rgba(80, 80, 80, 150));
        draw_rectangle_lines(icon_x, icon_y + kick_y, icon_w, icon_h, 1.0, GRAY);
    }

    // muzzle flash at muzzle tip of the sprite
    if weapon.muzzle_flash_timer > 0.0 {
        let t = weapon.muzzle_flash_timer / 0.08;
        let alpha = (t * 255.0) as u8;
        let flash_x = if stats.icon_flip_x { icon_x + 6.0 } else { icon_x + icon_w - 6.0 };
        let flash_y = icon_y + kick_y + 5.0;

        draw_circle(flash_x, flash_y, 14.0 * t, Color::from_rgba(255, 180, 50, alpha / 2));
        draw_circle(flash_x, flash_y, 6.0 * t, Color::from_rgba(255, 240, 180, alpha));
        for i in 0..4 {
            let angle = (i as f32 * 0.8) + t * 3.0;
            let len = 10.0 * t;
            draw_line(
                flash_x, flash_y,
                flash_x + angle.cos() * len,
                flash_y + angle.sin() * len,
                1.5,
                Color::from_rgba(255, 200, 80, alpha),
            );
        }
    }
}
