use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::maze::decoration::generate_props;
use crate::maze::types::{MazeLight, Prop, Room};

mod braid;
mod constants;
mod lights;
mod rooms;
#[cfg(test)]
mod tests;

use braid::braid_dead_ends;
use lights::generate_ceiling_lights;
use rooms::{carve_rooms, connect_rooms};

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
