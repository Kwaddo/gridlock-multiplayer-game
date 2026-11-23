use macroquad::prelude::*;

use crate::types::player::OtherPlayer;

pub fn draw_players(others: &[OtherPlayer]) {
    for p in others {
        let player_color = if p.skin.is_some() { WHITE } else { p.color };
        draw_cube(
            vec3(p.net.x, p.net.y + 0.6, p.net.z),
            vec3(0.6, 1.2, 0.4),
            None,
            player_color,
        );
        draw_cube(
            vec3(p.net.x, p.net.y + 1.4, p.net.z),
            vec3(0.4, 0.4, 0.4),
            None,
            player_color,
        );
    }
}
