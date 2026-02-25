use macroquad::prelude::*;

use super::super::PlayingState;

impl PlayingState {
    /// Draw the Search & Destroy HUD overlay
    pub(crate) fn draw_snd_hud(&self, player_id: u32) {
        let sw = screen_width();
        let sh = screen_height();

        // ── Round counter (top center) ──
        let round_text = format!("Round {}/{}", self.snd_round, self.snd_max_rounds);
        let round_w = crate::ui::fonts::measure_game_text(&round_text, 22);
        draw_text_ex(
            &round_text,
            (sw - round_w) / 2.0,
            30.0,
            crate::ui::fonts::text_params(22, Color::from_rgba(220, 220, 220, 255)),
        );

        // Team scores
        let red_score = format!("RED {}", self.snd_red_wins);
        let blue_score = format!("{} BLUE", self.snd_blue_wins);
        let red_w = crate::ui::fonts::measure_game_text(&red_score, 18);
        let blue_w = crate::ui::fonts::measure_game_text(&blue_score, 18);
        draw_text_ex(
            &red_score,
            sw / 2.0 - round_w / 2.0 - red_w - 16.0,
            30.0,
            crate::ui::fonts::text_params(18, Color::from_rgba(230, 100, 100, 255)),
        );
        draw_text_ex(
            &blue_score,
            sw / 2.0 + round_w / 2.0 + 16.0,
            30.0,
            crate::ui::fonts::text_params(18, Color::from_rgba(100, 150, 230, 255)),
        );

        // ── Phase indicator ──
        let phase_text = match self.snd_phase {
            0 => "FREEZE TIME",
            1 => "ROUND ACTIVE",
            2 => "BOMB PLANTED!",
            3 => "ROUND OVER",
            _ => "",
        };
        let phase_color = match self.snd_phase {
            0 => Color::from_rgba(150, 200, 255, 255),
            2 => Color::from_rgba(255, 80, 80, 255),
            3 => Color::from_rgba(255, 220, 100, 255),
            _ => Color::from_rgba(200, 200, 200, 200),
        };
        let pw = crate::ui::fonts::measure_game_text(phase_text, 16);
        draw_text_ex(
            phase_text,
            (sw - pw) / 2.0,
            52.0,
            crate::ui::fonts::text_params(16, phase_color),
        );

        // ── Bomb status indicator (bottom center) ──
        let local_team = self
            .connected_players
            .iter()
            .find(|(id, ..)| *id == player_id)
            .map(|(_, _name, _kills, _deaths, _gl, team, _wep, _is_bot)| *team)
            .unwrap_or(0u8);

        let is_attacker = local_team == self.snd_attacking_team;

        if self.snd_bomb_planted {
            // Bomb planted indicator
            let bomb_text = "BOMB PLANTED - DEFUSE IT!";
            let bomb_color = if is_attacker {
                Color::from_rgba(100, 255, 100, 255) // green for attackers (good for them)
            } else {
                Color::from_rgba(255, 80, 80, 255) // red for defenders (danger)
            };
            let bw = crate::ui::fonts::measure_game_text(bomb_text, 18);
            draw_text_ex(
                bomb_text,
                (sw - bw) / 2.0,
                sh - 120.0,
                crate::ui::fonts::text_params(18, bomb_color),
            );
        } else if self.snd_bomb_carrier_id == player_id as u32 && player_id != 0 {
            // We have the bomb
            let bomb_text = "YOU HAVE THE BOMB - Plant at bomb site [E]";
            let bw = crate::ui::fonts::measure_game_text(bomb_text, 16);
            draw_text_ex(
                bomb_text,
                (sw - bw) / 2.0,
                sh - 120.0,
                crate::ui::fonts::text_params(16, Color::from_rgba(255, 200, 50, 255)),
            );
        } else if self.snd_bomb_dropped.is_some() && is_attacker {
            // Bomb is on the ground
            let bomb_text = "BOMB DROPPED - Pick it up!";
            let bw = crate::ui::fonts::measure_game_text(bomb_text, 16);
            draw_text_ex(
                bomb_text,
                (sw - bw) / 2.0,
                sh - 120.0,
                crate::ui::fonts::text_params(16, Color::from_rgba(255, 150, 50, 255)),
            );
        }

        // ── Plant/Defuse progress bar ──
        if self.snd_plant_progress > 0.0 && self.snd_plant_progress < 1.0 {
            Self::draw_progress_bar(
                sw / 2.0,
                sh - 90.0,
                200.0,
                12.0,
                self.snd_plant_progress,
                "PLANTING...",
                Color::from_rgba(255, 180, 50, 255),
            );
        }
        if self.snd_defuse_progress > 0.0 && self.snd_defuse_progress < 1.0 {
            Self::draw_progress_bar(
                sw / 2.0,
                sh - 90.0,
                200.0,
                12.0,
                self.snd_defuse_progress,
                "DEFUSING...",
                Color::from_rgba(50, 180, 255, 255),
            );
        }

        // ── Spectating overlay when dead ──
        if self.is_dead && self.snd_phase != 3 {
            // Dark overlay
            draw_rectangle(0.0, 0.0, sw, 40.0, Color::from_rgba(0, 0, 0, 150));
            let spec_text = "SPECTATING - Waiting for round to end";
            let spec_w = crate::ui::fonts::measure_game_text(spec_text, 16);
            draw_text_ex(
                spec_text,
                (sw - spec_w) / 2.0,
                25.0,
                crate::ui::fonts::text_params(16, Color::from_rgba(200, 200, 200, 220)),
            );
        }

        // ── Bomb site proximity hint ──
        if !self.is_dead && self.snd_phase == 1 {
            let dx = self.position.x - self.snd_bomb_site.0;
            let dz = self.position.z - self.snd_bomb_site.2;
            let dist = (dx * dx + dz * dz).sqrt();

            if dist <= 3.5 {
                if is_attacker
                    && self.snd_bomb_carrier_id == player_id as u32
                    && !self.snd_bomb_planted
                {
                    let hint = "Hold [E] to plant the bomb";
                    let hw = crate::ui::fonts::measure_game_text(hint, 18);
                    draw_text_ex(
                        hint,
                        (sw - hw) / 2.0,
                        sh / 2.0 + 40.0,
                        crate::ui::fonts::text_params(18, Color::from_rgba(255, 220, 100, 230)),
                    );
                }
            }
        }

        // Defuse hint when near planted bomb
        if !self.is_dead && self.snd_phase == 2 && !is_attacker {
            let dx = self.position.x - self.snd_bomb_site.0;
            let dz = self.position.z - self.snd_bomb_site.2;
            let dist = (dx * dx + dz * dz).sqrt();

            if dist <= 3.5 {
                let hint = "Hold [E] to defuse the bomb";
                let hw = crate::ui::fonts::measure_game_text(hint, 18);
                draw_text_ex(
                    hint,
                    (sw - hw) / 2.0,
                    sh / 2.0 + 40.0,
                    crate::ui::fonts::text_params(18, Color::from_rgba(100, 200, 255, 230)),
                );
            }
        }
    }

    fn draw_progress_bar(
        center_x: f32,
        y: f32,
        width: f32,
        height: f32,
        progress: f32,
        label: &str,
        color: Color,
    ) {
        let x = center_x - width / 2.0;

        // Background
        draw_rectangle(
            x - 2.0,
            y - 2.0,
            width + 4.0,
            height + 4.0,
            Color::from_rgba(0, 0, 0, 180),
        );

        // Fill
        let fill_w = width * progress.clamp(0.0, 1.0);
        draw_rectangle(x, y, fill_w, height, color);

        // Border
        draw_rectangle_lines(
            x,
            y,
            width,
            height,
            1.0,
            Color::from_rgba(200, 200, 200, 150),
        );

        // Label
        let lw = crate::ui::fonts::measure_game_text(label, 14);
        draw_text_ex(
            label,
            center_x - lw / 2.0,
            y - 6.0,
            crate::ui::fonts::text_params(14, Color::from_rgba(255, 255, 255, 220)),
        );
    }
}
