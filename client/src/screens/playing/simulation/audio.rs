use crate::core::GameConfig;

use macroquad::prelude::*;

use super::constants::{
    FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_INTERVAL_WALK, FOOTSTEP_MAX_DIST,
    FOOTSTEP_VOLUME_CROUCH, FOOTSTEP_VOLUME_SPRINT, FOOTSTEP_VOLUME_WALK, SLIDE_SOUND_VOLUME,
};
use super::PlayingState;

// ── Spatial audio helpers ──
impl PlayingState {
    /// Play a sound with distance-based volume falloff and doppler pitch shift.
    /// `source_pos` is the world position of the emitter.
    /// `source_vel` is the emitter's velocity (world units/sec) — used for doppler.
    /// `base_volume` is the volume at distance 0 (before attenuation).
    pub fn play_spatial_sound(
        &self,
        sound_key: &str,
        source_pos: Vec3,
        source_vel: Vec3,
        base_volume: f32,
        config: &GameConfig,
    ) {
        if config.mute_audio {
            return;
        }
        if !self.audio.has(sound_key) {
            return;
        }

        let listener_pos = self.position;
        let diff = source_pos - listener_pos;
        let dist = diff.length();
        if dist > FOOTSTEP_MAX_DIST {
            return; // too far, don't bother
        }

        // distance attenuation: inverse-linear falloff to zero at max dist
        let attenuation = 1.0 - (dist / FOOTSTEP_MAX_DIST);
        let volume = base_volume * attenuation * attenuation; // squared for more natural falloff

        // doppler effect: pitch shifts based on relative velocity toward/away from listener
        let speed_of_sound = 50.0; // tuned for game units (not real-world 343 m/s)
        let doppler_pitch = if dist > 0.1 {
            // unit vector from source toward listener
            let dir_to_listener = diff / dist;
            // positive = source moving toward listener
            let v_source = source_vel.dot(-dir_to_listener);
            // listener velocity: approximate from position (we don't track it, so use 0)
            // doppler: f' = f * (v_sound) / (v_sound + v_source)
            // v_source positive when moving AWAY from listener
            let denom = (speed_of_sound + v_source).max(1.0);
            speed_of_sound / denom
        } else {
            1.0
        };

        // add small random variation to avoid robotic repetition
        let pitch_variation = macroquad::rand::gen_range(0.95f32, 1.05f32);
        let final_pitch = (doppler_pitch * pitch_variation).clamp(0.5, 2.0);

        self.audio.play(sound_key, final_pitch, volume);
    }

    /// Play a sound at the local player's position (no distance attenuation, no doppler).
    fn play_local_sound(&self, sound_key: &str, volume: f32, config: &GameConfig) {
        if config.mute_audio {
            return;
        }
        let pitch = macroquad::rand::gen_range(0.95f32, 1.05f32);
        self.audio.play(sound_key, pitch, volume);
    }

    /// The local player's own footstep and slide sounds.
    pub(super) fn update_local_footsteps(&mut self, delta: f32, config: &GameConfig) {
        // ── Local player footstep sounds ──
        if !self.is_dead && self.movement_state.is_grounded {
            let is_moving = self.move_dir.length_squared() > 0.01;
            let is_sliding = self.movement_state.is_sliding;

            if is_sliding {
                if self.local_footstep_timer <= 0.0 {
                    self.play_local_sound("slide", SLIDE_SOUND_VOLUME, config);
                    self.local_footstep_timer = 1.0;
                }
            } else if is_moving {
                self.local_footstep_timer -= delta;
                if self.local_footstep_timer <= 0.0 {
                    let (interval, volume) = if self.movement_state.is_sprinting {
                        (FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_VOLUME_SPRINT)
                    } else if self.movement_state.is_crouching {
                        (FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_VOLUME_CROUCH)
                    } else {
                        (FOOTSTEP_INTERVAL_WALK, FOOTSTEP_VOLUME_WALK)
                    };
                    self.play_local_sound("footstep", volume, config);
                    self.local_footstep_timer = interval;
                }
            } else {
                self.local_footstep_timer = 0.0;
            }
        } else {
            self.local_footstep_timer = 0.0;
        }
    }
}
