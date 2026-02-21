use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;

use crate::maze::types::{Prop, PropKind, Room};

/// Populate rooms with props (visual set dressing, cover). First room gets a
/// consistent spawn marker set; other rooms get randomized props with spacing.
pub fn generate_props(rooms: &[Room], rng: &mut StdRng) -> Vec<Prop> {
    let mut props = Vec::new();

    for (i, room) in rooms.iter().enumerate() {
        if i == 0 {
            decorate_spawn_room(room, &mut props);
        } else if is_suitable_for_computer_lab(room) {
            // Always attempt to make suitable rooms into offices
            if let Some(occupied) = create_computer_lab(room, &mut props) {
                scatter_additional_props(room, rng, &mut props, &occupied);
            } else {
                // Fallback if the strict pattern does not fit after border checks
                scatter_room_props(room, rng, &mut props);
            }
        } else {
            scatter_room_props(room, rng, &mut props);
        }
    }

    props
}

fn decorate_spawn_room(room: &Room, props: &mut Vec<Prop>) {
    // Center feature: supply crate + lamp to visually mark spawn area.
    let cx = room.x + (room.w / 2);
    let cy = room.y + (room.h / 2);

    props.push(Prop {
        kind: PropKind::SupplyCrate,
        x: cx,
        y: cy,
    });

    // Four lamps in a plus shape if space allows, to make spawn recognizable.
    let lamp_offsets = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)];
    for (dx, dy) in lamp_offsets {
        let lx = cx as i16 + dx as i16;
        let ly = cy as i16 + dy as i16;
        if lx >= room.x as i16
            && lx < (room.x + room.w) as i16
            && ly >= room.y as i16
            && ly < (room.y + room.h) as i16
        {
            props.push(Prop {
                kind: PropKind::Lamp,
                x: lx as u8,
                y: ly as u8,
            });
        }
    }
}

fn scatter_room_props(room: &Room, rng: &mut StdRng, props: &mut Vec<Prop>) {
    let kinds = [
        PropKind::Crate,
        PropKind::Barrel,
        PropKind::SupplyCrate,
        PropKind::Lamp,
        PropKind::Foosball,
    ];
    let density = if (room.w as usize * room.h as usize) > 36 {
        6
    } else {
        4
    };
    let target = rng.gen_range(2..=density); // ensure at least 2 props in non-spawn rooms

    let mut occupied = std::collections::HashSet::new();
    let mut placed = 0;
    let mut attempts = 0;
    while placed < target && attempts < target * 8 {
        attempts += 1;
        let x = rng.gen_range(room.x..room.x + room.w);
        let y = rng.gen_range(room.y..room.y + room.h);
        // keep a buffer from walls for easier navigation
        if x == room.x || y == room.y || x == room.x + room.w - 1 || y == room.y + room.h - 1 {
            continue;
        }
        if occupied.insert((x, y)) {
            let kind = kinds.choose(rng).unwrap();
            props.push(Prop { kind: *kind, x, y });
            placed += 1;
        }
    }
}

/// Scatter extra props in rooms that already have an office layout, keeping distance from office tiles.
fn scatter_additional_props(
    room: &Room,
    rng: &mut StdRng,
    props: &mut Vec<Prop>,
    occupied: &HashSet<(u8, u8)>,
) {
    let kinds = [PropKind::Crate, PropKind::Barrel, PropKind::Lamp];
    let mut taken = occupied.clone();

    let max_extra = 3; // light dressing only
    let mut placed = 0;
    let mut attempts = 0;
    while placed < max_extra && attempts < max_extra * 10 {
        attempts += 1;
        let x = rng.gen_range(room.x..room.x + room.w);
        let y = rng.gen_range(room.y..room.y + room.h);

        // Keep off walls and avoid office footprint by 1 tile in all directions
        if x <= room.x || y <= room.y || x >= room.x + room.w - 1 || y >= room.y + room.h - 1 {
            continue;
        }

        let too_close = (-1i8..=1).any(|dx| {
            (-1i8..=1).any(|dy| {
                let nx = x as i16 + dx as i16;
                let ny = y as i16 + dy as i16;
                nx >= room.x as i16
                    && nx < (room.x + room.w) as i16
                    && ny >= room.y as i16
                    && ny < (room.y + room.h) as i16
                    && taken.contains(&(nx as u8, ny as u8))
            })
        });

        if too_close {
            continue;
        }

        if taken.insert((x, y)) {
            let kind = kinds.choose(rng).unwrap();
            props.push(Prop { kind: *kind, x, y });
            placed += 1;
        }
    }
}

/// Check if room dimensions are suitable for an office layout
fn is_suitable_for_computer_lab(room: &Room) -> bool {
    let w = room.w as usize;
    let h = room.h as usize;
    // Need room with at least 5x7 to fit the strict 3x5 layout + 1 tile border
    w >= 5 && h >= 7
}

/// Create office with specific chair and table pattern
fn create_computer_lab(room: &Room, props: &mut Vec<Prop>) -> Option<HashSet<(u8, u8)>> {
    // Leave 1-cell border from walls for navigation
    let start_x = room.x + 1;
    let start_y = room.y + 1;
    let end_x = room.x + room.w.saturating_sub(2);
    let end_y = room.y + room.h.saturating_sub(2);

    // usable area excludes a 1-tile navigation buffer on every side
    let usable_width = end_x.saturating_sub(start_x) as usize + 1;
    let usable_height = end_y.saturating_sub(start_y) as usize + 1;

    // We want exactly a 3x5 footprint repeating the strict pattern.
    if usable_width < 3 || usable_height < 5 {
        return None; // Too small for the strict office pattern
    }

    let mut occupied: HashSet<(u8, u8)> = HashSet::new();

    // Anchor pattern at the top-left of usable space; only the first 3x5 area is populated.
    for y in 0..5 {
        let grid_y = start_y + y as u8;
        let row_is_even = y % 2 == 0;

        for x in 0..3 {
            let grid_x = start_x + x as u8;
            let pattern = match (row_is_even, x) {
                (true, 0) => Some(PropKind::OfficeChair),  // chair
                (true, 1) => Some(PropKind::Desk),         // table
                (true, 2) => None,                         // space
                (false, 0) => None,                        // space
                (false, 1) => Some(PropKind::Desk),        // table
                (false, 2) => Some(PropKind::OfficeChair), // chair
                _ => None,
            };

            match pattern {
                Some(PropKind::Desk) => {
                    // Place monitor before desk so adjacency checks see the monitor.
                    props.push(Prop {
                        kind: PropKind::Monitor,
                        x: grid_x,
                        y: grid_y,
                    });
                    props.push(Prop {
                        kind: PropKind::Desk,
                        x: grid_x,
                        y: grid_y,
                    });
                    occupied.insert((grid_x, grid_y));
                }
                Some(kind) => props.push(Prop {
                    kind,
                    x: grid_x,
                    y: grid_y,
                }),
                None => {}
            }
        }
    }

    Some(occupied)
}
