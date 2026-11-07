use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::maze::decoration::generate_props;
use crate::maze::types::{MazeLight, Prop, Room};

pub fn generate_maze(
    width: u8,
    height: u8,
    seed: u64,
) -> (Vec<u8>, Vec<Room>, Vec<Prop>, Vec<MazeLight>, (u8, u8)) {
    let w = width as usize;
    let h = height as usize;
    let mut grid = vec![0u8; w * h];
    let mut rng = StdRng::seed_from_u64(seed);

    if w < 3 || h < 3 {
        return (grid, Vec::new(), Vec::new(), Vec::new(), (1, 1));
    }

    let w_carve = if w % 2 == 0 { w - 1 } else { w };
    let h_carve = if h % 2 == 0 { h - 1 } else { h };

    let mut stack = Vec::new();
    let start = (1usize, 1usize);
    grid[start.1 * w + start.0] = 1;
    stack.push(start);
    while let Some(&(cx, cy)) = stack.last() {
        let mut neighbors = Vec::new();
        for (dx, dy) in [(2i32, 0i32), (-2, 0), (0, 2), (0, -2)] {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx > 0 && ny > 0 && (nx as usize) < (w_carve - 1) && (ny as usize) < (h_carve - 1) {
                let idx = ny as usize * w + nx as usize;
                if grid[idx] == 0 {
                    neighbors.push((nx as usize, ny as usize));
                }
            }
        }

        if let Some(&(nx, ny)) = neighbors.choose(&mut rng) {
            let wall_x = (cx + nx) / 2;
            let wall_y = (cy + ny) / 2;
            grid[wall_y * w + wall_x] = 1;
            grid[ny * w + nx] = 1;
            stack.push((nx, ny));
        } else {
            stack.pop();
        }
    }

    let rooms = carve_rooms(&mut grid, w, h, &mut rng);
    connect_rooms(&mut grid, w, h, &rooms, &mut rng);
    braid_dead_ends(&mut grid, w, h, &mut rng);

    let teleporter = if let Some(room) = rooms.first() {
        let tx = (room.x as usize + 1).min((room.x + room.w - 1) as usize);
        let ty = (room.y as usize + 1).min((room.y + room.h - 1) as usize);
        (tx as u8, ty as u8)
    } else {
        (1u8, 1u8)
    };

    let props = generate_props(&rooms, &mut rng);
    let lights = generate_ceiling_lights(width, height, &grid, &rooms);

    (grid, rooms, props, lights, teleporter)
}

fn carve_rooms(grid: &mut [u8], w: usize, h: usize, rng: &mut StdRng) -> Vec<Room> {
    let mut rooms: Vec<Room> = Vec::new();
    let desired_rooms = if w >= 40 && h >= 40 {
        8
    } else if w >= 25 && h >= 25 {
        5
    } else {
        3
    };
    // Prefer slightly larger rooms so the 3x5 office layout (with 1-tile border) fits more often.
    let mut size_bands: Vec<(usize, usize)> = vec![(5usize, 9usize)];
    if w >= 40 && h >= 40 {
        size_bands.push((6usize, 11usize));
    }

    for (min_size, max_size) in size_bands {
        let mut attempts = 0;
        let max_attempts = desired_rooms * 12;

        while attempts < max_attempts && rooms.len() < desired_rooms {
            attempts += 1;

            let rw = rng.gen_range(min_size..=max_size);
            let rh = rng.gen_range(min_size..=max_size);
            if rw >= w - 2 || rh >= h - 2 {
                continue;
            }

            let rx = rng.gen_range(2..(w - rw - 1));
            let ry = rng.gen_range(2..(h - rh - 1));

            let mut overlaps = false;
            for r in &rooms {
                let r_x1 = r.x.saturating_sub(1) as isize;
                let r_y1 = r.y.saturating_sub(1) as isize;
                let r_x2 = (r.x as usize + r.w as usize + 1) as isize;
                let r_y2 = (r.y as usize + r.h as usize + 1) as isize;

                let x1 = rx as isize;
                let y1 = ry as isize;
                let x2 = (rx + rw) as isize;
                let y2 = (ry + rh) as isize;

                if x1 <= r_x2 && x2 >= r_x1 && y1 <= r_y2 && y2 >= r_y1 {
                    overlaps = true;
                    break;
                }
            }
            if overlaps {
                continue;
            }

            for y in ry..(ry + rh) {
                for x in rx..(rx + rw) {
                    grid[y * w + x] = 1;
                }
            }

            rooms.push(Room {
                x: rx as u8,
                y: ry as u8,
                w: rw as u8,
                h: rh as u8,
            });
        }
    }

    if rooms.len() < desired_rooms {
        let target = desired_rooms;
        let rw = 4usize;
        let rh = 4usize;
        let mut ry = 2usize;
        while rooms.len() < target && ry + rh + 1 < h {
            let mut rx = 2usize;
            while rooms.len() < target && rx + rw + 1 < w {
                let mut overlaps = false;
                for r in &rooms {
                    let r_x1 = r.x.saturating_sub(1) as isize;
                    let r_y1 = r.y.saturating_sub(1) as isize;
                    let r_x2 = (r.x as usize + r.w as usize + 1) as isize;
                    let r_y2 = (r.y as usize + r.h as usize + 1) as isize;

                    let x1 = rx as isize;
                    let y1 = ry as isize;
                    let x2 = (rx + rw) as isize;
                    let y2 = (ry + rh) as isize;

                    if x1 <= r_x2 && x2 >= r_x1 && y1 <= r_y2 && y2 >= r_y1 {
                        overlaps = true;
                        break;
                    }
                }

                if !overlaps {
                    for y in ry..(ry + rh) {
                        for x in rx..(rx + rw) {
                            grid[y * w + x] = 1;
                        }
                    }
                    rooms.push(Room {
                        x: rx as u8,
                        y: ry as u8,
                        w: rw as u8,
                        h: rh as u8,
                    });
                }

                rx += rw + 2;
            }
            ry += rh + 2;
        }
    }

    rooms
}

/// Longest tunnel dug out of a room to reach the maze; longer ones made long
/// straight lanes across the map.
const MAX_ROOM_TUNNEL: usize = 5;

/// Share of the maze's dead ends that get opened into a loop. A perfect maze
/// is all dead ends, which means long walks back out and players hunting for
/// each other; loops give flanks and shorter routes between any two spots.
const DEAD_END_OPEN_CHANCE: f64 = 0.65;

/// A tunnel candidate out of a room: start cell on the room's edge, the
/// direction it digs, and the wall cells it would open.
struct Tunnel {
    from: (usize, usize),
    dir: (i32, i32),
    cells: Vec<usize>,
    /// runs right beside an open cell for part of its length
    side_by_side: bool,
}

/// Link each room to the maze on up to two sides with the shortest tunnels
/// available, preferring ones that don't run beside an existing corridor.
fn connect_rooms(grid: &mut [u8], w: usize, h: usize, rooms: &[Room], rng: &mut StdRng) {
    for room in rooms {
        let rx = room.x as usize;
        let ry = room.y as usize;
        let rw = room.w as usize;
        let rh = room.h as usize;

        let mut edges = Vec::new();
        for x in (rx + 1)..(rx + rw - 1) {
            edges.push((x, ry, 0i32, -1i32));
            edges.push((x, ry + rh - 1, 0, 1));
        }
        for y in (ry + 1)..(ry + rh - 1) {
            edges.push((rx, y, -1i32, 0));
            edges.push((rx + rw - 1, y, 1, 0));
        }
        // shuffle so equal-length tunnels are picked at random
        edges.shuffle(rng);

        let mut tunnels: Vec<Tunnel> = edges
            .iter()
            .filter_map(|&(sx, sy, dx, dy)| plan_tunnel(grid, w, h, (sx, sy), (dx, dy)))
            .collect();
        // short, clean tunnels first; a tunnel of length 0 means this side
        // already opens onto the maze
        tunnels.sort_by_key(|t| (t.side_by_side, t.cells.len() > MAX_ROOM_TUNNEL, t.cells.len()));

        let mut used_dirs: Vec<(i32, i32)> = Vec::new();
        for t in &tunnels {
            if used_dirs.len() >= 2 {
                break;
            }
            // one link per side of the room, so the two links lead different ways
            if used_dirs.contains(&t.dir) {
                continue;
            }
            for &idx in &t.cells {
                grid[idx] = 1;
            }
            grid[t.from.1 * w + t.from.0] = 1;
            used_dirs.push(t.dir);
        }
    }
}

/// Walk from a room edge cell outward until an open cell; `None` if the walk
/// hits the outer wall first.
fn plan_tunnel(
    grid: &[u8],
    w: usize,
    h: usize,
    from: (usize, usize),
    dir: (i32, i32),
) -> Option<Tunnel> {
    let (dx, dy) = dir;
    let open = |x: i32, y: i32| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && grid[y as usize * w + x as usize] == 1
    };
    let mut cells = Vec::new();
    let mut side_by_side = false;
    let (mut cx, mut cy) = (from.0 as i32, from.1 as i32);
    loop {
        let nx = cx + dx;
        let ny = cy + dy;
        if nx <= 0 || ny <= 0 || nx >= (w as i32 - 1) || ny >= (h as i32 - 1) {
            return None;
        }
        if open(nx, ny) {
            return Some(Tunnel { from, dir, cells, side_by_side });
        }
        // the cells either side of this one, across the dig direction
        if open(nx + dy, ny + dx) || open(nx - dy, ny - dx) {
            side_by_side = true;
        }
        cells.push(ny as usize * w + nx as usize);
        cx = nx;
        cy = ny;
    }
}

/// Open a share of the maze's dead ends by knocking through the wall to a
/// passage beyond, turning dead ends into loops. Only walls between two open
/// cells in a straight line are knocked out, so every corridor stays one cell
/// wide and the outer wall is never touched.
fn braid_dead_ends(grid: &mut [u8], w: usize, h: usize, rng: &mut StdRng) {
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

fn generate_ceiling_lights(width: u8, height: u8, cells: &[u8], rooms: &[Room]) -> Vec<MazeLight> {
    let width_usize = width as usize;
    let height_usize = height as usize;
    if width_usize == 0 || height_usize == 0 {
        return Vec::new();
    }

    let mut lights = Vec::new();
    let mut occupied = vec![false; width_usize * height_usize];
    let max_lights = ((width_usize * height_usize) / 24).max(12);

    for room in rooms {
        if room.w < 3 || room.h < 3 {
            continue;
        }

        let start_x = room.x.saturating_add(1);
        let end_x = room.x.saturating_add(room.w).saturating_sub(1);
        let start_y = room.y.saturating_add(1);
        let end_y = room.y.saturating_add(room.h).saturating_sub(1);

        if start_x >= end_x || start_y >= end_y {
            continue;
        }

        let start_x_usize = start_x as usize;
        let end_x_usize = end_x as usize;
        let start_y_usize = start_y as usize;
        let end_y_usize = end_y as usize;

        for x_idx in (start_x_usize..end_x_usize).step_by(4) {
            for y_idx in (start_y_usize..end_y_usize).step_by(4) {
                if lights.len() >= max_lights {
                    break;
                }
                try_add_light(
                    &mut lights,
                    &mut occupied,
                    width,
                    height,
                    cells,
                    x_idx as u8,
                    y_idx as u8,
                    max_lights,
                );
            }
            if lights.len() >= max_lights {
                break;
            }
        }

        let center_x = start_x_usize + ((end_x_usize - start_x_usize) / 2);
        let center_y = start_y_usize + ((end_y_usize - start_y_usize) / 2);
        if lights.len() < max_lights {
            try_add_light(
                &mut lights,
                &mut occupied,
                width,
                height,
                cells,
                center_x as u8,
                center_y as u8,
                max_lights,
            );
        }
    }

    if width_usize > 4 && height_usize > 4 {
        let spacing = 8;
        let max_x = width_usize - 2;
        let max_y = height_usize - 2;
        for gx in (2..max_x).step_by(spacing) {
            for gy in (2..max_y).step_by(spacing) {
                if lights.len() >= max_lights {
                    break;
                }
                try_add_light(
                    &mut lights,
                    &mut occupied,
                    width,
                    height,
                    cells,
                    gx as u8,
                    gy as u8,
                    max_lights,
                );
            }
            if lights.len() >= max_lights {
                break;
            }
        }
    }

    lights
}

fn try_add_light(
    lights: &mut Vec<MazeLight>,
    occupied: &mut [bool],
    width: u8,
    height: u8,
    cells: &[u8],
    x: u8,
    y: u8,
    max_lights: usize,
) {
    if x >= width || y >= height {
        return;
    }

    if lights.len() >= max_lights {
        return;
    }

    let width_usize = width as usize;
    let idx = y as usize * width_usize + x as usize;
    if occupied[idx] || cells[idx] == 0 {
        return;
    }

    if !has_clearance(cells, width, height, x, y) {
        return;
    }

    occupied[idx] = true;
    lights.push(MazeLight {
        x,
        y,
        variant: ((x as u16 + y as u16) % 3) as u8,
    });
}

fn has_clearance(cells: &[u8], width: u8, height: u8, x: u8, y: u8) -> bool {
    if width < 3 || height < 3 {
        return false;
    }

    if x == 0 || y == 0 || x >= width - 1 || y >= height - 1 {
        return false;
    }

    let width_usize = width as usize;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                return false;
            }
            let idx = (ny as usize) * width_usize + (nx as usize);
            if cells[idx] == 0 {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::game::{maze_side_for_players, MAZE_MAX_SIDE, MAZE_MIN_SIDE};
    use std::collections::VecDeque;

    fn sizes() -> Vec<u8> {
        let mut sizes: Vec<u8> = (1..=16).map(maze_side_for_players).collect();
        sizes.extend([MAZE_MIN_SIDE, MAZE_MAX_SIDE]);
        sizes.sort();
        sizes.dedup();
        sizes
    }

    /// Open cells reachable from the first open cell.
    fn reachable(cells: &[u8], w: usize, h: usize) -> Vec<bool> {
        let mut seen = vec![false; w * h];
        let Some(start) = cells.iter().position(|&c| c == 1) else { return seen };
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        while let Some(i) = queue.pop_front() {
            let (x, y) = (i % w, i / w);
            let mut next = Vec::new();
            if x > 0 { next.push(i - 1); }
            if x + 1 < w { next.push(i + 1); }
            if y > 0 { next.push(i - w); }
            if y + 1 < h { next.push(i + w); }
            for n in next {
                if cells[n] == 1 && !seen[n] {
                    seen[n] = true;
                    queue.push_back(n);
                }
            }
        }
        seen
    }

    #[test]
    fn every_open_cell_and_room_is_reachable_at_every_size() {
        for side in sizes() {
            for seed in 0..40u64 {
                let (cells, rooms, _, _, teleporter) = generate_maze(side, side, seed);
                let w = side as usize;
                let seen = reachable(&cells, w, w);
                for (i, &c) in cells.iter().enumerate() {
                    assert!(c == 0 || seen[i], "side {side} seed {seed}: cell {i} cut off");
                }
                assert!(!rooms.is_empty(), "side {side} seed {seed}: no rooms");
                for r in &rooms {
                    assert!(seen[r.y as usize * w + r.x as usize], "side {side} seed {seed}: room cut off");
                }
                let (tx, ty) = teleporter;
                assert_eq!(cells[ty as usize * w + tx as usize], 1, "side {side} seed {seed}: teleporter in a wall");
            }
        }
    }

    #[test]
    fn outer_wall_stays_closed() {
        for side in sizes() {
            let (cells, ..) = generate_maze(side, side, 7);
            let w = side as usize;
            for i in 0..w {
                for idx in [i, (w - 1) * w + i, i * w, i * w + w - 1] {
                    assert_eq!(cells[idx], 0, "side {side}: border cell {idx} open");
                }
            }
        }
    }

    #[test]
    fn most_dead_ends_are_opened_into_loops() {
        let side = MAZE_MAX_SIDE as usize;
        let (cells, ..) = generate_maze(MAZE_MAX_SIDE, MAZE_MAX_SIDE, 3);
        let open = |x: usize, y: usize| cells[y * side + x] == 1;
        let (mut open_cells, mut dead_ends) = (0, 0);
        for y in 1..side - 1 {
            for x in 1..side - 1 {
                if !open(x, y) {
                    continue;
                }
                open_cells += 1;
                let exits = [open(x + 1, y), open(x - 1, y), open(x, y + 1), open(x, y - 1)]
                    .iter()
                    .filter(|&&o| o)
                    .count();
                if exits == 1 {
                    dead_ends += 1;
                }
            }
        }
        // an unbraided backtracker maze of this size has roughly 1 in 10
        // open cells as a dead end
        assert!(dead_ends * 25 < open_cells, "{dead_ends} dead ends in {open_cells} open cells");
    }
}
