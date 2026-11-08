use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    pub(super) fn draw_fps_counter(&mut self) {
        // FPS counter — averaged over 0.5s window so the number is stable
        let dt = get_frame_time();
        self.fps_frame_count += 1;
        self.fps_time_acc += dt;
        if self.fps_time_acc >= 0.5 {
            self.fps_display = (self.fps_frame_count as f32 / self.fps_time_acc).round() as i32;
            self.fps_frame_count = 0;
            self.fps_time_acc = 0.0;
        }
        let fps = self.fps_display;
        let fps_text = format!("{} FPS", fps);
        let text_w = crate::ui::fonts::measure_game_text(&fps_text, 14);
        let pad = 10.0;
        let pill_w = text_w + pad * 2.0;
        let pill_h = 24.0;
        let pill_x = screen_width() - pill_w - 12.0;
        let pill_y = 12.0;
        draw_rectangle(pill_x, pill_y, pill_w, pill_h, Color::from_rgba(8, 10, 18, 200));
        draw_rectangle_lines(pill_x, pill_y, pill_w, pill_h, 1.0, Color::from_rgba(255, 255, 255, 30));
        let fps_color = if fps >= 60 {
            Color::from_rgba(100, 220, 100, 255)
        } else if fps >= 30 {
            Color::from_rgba(220, 180, 60, 255)
        } else {
            Color::from_rgba(220, 80, 80, 255)
        };
        draw_text_ex(
            &fps_text,
            pill_x + pad,
            pill_y + 17.0,
            crate::ui::fonts::text_params(14, fps_color),
        );
    }

    pub(super) fn draw_death_camera_overlay(&self) {
        use crate::screens::playing::state::DeathCameraPhase;
        match self.death_camera_phase {
            DeathCameraPhase::ZoomOut { timer, .. } => {
                // Slight vignette that builds during zoom-out
                let t = (timer / 0.5).clamp(0.0, 1.0);
                let alpha = (t * 40.0) as u8;
                draw_rectangle(
                    0.0, 0.0, screen_width(), screen_height(),
                    Color::from_rgba(0, 0, 0, alpha),
                );
            }
            DeathCameraPhase::Watching { timer, .. } => {
                // Gradual darkening over the 2s watching period
                let t = (timer / 2.0).clamp(0.0, 1.0);
                let alpha = (40.0 + t * 160.0) as u8;
                draw_rectangle(
                    0.0, 0.0, screen_width(), screen_height(),
                    Color::from_rgba(0, 0, 0, alpha),
                );
                // "Killed by" text during watching phase
                if let Some(killer) = &self.death_killer_name {
                    let txt = if let Some(weapon) = &self.death_weapon_name {
                        format!("Killed by {} with {}", killer, weapon)
                    } else {
                        format!("Killed by {}", killer)
                    };
                    let tw = crate::ui::fonts::measure_game_text(&txt, 22);
                    let text_alpha = ((timer / 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                    draw_text_ex(
                        &txt,
                        (screen_width() - tw) / 2.0,
                        screen_height() * 0.15,
                        crate::ui::fonts::text_params(
                            22,
                            Color::from_rgba(255, 160, 160, text_alpha),
                        ),
                    );
                }
            }
            _ => {}
        }
    }
}
