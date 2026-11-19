//! Emoting, Gmod `act` style: the view blends out to third person, the mouse
//! orbits the camera around the frozen body, the gun is put away, and any
//! movement key ends the emote (the view then blends back to first person
//! before the dance stops).

use macroquad::prelude::*;

use crate::core::GameConfig;
use crate::net::protocols::EMOTE_NONE;
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
/// Camera blend past which your own body is drawn; nearer, the camera is inside the head.
const SHOW_BODY_VIEW_T: f32 = 0.3;
/// Bindings that end an emote: only movement, the gun is disabled instead.
const ENDS_EMOTE: [&str; 9] =
    ["Forward", "Backward", "Left", "Right", "Sprint", "Crouch", "Jump", "Slow Walk", "Prone"];

impl PlayingState {
    /// An emote is playing, including the blend back to first person.
    pub(super) fn is_emoting(&self) -> bool {
        self.emote_wheel.is_playing()
    }

    /// Camera blend, 0 = first person (viewmodel and crosshair), above 0 = leaving it.
    pub(super) fn emote_view_t(&self) -> f32 {
        self.emote_timeline.view_t()
    }

    pub(super) fn shows_own_body(&self) -> bool {
        self.emote_view_t() > SHOW_BODY_VIEW_T
    }

    /// Where the body faces: frozen while emoting, otherwise where you look.
    pub(super) fn body_yaw(&self) -> f32 {
        if self.is_emoting() {
            self.emote_wheel.body_yaw
        } else {
            self.yaw
        }
    }

    /// Per-frame emote clock; clears the emote once the view is back in first person.
    pub(super) fn tick_emote(&mut self, dt: f32) {
        if self.emote_timeline.tick(dt, self.emote_wheel.playing) {
            self.emote_wheel.playing = EMOTE_NONE;
        }
    }

    /// End a running emote when this frame holds or presses a movement key.
    pub(super) fn interrupt_emote(&mut self, config: &GameConfig) {
        if !self.is_emoting() || self.emote_timeline.is_ending() {
            return;
        }
        let moved = self.move_dir != Vec3::ZERO
            || config
                .all_bindings()
                .iter()
                .any(|(name, bind)| ENDS_EMOTE.contains(name) && bind.is_pressed());
        if moved {
            self.emote_timeline.stop();
        }
    }

    /// Blend the first-person camera towards the third-person orbit by the emote's view blend.
    pub(super) fn apply_emote_view(&self, fp: Camera3D, config: &GameConfig, maze: Option<&Maze>) -> Camera3D {
        let t = self.emote_view_t();
        if t <= 0.0 || self.is_dead {
            return fp;
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

        let tp_fov = config.raw_fov().to_radians();
        Camera3D {
            position: fp.position.lerp(eye, t),
            target: fp.target.lerp(pivot, t),
            fovy: fp.fovy + (tp_fov - fp.fovy) * t,
            ..fp
        }
    }
}
