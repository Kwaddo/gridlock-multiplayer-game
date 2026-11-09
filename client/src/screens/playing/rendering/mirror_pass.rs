use macroquad::prelude::*;

use crate::world::mirror::{ pick_live_mirrors, MirrorCamera };

use super::super::PlayingState;

impl PlayingState {
    /// Renders the reflection of each live mirror into its own texture.
    /// Returns the indexes (into `self.mirrors`) of the mirrors that got one;
    /// the texture for `live[i]` is `self.mirror_targets[i]`.
    pub(crate) fn render_mirror_pass(
        &mut self,
        camera: &Camera3D,
        maze: Option<&crate::types::maze::Maze>,
    ) -> Vec<usize> {
        let live = pick_live_mirrors(&self.mirrors, camera.position, camera.matrix());
        if live.is_empty() {
            return live;
        }

        // screen-sized textures with a depth buffer. sample_count must be 0:
        // the default (1) gives a blank mirror in the browser.
        // On resize, drop the old textures now and create new ones next frame,
        // so the engine never frees a texture twice in one frame.
        let (w, h) = (screen_width() as u32, screen_height() as u32);
        let wrong_size = self.mirror_targets
            .iter()
            .any(|rt| (rt.texture.width() as u32) != w || (rt.texture.height() as u32) != h);
        if wrong_size {
            self.mirror_targets.clear();
            return Vec::new();
        }
        if self.mirror_targets.len() < live.len() {
            while self.mirror_targets.len() < live.len() {
                self.mirror_targets.push(
                    render_target_ex(w, h, RenderTargetParams { sample_count: 0, depth: true })
                );
            }
            return Vec::new();
        }

        for (slot, &index) in live.iter().enumerate() {
            let mirror = self.mirrors[index];
            set_camera(
                &(MirrorCamera {
                    matrix: mirror.camera_matrix(camera),
                    render_target: Some(self.mirror_targets[slot].clone()),
                })
            );
            clear_background(Color::from_rgba(16, 20, 28, 255));
            // the whole world, seen from the mirrored eye
            self.draw_world(maze, mirror.reflect_point(camera.position));
            // you: only in the mirror, never in the main view
            self.draw_self_avatar();
            // other mirrors seen inside this reflection become plain tinted panels
            for (other, m) in self.mirrors.iter().enumerate() {
                if other != index {
                    m.draw_body(None);
                }
            }
        }
        set_default_camera();
        live
    }

    /// Draws every mirror in the world: live ones show their reflection,
    /// the rest a plain tinted panel. Call with the main camera set.
    pub(crate) fn draw_mirrors(&self, live: &[usize]) {
        for (index, mirror) in self.mirrors.iter().enumerate() {
            let reflection = live
                .iter()
                .position(|&l| l == index)
                .and_then(|slot| self.mirror_targets.get(slot))
                .map(|rt| &rt.texture);
            mirror.draw_body(reflection);
        }
    }
}
