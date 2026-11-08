use super::Maze;

#[derive(Clone)]
pub struct BreakableWall {
    pub health: f32,
    pub max_health: f32,
}

impl BreakableWall {
    pub fn new() -> Self {
        Self {
            health: 150.0, // Takes ~3-4 shots from basic weapons
            max_health: 150.0,
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.health <= 0.0
    }
}

impl Maze {
    /// Damage a wall at the given cell position
    /// Returns true if the wall was destroyed
    pub fn damage_wall(&mut self, cell_x: u8, cell_y: u8, damage: f32) -> bool {
        // Check if it's a breakable wall
        if let Some(wall) = self.breakable_walls.get_mut(&(cell_x, cell_y)) {
            wall.health -= damage;

            if wall.is_destroyed() {
                // Remove the wall from the maze
                let idx = self.idx(cell_x as usize, cell_y as usize);
                self.cells[idx] = 1; // Make it open
                self.breakable_walls.remove(&(cell_x, cell_y));
                self.broken_wall_scars.insert((cell_x, cell_y));
                return true;
            }
        }
        false
    }

    pub fn force_break_wall(&mut self, cell_x: u8, cell_y: u8) {
        let idx = self.idx(cell_x as usize, cell_y as usize);
        if idx < self.cells.len() {
            self.cells[idx] = 1;
        }
        self.breakable_walls.remove(&(cell_x, cell_y));
        self.broken_wall_scars.insert((cell_x, cell_y));
    }

    pub fn is_broken_wall(&self, cell_x: u8, cell_y: u8) -> bool {
        self.broken_wall_scars.contains(&(cell_x, cell_y))
    }

    /// Check if a wall is breakable
    pub fn is_breakable(&self, cell_x: u8, cell_y: u8) -> bool {
        self.breakable_walls.contains_key(&(cell_x, cell_y))
    }

    /// Get wall health percentage (0.0 to 1.0)
    pub fn wall_health_percent(&self, cell_x: u8, cell_y: u8) -> Option<f32> {
        self.breakable_walls
            .get(&(cell_x, cell_y))
            .map(|w| w.health / w.max_health)
    }

    /// Spawn random breakable walls (about 2-3% of walls)
    pub fn spawn_breakable_walls(&mut self) {
        self.breakable_walls.clear();
        self.broken_wall_scars.clear();

        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[self.idx(x as usize, y as usize)] == 0 {
                    // Only make interior walls breakable (not outer edges)
                    let is_interior = x > 0 && x < self.width - 1 && y > 0 && y < self.height - 1;

                    if is_interior {
                        let hash = ((x as u32).wrapping_mul(73856093))
                            ^ ((y as u32).wrapping_mul(19349663))
                            ^ ((self.width as u32).wrapping_mul(83492791))
                            ^ ((self.height as u32).wrapping_mul(2654435761));
                        let roll = (hash % 1000) as f32 / 1000.0;

                        // Don't make walls adjacent to teleporter breakable
                        let dist_to_tele = ((x as i32 - self.teleporter.0 as i32).abs()
                            + (y as i32 - self.teleporter.1 as i32).abs())
                            as u8;

                        if dist_to_tele > 3 && roll < 0.025 {
                            self.breakable_walls.insert((x, y), BreakableWall::new());
                        }
                    }
                }
            }
        }
    }
}
