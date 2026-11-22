/*
    the smoke break map, currently it is 3 rooms in a row joined by 2 hallways and nothing else but id like to add different room presets later.
    the walls between areas are thick blocks so the client never picks a breakable wall here
    tbh idk how this works i just cheated the maze code to make a map
*/


use crate::maze::types::Room;

pub const ARENA_W: u8 = 34;
pub const ARENA_H: u8 = 10;


// every walkable area in cells (x, y, w, h), in walking order room, hall, room, hall, room
pub const ARENA_AREAS: [(u8, u8, u8, u8); 5] = [
    (1, 1, 8, 8),
    (9, 4, 4, 2),
    (13, 1, 8, 8),
    (21, 4, 4, 2),
    (25, 1, 8, 8),
];


// which of the areas are rooms, the rest are hallways
pub const ARENA_ROOMS: [usize; 3] = [0, 2, 4];

// back to the lobby from the first room's corner, same as every maze has one i guess
pub const ARENA_TELEPORTER: (u8, u8) = (1, 1);

pub fn arena_cells() -> Vec<u8> {
    let mut cells = vec![0u8; ARENA_W as usize * ARENA_H as usize];
    for (x, y, w, h) in ARENA_AREAS {
        for cy in y..y + h {
            for cx in x..x + w {
                cells[cy as usize * ARENA_W as usize + cx as usize] = 1;
            }
        }
    }
    cells
}

pub fn arena_rooms() -> Vec<Room> {
    ARENA_ROOMS
        .iter()
        .map(|&i| {
            let (x, y, w, h) = ARENA_AREAS[i];
            Room { x, y, w, h }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::MazeData;

    fn at(cells: &[u8], x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= ARENA_W as i32 || y >= ARENA_H as i32 {
            return 0;
        }
        cells[y as usize * ARENA_W as usize + x as usize]
    }

    #[test]
    fn the_border_is_all_wall() {
        let cells = arena_cells();
        for x in 0..ARENA_W as i32 {
            assert_eq!((at(&cells, x, 0), at(&cells, x, ARENA_H as i32 - 1)), (0, 0));
        }
        for y in 0..ARENA_H as i32 {
            assert_eq!((at(&cells, 0, y), at(&cells, ARENA_W as i32 - 1, y)), (0, 0));
        }
    }

    #[test]
    fn hallways_join_the_rooms_and_the_blocks_between_are_solid() {
        let cells = arena_cells();
        // walk the middle row end to end, room, hall, room, hall, room, no wall in the way
        for x in 1..ARENA_W as i32 - 1 {
            assert_eq!(at(&cells, x, 4), 1, "row 4 blocked at x {x}");
        }
        // above and below the first hallway is solid wall
        for x in 9..13 {
            for y in [1, 2, 3, 6, 7, 8] {
                assert_eq!(at(&cells, x, y), 0, "hole at ({x}, {y})");
            }
        }
    }

    // the client makes a wall breakable when one wall cell sits between two open cells,
    // either side to side or up and down, here that should never happen
    #[test]
    fn no_wall_is_thin_enough_to_break() {
        let cells = arena_cells();
        for y in 0..ARENA_H as i32 {
            for x in 0..ARENA_W as i32 {
                if at(&cells, x, y) == 1 {
                    continue;
                }
                let across = at(&cells, x - 1, y) == 1 && at(&cells, x + 1, y) == 1;
                let down = at(&cells, x, y - 1) == 1 && at(&cells, x, y + 1) == 1;
                assert!(!across && !down, "thin wall at ({x}, {y})");
            }
        }
    }

    #[test]
    fn it_builds_into_a_full_maze() {
        let maze = MazeData::smoke_break_arena();
        assert_eq!((maze.width, maze.height, maze.rooms.len()), (ARENA_W, ARENA_H, 3));
        assert!(maze.props.is_empty());
        assert!(!maze.lights.is_empty(), "no ceiling lights");
        let open = |pos: (f32, f32, f32)| {
            let c = crate::server::game::MAZE_CELL_SIZE;
            at(&maze.cells, (pos.0 / c).floor() as i32, (pos.2 / c).floor() as i32) == 1
        };
        for _ in 0..50 {
            let (x, y, z, _) = maze.random_spawn_world(crate::server::game::MAZE_CELL_SIZE);
            assert!(open((x, y, z)), "spawned in a wall");
        }
        assert!(open(maze.teleporter_world(crate::server::game::MAZE_CELL_SIZE)));
    }
}
