use crate::audio::catalog;
use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::types::world::WorldId;

use macroquad::prelude::*;

use super::constants::{
    FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_INTERVAL_WALK, FOOTSTEP_MAX_DIST,
    FOOTSTEP_VOLUME_CROUCH, FOOTSTEP_VOLUME_SPRINT, FOOTSTEP_VOLUME_WALK, SLIDE_SOUND_VOLUME,
    SOUND_EAR_HEIGHT, SOUND_MAX_PAN, SOUND_OCCLUDED_VOLUME,
};
use super::PlayingState;

// ── Spatial audio helpers ──
impl PlayingState {
    /// Play a sound placed in the world: distance falloff, stereo pan from
    /// where it is around the listener, muffled when a maze wall stands
    /// between them, and a doppler pitch shift.
    /// `source_vel` is the emitter's velocity (world units/sec).
    /// `base_volume` is the volume at distance 0 (before attenuation).
    pub fn play_spatial_sound(
        &self,
        sound_key: &str,
        source_pos: Vec3,
        source_vel: Vec3,
        base_volume: f32,
        maze: Option<&Maze>,
    ) {
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
        let mut volume = base_volume * attenuation * attenuation; // squared for more natural falloff
        if self.wall_between(listener_pos, source_pos, maze) {
            volume *= SOUND_OCCLUDED_VOLUME;
        }

        // doppler effect: pitch shifts based on relative velocity toward/away from listener
        let speed_of_sound = 50.0; // tuned for game units (not real-world 343 m/s)
        let (doppler_pitch, pan) = if dist > 0.1 {
            // unit vector from source toward listener
            let dir_to_listener = diff / dist;
            // positive = source moving away from listener
            let v_source = source_vel.dot(-dir_to_listener);
            // doppler: f' = f * (v_sound) / (v_sound + v_source)
            let denom = (speed_of_sound + v_source).max(1.0);
            // pan: how far the source sits to the listener's right (-1..1);
            // capped so the far ear still hears it
            let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
            let flat = vec3(diff.x, 0.0, diff.z).normalize_or_zero();
            (speed_of_sound / denom, flat.dot(right) * SOUND_MAX_PAN)
        } else {
            (1.0, 0.0)
        };

        // add small random variation to avoid robotic repetition
        let pitch_variation = macroquad::rand::gen_range(0.95f32, 1.05f32);
        let final_pitch = (doppler_pitch * pitch_variation).clamp(0.5, 2.0);

        self.audio.play_panned(sound_key, final_pitch, volume, pan);
    }

    /// Whether a maze wall blocks the straight line between two points
    /// (at ear height). Always false outside the Maze.
    fn wall_between(&self, a: Vec3, b: Vec3, maze: Option<&Maze>) -> bool {
        let Some(m) = maze.filter(|_| self.current_world == WorldId::Maze) else {
            return false;
        };
        let a = vec3(a.x, SOUND_EAR_HEIGHT, a.z);
        let b = vec3(b.x, SOUND_EAR_HEIGHT, b.z);
        let d = b - a;
        let len = d.length();
        if len < 0.01 {
            return false;
        }
        m.ray_hits_wall_before(a, d / len, len, len)
    }

    /// Play a sound at the local player's position (no distance attenuation, no doppler).
    fn play_local_sound(&self, sound_key: &str, volume: f32) {
        let pitch = macroquad::rand::gen_range(0.95f32, 1.05f32);
        self.audio.play(sound_key, pitch, volume);
    }

    /// The local player's own footstep, slide, jump and landing sounds.
    pub(super) fn update_local_footsteps(&mut self, delta: f32, _config: &GameConfig) {
        if self.just_jumped {
            self.audio.play_cue(catalog::JUMP);
        }
        if self.just_landed {
            self.audio.play_cue(catalog::LAND);
        }

        // ── Local player footstep sounds ──
        if !self.is_dead && self.movement_state.is_grounded {
            // slow walk is silent, stalking, before this it made normal footsteps
            let is_moving = self.move_dir.length_squared() > 0.01 && !self.movement_state.is_slow_walking;
            let is_sliding = self.movement_state.is_sliding;

            if is_sliding {
                if self.local_footstep_timer <= 0.0 {
                    self.play_local_sound(catalog::SLIDE.key, SLIDE_SOUND_VOLUME);
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
                    // your own steps sit a little under other players' so theirs stand out
                    self.play_local_sound(catalog::FOOTSTEP.key, volume * 0.7);
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
