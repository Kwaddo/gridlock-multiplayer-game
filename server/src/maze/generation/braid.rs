use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

use super::constants::DEAD_END_OPEN_CHANCE;

/// Open a share of the maze's dead ends by knocking through the wall to a
/// passage beyond, turning dead ends into loops. Only walls between two open
/// cells in a straight line are knocked out, so every corridor stays one cell
/// wide and the outer wall is never touched.
pub(super) fn braid_dead_ends(grid: &mut [u8], w: usize, h: usize, rng: &mut StdRng) {
    let open = |g: &[u8], x: usize, y: usize| g[y * w + x] == 1;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if !open(grid, x, y) {
                continue;
            }
            let exits = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter(|(dx, dy)| open(grid, (x as i32 + dx) as usize, (y as i32 + dy) as usize))
                .count();
            if exits != 1 || !rng.gen_bool(DEAD_END_OPEN_CHANCE) {
                continue;
            }
            let mut walls: Vec<(usize, usize)> = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter_map(|&(dx, dy)| {
                    let (wx, wy) = (x as i32 + dx, y as i32 + dy);
                    let (bx, by) = (x as i32 + 2 * dx, y as i32 + 2 * dy);
                    let inside = |a: i32, b: i32| a > 0 && b > 0 && a < w as i32 - 1 && b < h as i32 - 1;
                    if !inside(wx, wy) || !inside(bx, by) {
                        return None;
                    }
                    let (wx, wy, bx, by) = (wx as usize, wy as usize, bx as usize, by as usize);
                    if open(grid, wx, wy) || !open(grid, bx, by) {
                        return None;
                    }
                    // the wall's own side cells must be walls, or opening it
                    // would make a 2-wide gap
                    let side_open = if dx != 0 {
                        open(grid, wx, wy - 1) || open(grid, wx, wy + 1)
                    } else {
                        open(grid, wx - 1, wy) || open(grid, wx + 1, wy)
                    };
                    if side_open {
                        return None;
                    }
                    Some((wx, wy))
                })
                .collect();
            walls.shuffle(rng);
            if let Some(&(wx, wy)) = walls.first() {
                grid[wy * w + wx] = 1;
            }
        }
    }
}
