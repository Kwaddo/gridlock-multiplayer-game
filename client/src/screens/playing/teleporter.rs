use crate::net::networking::MAZE_CELL_SIZE;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    /*
        Most unnecessary function ever but I wanted to have some fun while separating
        lobby and maze, maybe we can still use this in the future
        (at least x amount of players to be able to teleport and start game)

    */
    pub(super) fn maze_teleporter_pos(&self, maze: Option<&Maze>) -> Vec3 {
        if let Some(m) = maze {
            m.teleporter_world()
        } else {
            vec3(1.5 * MAZE_CELL_SIZE, 0.0, 1.5 * MAZE_CELL_SIZE)
        }
    }

    pub(super) fn draw_teleporter(&self, maze: Option<&Maze>) {
        let pos = match self.current_world {
            WorldId::Lobby => vec3(0.0, 0.0, -10.0),
            WorldId::Maze => self.maze_teleporter_pos(maze),
        };

        let t = get_time() as f32;
        let alpha = (t * 2.0).sin() * 0.3 + 0.7;
        let color = Color::new(0.2, 0.8, 1.0, alpha);

        draw_cube(pos + vec3(0.0, 1.5, 0.0), vec3(2.0, 3.0, 2.0), None, color);
        draw_cube_wires(pos + vec3(0.0, 1.5, 0.0), vec3(2.0, 3.0, 2.0), WHITE);
    }

    pub(super) fn draw_teleporter_prompt(&self, maze: Option<&Maze>) {
        let (pos, label) = match self.current_world {
            WorldId::Lobby => (vec3(0.0, 0.0, -10.0), "ENTER MAZE"),
            WorldId::Maze => (self.maze_teleporter_pos(maze), "RETURN TO LOBBY"),
        };

        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();
        if dist < 2.5 {
            let text = format!("[E] {}", label);
            let measure = measure_text(&text, None, 24, 1.0);
            draw_text(
                &text,
                screen_width() / 2.0 - measure.width / 2.0,
                screen_height() / 2.0 + 60.0,
                24.0,
                WHITE,
            );
        }
    }

    pub(super) fn check_teleporter(&mut self, maze: Option<&Maze>) {
        let tp_pos = match self.current_world {
            WorldId::Lobby => vec3(0.0, 0.0, -10.0),
            WorldId::Maze => self.maze_teleporter_pos(maze),
        };

        let dist = vec3(self.position.x - tp_pos.x, 0.0, self.position.z - tp_pos.z).length();
        if dist < 2.0 && is_key_pressed(KeyCode::E) {
            self.pending_world_change = Some(match self.current_world {
                WorldId::Lobby => WorldId::Maze,
                WorldId::Maze => WorldId::Lobby,
            });
        }
    }
}
