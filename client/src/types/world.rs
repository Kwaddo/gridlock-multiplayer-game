#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldId {
    Lobby = 0,
    Maze = 1,
}

impl WorldId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(WorldId::Lobby),
            1 => Some(WorldId::Maze),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}