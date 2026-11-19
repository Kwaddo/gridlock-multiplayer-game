//! Emoting, Gmod `act` style: the view drops to third person, the mouse orbits
//! the camera around the frozen body, and any gameplay key ends the emote.

use macroquad::prelude::*;

use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::types::world::WorldId;

use super::PlayingState;

/// How far the orbit camera sits from the body.
const CAM_DISTANCE: f32 = 3.0;
/// Height of the point the camera orbits (about the chest).
const CAM_PIVOT_HEIGHT: f32 = 1.2;
/// Gap kept between the camera and a wall it would otherwise sit in.
const CAM_WALL_GAP: f32 = 0.2;
/// Lowest the camera goes above the feet, so it never sinks into the floor.
const CAM_MIN_HEIGHT: f32 = 0.2;
/// Bindings that don't count as an action: menus, chat, window, the wheel itself.
const NON_ACTIONS: [&str; 4] = ["Pause", "Fullscreen", "Chat", "Emote"];

impl PlayingState {
    pub(super) fn is_emoting(&self) -> bool {
        self.emote_wheel.is_playing()
    }

    /// Where the body faces: frozen while emoting, otherwise where you look.
    pub(super) fn body_yaw(&self) -> f32 {
        if self.is_emoting() {
            self.emote_wheel.body_yaw
        } else {
            self.yaw
        }
    }

    /// True when this frame holds a movement key or presses any gameplay binding.
    pub(super) fn emote_interrupted(&self, config: &GameConfig) -> bool {
        self.move_dir != Vec3::ZERO
            || config
                .all_bindings()
                .iter()
                .any(|(name, bind)| !NON_ACTIONS.contains(name) && bind.is_pressed())
    }

    /// Third-person orbit camera while emoting, pulled in front of walls.
    pub(super) fn emote_camera(&self, config: &GameConfig, maze: Option<&Maze>) -> Option<Camera3D> {
        if !self.is_emoting() || self.is_dead {
            return None;
        }
        let forward = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );
        let pivot = self.position + vec3(0.0, CAM_PIVOT_HEIGHT, 0.0);
        let back = -forward;
        let hit = match self.current_world {
            WorldId::Lobby => crate::world::lobby::ray_test_lobby(pivot, back, CAM_DISTANCE),
            WorldId::Maze => maze.and_then(|m| m.ray_test_walls(pivot, back, CAM_DISTANCE)).map(|(_, d)| d),
        };
        let dist = hit.map_or(CAM_DISTANCE, |d| (d - CAM_WALL_GAP).max(0.0));
        let mut eye = pivot + back * dist;
        eye.y = eye.y.max(self.position.y + CAM_MIN_HEIGHT);

        Some(Camera3D {
            position: eye,
            target: pivot,
            up: vec3(0.0, 1.0, 0.0),
            fovy: config.raw_fov().to_radians(),
            aspect: None,
            z_near: 0.01,
            z_far: crate::core::VIEW_DISTANCE,
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        })
    }
}
