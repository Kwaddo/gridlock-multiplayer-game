/*
    where a box can be on the smoke break map, the round start and the decoy brain both ask this
    the map is 5 rectangles (3 rooms, 2 hallways) so a spot is fine if a box fits inside them
*/

use rand::Rng;

use crate::maze::arena::ARENA_AREAS;
use crate::server::game::MAZE_CELL_SIZE;

// how far a box keeps from the walls
pub const BODY: f32 = 0.55;


// world rect (x0, z0, x1, z1) of an arena area in metres
fn area_rect(i: usize) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = ARENA_AREAS[i];
    let c = MAZE_CELL_SIZE;
    (x as f32 * c, y as f32 * c, (x + w) as f32 * c, (y + h) as f32 * c)
}

// which area a point is in, if any
pub fn area_of(x: f32, z: f32) -> Option<usize> {
    (0..ARENA_AREAS.len()).find(|&i| {
        let (x0, z0, x1, z1) = area_rect(i);
        x >= x0 && x <= x1 && z >= z0 && z <= z1
    })
}


// a box fits at (x, z) if its middle and all four corners are on floor
pub fn walkable(x: f32, z: f32) -> bool {
    [(0.0, 0.0), (BODY, BODY), (BODY, -BODY), (-BODY, BODY), (-BODY, -BODY)]
        .iter()
        .all(|(dx, dz)| area_of(x + dx, z + dz).is_some())
}



// a random spot a box fits in, bigger areas picked more often
pub fn random_spot(rng: &mut impl Rng) -> (f32, f32) {
    let sizes: Vec<f32> = (0..ARENA_AREAS.len())
        .map(|i| {
            let (x0, z0, x1, z1) = area_rect(i);
            (x1 - x0) * (z1 - z0)
        })
        .collect();
    let mut roll = rng.gen_range(0.0..sizes.iter().sum::<f32>());
    let mut pick = 0;
    for (i, size) in sizes.iter().enumerate() {
        if roll < *size {
            pick = i;
            break;
        }
        roll -= size;
    }
    let (x0, z0, x1, z1) = area_rect(pick);
    let m = BODY + 0.1;
    (rng.gen_range(x0 + m..x1 - m), rng.gen_range(z0 + m..z1 - m))
}



// waypoints from one spot to another, through every doorway in between, then the spot
pub fn route(from: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
    let (Some(a), Some(b)) = (area_of(from.0, from.1), area_of(to.0, to.1)) else {
        return vec![to];
    };
    // the doorway between area left and left + 1, in the middle of the hallway's side
    let door = |left: usize| {
        let (_, _, x1, _) = area_rect(left);
        let hall = if ARENA_AREAS[left].3 < ARENA_AREAS[left + 1].3 { left } else { left + 1 };
        let (_, hz0, _, hz1) = area_rect(hall);
        (x1, (hz0 + hz1) / 2.0)
    };
    let mut points = Vec::new();
    if a < b {
        for i in a..b {
            points.push(door(i));
        }
    } else {
        for i in (b..a).rev() {
            points.push(door(i));
        }
    }
    points.push(to);
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    // middle of room 1 and room 3, and a spot inside the wall block above the first hallway
    const ROOM_1: (f32, f32) = (9.0, 9.0);
    const ROOM_3: (f32, f32) = (52.2, 9.0);
    const IN_A_WALL: (f32, f32) = (19.8, 3.0);

    #[test]
    fn random_spots_always_fit_a_box() {
        let mut rng = rand::thread_rng();
        for _ in 0..500 {
            let (x, z) = random_spot(&mut rng);
            assert!(walkable(x, z), "({x}, {z}) doesn't fit a box");
        }
    }

    #[test]
    fn a_box_doesnt_fit_in_a_wall_or_half_in_one() {
        assert!(walkable(ROOM_1.0, ROOM_1.1));
        assert!(!walkable(IN_A_WALL.0, IN_A_WALL.1));
        // middle on floor but a corner through the room's outer wall (it starts at 1.8 m)
        assert!(!walkable(2.0, 9.0));
    }

    #[test]
    fn room_1_to_room_3_goes_through_all_4_doorways() {
        let r = route(ROOM_1, ROOM_3);
        assert_eq!(r.len(), 5, "4 doorways then the spot: {r:?}");
        assert_eq!(*r.last().unwrap(), ROOM_3);
        // every doorway is somewhere a box fits, and they go left to right
        for w in r.windows(2) {
            assert!(walkable(w[0].0, w[0].1), "doorway {:?} doesn't fit a box", w[0]);
            assert!(w[0].0 < w[1].0);
        }
    }

    #[test]
    fn room_3_to_room_1_walks_them_backwards() {
        let there = route(ROOM_1, ROOM_3);
        let back = route(ROOM_3, ROOM_1);
        let doors_there: Vec<_> = there[..4].to_vec();
        let mut doors_back: Vec<_> = back[..4].to_vec();
        doors_back.reverse();
        assert_eq!(doors_there, doors_back);
    }

    #[test]
    fn same_room_is_a_straight_line() {
        assert_eq!(route(ROOM_1, (12.0, 5.0)), vec![(12.0, 5.0)]);
    }
}
