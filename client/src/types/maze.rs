use ::rand::{thread_rng, Rng};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
pub struct Room {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, Copy)]
pub enum DecorationKind {
    Table = 1,
    Chair = 2,
}

impl DecorationKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(DecorationKind::Table),
            2 => Some(DecorationKind::Chair),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Decoration {
    pub kind: DecorationKind,
    pub x: u8,
    pub y: u8,
}

pub struct Maze {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>, // 1 = open path, 0 = wall
    pub cell_size: f32,
    pub wall_height: f32,
    pub teleporter: (u8, u8),
    pub rooms: Vec<Room>,
    pub decorations: Vec<Decoration>,
}

impl Maze {
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width as usize + x
    }

    pub fn is_open(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        self.cells[self.idx(x as usize, y as usize)] == 1
    }

    pub fn cell_from_world(&self, pos: Vec3) -> (i32, i32) {
        let cx = (pos.x / self.cell_size).floor() as i32;
        let cy = (pos.z / self.cell_size).floor() as i32;
        (cx, cy)
    }

    pub fn is_blocked(&self, pos: Vec3, radius: f32) -> bool {
        for &(ox, oz) in [
            (-radius, -radius),
            (-radius, radius),
            (radius, -radius),
            (radius, radius),
        ]
        .iter()
        {
            let sample = vec3(pos.x + ox, pos.y, pos.z + oz);
            let (cx, cy) = self.cell_from_world(sample);
            if !self.is_open(cx, cy) {
                return true;
            }
        }
        false
    }

    pub fn clamp_move(&self, current: Vec3, desired: Vec3, radius: f32) -> Vec3 {
        let mut result = desired;
        if self.is_blocked(result, radius) {
            result.x = current.x;
            if self.is_blocked(result, radius) {
                result.x = desired.x;
                result.z = current.z;
                if self.is_blocked(result, radius) {
                    result = current;
                }
            }
        }
        result
    }

    pub fn cell_to_world(&self, x: i32, y: i32) -> Vec3 {
        vec3(
            (x as f32 + 0.5) * self.cell_size,
            0.0,
            (y as f32 + 0.5) * self.cell_size,
        )
    }

    pub fn teleporter_world(&self) -> Vec3 {
        self.cell_to_world(self.teleporter.0 as i32, self.teleporter.1 as i32)
    }

    pub fn spawn_point(&self) -> Vec3 {
        let mut open_cells = Vec::new();
        for y in 1..(self.height as i32 - 1) {
            for x in 1..(self.width as i32 - 1) {
                if self.is_open(x, y) {
                    open_cells.push((x, y));
                }
            }
        }

        if !open_cells.is_empty() {
            let mut rng = thread_rng();
            let idx = rng.gen_range(0..open_cells.len());
            let (x, y) = open_cells[idx];
            self.cell_to_world(x, y)
        } else {
            vec3(0.0, 0.0, 0.0)
        }
    }
}
