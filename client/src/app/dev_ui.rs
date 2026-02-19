use client::screens::dev_mode::{
    draw_action_button_public, draw_section_header_public, draw_toggle_public,
};
use client::types::ai::simulated_player::{MovementPattern, SimulatedPlayer, YawMode};
use client::types::world::WorldId;
use macroquad::prelude::*;

pub fn draw_bot_tab(
    bots: &mut Vec<SimulatedPlayer>,
    next_bot_id: &mut u32,
    current_world: WorldId,
    x: f32,
    mut y: f32,
    _w: f32,
    mouse: (f32, f32),
    clicked: bool,
) {
    use client::ui::fonts::text_params;

    y += draw_section_header_public("Bots", x, y);

    draw_text_ex(
        &format!("Active Bots: {}", bots.len()),
        x,
        y + 14.0,
        text_params(14, WHITE),
    );
    y += 24.0;

    let (btn_h, spawn_pressed) =
        draw_action_button_public("Spawn Bot", x, y, 140.0, mouse, clicked);
    y += btn_h;

    if spawn_pressed {
        let spawn_pos = vec3(0.0, 0.0, 5.0);
        let bot = SimulatedPlayer::new(
            *next_bot_id,
            format!("Bot{}", next_bot_id),
            current_world,
            spawn_pos,
        );
        bots.push(bot);
        *next_bot_id += 1;
    }

    let (btn_h, clear_pressed) =
        draw_action_button_public("Clear All Bots", x, y, 140.0, mouse, clicked);
    y += btn_h;

    if clear_pressed {
        bots.clear();
    }

    y += 8.0;

    if !bots.is_empty() {
        y += draw_section_header_public("Selected Bot", x, y);

        if let Some(bot) = bots.first_mut() {
            draw_text_ex(
                &format!("Bot ID: {}", bot.player.id),
                x,
                y + 14.0,
                text_params(14, WHITE),
            );
            y += 24.0;

            let skin_key = bot.player.skin_type.model_key().unwrap_or("None");
            draw_text_ex(
                &format!(
                    "Skin Debug: {} ({})",
                    bot.player.skin_type.label(),
                    skin_key
                ),
                x,
                y + 14.0,
                text_params(13, Color::from_rgba(180, 220, 255, 255)),
            );
            y += 20.0;

            y += draw_section_header_public("Movement Pattern", x, y);

            let pattern_label = match bot.pattern {
                MovementPattern::None => "None",
                MovementPattern::ForwardBack => "Forward/Back",
                MovementPattern::SideToSide => "Side to Side",
                MovementPattern::Forward => "Forward",
                MovementPattern::Back => "Back",
                MovementPattern::Left => "Left",
                MovementPattern::Right => "Right",
                MovementPattern::Circle => "Circle",
                MovementPattern::Random => "Random",
            };

            let (btn_h, pattern_pressed) = draw_action_button_public(
                &format!("Pattern: {}", pattern_label),
                x,
                y,
                220.0,
                mouse,
                clicked,
            );
            y += btn_h;

            if pattern_pressed {
                bot.pattern = match bot.pattern {
                    MovementPattern::None => MovementPattern::ForwardBack,
                    MovementPattern::ForwardBack => MovementPattern::SideToSide,
                    MovementPattern::SideToSide => MovementPattern::Forward,
                    MovementPattern::Forward => MovementPattern::Back,
                    MovementPattern::Back => MovementPattern::Left,
                    MovementPattern::Left => MovementPattern::Right,
                    MovementPattern::Right => MovementPattern::Circle,
                    MovementPattern::Circle => MovementPattern::Random,
                    MovementPattern::Random => MovementPattern::None,
                };
            }

            let skin_mode_label = if bot.skin_override.is_some() {
                "Manual"
            } else {
                "Auto"
            };
            let (btn_h, mode_pressed) = draw_action_button_public(
                &format!("Skin Mode: {}", skin_mode_label),
                x,
                y,
                220.0,
                mouse,
                clicked,
            );
            y += btn_h;

            if mode_pressed {
                if bot.skin_override.is_some() {
                    bot.skin_override = None;
                } else {
                    bot.skin_override = Some(bot.player.skin_type);
                }
            }

            let (btn_h, skin_pressed) = draw_action_button_public(
                &format!("Skin: {}", bot.player.skin_type.label()),
                x,
                y,
                220.0,
                mouse,
                clicked,
            );
            y += btn_h;

            if skin_pressed {
                let next_skin = bot.player.skin_type.next();
                bot.player.skin_type = next_skin;
                bot.skin_override = Some(next_skin);
            }

            let yaw_label = match bot.yaw_mode {
                YawMode::Fixed => "Fixed",
                YawMode::FacePlayer => "Face Player",
                YawMode::Spin => "Spin",
            };
            let (btn_h, yaw_pressed) = draw_action_button_public(
                &format!("Yaw: {}", yaw_label),
                x,
                y,
                220.0,
                mouse,
                clicked,
            );
            y += btn_h;

            if yaw_pressed {
                bot.yaw_mode = match bot.yaw_mode {
                    YawMode::Fixed => YawMode::FacePlayer,
                    YawMode::FacePlayer => YawMode::Spin,
                    YawMode::Spin => YawMode::Fixed,
                };
            }

            if bot.yaw_mode == YawMode::Fixed {
                let yaw_deg = bot.player.yaw.to_degrees();
                draw_text_ex(
                    &format!("Yaw: {:.0}°", yaw_deg),
                    x,
                    y + 14.0,
                    text_params(14, WHITE),
                );
                y += 20.0;

                let (btn_h, left_pressed) =
                    draw_action_button_public("< -45°", x, y, 100.0, mouse, clicked);
                let (_, right_pressed) =
                    draw_action_button_public("> +45°", x + 110.0, y, 100.0, mouse, clicked);
                y += btn_h;

                if left_pressed {
                    bot.set_yaw(bot.player.yaw - std::f32::consts::FRAC_PI_4);
                }
                if right_pressed {
                    bot.set_yaw(bot.player.yaw + std::f32::consts::FRAC_PI_4);
                }
            }

            y += draw_section_header_public("Actions", x, y);
            y += draw_toggle_public(
                "Jump Spam",
                &mut bot.action_state.jump_spam,
                x,
                y,
                mouse,
                clicked,
            );
            y += draw_toggle_public("Crouch", &mut bot.action_state.crouch, x, y, mouse, clicked);
            y += draw_toggle_public("Sprint", &mut bot.action_state.sprint, x, y, mouse, clicked);
            y += draw_toggle_public("Slide", &mut bot.action_state.slide, x, y, mouse, clicked);
            y += draw_toggle_public("Shoot", &mut bot.action_state.shoot, x, y, mouse, clicked);
            draw_toggle_public("ADS", &mut bot.action_state.ads, x, y, mouse, clicked);
        }
    }
}
