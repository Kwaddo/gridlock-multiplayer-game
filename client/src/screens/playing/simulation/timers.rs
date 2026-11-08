use crate::types::maze::Maze;
use crate::types::world::WorldId;

use super::PlayingState;

impl PlayingState {
    /// Count down HUD, match and effect timers and expire what they gate.
    pub(super) fn tick_timers(&mut self, delta: f32, maze: Option<&Maze>) {
        if self.hit_marker_timer > 0.0 {
            self.hit_marker_timer = (self.hit_marker_timer - delta).max(0.0);
        }

        // Damage flash timer (short red vignette + optional attacker name)
        if self.damage_flash_timer > 0.0 {
            self.damage_flash_timer = (self.damage_flash_timer - delta).max(0.0);
            if self.damage_flash_timer <= 0.0 {
                self.last_damage_attacker = None;
            }
        }

        // Respawn feedback timer
        if self.spawn_feedback_timer > 0.0 {
            self.spawn_feedback_timer = (self.spawn_feedback_timer - delta).max(0.0);
            if self.spawn_feedback_timer <= 0.0 {
                self.spawn_feedback_message = None;
            }
        }

        // Lobby console feedback timer
        if self.lobby_console_message_timer > 0.0 {
            self.lobby_console_message_timer = (self.lobby_console_message_timer - delta).max(0.0);
            if self.lobby_console_message_timer <= 0.0 {
                self.lobby_console_message = None;
            }
        }

        // Match-start countdown (server-initiated, display only)
        // The actual freeze timer is set when the MSG_WORLD_CHANGE arrives from the
        // server (with an authoritative freeze duration) so that all clients are
        // synchronized regardless of connection speed.
        if self.match_start_countdown_timer > 0.0 {
            self.match_start_countdown_timer = (self.match_start_countdown_timer - delta).max(0.0);
            if self.match_start_countdown_timer <= 0.0 {
                self.announcements.push(("GO!".to_string(), 1.5));
            }
        }

        // Match-end countdown (server-initiated)
        if self.match_end_countdown_timer > 0.0 {
            self.match_end_countdown_timer = (self.match_end_countdown_timer - delta).max(0.0);
            if self.match_end_countdown_timer <= 0.0 {
                // clear winner toast once countdown completes
                self.match_end_winner = None;
            }
        }

        // Post-teleport movement freeze timer (client-side display + input gating)
        if self.post_teleport_freeze_timer > 0.0 {
            self.post_teleport_freeze_timer = (self.post_teleport_freeze_timer - delta).max(0.0);
            if self.post_teleport_freeze_timer <= 0.0 {
                self.announcements
                    .push(("Movement unlocked".to_string(), 2.0));
            }
        }

        // Match timer (client-side) — started when client enters Maze
        if self.current_world == WorldId::Maze && self.match_time_left > 0.0 {
            self.match_time_left = (self.match_time_left - delta).max(0.0);
        } else if self.current_world != WorldId::Maze {
            // ensure timer is cleared outside of match
            self.match_time_left = 0.0;
        }

        // Announcements timers
        for ann in &mut self.announcements {
            ann.1 = (ann.1 - delta).max(0.0);
        }
        self.announcements.retain(|ann| ann.1 > 0.0);
        self.chat.tick(delta);

        self.damage_numbers.retain_mut(|dn| {
            dn.lifetime -= delta;
            dn.screen_y -= 60.0 * delta;
            dn.lifetime > 0.0
        });

        self.tracers.retain_mut(|t| {
            if t.progress < 1.0 {
                let dist = (t.end - t.start).length();
                t.progress = (t.progress + (t.speed * delta) / dist).min(1.0);
            }
            t.lifetime -= delta;
            t.lifetime > 0.0
        });

        self.world_muzzle_flashes.retain_mut(|f| {
            f.lifetime -= delta;
            f.lifetime > 0.0
        });

        // bullet hole lifetime decay + remove holes on broken walls
        self.bullet_holes.retain_mut(|h| {
            h.lifetime -= delta;
            if h.lifetime <= 0.0 {
                return false;
            }
            // if this hole is on a breakable wall, check if the wall still exists
            if let Some((cx, cy)) = h.wall_cell {
                if let Some(m) = maze {
                    if !m.is_wall(cx, cy) {
                        return false;
                    }
                }
            }
            true
        });
    }
}
