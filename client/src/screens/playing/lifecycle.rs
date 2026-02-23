use crate::types::world::WorldId;

use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    pub fn clear_input(&mut self) {
        self.move_dir = Vec3::ZERO;
        self.movement_state.is_sprinting = false;
        self.jump_requested = false;
    }

    pub fn take_damage(&mut self, damage: f32, attacker_id: Option<u32>) {
        if self.dev_mode.god_mode || self.is_dead || self.invulnerability_timer > 0.0 {
            return;
        }

        if self.shield > 0.0 {
            let shield_damage = damage.min(self.shield);
            self.shield -= shield_damage;
            let remaining_damage = damage - shield_damage;

            if remaining_damage > 0.0 {
                self.hp -= remaining_damage;
            }
        } else {
            self.hp = (self.hp - damage).max(0.0);
        }

        // brief damage flash and attacker info for HUD
        self.damage_flash_timer = 0.6;
        self.last_damage_attacker = attacker_id.and_then(|aid| {
            self.connected_players
                .iter()
                .find(|(id, ..)| *id == aid)
                .map(|(_, name, ..)| name.clone())
        });

        if self.hp <= 0.0 {
            self.die(attacker_id)
        }
    }

    pub fn die(&mut self, _killer_id: Option<u32>) {
        self.is_dead = true;
        self.hp = 0.0;
        self.death_timer = 3.0;
    }

    pub fn draw_death_screen(&self) {
        use crate::ui::fonts::{measure_game_text, text_params};

        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 230),
        );

        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        let death_text = "YOU DIED";
        let text_w = measure_game_text(death_text, 48);
        draw_text_ex(
            death_text,
            cx - text_w / 2.0,
            cy - 80.0,
            text_params(48, RED),
        );

        // show who killed us and with what weapon
        if let Some(killer) = &self.death_killer_name {
            let killed_by = if let Some(weapon) = &self.death_weapon_name {
                format!("Killed by {} with {}", killer, weapon)
            } else {
                format!("Killed by {}", killer)
            };
            let kw = measure_game_text(&killed_by, 20);
            draw_text_ex(
                &killed_by,
                cx - kw / 2.0,
                cy - 40.0,
                text_params(20, Color::from_rgba(255, 160, 160, 255)),
            );
        }

        if self.current_world == WorldId::Lobby {
            if self.death_timer <= 0.0 {
                let respawn_text = "Press SPACE to Respawn";
                let tw = measure_game_text(respawn_text, 24);
                draw_text_ex(
                    respawn_text,
                    cx - tw / 2.0,
                    cy + 20.0,
                    text_params(24, WHITE),
                );
            } else {
                let timer_text = format!("Respawning in {:.1}s", self.death_timer);
                let tw = measure_game_text(&timer_text, 24);
                draw_text_ex(&timer_text, cx - tw / 2.0, cy + 20.0, text_params(24, GRAY));
            }
        } else if self.current_world == WorldId::Maze {
            if self.death_timer <= 0.0 {
                let respawn_text = "Waiting to Respawn...";
                let tw = measure_game_text(respawn_text, 24);
                draw_text_ex(
                    respawn_text,
                    cx - tw / 2.0,
                    cy + 20.0,
                    text_params(24, GRAY),
                );

                let hint_text = "(Game mode will respawn you)";
                let hw = measure_game_text(hint_text, 16);
                draw_text_ex(
                    hint_text,
                    cx - hw / 2.0,
                    cy + 50.0,
                    text_params(16, DARKGRAY),
                );
            } else {
                let timer_text = format!("{:.1}s", self.death_timer);
                let tw = measure_game_text(&timer_text, 48);
                draw_text_ex(
                    &timer_text,
                    cx - tw / 2.0,
                    cy + 20.0,
                    text_params(48, Color::from_rgba(255, 100, 100, 255)),
                );
            }
        }
    }

    pub fn request_respawn(&self, socket: &std::net::UdpSocket, server_addr: &str) {
        use crate::protocols::MSG_RESPAWN_REQUEST;
        let msg = vec![MSG_RESPAWN_REQUEST];
        let _ = socket.send_to(&msg, server_addr);
    }

    pub fn respawn(&mut self, spawn_pos: Vec3) {
        self.is_dead = false;
        self.hp = self.max_hp;
        self.shield = 0.0;
        self.death_timer = 0.0;
        self.position = spawn_pos;
        self.movement_state.velocity = Vec3::ZERO;
        self.death_killer_name = None;
        self.death_weapon_name = None;
    }

    /// Full-screen victory overlay displayed when the match ends.
    /// Uses the same dark navy + orange accent design as the login/settings screens.
    pub fn draw_victory_screen(&self, winner_name: &str, local_player_id: u32) {
        use crate::ui::fonts::{measure_game_text, text_params};

        let sw = screen_width();
        let sh = screen_height();

        // --- Design colors (matching login/settings theme) ---
        let overlay_bg = Color::new(0.06, 0.06, 0.12, 0.92);
        let panel_bg = Color::new(0.09, 0.11, 0.19, 0.95);
        let panel_border = Color::new(0.20, 0.24, 0.40, 0.7);
        let accent = Color::new(1.0, 0.42, 0.21, 1.0);
        let text_bright = Color::new(0.92, 0.93, 0.96, 1.0);
        let text_dim = Color::new(0.50, 0.52, 0.62, 1.0);
        let text_label = Color::new(0.72, 0.74, 0.84, 1.0);
        let row_bg = Color::new(0.07, 0.08, 0.15, 1.0);
        let row_highlight = Color::new(0.12, 0.14, 0.24, 1.0);
        let gold = Color::new(1.0, 0.85, 0.30, 1.0);

        // Full screen overlay
        draw_rectangle(0.0, 0.0, sw, sh, overlay_bg);

        // Panel dimensions
        let panel_w = 500.0_f32.min(sw * 0.85);
        let scoreboard_rows = self.connected_players.len().min(12);
        let panel_h = 260.0 + (scoreboard_rows as f32 * 30.0);
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = (sh - panel_h) / 2.0;

        // Panel background + border
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, panel_bg);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.5, panel_border);

        // Top accent line
        draw_rectangle(panel_x, panel_y, panel_w, 3.0, accent);

        let mut cy = panel_y + 36.0;

        // "MATCH OVER" header
        let header = "MATCH OVER";
        let hw = measure_game_text(header, 16);
        draw_text_ex(header, (sw - hw) / 2.0, cy, text_params(16, text_dim));
        cy += 36.0;

        // Winner name (large, gold/orange)
        let winner_text = format!("{}", winner_name);
        let ww = measure_game_text(&winner_text, 42);
        draw_text_ex(&winner_text, (sw - ww) / 2.0, cy, text_params(42, gold));
        cy += 14.0;

        // "WINS THE MATCH" subtitle
        let subtitle = "WINS THE MATCH";
        let stw = measure_game_text(subtitle, 16);
        draw_text_ex(subtitle, (sw - stw) / 2.0, cy, text_params(16, text_label));
        cy += 32.0;

        // Separator line
        draw_rectangle(panel_x + 20.0, cy, panel_w - 40.0, 1.0, panel_border);
        cy += 16.0;

        // Scoreboard header
        let col_rank_x = panel_x + 24.0;
        let col_name_x = panel_x + 60.0;
        let col_kills_x = panel_x + panel_w - 80.0;
        draw_text_ex("#", col_rank_x, cy, text_params(13, text_dim));
        draw_text_ex("PLAYER", col_name_x, cy, text_params(13, text_dim));
        draw_text_ex("KILLS", col_kills_x, cy, text_params(13, text_dim));
        cy += 22.0;

        // Sort players by kills (descending)
        let mut sorted_players = self.connected_players.clone();
        // connected_players: (id, name, kills, deaths, gun_level, team, weapon)
        sorted_players.sort_by(|a, b| b.2.cmp(&a.2));

        for (i, (pid, name, kills, _deaths, _gun_level, _team, _weapon)) in sorted_players.iter().take(12).enumerate() {
            let is_local = *pid == local_player_id;
            let is_winner = name == winner_name;
            let row_y = cy - 13.0;

            // Row background
            let bg = if is_local { row_highlight } else { row_bg };
            draw_rectangle(panel_x + 12.0, row_y, panel_w - 24.0, 26.0, bg);

            // Rank
            let rank = format!("{}", i + 1);
            let rank_color = if i == 0 { gold } else { text_dim };
            draw_text_ex(&rank, col_rank_x, cy, text_params(14, rank_color));

            // Player name
            let name_color = if is_winner {
                accent
            } else if is_local {
                text_bright
            } else {
                text_label
            };
            draw_text_ex(name, col_name_x, cy, text_params(14, name_color));

            // Kills
            let kills_text = format!("{}", kills);
            let kw = measure_game_text(&kills_text, 14);
            draw_text_ex(
                &kills_text,
                col_kills_x + (40.0 - kw) / 2.0,
                cy,
                text_params(14, text_bright),
            );

            cy += 28.0;
        }

        cy += 12.0;

        // Countdown
        let secs = self.match_end_countdown_timer.ceil() as i32;
        let countdown_text = format!("Returning to lobby in {}s", secs);
        let ctw = measure_game_text(&countdown_text, 14);
        draw_text_ex(
            &countdown_text,
            (sw - ctw) / 2.0,
            cy,
            text_params(14, text_dim),
        );

        // Bottom accent line
        draw_rectangle(panel_x, panel_y + panel_h - 3.0, panel_w, 3.0, accent);
    }
}
