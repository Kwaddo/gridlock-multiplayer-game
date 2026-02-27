use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node {
    idx: u16,
    g: u32,
    f: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering so BinaryHeap behaves like a min-heap on f-score.
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[inline]
fn to_idx(x: u8, y: u8, width: u8) -> usize {
    (y as usize) * (width as usize) + (x as usize)
}

#[inline]
fn to_xy(idx: usize, width: u8) -> (u8, u8) {
    let w = width as usize;
    ((idx % w) as u8, (idx / w) as u8)
}

#[inline]
fn manhattan(a: (u8, u8), b: (u8, u8)) -> u32 {
    let dx = (a.0 as i32 - b.0 as i32).abs() as u32;
    let dy = (a.1 as i32 - b.1 as i32).abs() as u32;
    dx + dy
}

pub fn astar(
    cells: &[u8],
    width: u8,
    height: u8,
    start: (u8, u8),
    goal: (u8, u8),
) -> Option<Vec<(u8, u8)>> {
    // validate inputs
    if start.0 >= width || start.1 >= height || goal.0 >= width || goal.1 >= height {
        return None; // out of bounds
    }
    if start == goal {
        return Some(vec![start]); // trivial case
    }

    // array allocations
    let n = (width as usize) * (height as usize);
    if cells.len() != n {
        return None; // invalid grid size
    }

    if cells[to_idx(start.0, start.1, width)] == 0 || cells[to_idx(goal.0, goal.1, width)] == 0 {
        return None; // start or goal is blocked
    }

    // convert start and goal to indices
    let start_idx = to_idx(start.0, start.1, width) as u16;
    let goal_idx = to_idx(goal.0, goal.1, width) as u16;

    let mut g_scores = vec![u32::MAX; n]; // cost from start to node
    let mut came_from = vec![u16::MAX; n]; // parent node index
    let mut closed = vec![false; n]; // closed set

    // open set as a min-heap priority queue
    let mut open = BinaryHeap::new();
    g_scores[start_idx as usize] = 0;

    let f0 = manhattan(start, goal);
    open.push(Node { idx: start_idx, g: 0, f: f0 });

    // A* search loop
    while let Some(current) = open.pop() {
        // while heap is not empty
        let curr_idx = current.idx as usize;
        if closed[curr_idx] {
            continue; // skip if already processed
        }

        if current.idx == goal_idx {
            // reconstruct path
            let mut path = Vec::new();
            let mut idx = current.idx;

            loop {
                let (x, y) = to_xy(idx as usize, width);
                path.push((x, y));

                if idx == start_idx {
                    break;
                }

                let parent = came_from[idx as usize];
                if parent == u16::MAX {
                    return None; // safety guard
                }
                idx = parent;
            }

            path.reverse(); // reverse to get path from start to goal
            return Some(path);
        }

        closed[curr_idx] = true; // mark current node as processed

        let (cx, cy) = to_xy(curr_idx, width);
        let neighbors = [
            (cx.wrapping_sub(1), cy), // left
            (cx.wrapping_add(1), cy), // right
            (cx, cy.wrapping_sub(1)), // up
            (cx, cy.wrapping_add(1)), // down
        ];

        for (nx, ny) in neighbors.iter() {
            if *nx >= width || *ny >= height {
                continue; // out of bounds
            }
            let neighbor_idx = to_idx(*nx, *ny, width) as u16;
            if cells[neighbor_idx as usize] == 0 || closed[neighbor_idx as usize] {
                continue; // blocked or already processed
            }
            let tentative_g = g_scores[curr_idx].saturating_add(1); // saturating add to prevent overflow
            if tentative_g < g_scores[neighbor_idx as usize] {
                // better path found
                g_scores[neighbor_idx as usize] = tentative_g;
                came_from[neighbor_idx as usize] = current.idx;

                let h = manhattan((*nx, *ny), goal);
                let f = tentative_g.saturating_add(h);
                open.push(Node { idx: neighbor_idx, g: tentative_g, f });
            }
        }
    }
    None
}


/*
    World to cell and cell to world are convertions between coordinate systems used for pathfinding
    cell to world takes a maze cell coordinate and returns a world coordinate at the center of that cell
    and vice versa for world to cell. World to cell also has some extra logic to handle the case where the world coordinate is outside the maze or in a wall, 
    it will search for the nearest valid cell within a 3 cell radius and return that instead.
*/
pub fn world_to_cell(
    wx: f32,
    wz: f32,
    cell_size: f32,
    cells: &[u8],
    width: u8,
    height: u8,
) -> Option<(u8, u8)> {
    if cell_size <= 0.0 || width == 0 || height == 0 {
        return None;
    }

    let n = width as usize * height as usize;
    if cells.len() != n {
        return None;
    }

    let base_x = (wx / cell_size).floor() as i16;
    let base_y = (wz / cell_size).floor() as i16;

    let in_bounds =
        |x: i16, y: i16| x >= 0 && y >= 0 && x < width as i16 && y < height as i16;

    if in_bounds(base_x, base_y) {
        let idx = to_idx(base_x as u8, base_y as u8, width);
        if cells[idx] == 1 {
            return Some((base_x as u8, base_y as u8));
        }
    }

    let mut best: Option<((u8, u8), i32)> = None;
    for r in 1..=3i16 {
        for y in (base_y - r)..=(base_y + r) {
            for x in (base_x - r)..=(base_x + r) {
                if !in_bounds(x, y) {
                    continue;
                }
                let idx = to_idx(x as u8, y as u8, width);
                if cells[idx] == 0 {
                    continue;
                }

                let dx = x - base_x;
                let dy = y - base_y;
                let dist2 = (dx as i32 * dx as i32) + (dy as i32 * dy as i32);
                match best {
                    Some((_, best_d2)) if dist2 >= best_d2 => {}
                    _ => best = Some(((x as u8, y as u8), dist2)),
                }
            }
        }
        if best.is_some() {
            break;
        }
    }

    best.map(|(cell, _)| cell)
}

pub fn cell_to_world(cx: u8, cy: u8, cell_size: f32) -> (f32, f32) {
    (
        (cx as f32 + 0.5) * cell_size,
        (cy as f32 + 0.5) * cell_size,
    )
}
