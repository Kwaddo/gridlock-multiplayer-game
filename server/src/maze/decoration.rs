use crate::maze::types::{Decoration, DecorationKind, Room};

pub fn decorate_first_room(rooms: &[Room]) -> Vec<Decoration> {
    let mut decorations = Vec::new();
    if let Some(room) = rooms.first() {
        let table_x_u8 = room.x + (room.w / 2);
        let table_y_u8 = room.y + (room.h / 2);
        decorations.push(Decoration {
            kind: DecorationKind::Table,
            x: table_x_u8,
            y: table_y_u8,
        });

        let chair_positions = [
            (table_x_u8.wrapping_sub(1), table_y_u8),
            (table_x_u8 + 1, table_y_u8),
            (table_x_u8, table_y_u8.wrapping_sub(1)),
            (table_x_u8, table_y_u8 + 1),
        ];

        for (x, y) in chair_positions {
            decorations.push(Decoration {
                kind: DecorationKind::Chair,
                x,
                y,
            });
        }
    }

    decorations
}
