use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::net::{SocketAddr, UdpSocket};

use crate::protocols::MSG_MAZE;

pub struct MazeData {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>,
    pub seed: u64,
}

impl MazeData {
    pub fn new(width: u8, height: u8) -> Self {
        let seed = rand::thread_rng().gen();
        let cells = generate_maze(width, height, seed);
        Self {
            width,
            height,
            cells,
            seed,
        }
    }

    pub fn send(&self, socket: &UdpSocket, addr: SocketAddr) {
        let mut msg = Vec::with_capacity(3 + self.cells.len());
        msg.push(MSG_MAZE);
        msg.push(self.width);
        msg.push(self.height);
        msg.extend_from_slice(&self.cells);
        let _ = socket.send_to(&msg, addr);
    }
}

// Depth-first perfect maze generator. Cells: 1 open, 0 wall.
fn generate_maze(width: u8, height: u8, seed: u64) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut grid = vec![0u8; w * h];
    let mut rng = StdRng::seed_from_u64(seed);

    if w < 3 || h < 3 || w % 2 == 0 || h % 2 == 0 {
        return grid;
    }

    let mut stack = Vec::new();
    let start = (1usize, 1usize);
    grid[start.1 * w + start.0] = 1;
    stack.push(start);

    while let Some(&(cx, cy)) = stack.last() {
        let mut neighbors = Vec::new();
        for (dx, dy) in [(2i32, 0i32), (-2, 0), (0, 2), (0, -2)] {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx > 0 && ny > 0 && nx < (w as i32 - 1) && ny < (h as i32 - 1) {
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

    grid
}
