use super::Maze;

/// Share of useful spots (see `Maze::is_useful_breakable_spot`) that get a
/// breakable wall.
const BREAKABLE_CHANCE: f32 = 0.2;
/// Fewest steps around between the two sides of a breakable wall; going
/// through it takes 2. The tightest loop past a divider is 6 steps, so this
/// skips walls whose hole would save almost nothing.
const BREAKABLE_MIN_DETOUR: u32 = 8;

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

    /// Whether the cell is inside the maze and open (corridor or room).
    fn is_passable(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && x < self.width as i32
            && y < self.height as i32
            && self.cells[self.idx(x as usize, y as usize)] != 0
    }

    /// The two open cells a breakable wall at (x, y) would join: open on
    /// both sides along one axis and solid on both sides along the other, so
    /// it is a single-thickness divider between two spaces (not the face of
    /// a wall block, a corner, or a lone pillar).
    fn divided_cells(&self, x: u8, y: u8) -> Option<((i32, i32), (i32, i32))> {
        let (x, y) = (x as i32, y as i32);
        let open = |dx: i32, dy: i32| self.is_passable(x + dx, y + dy);
        if open(-1, 0) && open(1, 0) && !open(0, -1) && !open(0, 1) {
            Some(((x - 1, y), (x + 1, y)))
        } else if open(0, -1) && open(0, 1) && !open(-1, 0) && !open(1, 0) {
            Some(((x, y - 1), (x, y + 1)))
        } else {
            None
        }
    }

    /// Whether `a` reaches `b` through open cells within `max_steps` steps.
    fn reachable_within(&self, a: (i32, i32), b: (i32, i32), max_steps: u32) -> bool {
        let mut seen = std::collections::HashSet::from([a]);
        let mut frontier = vec![a];
        for _ in 0..max_steps {
            let mut next = Vec::new();
            for (x, y) in frontier {
                for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                    if (nx, ny) == b {
                        return true;
                    }
                    if self.is_passable(nx, ny) && seen.insert((nx, ny)) {
                        next.push((nx, ny));
                    }
                }
            }
            if next.is_empty() {
                return false;
            }
            frontier = next;
        }
        false
    }

    /// Whether breaking the wall at (x, y) would open a useful passage: it
    /// divides two open cells, and walking between them the long way takes
    /// at least `BREAKABLE_MIN_DETOUR` steps (otherwise the hole saves nothing).
    pub fn is_useful_breakable_spot(&self, x: u8, y: u8) -> bool {
        if self.is_passable(x as i32, y as i32) {
            return false;
        }
        match self.divided_cells(x, y) {
            Some((a, b)) => !self.reachable_within(a, b, BREAKABLE_MIN_DETOUR - 1),
            None => false,
        }
    }

    /// Spawn breakable walls at a share of the useful spots: single walls
    /// between two open spaces that are far apart on foot. Deterministic in
    /// the layout, so every client picks the same walls.
    pub fn spawn_breakable_walls(&mut self) {
        self.breakable_walls.clear();
        self.broken_wall_scars.clear();

        for y in 1..self.height.saturating_sub(1) {
            for x in 1..self.width.saturating_sub(1) {
                let hash = ((x as u32).wrapping_mul(73856093))
                    ^ ((y as u32).wrapping_mul(19349663))
                    ^ ((self.width as u32).wrapping_mul(83492791))
                    ^ ((self.height as u32).wrapping_mul(2654435761));
                let roll = (hash % 1000) as f32 / 1000.0;
                if roll >= BREAKABLE_CHANCE {
                    continue;
                }

                // Don't make walls near the teleporter breakable
                let dist_to_tele = (x as i32 - self.teleporter.0 as i32).abs()
                    + (y as i32 - self.teleporter.1 as i32).abs();
                if dist_to_tele <= 3 {
                    continue;
                }

                if self.is_useful_breakable_spot(x, y) {
                    self.breakable_walls.insert((x, y), BreakableWall::new());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::super::Maze;

    /// A maze from rows of `#` (wall) and `.` (open).
    fn maze(rows: &[&str]) -> Maze {
        let height = rows.len() as u8;
        let width = rows[0].len() as u8;
        let cells = rows.iter().flat_map(|r| r.bytes().map(|b| (b == b'.') as u8)).collect();
        Maze {
            width,
            height,
            cells,
            cell_size: 1.8,
            wall_height: 2.4,
            teleporter: (200, 200),
            rooms: Vec::new(),
            props: Vec::new(),
            ceiling_lights: Vec::new(),
            breakable_walls: HashMap::new(),
            broken_wall_scars: HashSet::new(),
        }
    }

    #[test]
    fn a_divider_between_far_apart_corridors_is_useful() {
        // (3, 1) splits the top corridor; the way around is 10 steps
        let m = maze(&[
            "#######",
            "#..#..#",
            "#.###.#",
            "#.....#",
            "#######",
        ]);
        assert!(m.is_useful_breakable_spot(3, 1));
    }

    #[test]
    fn wall_faces_corners_and_pillars_are_not() {
        let m = maze(&[
            "#######",
            "#.....#",
            "#.#...#",
            "#.....#",
            "#######",
        ]);
        assert!(!m.is_useful_breakable_spot(2, 2), "lone pillar");
        assert!(!m.is_useful_breakable_spot(3, 0), "outer wall face");
        assert!(!m.is_useful_breakable_spot(0, 0), "corner");
        assert!(!m.is_useful_breakable_spot(3, 1), "open cell");
    }

    #[test]
    fn a_divider_with_a_short_way_around_is_not() {
        // (2, 3) splits the bottom row, but going round over the top is 6 steps
        let m = maze(&[
            "#####",
            "#...#",
            "#.#.#",
            "#.#.#",
            "#####",
        ]);
        assert!(!m.is_useful_breakable_spot(2, 3));
    }

    /// A perfect maze on odd cells (recursive backtracker), braided a little.
    fn generated(side: u8, seed: u32) -> Maze {
        let w = side as usize;
        let mut cells = vec![0u8; w * w];
        let mut rng = seed | 1;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        let mut stack = vec![(1usize, 1usize)];
        cells[w + 1] = 1;
        while let Some(&(x, y)) = stack.last() {
            let mut dirs: Vec<(isize, isize)> = vec![(2, 0), (-2, 0), (0, 2), (0, -2)];
            dirs.retain(|&(dx, dy)| {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                nx > 0 && ny > 0 && nx < w as isize - 1 && ny < w as isize - 1
                    && cells[ny as usize * w + nx as usize] == 0
            });
            if dirs.is_empty() {
                stack.pop();
                continue;
            }
            let (dx, dy) = dirs[next() as usize % dirs.len()];
            let (nx, ny) = ((x as isize + dx) as usize, (y as isize + dy) as usize);
            cells[(y + ny) / 2 * w + (x + nx) / 2] = 1;
            cells[ny * w + nx] = 1;
            stack.push((nx, ny));
        }
        let rows: Vec<String> = cells
            .chunks(w)
            .map(|r| r.iter().map(|&c| if c == 1 { '.' } else { '#' }).collect())
            .collect();
        let rows: Vec<&str> = rows.iter().map(|r| r.as_str()).collect();
        maze(&rows)
    }

    #[test]
    fn spawned_walls_all_sit_between_two_open_spaces() {
        let mut total = 0;
        for seed in 1..20 {
            let mut m = generated(31, seed);
            m.spawn_breakable_walls();
            for &(x, y) in m.breakable_walls.keys() {
                let (x, y) = (x as i32, y as i32);
                let horizontal = m.is_passable(x - 1, y) && m.is_passable(x + 1, y);
                let vertical = m.is_passable(x, y - 1) && m.is_passable(x, y + 1);
                assert!(horizontal != vertical, "({x}, {y}) is not a divider");
                assert!(!m.is_passable(x, y));
            }
            total += m.breakable_walls.len();
        }
        assert!(total >= 19 * 3, "only {total} breakable walls in 19 mazes");
    }
}
