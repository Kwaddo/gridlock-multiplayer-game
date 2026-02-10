#[derive(Clone, Copy, Debug)]
pub struct Room {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum DecorationKind {
    Table = 1,
    Chair = 2,
}

impl DecorationKind {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Decoration {
    pub kind: DecorationKind,
    pub x: u8,
    pub y: u8,
}
