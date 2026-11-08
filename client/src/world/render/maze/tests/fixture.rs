//! A small maze with every case the baker handles: border and interior
//! walls, wall runs crossing chunk borders, breakable walls, rooms, props
//! (a monitor facing a chair) and lights.

use std::collections::{HashMap, HashSet};

use crate::types::maze::{BreakableWall, CeilingLightInfo, Maze, Prop, PropKind, Room};

pub const W: u8 = 21;
pub const H: u8 = 13;

pub fn test_maze() -> Maze {
    let mut cells = vec![1u8; W as usize * H as usize];
    for y in 0..H as usize {
        for x in 0..W as usize {
            let border = x == 0 || y == 0 || x == W as usize - 1 || y == H as usize - 1;
            // A wall run along x = 7..=9 (across the chunk border at x = 8),
            // a column at x = 4 and a scattering of single walls.
            let run = y == 5 && (6..=10).contains(&x);
            let column = x == 4 && (2..=9).contains(&y);
            let speckle = (x * 7 + y * 13) % 11 == 0 && x > 11;
            if border || run || column || speckle {
                cells[y * W as usize + x] = 0;
            }
        }
    }
    let mut breakable_walls = HashMap::new();
    breakable_walls.insert((7, 5), BreakableWall::new());
    breakable_walls.insert((8, 5), BreakableWall::new());
    breakable_walls.insert((4, 3), BreakableWall::new());
    let prop = |kind, x, y| Prop { kind, x, y };
    Maze {
        width: W,
        height: H,
        cells,
        cell_size: 1.8,
        wall_height: 2.4,
        teleporter: (2, 2),
        rooms: vec![Room { x: 1, y: 1, w: 3, h: 3 }, Room { x: 9, y: 7, w: 4, h: 3 }],
        props: vec![
            prop(PropKind::Desk, 10, 8),
            prop(PropKind::Monitor, 10, 8),
            prop(PropKind::OfficeChair, 10, 9),
            prop(PropKind::Lamp, 2, 2),
            prop(PropKind::Crate, 6, 3),
            prop(PropKind::Barrel, 6, 9),
            prop(PropKind::SupplyCrate, 9, 2),
            prop(PropKind::Foosball, 15, 9),
        ],
        ceiling_lights: vec![
            CeilingLightInfo { x: 2, y: 2, variant: 0 },
            CeilingLightInfo { x: 9, y: 8, variant: 1 },
            CeilingLightInfo { x: 16, y: 3, variant: 2 },
        ],
        breakable_walls,
        broken_wall_scars: HashSet::new(),
    }
}
