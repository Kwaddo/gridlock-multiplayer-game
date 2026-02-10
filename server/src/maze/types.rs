#[derive(Clone, Copy, Debug)]
pub struct Room {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum PropKind {
    Crate = 1,
    Barrel = 2,
    SupplyCrate = 3,
    Lamp = 4,
    Desk = 5,
    Monitor = 6,
    OfficeChair = 7,
}

impl PropKind {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub kind: PropKind,
    pub x: u8,
    pub y: u8,
}
