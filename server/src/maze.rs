use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::net::{SocketAddr, UdpSocket};

use crate::protocols::MSG_MAZE;

#[derive(Clone, Copy, Debug)]
pub struct Room {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum DecorationKind {
    Table = 1,
    Chair = 2,
}

impl DecorationKind {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Decoration {
    pub kind: DecorationKind,
    pub x: u8,
    pub y: u8,
}

pub struct MazeData {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>,
    pub seed: u64,
    pub rooms: Vec<Room>,
    pub decorations: Vec<Decoration>,
    pub teleporter: (u8, u8),
}

impl MazeData {
    pub fn new(width: u8, height: u8) -> Self {
        let seed = rand::thread_rng().gen();
        let (cells, rooms, decorations, teleporter) = generate_maze(width, height, seed);
        Self {
            width,
            height,
            cells,
            seed,
            rooms,
            decorations,
            teleporter,
        }
    }

    pub fn send(&self, socket: &UdpSocket, addr: SocketAddr) {
        let header_len = 5; // msg, w, h, tele_x, tele_y
        let rooms_len = 1 + self.rooms.len() * 4; // count + (x,y,w,h)
        let decor_len = 1 + self.decorations.len() * 3; // count + (kind,x,y)
        let mut msg = Vec::with_capacity(header_len + rooms_len + decor_len + self.cells.len());

        msg.push(MSG_MAZE);
        msg.push(self.width);
        msg.push(self.height);
        msg.push(self.teleporter.0);
        msg.push(self.teleporter.1);

        msg.push(self.rooms.len() as u8);
        for room in &self.rooms {
            msg.push(room.x);
            msg.push(room.y);
            msg.push(room.w);
            msg.push(room.h);
        }

        msg.push(self.decorations.len() as u8);
        for deco in &self.decorations {
            msg.push(deco.kind.as_u8());
            msg.push(deco.x);
            msg.push(deco.y);
        }

        msg.extend_from_slice(&self.cells);
        let _ = socket.send_to(&msg, addr);
    }

    pub fn teleporter_world(&self, cell_size: f32) -> (f32, f32, f32) {
        let (tx, ty) = self.teleporter;
        (
            (tx as f32 + 0.5) * cell_size,
            0.0,
            (ty as f32 + 0.5) * cell_size,
        )
    }
}

// Depth-first perfect maze generator with post-process rooms. Cells: 1 open, 0 wall.
fn generate_maze(
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

    // Work with an odd-sized carving area so even-sized mazes still generate.
    let w_carve = if w % 2 == 0 { w - 1 } else { w };
    let h_carve = if h % 2 == 0 { h - 1 } else { h };

    // Base perfect maze with recursive backtracker.
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

    // Place teleporter in the first room if available; otherwise use entrance.
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
    let desired_rooms = if w >= 50 && h >= 50 { 10 } else { 4 };
    let mut size_bands: Vec<(usize, usize)> = vec![(4usize, 8usize)];
    if w >= 50 && h >= 50 {
        size_bands.push((4usize, 6usize)); // tighter fallback sizes to reach the quota
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
                // 1-cell padding keeps corridors between rooms
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

    // deterministic fallback pass to guarantee quota when space allows
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
        // Build candidate doorway cells along perimeter.
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

        // Create 2 connections per room (or as many as fit).
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
                    // reached a corridor; carve doorway cell and stop.
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

fn decorate_first_room(rooms: &[Room]) -> Vec<Decoration> {
    let mut decorations = Vec::new();
    if let Some(room) = rooms.first() {
        let table_x_u8 = room.x + (room.w / 2);
        let table_y_u8 = room.y + (room.h / 2);
        decorations.push(Decoration {
            kind: DecorationKind::Table,
            x: table_x_u8,
            y: table_y_u8,
        });

        let chair_positions = [
            (table_x_u8.wrapping_sub(1), table_y_u8),
            (table_x_u8 + 1, table_y_u8),
            (table_x_u8, table_y_u8.wrapping_sub(1)),
            (table_x_u8, table_y_u8 + 1),
        ];

        for (x, y) in chair_positions {
            decorations.push(Decoration {
                kind: DecorationKind::Chair,
                x,
                y,
            });
        }
    }

    decorations
}
