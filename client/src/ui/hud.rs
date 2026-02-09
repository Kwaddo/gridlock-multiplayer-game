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
    hit_marker_timer: f32,
    hit_marker_headshot: bool,
) {
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;

    let weapon = weapons.active();
    let stats = weapon.stats();

    // ── DYNAMIC CROSSHAIR (texture-based) ──────────────────────────
    // Bloom scales the hip crosshair up; ADS transitions between hip and ADS textures.
    // For sniper Scope style, ADS shows a full vignette + scope reticle.
    let base_size = stats.crosshair_base_size;
    let bloom_scale = 1.0 + weapon.bloom * 800.0; // bloom radians → size multiplier
    let ads_t = weapon.ads_progress; // 0.0 = hip, 1.0 = full ADS

    // flash tint briefly on shot
    let tint = if weapon.muzzle_flash_timer > 0.04 {
        Color::from_rgba(255, 255, 200, 255)
    } else {
        WHITE
    };

    match stats.crosshair_style {
        CrosshairStyle::Standard => {
            // Hip-fire crosshair: visible when NOT fully aimed, scales with bloom
            if ads_t < 1.0 {
                let hip_alpha = ((1.0 - ads_t) * 255.0) as u8;
                let hip_tint = Color::from_rgba((tint.r * 255.0) as u8, (tint.g * 255.0) as u8, (tint.b * 255.0) as u8, hip_alpha);
                let hip_size = base_size * bloom_scale;
                draw_crosshair_texture(cx, cy, hip_size, hip_tint, stats.crosshair_hip_key, assets);
            }

            // ADS crosshair: fades in as you aim, no bloom scaling (tight)
            if ads_t > 0.0 {
                let ads_alpha = (ads_t * 255.0) as u8;
                let ads_tint = Color::from_rgba((tint.r * 255.0) as u8, (tint.g * 255.0) as u8, (tint.b * 255.0) as u8, ads_alpha);
                let ads_size = base_size * 0.75; // ADS is tighter than base
                draw_crosshair_texture(cx, cy, ads_size, ads_tint, stats.crosshair_ads_key, assets);
            }
        }
        CrosshairStyle::Scope => {
            if ads_t < 0.3 {
                // Hip-fire: normal crosshair scaled with bloom (sniper hip = inaccurate)
                let hip_alpha = ((1.0 - ads_t / 0.3).min(1.0) * 255.0) as u8;
                let hip_tint = Color::from_rgba(255, 255, 255, hip_alpha);
                let hip_size = base_size * bloom_scale * 1.5; // extra large = "don't hip-fire me"
                draw_crosshair_texture(cx, cy, hip_size, hip_tint, stats.crosshair_hip_key, assets);
            }

            if ads_t > 0.2 {
                // Scope overlay
                let scope_t = ((ads_t - 0.2) / 0.8).min(1.0); // 0→1 over the last 80% of ADS
                let scope_alpha = (scope_t * 255.0) as u8;

                // Black vignette with circular cutout
                let scope_radius = screen_height().min(screen_width()) * 0.38;
                draw_scope_vignette(cx, cy, scope_radius, scope_alpha);

                // Scope reticle inside the circle
                // Active: crosshair148 — alternatives commented below
                let reticle_key = "xhair_scope_148";
                // let reticle_key = "xhair_scope_141";
                // let reticle_key = "xhair_scope_162";
                let reticle_size = scope_radius * 1.6;
                let reticle_tint = Color::from_rgba(255, 255, 255, scope_alpha);
                draw_crosshair_texture(cx, cy, reticle_size, reticle_tint, reticle_key, assets);
            }
        }
    }

    // ── HIT MARKER ───────────────────────────────────────────────────
    if hit_marker_timer > 0.0 {
        let t = (hit_marker_timer / 0.25).min(1.0);
        let alpha = (t * 255.0) as u8;
        let hm_color = if hit_marker_headshot {
            Color::from_rgba(255, 50, 50, alpha) // red for headshot
        } else {
            Color::from_rgba(255, 255, 255, alpha) // white for body
        };
        let hm_size = 8.0 + (1.0 - t) * 4.0; // starts bigger, shrinks
        let hm_thick = 2.0;
        // X shape
        draw_line(cx - hm_size, cy - hm_size, cx - 3.0, cy - 3.0, hm_thick, hm_color);
        draw_line(cx + 3.0, cy - 3.0, cx + hm_size, cy - hm_size, hm_thick, hm_color);
        draw_line(cx - hm_size, cy + hm_size, cx - 3.0, cy + 3.0, hm_thick, hm_color);
        draw_line(cx + 3.0, cy + 3.0, cx + hm_size, cy + hm_size, hm_thick, hm_color);
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

      // VIEWMODEL
    let vm_keys = stats.viewmodel_frame_keys;
    if !vm_keys.is_empty() {
        let frame_key = vm_keys[weapon.viewmodel_frame];
        if let Some(tex) = assets.get(frame_key) {
            let vm_h = screen_height() * 0.55;
            let vm_w = vm_h; // keep square aspect
            let vm_x = screen_width() - vm_w - 20.0;
            let vm_y = screen_height() - vm_h + 40.0;

            // subtle bob when walking
            let bob_y = if movement_state.is_grounded {
                let speed = movement_state.stamina; // hacky proxy for "moving"
                if speed < 100.0 {
                    (get_time() as f32 * 5.0).sin() * 3.0
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // kick up on fire
            let fire_kick = if weapon.muzzle_flash_timer > 0.0 {
                -(weapon.muzzle_flash_timer / 0.08) * 20.0
            } else {
                0.0
            };

            draw_texture_ex(tex, vm_x, vm_y + bob_y + fire_kick, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vm_w, vm_h)),
                ..Default::default()
            });
        }

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

     //jh recoil kick on fire
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

fn draw_crosshair_texture(cx: f32, cy: f32, size: f32, color: Color, key: &str, assets: &HashMap<String, Texture2D>) {
    if let Some(tex) = assets.get(key) {
        let half = size / 2.0;
        draw_texture_ex(tex, cx - half, cy - half, color, DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            ..Default::default()
        });
    } else {
        draw_circle(cx, cy, size / 2.0, color);  // fallback dot
    }
}

fn draw_scope_vignette(cx: f32, cy: f32, radius: f32, alpha: u8) {
    let color = Color::from_rgba(0, 0, 0, alpha);
    let far = screen_width().max(screen_height());
    let steps = 64;

    for i in 0..steps {
        let a0 = (i as f32 / steps as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / steps as f32) * std::f32::consts::TAU;

        let c0 = vec2(cx+ a0.cos() * radius, cy + a0.sin() * radius);
        let c1 = vec2(cx + a1.cos() * radius, cy + a1.sin() * radius);

        let e0 = vec2(cx + a0.cos() * far, cy + a0.sin() * far);
        let e1 = vec2(cx + a1.cos() * far, cy + a1.sin() * far);

        draw_triangle(c0, c1, e0, color);
        draw_triangle(c1, e0, e1, color);
    }

    draw_circle_lines(cx, cy, radius, 3.0, Color::from_rgba(40, 40, 40, alpha));
    draw_circle_lines(cx, cy, radius - 1.5, 1.0, Color::from_rgba(80, 80, 80, alpha));
    draw_circle(cx, cy, radius - 2.0, Color::from_rgba(0, 10, 20, (alpha as f32 * 0.12) as u8));
}
