use glam::Vec3;

pub type Pos3 = (f32,f32,f32);

#[inline] // inline means function is expanded at call site, improving performance for small functions
pub fn to_vec3(pos: Pos3) -> Vec3 {
    Vec3::new(pos.0, pos.1, pos.2)
}

#[inline]
pub fn from_vec3(vec: Vec3) -> Pos3 {
    (vec.x, vec.y, vec.z)
}

