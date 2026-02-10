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
pub enum PropKind {
    Crate = 1,
    Barrel = 2,
    SupplyCrate = 3,
    Lamp = 4,
    Desk = 5,
    Monitor = 6,
    OfficeChair = 7,
}

impl PropKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(PropKind::Crate),
            2 => Some(PropKind::Barrel),
            3 => Some(PropKind::SupplyCrate),
            4 => Some(PropKind::Lamp),
            5 => Some(PropKind::Desk),
            6 => Some(PropKind::Monitor),
            7 => Some(PropKind::OfficeChair),
            _ => None,
        }
    }

    /// Get collision radius for this prop type (0.0 = no collision)
    pub fn collision_radius(self) -> f32 {
        match self {
            PropKind::Crate => 0.35,
            PropKind::Barrel => 0.28,
            PropKind::SupplyCrate => 0.35,
            PropKind::Lamp => 0.0, // lamps don't block movement
            PropKind::Desk => 0.40,
            PropKind::Monitor => 0.0,     // monitors are on desks
            PropKind::OfficeChair => 0.0, // chairs can be pushed through
        }
    }
}

#[derive(Clone, Copy)]
pub struct Prop {
    pub kind: PropKind,
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
    pub props: Vec<Prop>,
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

    pub fn get_prop_at(&self, x: u8, y: u8) -> Option<&Prop> {
        self.props.iter().find(|p| p.x == x && p.y == y)
    }

    pub fn cell_from_world(&self, pos: Vec3) -> (i32, i32) {
        let cx = (pos.x / self.cell_size).floor() as i32;
        let cy = (pos.z / self.cell_size).floor() as i32;
        (cx, cy)
    }

    pub fn is_blocked(&self, pos: Vec3, radius: f32) -> bool {
        // Check walls
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

        // Check prop collisions
        for prop in &self.props {
            let prop_radius = prop.kind.collision_radius();
            if prop_radius > 0.0 {
                let prop_pos = self.cell_to_world(prop.x as i32, prop.y as i32);
                let dx = pos.x - prop_pos.x;
                let dz = pos.z - prop_pos.z;
                let dist_sq = dx * dx + dz * dz;
                let collision_dist = radius + prop_radius;
                if dist_sq < collision_dist * collision_dist {
                    return true;
                }
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
