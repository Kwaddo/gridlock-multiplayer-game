//! The maze minimap's cell colours as texels (one per cell), baked once per
//! maze change instead of one rectangle per cell per frame.

use macroquad::prelude::Color;

use crate::types::maze::CellKind;

pub(super) const WALL_COLOR: Color = Color::new(0.38, 0.40, 0.48, 1.0);
pub(super) const FLOOR_COLOR: Color = Color::new(0.13, 0.15, 0.20, 1.0);
pub(super) const ROOM_FLOOR_COLOR: Color = Color::new(0.16, 0.18, 0.24, 1.0);
pub(super) const BREAKABLE_COLOR: Color = Color::new(0.50, 0.38, 0.22, 1.0);

/// Wall texels around the maze. The texture clamps to its edge, so with this
/// border everything outside the maze reads as wall, as the old cell loop drew it.
pub(super) const TEX_PAD: i32 = 1;

pub(super) fn kind_color(kind: CellKind) -> Color {
    match kind {
        CellKind::RoomFloor => ROOM_FLOOR_COLOR,
        CellKind::Floor => FLOOR_COLOR,
        CellKind::BreakableWall => BREAKABLE_COLOR,
        CellKind::Wall => WALL_COLOR,
    }
}

/// RGBA8 texels of a `width` x `height` maze plus [`TEX_PAD`] on every side,
/// row by row from the top (cell row `-TEX_PAD`). Returns (width, height, bytes)
/// of the texture. Colours convert to bytes as vertex colours do, so a texel
/// is the exact colour the old per-cell rectangle had.
pub(super) fn bake_cell_texels(
    width: i32,
    height: i32,
    kind_at: impl Fn(i32, i32) -> CellKind,
) -> (u16, u16, Vec<u8>) {
    let tw = width.max(0) + 2 * TEX_PAD;
    let th = height.max(0) + 2 * TEX_PAD;
    let mut bytes = Vec::with_capacity((tw * th * 4) as usize);
    for cy in -TEX_PAD..th - TEX_PAD {
        for cx in -TEX_PAD..tw - TEX_PAD {
            let rgba: [u8; 4] = kind_color(kind_at(cx, cy)).into();
            bytes.extend_from_slice(&rgba);
        }
    }
    (tw as u16, th as u16, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texel(bytes: &[u8], tw: u16, x: i32, y: i32) -> [u8; 4] {
        let i = ((y + TEX_PAD) as usize * tw as usize + (x + TEX_PAD) as usize) * 4;
        [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
    }

    #[test]
    fn one_texel_per_cell_with_a_wall_border() {
        // 3 x 2 maze: row 0 floor, room, breakable; row 1 wall, floor, room
        let grid = [
            [CellKind::Floor, CellKind::RoomFloor, CellKind::BreakableWall],
            [CellKind::Wall, CellKind::Floor, CellKind::RoomFloor],
        ];
        let kind_at = |x: i32, y: i32| {
            if x < 0 || y < 0 || x >= 3 || y >= 2 {
                CellKind::Wall
            } else {
                grid[y as usize][x as usize]
            }
        };
        let (tw, th, bytes) = bake_cell_texels(3, 2, kind_at);
        assert_eq!((tw, th), (5, 4));
        assert_eq!(bytes.len(), 5 * 4 * 4);
        for y in -TEX_PAD..2 + TEX_PAD {
            for x in -TEX_PAD..3 + TEX_PAD {
                let expected: [u8; 4] = kind_color(kind_at(x, y)).into();
                assert_eq!(texel(&bytes, tw, x, y), expected, "cell ({x}, {y})");
            }
        }
        let wall: [u8; 4] = WALL_COLOR.into();
        assert_eq!(texel(&bytes, tw, -1, -1), wall);
        assert_eq!(texel(&bytes, tw, 3, 1), wall);
        let (breakable, room): ([u8; 4], [u8; 4]) = (BREAKABLE_COLOR.into(), ROOM_FLOOR_COLOR.into());
        assert_eq!(texel(&bytes, tw, 2, 0), breakable);
        assert_eq!(texel(&bytes, tw, 2, 1), room);
    }

    #[test]
    fn colours_are_opaque_and_distinct() {
        let kinds = [CellKind::Wall, CellKind::BreakableWall, CellKind::Floor, CellKind::RoomFloor];
        let rgba: Vec<[u8; 4]> = kinds.iter().map(|&k| kind_color(k).into()).collect();
        for (i, a) in rgba.iter().enumerate() {
            assert_eq!(a[3], 255);
            for b in &rgba[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn empty_maze_is_all_border() {
        let (tw, th, bytes) = bake_cell_texels(0, 0, |_, _| CellKind::Wall);
        assert_eq!((tw, th), (2, 2));
        assert_eq!(bytes.len(), 16);
    }
}
