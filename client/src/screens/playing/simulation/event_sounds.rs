//! Sounds for things the game learns about from state changes rather than
//! from one call site: weapon swaps, reloads, damage, deaths, kills, match
//! countdowns, teleports, Gun Game level-ups, walls breaking and chat. Each
//! frame compares the state with what it was last frame and plays a cue for
//! every edge, so networking and input code need no audio plumbing.

use std::collections::HashSet;

use crate::audio::catalog;
use crate::net::protocols::MODE_GUN_GAME;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::weapons::stats::WeaponId;

use macroquad::prelude::*;

use super::PlayingState;

/// Last frame's view of everything that has an event sound.
#[derive(Default)]
pub struct EventSoundState {
    primed: bool,
    /// (knife out, active slot, weapon in that slot)
    held: Option<(bool, usize, WeaponId)>,
    reloading: bool,
    hp: f32,
    dead: bool,
    kill_confirm_timer: f32,
    countdown_secs: i32,
    freeze_secs: i32,
    world: Option<WorldId>,
    gun_game_weapon: Option<WeaponId>,
    match_won: bool,
    chat_received: u32,
    broken_walls: HashSet<(u8, u8)>,
}

/// Countdowns only tick audibly for their last seconds.
const COUNTDOWN_TICK_FROM: i32 = 5;

impl PlayingState {
    pub(super) fn play_event_sounds(&mut self, maze: Option<&Maze>) {
        let held = (
            self.weapons.knife_active,
            self.weapons.active_slot,
            self.weapons.active().id,
        );
        let reloading = self.weapons.active().is_reloading;
        let countdown_secs = self.match_start_countdown_timer.ceil() as i32;
        let freeze_secs = self.post_teleport_freeze_timer.ceil() as i32;
        let gun_game_weapon = (self.match_mode == Some(MODE_GUN_GAME))
            .then(|| self.weapons.weapons.first().map(|w| w.id))
            .flatten();
        let match_won = self.match_end_winner.is_some();
        let chat_received = self.chat.received();

        let mut prev = std::mem::take(&mut self.event_sounds);
        if prev.primed {
            let a = &self.audio;

            // life and death first: they decide whether the rest is worth hearing
            let died = self.is_dead && !prev.dead;
            let respawned = !self.is_dead && prev.dead;
            if died {
                a.play_cue(catalog::DEATH);
            } else if respawned {
                a.play_cue(catalog::RESPAWN);
            } else if !self.is_dead && self.hp < prev.hp - 0.5 {
                a.play_cue(catalog::HURT);
            }

            if self.kill_confirm_timer > prev.kill_confirm_timer + 0.01 {
                a.play_cue(catalog::KILL_CONFIRM);
            }

            let world_changed = prev.world != Some(self.current_world);
            if world_changed {
                a.play_cue(catalog::TELEPORT);
            }

            // Gun Game hands over the next weapon after a kill
            let leveled_up = gun_game_weapon.is_some()
                && prev.gun_game_weapon.is_some()
                && gun_game_weapon != prev.gun_game_weapon;
            if leveled_up && !world_changed && !respawned {
                a.play_cue(catalog::LEVEL_UP);
            }

            if !self.is_dead && !died && !respawned && !world_changed && !leveled_up {
                if prev.held != Some(held) {
                    a.play_cue(catalog::WEAPON_SWITCH);
                } else if reloading && !prev.reloading {
                    a.play_cue(catalog::RELOAD_START);
                } else if !reloading && prev.reloading {
                    // finished (not cancelled by a swap, which changes `held`)
                    a.play_cue(catalog::RELOAD_END);
                }
            }

            // match countdown in the lobby, then the freeze after teleporting in
            if countdown_secs != prev.countdown_secs
                && countdown_secs > 0
                && countdown_secs <= COUNTDOWN_TICK_FROM
            {
                a.play_cue(catalog::COUNTDOWN_TICK);
            }
            if freeze_secs != prev.freeze_secs && self.current_world == WorldId::Maze {
                if freeze_secs > 0 && freeze_secs <= COUNTDOWN_TICK_FROM {
                    a.play_cue(catalog::COUNTDOWN_TICK);
                } else if freeze_secs <= 0 && prev.freeze_secs > 0 && !world_changed {
                    a.play_cue(catalog::MATCH_START);
                }
            }

            if match_won && !prev.match_won {
                a.play_cue(catalog::MATCH_END);
            }

            if chat_received != prev.chat_received {
                a.play_cue(catalog::CHAT);
            }
        }

        // walls broken since last frame, by anyone, heard where they fell
        if let Some(m) = maze {
            if m.broken_wall_scars.len() != prev.broken_walls.len() {
                if prev.primed && m.broken_wall_scars.len() > prev.broken_walls.len() {
                    let cue = catalog::WALL_BREAK;
                    for &(x, y) in m.broken_wall_scars.difference(&prev.broken_walls) {
                        let pos = m.cell_to_world(x as i32, y as i32);
                        self.play_spatial_sound(cue.key, pos, Vec3::ZERO, cue.volume, maze);
                    }
                }
                prev.broken_walls.clone_from(&m.broken_wall_scars);
            }
        }

        prev.primed = true;
        prev.held = Some(held);
        prev.reloading = reloading;
        prev.hp = self.hp;
        prev.dead = self.is_dead;
        prev.kill_confirm_timer = self.kill_confirm_timer;
        prev.countdown_secs = countdown_secs;
        prev.freeze_secs = freeze_secs;
        prev.world = Some(self.current_world);
        prev.gun_game_weapon = gun_game_weapon;
        prev.match_won = match_won;
        prev.chat_received = chat_received;
        self.event_sounds = prev;
    }
}
