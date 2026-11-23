use macroquad::prelude::*;

use super::net::NetState;

/// Where another player's body is drawn: eased toward the server's position
/// (`NetState`) every frame, so 20 Hz updates move it smoothly.
pub struct RenderSmoothing {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    // the smoothed pitch the body bends to
    pub pitch: f32,
    has_snapshot: bool,
}

impl RenderSmoothing {
    const POSITION_SMOOTH_RATE: f32 = 22.0;
    const YAW_SMOOTH_RATE: f32 = 18.0;
    const SNAP_DISTANCE: f32 = 5.0;

    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, yaw: 0.0, pitch: 0.0, has_snapshot: false }
    }

    pub fn position(&self) -> Vec3 {
        vec3(self.x, self.y, self.z)
    }

    /// A new server position came in. The first one, or one more than
    /// `SNAP_DISTANCE` away (a respawn or teleport), is jumped to; anything
    /// else is eased toward by `ease_toward`.
    pub fn on_snapshot(&mut self, x: f32, y: f32, z: f32, yaw: f32) {
        if !self.has_snapshot {
            self.x = x;
            self.y = y;
            self.z = z;
            self.yaw = yaw;
            self.has_snapshot = true;
            return;
        }

        let dx = self.x - x;
        let dy = self.y - y;
        let dz = self.z - z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq > Self::SNAP_DISTANCE * Self::SNAP_DISTANCE {
            self.x = x;
            self.y = y;
            self.z = z;
            self.yaw = yaw;
        }
    }

    pub fn ease_toward(&mut self, net: &NetState, delta: f32) {
        if !self.has_snapshot {
            return;
        }

        let pos_alpha = 1.0 - (-Self::POSITION_SMOOTH_RATE * delta).exp();
        self.x += (net.x - self.x) * pos_alpha;
        self.y += (net.y - self.y) * pos_alpha;
        self.z += (net.z - self.z) * pos_alpha;

        let yaw_alpha = 1.0 - (-Self::YAW_SMOOTH_RATE * delta).exp();
        self.yaw = lerp_angle(self.yaw, net.yaw, yaw_alpha);
        // pitch never wraps (its +-89 degrees), a plain ease is enough
        self.pitch += (net.pitch - self.pitch) * yaw_alpha;
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut delta = to - from;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    from + delta * t
}
