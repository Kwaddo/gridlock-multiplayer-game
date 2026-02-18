use macroquad::prelude::*;

pub fn make_camera(position: Vec3, forward: Vec3) -> Camera3D {
    Camera3D {
        position,
        target: position + forward,
        up: vec3(0.0, 1.0, 0.0),
        fovy: 45.0,
        aspect: None,
        z_near: 0.01,
        z_far: 100.0,
        projection: Projection::Perspective,
        render_target: None,
        viewport: None,
    }
}
