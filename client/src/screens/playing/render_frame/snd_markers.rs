use macroquad::prelude::*;

use super::PlayingState;

/// Segments in each bomb-site ring.
const RING_STEPS: usize = 32;

/// (cos, sin) at each of the ring's RING_STEPS + 1 points, worked out once.
fn unit_circle() -> &'static [(f32, f32); RING_STEPS + 1] {
    static CIRCLE: std::sync::OnceLock<[(f32, f32); RING_STEPS + 1]> = std::sync::OnceLock::new();
    CIRCLE.get_or_init(|| {
        std::array::from_fn(|i| {
            let a = (i as f32 / RING_STEPS as f32) * std::f32::consts::TAU;
            (a.cos(), a.sin())
        })
    })
}

impl PlayingState {
    /// S&D: the bomb site marker and the dropped/planted bomb in 3D.
    pub(crate) fn draw_snd_world_markers(&self) {
        let cam_pos = self.position;
        let (site_x, site_y, site_z) = self.snd_bomb_site;
        let site_dist =
            ((cam_pos.x - site_x).powi(2) + (cam_pos.z - site_z).powi(2)).sqrt();

        // Bomb site marker — always visible (fade at very close range to avoid clutter)
        let alpha = if site_dist <= 30.0 {
            1.0f32
        } else {
            1.0 - ((site_dist - 30.0) / 10.0).clamp(0.0, 1.0)
        };
        // fully faded out (40 m and further): nothing to draw
        if alpha > 0.0 {
            let pulse = ((get_time() as f32 * (if self.snd_bomb_planted { 4.0 } else { 1.5 })).sin() * 0.5 + 0.5) as f32;
            let site_color = if self.snd_bomb_planted {
                Color::from_rgba(255, 50, 50, (220.0 * alpha) as u8)
            } else {
                Color::from_rgba(255, 200, 50, ((160.0 + 60.0 * pulse) * alpha) as u8)
            };
            let circle = unit_circle();
            // Outer ring
            let r_outer = 2.0;
            for w in circle.windows(2) {
                let ((c1, s1), (c2, s2)) = (w[0], w[1]);
                draw_line_3d(
                    vec3(site_x + c1 * r_outer, 0.02, site_z + s1 * r_outer),
                    vec3(site_x + c2 * r_outer, 0.02, site_z + s2 * r_outer),
                    site_color,
                );
            }
            // Inner ring
            let r_inner = 1.0;
            for w in circle.windows(2) {
                let ((c1, s1), (c2, s2)) = (w[0], w[1]);
                draw_line_3d(
                    vec3(site_x + c1 * r_inner, 0.02, site_z + s1 * r_inner),
                    vec3(site_x + c2 * r_inner, 0.02, site_z + s2 * r_inner),
                    site_color,
                );
            }
            // Cross lines through center
            draw_line_3d(
                vec3(site_x - r_outer, 0.02, site_z),
                vec3(site_x + r_outer, 0.02, site_z),
                site_color,
            );
            draw_line_3d(
                vec3(site_x, 0.02, site_z - r_outer),
                vec3(site_x, 0.02, site_z + r_outer),
                site_color,
            );
            // Vertical pillar so the site is visible over obstacles
            let pillar_alpha = (alpha * 180.0) as u8;
            let pillar_color = if self.snd_bomb_planted {
                Color::from_rgba(255, 50, 50, pillar_alpha)
            } else {
                Color::from_rgba(255, 200, 50, pillar_alpha)
            };
            for seg in 0..8u32 {
                let y0 = site_y + seg as f32 * 0.4;
                let y1 = site_y + (seg + 1) as f32 * 0.4;
                draw_line_3d(vec3(site_x, y0, site_z), vec3(site_x, y1, site_z), pillar_color);
            }
        }

        // Dropped bomb — always visible to dead spectators and to attackers (who
        // need to find it), distance-gated for live defenders
        if let Some((bx, by, bz)) = self.snd_bomb_dropped {
            let bomb_dist =
                ((cam_pos.x - bx).powi(2) + (cam_pos.z - bz).powi(2)).sqrt();
            let is_attacker =
                self.snd_attacking_team != 0 && self.local_team == self.snd_attacking_team;
            if is_attacker && !self.is_dead {
                // Beam above the bomb so attackers can spot it over the walls
                let pulse = (get_time() as f32 * 3.0).sin() * 0.5 + 0.5;
                let beam_color = Color::from_rgba(255, 150, 50, (140.0 + 80.0 * pulse) as u8);
                for seg in 0..10u32 {
                    let y0 = by + 0.6 + seg as f32 * 0.4;
                    let y1 = y0 + 0.4;
                    draw_line_3d(vec3(bx, y0, bz), vec3(bx, y1, bz), beam_color);
                }
            }
            if self.is_dead || is_attacker || bomb_dist <= 10.0 {
                let bomb_alpha = if self.is_dead || is_attacker || bomb_dist <= 6.0 {
                    1.0
                } else {
                    1.0 - ((bomb_dist - 6.0) / 4.0).clamp(0.0, 1.0)
                };
                let bob = (get_time() as f32 * 2.0).sin() * 0.1;
                draw_cube(
                    vec3(bx, 0.3 + bob, bz),
                    vec3(0.3, 0.3, 0.3),
                    None,
                    Color::from_rgba(255, 180, 50, (255.0 * bomb_alpha) as u8),
                );
                draw_cube_wires(
                    vec3(bx, 0.3 + bob, bz),
                    vec3(0.35, 0.35, 0.35),
                    Color::from_rgba(255, 100, 0, (200.0 * bomb_alpha) as u8),
                );
            }
        }

        // Planted bomb at site — always visible to dead spectators
        if self.snd_bomb_planted && (self.is_dead || site_dist <= 10.0) {
            let bomb_alpha = if self.is_dead || site_dist <= 6.0 {
                1.0
            } else {
                1.0 - ((site_dist - 6.0) / 4.0).clamp(0.0, 1.0)
            };
            let pulse =
                ((get_time() as f32 * 4.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let r = (200.0 + 55.0 * pulse) as u8;
            draw_cube(
                vec3(site_x, 0.15, site_z),
                vec3(0.35, 0.25, 0.35),
                None,
                Color::from_rgba(r, 50, 50, (255.0 * bomb_alpha) as u8),
            );
            draw_cube_wires(
                vec3(site_x, 0.15, site_z),
                vec3(0.40, 0.30, 0.40),
                Color::from_rgba(255, 0, 0, (180.0 * bomb_alpha) as u8),
            );
        }
    }
}
