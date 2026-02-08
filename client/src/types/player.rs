use macroquad::prelude::*;

pub struct OtherPlayer {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub username: String,
    pub skin: Option<Texture2D>,
    pub color: Color,
}

impl OtherPlayer {
    pub fn color_for_id(id: u32) -> Color {
        let r = ((id * 73 + 29) % 256) as u8;
        let g = ((id * 97 + 53) % 256) as u8;
        let b = ((id * 61 + 19) % 256) as u8;
        Color::from_rgba(r, g, b, 255)
    }
}
