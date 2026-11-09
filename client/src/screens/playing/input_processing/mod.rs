mod movement_input;
mod shooting;

use crate::core::{GameConfig, GameState};
use crate::screens::input;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::weapons::recoil::recover;
use crate::weapons::{recoil, SpreadContext};

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use super::PlayingState;

use crate::weapons::stats::melee::{
    KNIFE_LIGHT_ANIM_DURATION, KNIFE_SWITCH_FROM_TIME, KNIFE_SWITCH_TO_TIME,
};

impl PlayingState {
    #[inline(always)]
    pub fn process_local_input(
        &mut self,
        config: &GameConfig,
        mut maze: Option<&mut Maze>,
    ) -> Option<GameState> {
        let raw_delta = get_frame_time();
        if raw_delta > 0.2 {
            self.clear_input();
            self.move_dir = Vec3::ZERO;
            self.jump_requested = false;
            self.movement_state.velocity = Vec3::ZERO;
            self.last_mouse_pos = mouse_position();
            return None;
        }

        // block weapon input during punch animation
        let mut weapon_input_locked = false;
        if self.knife_attack_locked {
            self.knife_attack_lock_timer -= raw_delta;
            if self.knife_attack_lock_timer <= 0.0 {
                self.knife_attack_locked = false;
                // auto-return to gun after punch
                if self.punch_auto_return && self.weapons.knife_active {
                    self.weapons.knife_active = false;
                    self.weapons.switch_cooldown = KNIFE_SWITCH_FROM_TIME;
                    self.punch_auto_return = false;
                }
            } else {
                // during lock prevent sprint and weapon switching
                self.movement_state.is_sprinting = false;
                weapon_input_locked = true;
            }
        }

        if self.knife_miss_slow_timer > 0.0 {
            self.knife_miss_slow_timer = (self.knife_miss_slow_timer - raw_delta).max(0.0);
        }

        // Typing a chat line takes every key (ESC cancels it rather than pausing).
        if self.chat.is_open() {
            self.chat.update_input();
            self.halt_local_input();
            return None;
        }

        // Lobby console UI takes input precedence. Check it first so ESC closes the console
        // instead of opening the pause menu while the console is visible.
        self.check_shooting_range_start();
        self.check_lobby_console();
        if self.lobby_console_open {
            // keep UI responsive (mouse) but prevent movement/weapon input and stop movement
            self.halt_local_input();
            return None;
        }

        if config.bind_chat.is_pressed() || is_key_pressed(KeyCode::Enter) {
            self.chat.open();
            self.halt_local_input();
            return None;
        }

        if config.bind_settings.is_pressed() {
            return Some(GameState::Paused);
        }

        if self.is_dead {
            let is_snd_spectating = self.match_mode
                == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY)
                && self.current_world == WorldId::Maze;
            if is_snd_spectating {
                // Allow free look while spectating in S&D
                let sens = config.raw_sensitivity();
                input::process_mouse(
                    &mut self.last_mouse_pos,
                    &mut self.yaw,
                    &mut self.pitch,
                    sens,
                );
            } else {
                // Consume the mouse delta so it doesn't accumulate and cause a
                // camera jump on the first frame after respawning.
                self.last_mouse_pos = mouse_position();
            }
            if self.current_world == WorldId::Lobby
                && self.death_timer <= 0.0
                && is_key_pressed(KeyCode::Space)
            {
                self.pending_respawn = true;
            }
            self.clear_input();
            return None;
        }

        self.process_look_and_movement(config, weapon_input_locked);

        let delta = raw_delta.min(0.05);
        let vel = self.movement_state.velocity;
        let is_moving = vec2(vel.x, vel.z).length() > 0.5;
        recover(self.weapons.active_mut(), delta, is_moving);

        let vel = self.movement_state.velocity;
        let ctx = SpreadContext {
            is_moving: vec2(vel.x, vel.z).length() > 0.5,
            is_sprinting: self.movement_state.is_sprinting,
            is_crouching: self.movement_state.is_crouching,
            is_airborne: !self.movement_state.is_grounded,
            is_slow_walking: self.movement_state.is_slow_walking,
        };
        let spread = recoil::current_spread(self.weapons.active(), &ctx);
        self.weapons.active_mut().computed_spread = spread;

        let mut fire_sound_key: Option<&'static str> = None;

        // Punch button: fire melee hit immediately, auto-return to gun when animation ends.
        let mut punch_fire_this_frame = false;
        if (!weapon_input_locked)
            && (self.post_teleport_freeze_timer <= 0.0 || self.current_world != WorldId::Maze)
            && !self.weapons.knife_active
            && !self.knife_attack_locked
            && self.weapons.switch_cooldown <= 0.0
            && config.bind_punch.is_pressed()
        {
            self.weapons.knife_active = true;
            self.weapons.switch_cooldown = KNIFE_SWITCH_TO_TIME;
            self.punch_auto_return = true;
            punch_fire_this_frame = true;
        }

        let weapon = self.weapons.active_mut();

        // Firing: blocked while in post-teleport freeze inside the Maze
        if (!weapon_input_locked)
            && (self.post_teleport_freeze_timer <= 0.0 || self.current_world != WorldId::Maze)
        {
            // Burst weapons keep firing remaining shots even after trigger release
            let wants_to_fire =
                config.bind_fire.is_down() || weapon.burst_remaining > 0 || punch_fire_this_frame;
            if wants_to_fire {
                if weapon.fire() {
                    // Signal punch swing animation when melee fires
                    if weapon.id == crate::weapons::stats::WeaponId::Knife {
                        self.knife_swing_requested = true;
                        self.knife_attack_locked = true;
                        self.knife_attack_lock_timer = KNIFE_LIGHT_ANIM_DURATION;
                    }
                    // Signal sawnoff shot animation
                    if weapon.id == crate::weapons::stats::WeaponId::Sawnoff {
                        self.sawnoff_shot_requested = true;
                    }
                    fire_sound_key = Some(weapon.stats().fire_sound_asset_key);

                    self.fire_hitscan(&mut maze);
                }
                // `fire_hitscan` borrowed all of `self`; take the active weapon again
                let weapon = self.weapons.active_mut();
                weapon.trigger_held = true;
            } else {
                weapon.trigger_held = false;
            }
        } else {
            // while frozen, ensure trigger_held is false so semi-auto isn't stuck
            weapon.trigger_held = false;
        }

        if let Some(sound_key) = fire_sound_key {
            self.play_weapon_shot_sound(sound_key, config);
        }

        // teleporter interaction removed; lobby console handled earlier

        return None;
    }

    fn play_weapon_shot_sound(&self, sound_key: &str, config: &GameConfig) {
        if config.mute_audio {
            return;
        }
        let pitch = gen_range(0.94f32, 1.06f32);
        let volume = gen_range(0.7f32, 0.82f32);
        self.audio.play(sound_key, pitch, volume);
    }
}
