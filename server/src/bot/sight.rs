//! Bot line of sight through the maze's wall grid.

use crate::maze::MazeData;
use crate::player::{Player, WorldId};
use crate::server::game::MAZE_CELL_SIZE;

/// Two grid-line crossings closer than this (in cells) count as one: the ray
/// goes through the corner. Well above float error, far below anything visible.
const CORNER_EPS: f32 = 1e-4;

pub fn has_line_of_sight(bot: &Player, target: &Player, maze: Option<&MazeData>) -> bool {
    // Lobby has no server-side wall grid representation; only maze mode is occluded here.
    if bot.world != WorldId::Maze {
        return true;
    }
    let Some(maze) = maze else {
        return true;
    };

    let (x0, z0) = (bot.position.0, bot.position.2);
    let (x1, z1) = (target.position.0, target.position.2);
    let (dx, dz) = (x1 - x0, z1 - z0);
    if (dx * dx + dz * dz).sqrt() <= 1e-5 {
        return true;
    }
    segment_clear(&maze.cells, maze.width, maze.height, (x0, z0), (x1, z1))
}

/// Whether the segment `from`..`to` (world x/z) crosses open cells only. An
/// exact walk over every grid cell it passes through (Amanatides-Woo), so it
/// cannot skip the corner of a wall the way point samples along the ray did.
/// Through a cell corner exactly, both cells beside the corner count, so it
/// never slips between two walls that touch diagonally.
fn segment_clear(cells: &[u8], width: u8, height: u8, from: (f32, f32), to: (f32, f32)) -> bool {
    let open = |cx: i32, cz: i32| {
        cx >= 0
            && cz >= 0
            && cx < width as i32
            && cz < height as i32
            && cells.get(cz as usize * width as usize + cx as usize).copied().unwrap_or(0) != 0
    };
    // grid units: one cell is 1.0
    let (gx, gz) = (from.0 / MAZE_CELL_SIZE, from.1 / MAZE_CELL_SIZE);
    let (gdx, gdz) = ((to.0 - from.0) / MAZE_CELL_SIZE, (to.1 - from.1) / MAZE_CELL_SIZE);
    let len = (gdx * gdx + gdz * gdz).sqrt();
    let (mut cx, mut cz) = (gx.floor() as i32, gz.floor() as i32);
    if !open(cx, cz) {
        return false;
    }

    // For each axis: direction, the segment parameter t (0..1) at which it
    // next crosses a grid line, and how much t one whole cell takes.
    let axis = |g: f32, d: f32, c: i32| -> (i32, f32, f32) {
        if d > 0.0 {
            (1, (c as f32 + 1.0 - g) / d, 1.0 / d)
        } else if d < 0.0 {
            (-1, (c as f32 - g) / d, -1.0 / d)
        } else {
            (0, f32::INFINITY, f32::INFINITY)
        }
    };
    let (step_x, mut t_x, dt_x) = axis(gx, gdx, cx);
    let (step_z, mut t_z, dt_z) = axis(gz, gdz, cz);

    loop {
        if t_x.min(t_z) >= 1.0 {
            return true;
        }
        if (t_x - t_z).abs() * len <= CORNER_EPS {
            // through a corner: both cells beside it, then the diagonal one
            if !open(cx + step_x, cz) || !open(cx, cz + step_z) {
                return false;
            }
            cx += step_x;
            cz += step_z;
            t_x += dt_x;
            t_z += dt_z;
        } else if t_x < t_z {
            cx += step_x;
            t_x += dt_x;
        } else {
            cz += step_z;
            t_z += dt_z;
        }
        if !open(cx, cz) {
            return false;
        }
    }
}

/// The previous line of sight: point samples every 0.2 cells along the ray.
/// Kept to check the exact walk against.
#[cfg(test)]
fn sampled_clear(cells: &[u8], width: u8, height: u8, from: (f32, f32), to: (f32, f32)) -> bool {
    let (x0, z0) = from;
    let dx = to.0 - x0;
    let dz = to.1 - z0;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist <= 1e-5 {
        return true;
    }

    // Sample several points per cell along the segment and reject as soon as it enters a wall cell.
    let steps = ((dist / (MAZE_CELL_SIZE * 0.2)).ceil() as i32).max(1);
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let x = x0 + dx * t;
        let z = z0 + dz * t;
        let cx = (x / MAZE_CELL_SIZE).floor() as i32;
        let cy = (z / MAZE_CELL_SIZE).floor() as i32;

        if cx < 0 || cy < 0 || cx >= width as i32 || cy >= height as i32 {
            return false;
        }

        let idx = cy as usize * width as usize + cx as usize;
        if cells.get(idx).copied().unwrap_or(0) == 0 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    const S: f32 = MAZE_CELL_SIZE;

    /// The exact walk blocks wherever the old sampler did, on real mazes.
    #[test]
    fn exact_walk_never_sees_through_a_wall_the_sampler_blocked() {
        let mut rng = rand::thread_rng();
        let (mut agree, mut corner_clips) = (0, 0);
        for _ in 0..20 {
            let maze = MazeData::new(25, 25);
            let open: Vec<(usize, usize)> = (0..maze.height as usize)
                .flat_map(|y| (0..maze.width as usize).map(move |x| (x, y)))
                .filter(|&(x, y)| maze.cells[y * maze.width as usize + x] == 1)
                .collect();
            let mut point = || {
                let (x, y) = open[rng.gen_range(0..open.len())];
                ((x as f32 + rng.gen::<f32>()) * S, (y as f32 + rng.gen::<f32>()) * S)
            };
            for _ in 0..2000 {
                let (a, b) = (point(), point());
                let exact = segment_clear(&maze.cells, maze.width, maze.height, a, b);
                let sampled = sampled_clear(&maze.cells, maze.width, maze.height, a, b);
                assert!(sampled || !exact, "exact walk saw through a wall: {:?} -> {:?}", a, b);
                if exact == sampled { agree += 1 } else { corner_clips += 1 }
            }
        }
        // the only difference: rays the sampler let clip a wall's corner
        assert!(corner_clips * 20 < agree, "{} of {} differ", corner_clips, agree + corner_clips);
    }

    /// A ray grazing a wall's corner between two samples: the sampler missed it.
    #[test]
    fn exact_walk_catches_a_clipped_corner() {
        // 3x3 open grid with a wall in the middle
        let mut cells = vec![1u8; 9];
        cells[4] = 0;
        // from the left column to the top row, cutting just inside the
        // wall's top-left corner at (1, 1)
        let from = (0.5 * S, 1.5 * S);
        let clipping = (1.5 * S, 0.53 * S);
        assert!(sampled_clear(&cells, 3, 3, from, clipping), "the sampler missed it");
        assert!(!segment_clear(&cells, 3, 3, from, clipping));
        // passing just outside the corner: clear
        let passing = (1.5 * S, 0.47 * S);
        assert!(segment_clear(&cells, 3, 3, from, passing));
    }

    #[test]
    fn exact_walk_blocks_between_diagonal_walls() {
        // walls at (1, 0) and (0, 1) touch at the corner (1, 1)
        let cells = vec![1, 0, 1, 0, 1, 1, 1, 1, 1];
        assert!(!segment_clear(&cells, 3, 3, (0.5 * S, 0.5 * S), (1.5 * S, 1.5 * S)));
        assert!(segment_clear(&cells, 3, 3, (1.5 * S, 1.5 * S), (2.5 * S, 2.5 * S)));
    }

    #[test]
    fn leaving_the_grid_blocks() {
        let cells = vec![1u8; 4];
        assert!(!segment_clear(&cells, 2, 2, (0.5 * S, 0.5 * S), (2.5 * S, 0.5 * S)));
        assert!(segment_clear(&cells, 2, 2, (0.5 * S, 0.5 * S), (1.5 * S, 1.5 * S)));
    }
}
