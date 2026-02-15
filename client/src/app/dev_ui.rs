use client::screens::dev_mode::{draw_action_button_public, draw_section_header_public, draw_toggle_public};
use client::types::ai::simulated_player::{MovementPattern, SimulatedPlayer};
use client::types::world::WorldId;
use client::ui::fonts::text_params;
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
    y += draw_section_header_public("Bots", x, y);

    draw_text_ex(
        &format!("Active Bots: {}", bots.len()),
        x,
        y + 14.0,
        text_params(14, WHITE),
    );
    y += 24.0;

    let (btn_h, spawn_pressed) = draw_action_button_public("Spawn Bot", x, y, 140.0, mouse, clicked);
    y += btn_h;

    if spawn_pressed {
        let spawn_pos = vec3(0.0, 0.0, 5.0);
        let bot = SimulatedPlayer::new(*next_bot_id, format!("Bot{}", next_bot_id), current_world, spawn_pos);
        bots.push(bot);
        *next_bot_id += 1;
    }

    let (btn_h, clear_pressed) = draw_action_button_public("Clear All Bots", x, y, 140.0, mouse, clicked);
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

            y += draw_section_header_public("Actions", x, y);
            y += draw_toggle_public("Jump Spam", &mut bot.action_state.jump_spam, x, y, mouse, clicked);
            y += draw_toggle_public("Crouch", &mut bot.action_state.crouch, x, y, mouse, clicked);
            y += draw_toggle_public("Slide", &mut bot.action_state.slide, x, y, mouse, clicked);
            let _ = draw_toggle_public("Shoot", &mut bot.action_state.shoot, x, y, mouse, clicked);
        }
    }
}
