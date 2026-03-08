use crate::maze::types::Room;
use crate::maze::pathfinding::*;

pub struct MazeAnalysis {
    pub chokepoints: Vec<(u8, u8)>, // chokepoints are pairs of room that are connected by a single path
    pub intersections: Vec<(u8, u8)>,
    pub long_sightlines: Vec<(u8, u8)>, // long sightlines are pairs of rooms that have a direct line of sight between them, will be favoried by angle holders
    pub room_entrances: Vec<(u8, u8)>, 
}

pub struct Sightline {
    pub cells: Vec<(u8, u8)>,
    pub direction: (i8, i8), // Horizontal: (1, 0), Vertical: (0, 1), 
}

fn is_open(cells: &[u8], width: u8, x: u8, y: u8) -> bool {
    if x >= width || y >= cells.len() as u8 / width {
        return false;
    }
    let index = (y as usize) * (width as usize) + (x as usize);
    cells[index] == 1
}

fn in_bounds(width: u8, height: u8, x: u8, y: u8) -> bool {
    x < width && y < height
}

fn is_inside_room(x: u8, y: u8, room: &Room) -> bool {
    x >= room.x
        && x < room.x.saturating_add(room.w)
        && y >= room.y
        && y < room.y.saturating_add(room.h)
}

fn is_inside_any_room(x: u8, y: u8, rooms: &[Room]) -> bool {
    rooms.iter().any(|room| is_inside_room(x, y, room))
}

 
pub fn compute(cells: &[u8], width: u8, height: u8, rooms: &[Room]) -> MazeAnalysis {
    let mut chokepoints = Vec::new();
    let mut intersections = Vec::new();
    let mut long_sightlines = Vec::new();
    let mut room_entrances = Vec::new();

    
    /* 
    find chokepoints and intersections
    */
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            // if cell is not open, then it can't be a chokepoint or intersection
            if !is_open(cells, width, x, y) {
                continue;
            }
            let left = is_open(cells, width, x - 1, y);
            let right = is_open(cells, width, x + 1, y);
            let up = is_open(cells, width, x, y - 1);
            let down = is_open(cells, width, x, y + 1);

            let open_neighbors = [left, right, up, down].iter().filter(|&&open| open).count();

            if open_neighbors >= 3 {
                intersections.push((x, y));
            } else if open_neighbors == 2 {
                // check if neighbors are opposite each other
                if (left && right) || (up && down) {
                    chokepoints.push((x, y));
                }
            }
        }
    }

    // horizontal sightlines

    for y in 0..height {
        let mut run_start: Option<u8> = None;
        for x in 0..width {
            if is_open(cells, width, x, y) {
                if run_start.is_none() {
                    run_start = Some(x);
                }
            } else {
                if let Some(start) = run_start {
                    if x - start >= 4 {
                        long_sightlines.push((start, y));
                    }
                    run_start = None;
                }
            }
        }
        // if sightline reaches edge without hitting wall, check if it is long enough
        if let Some(start) = run_start {
            if width - start >= 4 {
                long_sightlines.push((start, y));
            }
        }
    }
    // vertical sightlines
    for x in 0..width {
        let mut run_start: Option<u8> = None;
        for y in 0..height {
            if is_open(cells, width, x, y) {
                if run_start.is_none() {
                    run_start = Some(y);
                }
            } else {
                if let Some(start) = run_start {
                    if y - start >= 4 {
                        long_sightlines.push((x, start));
                    }
                    run_start = None;
                }
            }
        }
        if let Some(start) = run_start {
            if height - start >= 4 {
                long_sightlines.push((x, start));
            }
        }
    }

    // find room entrances
    for room in rooms {
        let mut entrances = Vec::new();
        // check top and bottom edges of the room
        for x in room.x..room.x + room.w {
            if is_open(cells, width, x, room.y.saturating_sub(1)) {
                entrances.push((x, room.y));
            }
            if is_open(cells, width, x, room.y + room.h) {
                entrances.push((x, room.y + room.h - 1));
            }
        }
        // check left and right edges of the room
        for y in room.y..room.y + room.h {
            if is_open(cells, width, room.x.saturating_sub(1), y) {
                entrances.push((room.x, y));
            }
            if is_open(cells, width, room.x + room.w, y) {
                entrances.push((room.x + room.w - 1, y));
            }
        }
        // add unique entrances to the list
        for entrance in entrances {
            if !room_entrances.contains(&entrance) {
                room_entrances.push(entrance);
            }
        }
    }
    
    MazeAnalysis { chokepoints, intersections, long_sightlines, room_entrances }
}
