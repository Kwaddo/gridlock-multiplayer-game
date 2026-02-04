use macroquad::prelude::*;
pub mod protocols;

#[macroquad::main("Multiplayer FPS")]
async fn main() {
    // Capture mouse
    set_cursor_grab(true);
    show_mouse(false);

    // Player position and rotation
    let mut position = vec3(0.0, 1.0, 0.0);
    let mut yaw = 0.0; // horizontal rotation
    let mut pitch = 0.0; // vertical rotation

    let mut last_mouse_pos = mouse_position();

    loop {
        let delta_time = get_frame_time();

        // Handle mouse look
        let mouse_pos = mouse_position();
        let mouse_delta = (
            mouse_pos.0 - last_mouse_pos.0,
            mouse_pos.1 - last_mouse_pos.1,
        );
        last_mouse_pos = mouse_pos;

        yaw -= mouse_delta.0 * 0.002;
        pitch -= mouse_delta.1 * 0.002;
        pitch = pitch.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

        // Calculate forward and right vectors
        let forward = vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        );
        let right = vec3(-yaw.sin(), 0.0, yaw.cos());

        // Handle movement
        let mut move_dir = vec3(0.0, 0.0, 0.0);
        if is_key_down(KeyCode::W) {
            move_dir += forward;
        }
        if is_key_down(KeyCode::S) {
            move_dir -= forward;
        }
        if is_key_down(KeyCode::A) {
            move_dir -= right;
        }
        if is_key_down(KeyCode::D) {
            move_dir += right;
        }
        if move_dir != vec3(0.0, 0.0, 0.0) {
            move_dir = move_dir.normalize();
            position += move_dir * 5.0 * delta_time;
        }

        // Update camera
        let camera = Camera3D {
            position,
            target: position + forward,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 45.0,
            aspect: None,
            z_near: 0.1,
            z_far: 100.0,
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        };

        clear_background(BLACK);

        set_camera(&camera);

        // Draw ground
        draw_cube(vec3(0.0, -1.0, 0.0), vec3(20.0, 0.1, 20.0), None, GRAY);

        // Draw some cubes
        draw_cube(vec3(0.0, 0.0, -5.0), vec3(1.0, 1.0, 1.0), None, RED);
        draw_cube(vec3(3.0, 0.0, -5.0), vec3(1.0, 1.0, 1.0), None, GREEN);

        // Draw 2D UI
        set_default_camera();
        draw_text("Multiplayer FPS", 20.0, 20.0, 30.0, WHITE);
        draw_text(
            &format!(
                "Pos: {:.2}, {:.2}, {:.2}",
                position.x, position.y, position.z
            ),
            20.0,
            50.0,
            20.0,
            WHITE,
        );

        next_frame().await
    }
}
