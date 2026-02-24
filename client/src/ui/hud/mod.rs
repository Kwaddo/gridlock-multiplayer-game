use macroquad::prelude::*;
use std::collections::HashMap;

mod crosshair;
mod minimap;
mod players;
pub use players::draw_player_list;
mod scope;
mod status;
mod weapon_feed;

use crate::core::GameConfig;
use crate::physics::movement::MovementState;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::ui::OtherPlayer;
use crate::weapons::effects::DamageNumber;
use crate::weapons::WeaponInventory;

pub fn draw_hud(
    username: &str,
    player_id: u32,
    position: Vec3,
    yaw: f32,
    movement_state: &MovementState,
    other_players: &[OtherPlayer],
    weapons: &WeaponInventory,
    assets: &HashMap<String, Texture2D>,
    hit_marker_timer: f32,
    hit_marker_headshot: bool,
    damage_numbers: &[DamageNumber],
    hit_marker_pellet_mask: u16,
    hp: f32,
    max_hp: f32,
    shield: f32,
    max_shield: f32,
    kill_feed: &[crate::screens::playing::KillFeedEntry],
    match_time_left: f32,
    damage_flash_timer: f32,
    last_damage_attacker: Option<&str>,
    maze: Option<&Maze>,
    current_world: WorldId,
    local_team: u8,
    config: &GameConfig,
) {
    let cy = screen_height() / 2.0;

    let sprint_sway = if movement_state.is_sprinting {
        let freq = 8.0;
        let amp = screen_height() * 0.4 / 100.0;
        (macroquad::time::get_time() as f32 * freq).sin() * amp
    } else {
        0.0
    };
    let cx = screen_width() / 2.0 + sprint_sway;

    // Damage flash overlay (subtle red vignette) when we take damage
    if damage_flash_timer > 0.0 {
        let alpha = ((damage_flash_timer / 0.6).min(1.0) * 140.0) as u8;
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(255, 40, 40, alpha),
        );
        if let Some(att) = last_damage_attacker {
            draw_text_ex(
                &format!("Hit by {}", att),
                12.0,
                36.0,
                crate::ui::fonts::text_params(18, Color::from_rgba(255, 255, 255, alpha)),
            );
        } else {
            draw_text_ex(
                "Hit!",
                12.0,
                36.0,
                crate::ui::fonts::text_params(18, Color::from_rgba(255, 255, 255, alpha)),
            );
        }
    }

    crosshair::draw_crosshair_and_combat_feedback(
        cx,
        cy,
        weapons,
        hit_marker_timer,
        hit_marker_headshot,
        damage_numbers,
        hit_marker_pellet_mask,
    );

    status::draw_status_panels(
        movement_state,
        hp,
        max_hp,
        shield,
        max_shield,
        match_time_left,
        config,
    );

    // nametags are now drawn by draw_player_overhead (camera-based projection)

    weapon_feed::draw_weapon_and_kill_feed(
        weapons,
        assets,
        other_players,
        kill_feed,
        player_id,
        username,
    );

    minimap::draw_minimap(
        maze,
        position,
        yaw,
        other_players,
        current_world,
        local_team,
        config,
    );
}
