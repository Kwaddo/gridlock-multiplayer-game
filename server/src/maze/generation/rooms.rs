//! Rooms: carving them into the grid and tunnelling each one to the maze.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::maze::types::Room;

use super::constants::MAX_ROOM_TUNNEL;

pub(super) fn carve_rooms(grid: &mut [u8], w: usize, h: usize, rng: &mut StdRng) -> Vec<Room> {
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
pub(super) fn connect_rooms(grid: &mut [u8], w: usize, h: usize, rooms: &[Room], rng: &mut StdRng) {
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
