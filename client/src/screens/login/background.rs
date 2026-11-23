use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::LoginScreen;
use super::{ACCENT, MENU_BG, PANEL_BG, PANEL_BORDER, TEXT_DIM, TEXT_LABEL};

/// Sides of macroquad's draw_circle polygon.
const CIRCLE_SIDES: usize = 20;

/// Reused geometry for the animated background. It holds the same triangles
/// draw_circle / draw_line would build, without their per-call trig, Vec
/// allocations and (for the scanlines) per-frame rebuild.
pub(super) struct BackgroundMesh {
    /// Unit-circle rim points, exactly as draw_poly computes them.
    rim: [(f32, f32); CIRCLE_SIDES + 1],
    particles: Mesh,
    scanlines: Mesh,
    /// Screen size the scanlines were built for.
    scan_size: (f32, f32),
}

impl BackgroundMesh {
    pub(super) fn new() -> Self {
        let rot = 0.0_f32.to_radians();
        let sides = CIRCLE_SIDES as f32;
        let rim = std::array::from_fn(|i| {
            let a = i as f32 / sides * std::f32::consts::PI * 2. + rot;
            (a.cos(), a.sin())
        });
        let empty = || Mesh { vertices: Vec::new(), indices: Vec::new(), texture: None };
        Self { rim, particles: empty(), scanlines: empty(), scan_size: (-1.0, -1.0) }
    }

    /// Rebuilds the scanlines (one draw_line quad every 4 px) for a new size.
    fn build_scanlines(&mut self, sw: f32, sh: f32) {
        let color = Color::new(0.0, 0.0, 0.0, 0.06);
        let mesh = &mut self.scanlines;
        mesh.vertices.clear();
        mesh.indices.clear();
        // same maths as draw_line(0.0, y, sw, y, 1.0, color)
        let (nx, ny) = (-0.0_f32, sw);
        let tlen = (nx * nx + ny * ny).sqrt() / (1.0 * 0.5);
        if tlen >= f32::EPSILON {
            let (tx, ty) = (nx / tlen, ny / tlen);
            let mut y = 0.0;
            while y < sh {
                let base = mesh.vertices.len() as u16;
                mesh.vertices.extend_from_slice(&[
                    Vertex::new(0.0 + tx, y + ty, 0., 0., 0., color),
                    Vertex::new(0.0 - tx, y - ty, 0., 0., 0., color),
                    Vertex::new(sw + tx, y + ty, 0., 0., 0., color),
                    Vertex::new(sw - tx, y - ty, 0., 0., 0., color),
                ]);
                mesh.indices.extend([0, 1, 2, 2, 1, 3].map(|i| base + i));
                y += 4.0;
            }
        }
        self.scan_size = (sw, sh);
    }
}

impl LoginScreen {
    pub fn update_background(&mut self) {
        let dt = get_frame_time();
        self.anim_time += dt;
        let sw = screen_width();
        let sh = screen_height();
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.x < 0.0 {
                p.x += sw;
            }
            if p.x > sw {
                p.x -= sw;
            }
            if p.y < 0.0 {
                p.y += sh;
            }
            if p.y > sh {
                p.y -= sh;
            }
        }
    }

    pub fn draw_background(&self) {
        clear_background(MENU_BG);

        let sw = screen_width();
        let sh = screen_height();
        let mut bg = self.background_mesh.borrow_mut();
        let bg = &mut *bg;

        // the particle dots: draw_circle's 20-gon, all in one mesh
        let mesh = &mut bg.particles;
        mesh.vertices.clear();
        mesh.indices.clear();
        for p in &self.particles {
            let color = Color::new(0.50, 0.55, 0.80, p.alpha);
            let base = mesh.vertices.len() as u16;
            mesh.vertices.push(Vertex::new(p.x, p.y, 0., 0., 0., color));
            for (i, &(rx, ry)) in bg.rim.iter().enumerate() {
                mesh.vertices.push(Vertex::new(p.x + p.size * rx, p.y + p.size * ry, 0., rx, ry, color));
                if i != CIRCLE_SIDES {
                    let i = i as u16;
                    mesh.indices.extend_from_slice(&[base, base + i + 1, base + i + 2]);
                }
            }
        }
        draw_mesh(mesh);

        // u16 indices cover up to 16k scanlines (a 64k px tall window)
        if sh / 4.0 < 16_000.0 {
            if bg.scan_size != (sw, sh) {
                bg.build_scanlines(sw, sh);
            }
            draw_mesh(&bg.scanlines);
        } else {
            let mut y = 0.0;
            while y < sh {
                draw_line(0.0, y, sw, y, 1.0, Color::new(0.0, 0.0, 0.0, 0.06));
                y += 4.0;
            }
        }
    }

    pub fn draw_title(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let title = "GRIDLOCK";
        let ts: u16 = 52;
        let tw = measure_game_text(title, ts);
        let tx = (sw - tw) / 2.0;
        let ty = sh * 0.15;

        draw_text_ex(
            title,
            tx + 2.0,
            ty + 2.0,
            text_params(ts, Color::new(0.0, 0.0, 0.0, 0.5)),
        );
        let g = ((self.anim_time * 0.8).sin() * 0.12 + 0.88).clamp(0.76, 1.0);
        draw_text_ex(title, tx, ty, text_params(ts, Color::new(g, g, g, 1.0)));

        let lw = tw * 0.55;
        let lx = (sw - lw) / 2.0;
        let ly = ty + 14.0;
        draw_rectangle(lx, ly, lw, 2.0, ACCENT);
    }

    /// A quiet note in the corner while models download in the background.
    pub fn draw_preload(&self, done: u32, total: u32) {
        let text = format!("Downloading game assets {done} / {total}");
        let w = measure_game_text(&text, 13);
        draw_text_ex(&text, screen_width() - w - 16.0, screen_height() - 16.0, text_params(13, TEXT_DIM));
    }

    /// `download` is how far the asset being waited on has downloaded, as
    /// (received, total) bytes, when that is known.
    pub fn draw_loading(
        &self,
        total: usize,
        loaded: usize,
        current: &str,
        download: Option<(f64, f64)>,
    ) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        let pw = 480.0_f32.min(sw * 0.85);
        let ph = 150.0;
        let px = (sw - pw) / 2.0;
        let py = sh * 0.38;
        draw_rectangle(px, py, pw, ph, PANEL_BG);
        draw_rectangle_lines(px, py, pw, ph, 1.0, PANEL_BORDER);

        let label = if total == 0 {
            "Joining server..."
        } else if download.is_some_and(|(received, size)| received < size) {
            "Downloading assets..."
        } else {
            "Loading assets..."
        };
        let lw = measure_game_text(label, 20);
        draw_text_ex(label, (sw - lw) / 2.0, py + 34.0, text_params(20, WHITE));

        let pad = 30.0;
        let bx = px + pad;
        let bw = pw - pad * 2.0;
        let bh = 22.0;
        let by = py + 54.0;
        draw_rectangle(bx, by, bw, bh, super::INPUT_BG);
        draw_rectangle_lines(bx, by, bw, bh, 1.0, PANEL_BORDER);
        if total > 0 {
            // The asset in progress counts for the part already downloaded.
            let partial = download.map_or(0.0, |(received, size)| (received / size) as f32);
            let frac = ((loaded as f32 + partial.clamp(0.0, 1.0)) / total as f32).min(1.0);
            let fw = (bw - 4.0) * frac;
            draw_rectangle(bx + 2.0, by + 2.0, fw, bh - 4.0, ACCENT);
        }

        if total > 0 {
            let count = format!("{} / {}", loaded, total);
            let cw = measure_game_text(&count, 15);
            draw_text_ex(
                &count,
                px + pw - pad - cw,
                by + bh + 18.0,
                text_params(15, TEXT_LABEL),
            );
        }

        if !current.is_empty() {
            let line = match download {
                Some((received, size)) if received < size => format!(
                    "{}  {:.1} / {:.1} MB",
                    current,
                    received / 1_048_576.0,
                    size / 1_048_576.0
                ),
                _ => current.to_string(),
            };
            draw_text_ex(&line, bx, by + bh + 18.0, text_params(13, TEXT_DIM));
        }
    }
}
