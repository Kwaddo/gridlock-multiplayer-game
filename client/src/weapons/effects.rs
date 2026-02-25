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
    pub progress: f32,         // 0..1 along the path
    pub tracer_length: f32,    // length of the visible tracer behind the head
    pub tracer_color: [u8; 4], // RGBA color of the tracer
    pub tracer_thickness: f32, // thickness of the tracer line
}

// 3rd-person muzzle flash (shown at another player/bot's gun position)
pub struct WorldMuzzleFlash {
    pub position: Vec3,
    pub lifetime: f32, // starts at DURATION, counts down
}

impl WorldMuzzleFlash {
    pub const DURATION: f32 = 0.07;
}

// bullet hole / impact mark left on walls/surfaces
// A global scale factor applied when the hole is spawned; tweaking this value
// shrinks or enlarges every bullet decal without requiring changes to the
// weapon stat tables. Smaller than 1.0 will make holes tinier on the walls.
pub const BULLET_HOLE_SCALE: f32 = 0.1;

pub struct BulletHole {
    pub position: Vec3,              // world position of the impact
    pub normal: Vec3,                // surface normal at impact (for orienting the decal)
    pub lifetime: f32,               // remaining time before fading out (seconds)
    pub max_lifetime: f32,           // initial lifetime (for fade calculation)
    pub size: f32,                   // radius of the bullet hole decal
    pub color: [u8; 4],              // RGBA tint of the hole (weapon-dependent)
    pub wall_cell: Option<(u8, u8)>, // if on a breakable wall, the maze cell coords
}
