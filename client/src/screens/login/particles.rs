use macroquad::rand::gen_range;

pub(super) struct MenuParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub alpha: f32,
}

impl MenuParticle {
    pub fn random(w: f32, h: f32) -> Self {
        Self {
            x: gen_range(0.0, w),
            y: gen_range(0.0, h),
            vx: gen_range(-12.0, 12.0),
            vy: gen_range(-8.0, 8.0),
            size: gen_range(0.5, 2.5),
            alpha: gen_range(0.08, 0.30),
        }
    }
}
