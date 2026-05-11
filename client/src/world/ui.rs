use macroquad::prelude::*;

use crate::types::player::OtherPlayer;

pub fn draw_ui(username: &str, player_id: u32, position: Vec3, others: &[OtherPlayer]) {
    draw_text("Gridlock", 20.0, 20.0, 30.0, WHITE);
    draw_text(
        &format!("Player: {} (ID: {})", username, player_id),
        20.0,
        50.0,
        20.0,
        WHITE,
    );
    draw_text(
        &format!(
            "Pos: {:.2}, {:.2}, {:.2}",
            position.x, position.y, position.z
        ),
        20.0,
        70.0,
        20.0,
        WHITE,
    );

    draw_text(
        &format!("Players online: {}", others.len() + 1),
        20.0,
        100.0,
        20.0,
        YELLOW,
    );
    for (i, p) in others.iter().enumerate() {
        draw_text(
            &format!("   {} (ID: {})", p.username, p.id),
            20.0,
            120.0 + (i as f32) * 20.0,
            18.0,
            p.color,
        );
    }
}
