use macroquad::prelude::*;

// Floating Damage Numbers
pub struct DamageNumber {
    pub amount: f32,
    pub is_headshot: bool,
    pub screen_x: f32,
    pub screen_y: f32,
    pub lifetime: f32, // starts at 1.0, counts down
}

// bullet tracer

pub struct BulletTracer {
    pub start: Vec3,
    pub end: Vec3,
    pub lifetime: f32,
    pub speed: f32,
    pub progress: f32, // 0..1 along the path
    pub tracer_length: f32, // length of the visible tracer behind the head
    pub tracer_color: [u8; 4], // RGBA color of the tracer
    pub tracer_thickness: f32, // thickness of the tracer line
}


