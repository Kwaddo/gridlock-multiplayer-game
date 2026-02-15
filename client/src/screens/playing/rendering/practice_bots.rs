use super::super::PlayingState;
use macroquad::prelude::*;

impl PlayingState {
    pub(super) fn draw_practice_bots(&self) {
        for bot in &self.practice_bots {
            let p = &bot.player;
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
}
