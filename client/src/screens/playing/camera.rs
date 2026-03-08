use crate::core::GameConfig;
use macroquad::prelude::*;

use super::state::DeathCameraPhase;
use super::PlayingState;

/// Duration of the zoom-out phase (seconds)
const DEATH_CAM_ZOOM_DURATION: f32 = 0.5;
/// How far behind the corpse the camera ends up
const DEATH_CAM_BEHIND_DIST: f32 = 3.0;
/// How far above the corpse the camera ends up
const DEATH_CAM_HEIGHT: f32 = 2.0;

impl PlayingState {
    pub(super) fn build_camera(&self, config: &GameConfig) -> Camera3D {
        // If in death camera mode, use the death camera instead
        if let Some((t_interp, corpse_pos, corpse_yaw)) = match self.death_camera_phase {
            DeathCameraPhase::ZoomOut { timer, corpse_pos, corpse_yaw } => {
                Some(((timer / DEATH_CAM_ZOOM_DURATION).clamp(0.0, 1.0), corpse_pos, corpse_yaw))
            }
            DeathCameraPhase::Watching { corpse_pos, corpse_yaw, .. } => {
                Some((1.0, corpse_pos, corpse_yaw))
            }
            _ => None,
        } {
            // Smooth ease-out curve
            let t = 1.0 - (1.0 - t_interp) * (1.0 - t_interp);

            // First-person eye position (start)
            let fp_eye = vec3(
                corpse_pos.x,
                corpse_pos.y + self.movement_state.height,
                corpse_pos.z,
            );

            // Third-person position: behind and above the corpse
            let behind_dir = vec3(-corpse_yaw.cos(), 0.0, -corpse_yaw.sin());
            let tp_eye = vec3(
                corpse_pos.x + behind_dir.x * DEATH_CAM_BEHIND_DIST,
                corpse_pos.y + DEATH_CAM_HEIGHT,
                corpse_pos.z + behind_dir.z * DEATH_CAM_BEHIND_DIST,
            );

            // Interpolate between first-person and third-person
            let eye = fp_eye + (tp_eye - fp_eye) * t;
            // Look at the corpse's chest area
            let look_at = vec3(corpse_pos.x, corpse_pos.y + 0.8, corpse_pos.z);

            return Camera3D {
                position: eye,
                target: look_at,
                up: vec3(0.0, 1.0, 0.0),
                fovy: config.raw_fov().to_radians(),
                aspect: None,
                z_near: 0.01,
                z_far: 100.0,
                projection: Projection::Perspective,
                render_target: None,
                viewport: None,
            };
        }

        let bob_offset = if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer.sin() * 0.05
        } else {
            0.0
        };

        let forward = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );

        let eye_pos = vec3(
            self.position.x,
            self.position.y + self.movement_state.height + bob_offset,
            self.position.z,
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
            z_near: 0.01,
            z_far: 100.0,
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        }
    }
}
