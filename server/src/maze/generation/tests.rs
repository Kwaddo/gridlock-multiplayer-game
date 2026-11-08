use super::*;
use crate::server::game::{maze_side_for_players, MAZE_MAX_SIDE, MAZE_MIN_SIDE};
use std::collections::VecDeque;

fn sizes() -> Vec<u8> {
    let mut sizes: Vec<u8> = (1..=16).map(maze_side_for_players).collect();
    sizes.extend([MAZE_MIN_SIDE, MAZE_MAX_SIDE]);
    sizes.sort();
    sizes.dedup();
    sizes
}

/// Open cells reachable from the first open cell.
fn reachable(cells: &[u8], w: usize, h: usize) -> Vec<bool> {
    let mut seen = vec![false; w * h];
    let Some(start) = cells.iter().position(|&c| c == 1) else { return seen };
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        let mut next = Vec::new();
        if x > 0 { next.push(i - 1); }
        if x + 1 < w { next.push(i + 1); }
        if y > 0 { next.push(i - w); }
        if y + 1 < h { next.push(i + w); }
        for n in next {
            if cells[n] == 1 && !seen[n] {
                seen[n] = true;
                queue.push_back(n);
            }
        }
    }
    seen
}

#[test]
fn every_open_cell_and_room_is_reachable_at_every_size() {
    for side in sizes() {
        for seed in 0..40u64 {
            let (cells, rooms, _, _, teleporter) = generate_maze(side, side, seed);
            let w = side as usize;
            let seen = reachable(&cells, w, w);
            for (i, &c) in cells.iter().enumerate() {
                assert!(c == 0 || seen[i], "side {side} seed {seed}: cell {i} cut off");
            }
            assert!(!rooms.is_empty(), "side {side} seed {seed}: no rooms");
            for r in &rooms {
                assert!(seen[r.y as usize * w + r.x as usize], "side {side} seed {seed}: room cut off");
            }
            let (tx, ty) = teleporter;
            assert_eq!(cells[ty as usize * w + tx as usize], 1, "side {side} seed {seed}: teleporter in a wall");
        }
    }
}

#[test]
fn outer_wall_stays_closed() {
    for side in sizes() {
        let (cells, ..) = generate_maze(side, side, 7);
        let w = side as usize;
        for i in 0..w {
            for idx in [i, (w - 1) * w + i, i * w, i * w + w - 1] {
                assert_eq!(cells[idx], 0, "side {side}: border cell {idx} open");
            }
        }
    }
}

#[test]
fn most_dead_ends_are_opened_into_loops() {
    let side = MAZE_MAX_SIDE as usize;
    let (cells, ..) = generate_maze(MAZE_MAX_SIDE, MAZE_MAX_SIDE, 3);
    let open = |x: usize, y: usize| cells[y * side + x] == 1;
    let (mut open_cells, mut dead_ends) = (0, 0);
    for y in 1..side - 1 {
        for x in 1..side - 1 {
            if !open(x, y) {
                continue;
            }
            open_cells += 1;
            let exits = [open(x + 1, y), open(x - 1, y), open(x, y + 1), open(x, y - 1)]
                .iter()
                .filter(|&&o| o)
                .count();
            if exits == 1 {
                dead_ends += 1;
            }
        }
    }
    // an unbraided backtracker maze of this size has roughly 1 in 10
    // open cells as a dead end
    assert!(dead_ends * 25 < open_cells, "{dead_ends} dead ends in {open_cells} open cells");
}
