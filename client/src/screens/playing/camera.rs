use macroquad::prelude::*;
use crate::core::GameConfig;

use super::PlayingState;

impl PlayingState {
    pub(super) fn build_camera(&self, config: &GameConfig) -> Camera3D {
        let bob_offset = if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer.sin() * 0.05
        } else {
            0.0
        };

        let forward = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos()
        );

        let eye_pos = vec3(
            self.position.x,
            self.position.y + self.movement_state.height + bob_offset,
            self.position.z
        );

        // ADS zoom: interpolate FOV based on ads_progress
        let weapon = self.weapons.active();
        let base_fov = config.raw_fov();
        let ads_fov = weapon.stats().ads_zoom_fov;
        let fov = base_fov + (ads_fov - base_fov) * weapon.ads_progress;

        Camera3D {
            position: eye_pos,
            target: eye_pos + forward,
            up: vec3(0.0, 1.0, 0.0),
            fovy: fov.to_radians(),
            aspect: None,
            z_near: 0.1,
            z_far: 100.0,
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        }
    }
}
