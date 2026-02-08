use macroquad::prelude::*;
use crate::ui::OtherPlayer;
use crate::movement::MovementState;
use crate::fonts::{text_params, measure_game_text};

pub fn draw_hud(
    username: &str,
    player_id: u32,
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    pitch: f32,
    movement_state: &MovementState,
    other_players: &[OtherPlayer],
) {
    // CROSSHAIR
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;
    let size = 10.0;
    draw_line(cx - size, cy, cx + size, cy, 1.0, WHITE);
    draw_line(cx, cy - size, cx, cy + size, 1.0, WHITE);

    // HUD text
    draw_text_ex("Multiplayer FPS", 20.0, 20.0, text_params(30, WHITE));
    draw_text_ex(
        &format!("Player: {} (ID: {})", username, player_id),
        20.0, 50.0, text_params(20, WHITE),
    );
    draw_text_ex(
        &format!("Pos: {:.2}, {:.2}, {:.2}", position.x, position.y, position.z),
        20.0, 70.0, text_params(20, WHITE),
    );

    // STAMINA BAR
    let bar_width = 200.0;
    let bar_height = 8.0;
    let bar_x = screen_width() - bar_width - 20.0;
    let bar_y = screen_height() - 40.0;
    draw_rectangle(bar_x, bar_y, bar_width, bar_height, Color::from_rgba(50, 50, 50, 255));
    let fill = movement_state.stamina / 100.0;
    let bar_color = if movement_state.stamina < 20.0 {
        RED
    } else {
        Color::from_rgba(80, 180, 80, 255)
    };
    draw_rectangle(bar_x, bar_y, bar_width * fill, bar_height, bar_color);
    draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 1.0, GRAY);

    // Crouch indicator - bottom left, gray
    if movement_state.is_crouching {
        draw_text_ex(
            "CROUCHING",
            20.0,
            screen_height() - 20.0,
            text_params(16, Color::from_rgba(150, 150, 150, 255)),
        );
    }

    // Draw nametags above other players
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
            draw_text_ex(&p.username, screen_x - name_w / 2.0, screen_y, text_params(16, p.color));
        }
    }

    // Player list
    draw_text_ex(
        &format!("Players online: {}", other_players.len() + 1),
        20.0, 100.0, text_params(20, YELLOW),
    );
    for (i, p) in other_players.iter().enumerate() {
        draw_text_ex(
            &format!("   {} (ID: {})", p.username, p.id),
            20.0, 120.0 + (i as f32) * 20.0, text_params(18, p.color),
        );
    }
}
