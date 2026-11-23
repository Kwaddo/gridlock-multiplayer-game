use macroquad::prelude::*;
use std::collections::HashMap;

mod crosshair;
mod loading;
mod minimap;
mod players;
pub use players::{draw_player_list, draw_team_scoreboard};
mod scope;
mod status;
pub(crate) use status::CachedText;
mod team_score;
pub use team_score::draw_team_score_bar;
pub use loading::draw_background_loading;
mod weapon_feed;
pub use weapon_feed::draw_kill_feed;

use crate::core::GameConfig;
use crate::physics::movement::MovementState;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::ui::OtherPlayer;
use crate::weapons::effects::DamageNumber;
use crate::weapons::WeaponInventory;

thread_local! {
    static HIT_BY_TEXT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

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
    kill_feed: &[crate::screens::playing::KillFeedEntry],
    match_time_left: f32,
    damage_flash_timer: f32,
    last_damage_attacker: Option<&str>,
    maze: Option<&Maze>,
    current_world: WorldId,
    local_team: u8,
    config: &GameConfig,
    smoke_minimap: Option<(Vec<u32>, Vec<(Vec3, Color)>)>,
    // smoke break hider: no gun crosshair, hp, stamina or weapon slots
    hider: bool,
    // smoke break locked gun: no crosshair or weapon slots right now
    no_gun: bool,
) {
    let no_gun = hider || no_gun;
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
        // just above the crosshair where you're already looking, the kill feed keeps the top left
        let draw_centred = |text: &str| {
            let w = crate::ui::fonts::measure_game_text(text, 18);
            draw_text_ex(
                text,
                screen_width() / 2.0 - w / 2.0,
                cy - 70.0,
                crate::ui::fonts::text_params(18, Color::from_rgba(255, 255, 255, alpha)),
            );
        };
        if let Some(att) = last_damage_attacker {
            HIT_BY_TEXT.with_borrow_mut(|text| {
                // rebuilt only when the attacker changes
                if text.strip_prefix("Hit by ") != Some(att) {
                    text.clear();
                    text.push_str("Hit by ");
                    text.push_str(att);
                }
                draw_centred(text);
            });
        } else {
            draw_centred("Hit!");
        }
    }

    if !no_gun {
        crosshair::draw_crosshair_and_combat_feedback(
            cx,
            cy,
            weapons,
            hit_marker_timer,
            hit_marker_headshot,
            damage_numbers,
            hit_marker_pellet_mask,
        );
    } else if hider && config.smoke_hider_dot {
        // the setting for a tiny centre dot, off by default
        draw_circle(screen_width() / 2.0, cy, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }

    status::draw_status_panels(
        movement_state,
        hp,
        max_hp,
        match_time_left,
        config,
        hider,
    );

    // nametags are now drawn by draw_player_overhead (camera-based projection)

    weapon_feed::draw_weapon_and_kill_feed(
        weapons,
        assets,
        other_players,
        kill_feed,
        player_id,
        username,
        local_team,
        config,
        no_gun,
    );

    minimap::draw_minimap(
        maze,
        position,
        yaw,
        other_players,
        current_world,
        local_team,
        config,
        smoke_minimap,
    );
}
