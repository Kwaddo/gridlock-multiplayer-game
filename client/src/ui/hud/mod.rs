use macroquad::prelude::*;
use std::collections::HashMap;

mod crosshair;
mod minimap;
mod players;
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
    forward: Vec3,
    right: Vec3,
    pitch: f32,
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
    maze: Option<&Maze>,
    current_world: WorldId,
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
        username,
        player_id,
        position,
        movement_state,
        hp,
        max_hp,
        shield,
        max_shield,
    );

    players::draw_players_overlay(position, forward, right, pitch, other_players);

    weapon_feed::draw_weapon_and_kill_feed(weapons, assets, other_players, kill_feed);

    minimap::draw_minimap(maze, position, yaw, other_players, current_world, config);
}
