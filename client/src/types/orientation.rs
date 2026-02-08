use macroquad::prelude::*;

pub struct Orientation {
    pub yaw: f32,
    pub pitch: f32,
    pub last_mouse_pos: (f32, f32),
    pub forward: Vec3,
    pub forward_flat: Vec3,
    pub right: Vec3,
}

impl Orientation {
    pub fn new() -> Self {
        let last_mouse_pos = mouse_position();
        Self {
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos,
            forward: vec3(0.0, 0.0, 1.0),
            forward_flat: vec3(0.0, 0.0, 1.0),
            right: vec3(1.0, 0.0, 0.0),
        }
    }

    pub fn update(&mut self, sensitivity: f32) {
        let mouse_pos = mouse_position();
        let mouse_delta = (mouse_pos.0 - self.last_mouse_pos.0, mouse_pos.1 - self.last_mouse_pos.1);
        self.last_mouse_pos = mouse_pos;

        self.yaw += mouse_delta.0 * sensitivity;
        self.pitch -= mouse_delta.1 * sensitivity;
        self.pitch = self.pitch.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

        self.forward = vec3(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos());
        self.forward_flat = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        self.right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
    }
}
