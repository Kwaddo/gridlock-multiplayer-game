use crate::types::world::WorldId;
use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
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
            Color::from_rgba(0, 0, 0, 180),
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

    pub fn respawn(&mut self, spawn_pos: Vec3) {
        self.is_dead = false;
        self.hp = self.max_hp;
        self.shield = 0.0;
        self.death_timer = 0.0;
        self.position = spawn_pos;
        self.movement_state.velocity = Vec3::ZERO;
    }
}
