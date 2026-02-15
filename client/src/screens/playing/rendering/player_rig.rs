use super::super::PlayingState;
use macroquad::{models::Vertex, prelude::*};

impl PlayingState {
    pub(super) fn draw_other_players(&self) {
        for p in &self.other_players {
            let base = if p.skin.is_some() { WHITE } else { p.color };
            let color = if p.hit_timer > 0.0 {
                Color {
                    r: (base.r * 0.6 + 0.4).min(1.0),
                    g: (base.g * 0.4 + 0.2).min(1.0),
                    b: (base.b * 0.4 + 0.2).min(1.0),
                    a: base.a,
                }
            } else {
                base
            };

            self.draw_player_rig(p, color);
        }
    }

    pub(super) fn draw_player_rig(&self, p: &crate::types::player::OtherPlayer, color: Color) {
        let pos = vec3(p.x, p.y, p.z);
        let yaw = p.yaw;
        let clamped_pitch = p.pitch.clamp(-1.3, 1.3);

        let forward = vec3(yaw.cos(), 0.0, yaw.sin());
        let right = vec3(-yaw.sin(), 0.0, yaw.cos());
        let head_dir = vec3(
            yaw.cos() * clamped_pitch.cos(),
            clamped_pitch.sin(),
            yaw.sin() * clamped_pitch.cos(),
        );

        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z,
        )
        .length();
        let walk_frac = (horizontal_speed / 4.0).clamp(0.0, 1.0);
        let sway_phase = self.bob_timer * 1.6;
        let sway_side = sway_phase.sin() * 0.06 * walk_frac;
        let sway_fwd = sway_phase.cos() * 0.04 * walk_frac;
        let sway_vert = sway_phase.sin().abs() * 0.03 * walk_frac;

        let hips_center = pos + vec3(0.0, 0.6 + sway_vert * 0.3, 0.0)
            - forward * (0.02 - sway_fwd * 0.25)
            + right * (sway_side * 0.6);
        self.draw_oriented_box(hips_center, vec3(0.20, 0.17, 0.15), yaw, 0.06, color);

        let lower_torso_center =
            hips_center + vec3(0.0, 0.22 + sway_vert * 0.15, 0.03 + sway_fwd * 0.12);
        self.draw_oriented_box(
            lower_torso_center,
            vec3(0.23, 0.18, 0.15),
            yaw,
            0.10 + sway_side * 0.15,
            Self::tint_color(color, 1.02, 0.0),
        );

        let upper_torso_center = lower_torso_center + vec3(0.0, 0.26, 0.02);
        self.draw_oriented_box(
            upper_torso_center,
            vec3(0.26, 0.20, 0.16),
            yaw,
            0.12 + sway_side * 0.18,
            Self::tint_color(color, 1.04, 0.0),
        );

        let neck_center =
            upper_torso_center + vec3(0.0, 0.24 + sway_vert * 0.15, 0.05 + sway_fwd * 0.08);
        self.draw_oriented_box(
            neck_center,
            vec3(0.08, 0.09, 0.08),
            yaw,
            0.12,
            Self::tint_color(color, 1.05, 0.01),
        );

        let head_center = neck_center
            + vec3(0.0, 0.16 + sway_vert * 0.15, 0.02 + sway_fwd * 0.08)
            + head_dir * 0.06
            + right * (sway_side * 0.35);
        let head_color = Self::tint_color(color, 1.10, 0.025);
        self.draw_oriented_box(
            head_center,
            vec3(0.15, 0.16, 0.14),
            yaw,
            clamped_pitch,
            head_color,
        );
        draw_line_3d(
            head_center,
            head_center + head_dir * 0.32,
            Color::from_rgba(255, 255, 255, 140),
        );

        let limb_color = Self::tint_color(color, 0.95, 0.0);
        let arm_offset = right * 0.42;
        let arm_drop = vec3(0.0, -0.04, 0.06);
        let arm_sway = right * (sway_side * 0.5)
            + forward * (sway_fwd * 0.6)
            + vec3(0.0, sway_vert * 0.3, 0.0);
        self.draw_oriented_box(
            upper_torso_center + arm_offset + arm_drop + arm_sway,
            vec3(0.10, 0.34, 0.09),
            yaw,
            0.05 + sway_side * 0.3,
            limb_color,
        );
        self.draw_oriented_box(
            upper_torso_center - arm_offset + arm_drop - arm_sway,
            vec3(0.10, 0.34, 0.09),
            yaw,
            0.05 - sway_side * 0.3,
            limb_color,
        );

        let leg_offset = right * 0.17;
        let leg_pitch = 0.05;
        let step = 0.22 * walk_frac;
        self.draw_oriented_box(
            pos + vec3(0.0, 0.28 - sway_vert * 0.18, 0.0)
                + leg_offset
                + forward * (sway_phase.sin() * step),
            vec3(0.10, 0.27, 0.10),
            yaw,
            leg_pitch + sway_side * 0.15,
            limb_color,
        );
        self.draw_oriented_box(
            pos + vec3(0.0, 0.28 - sway_vert * 0.18, 0.0)
                - leg_offset
                - forward * (sway_phase.sin() * step),
            vec3(0.10, 0.27, 0.10),
            yaw,
            leg_pitch - sway_side * 0.15,
            limb_color,
        );
    }

    fn draw_oriented_box(
        &self,
        center: Vec3,
        half_extents: Vec3,
        yaw: f32,
        pitch: f32,
        color: Color,
    ) {
        let hx = half_extents.x;
        let hy = half_extents.y;
        let hz = half_extents.z;

        let rot = Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch);

        let faces = [
            (
                vec3(0.0, 0.0, -1.0),
                [
                    vec3(-hx, -hy, -hz),
                    vec3(hx, -hy, -hz),
                    vec3(hx, hy, -hz),
                    vec3(-hx, hy, -hz),
                ],
            ),
            (
                vec3(0.0, 0.0, 1.0),
                [
                    vec3(-hx, -hy, hz),
                    vec3(hx, -hy, hz),
                    vec3(hx, hy, hz),
                    vec3(-hx, hy, hz),
                ],
            ),
            (
                vec3(-1.0, 0.0, 0.0),
                [
                    vec3(-hx, -hy, -hz),
                    vec3(-hx, -hy, hz),
                    vec3(-hx, hy, hz),
                    vec3(-hx, hy, -hz),
                ],
            ),
            (
                vec3(1.0, 0.0, 0.0),
                [
                    vec3(hx, -hy, -hz),
                    vec3(hx, -hy, hz),
                    vec3(hx, hy, hz),
                    vec3(hx, hy, -hz),
                ],
            ),
            (
                vec3(0.0, 1.0, 0.0),
                [
                    vec3(-hx, hy, -hz),
                    vec3(hx, hy, -hz),
                    vec3(hx, hy, hz),
                    vec3(-hx, hy, hz),
                ],
            ),
            (
                vec3(0.0, -1.0, 0.0),
                [
                    vec3(-hx, -hy, -hz),
                    vec3(hx, -hy, -hz),
                    vec3(hx, -hy, hz),
                    vec3(-hx, -hy, hz),
                ],
            ),
        ];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (face_index, (_normal, corners)) in faces.into_iter().enumerate() {
            let base = (face_index * 4) as u16;

            for corner in corners {
                let world_pos = rot.transform_point3(corner) + center;
                vertices.push(Vertex::new2(world_pos, vec2(0.0, 0.0), color));
            }

            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        let mesh = Mesh {
            vertices,
            indices,
            texture: None,
        };

        draw_mesh(&mesh);
    }

    fn tint_color(color: Color, mul: f32, add: f32) -> Color {
        fn clamp(v: f32) -> f32 {
            v.max(0.0).min(1.0)
        }

        Color {
            r: clamp(color.r * mul + add),
            g: clamp(color.g * mul + add),
            b: clamp(color.b * mul + add),
            a: color.a,
        }
    }
}
