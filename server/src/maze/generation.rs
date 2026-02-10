use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::maze::decoration::decorate_first_room;
use crate::maze::types::{Decoration, Room};

pub fn generate_maze(
    width: u8,
    height: u8,
    seed: u64,
) -> (Vec<u8>, Vec<Room>, Vec<Decoration>, (u8, u8)) {
    let w = width as usize;
    let h = height as usize;
    let mut grid = vec![0u8; w * h];
    let mut rng = StdRng::seed_from_u64(seed);

    if w < 3 || h < 3 {
        return (grid, Vec::new(), Vec::new(), (1, 1));
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

    let teleporter = if let Some(room) = rooms.first() {
        let tx = (room.x as usize + 1).min((room.x + room.w - 1) as usize);
        let ty = (room.y as usize + 1).min((room.y + room.h - 1) as usize);
        (tx as u8, ty as u8)
    } else {
        (1u8, 1u8)
    };

    let decorations = decorate_first_room(&rooms);

    (grid, rooms, decorations, teleporter)
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
    let mut size_bands: Vec<(usize, usize)> = vec![(4usize, 8usize)];
    if w >= 40 && h >= 40 {
        size_bands.push((4usize, 6usize));
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

fn connect_rooms(grid: &mut [u8], w: usize, h: usize, rooms: &[Room], rng: &mut StdRng) {
    for room in rooms {
        let mut edges = Vec::new();
        let rx = room.x as usize;
        let ry = room.y as usize;
        let rw = room.w as usize;
        let rh = room.h as usize;

        for x in (rx + 1)..(rx + rw - 1) {
            edges.push((x, ry, 0i32, -1i32));
            edges.push((x, ry + rh - 1, 0, 1));
        }
        for y in (ry + 1)..(ry + rh - 1) {
            edges.push((rx, y, -1i32, 0));
            edges.push((rx + rw - 1, y, 1, 0));
        }

        edges.shuffle(rng);

        let mut links = 0;
        for &(sx, sy, dx, dy) in &edges {
            if links >= 2 {
                break;
            }

            let mut cx = sx as i32;
            let mut cy = sy as i32;
            loop {
                let nx = cx + dx;
                let ny = cy + dy;
                if nx <= 0 || ny <= 0 || nx >= (w as i32 - 1) || ny >= (h as i32 - 1) {
                    break;
                }
                let idx = ny as usize * w + nx as usize;
                if grid[idx] == 1 {
                    grid[sy * w + sx] = 1;
                    links += 1;
                    break;
                }
                grid[idx] = 1;
                cx = nx;
                cy = ny;
            }
        }
    }
}
