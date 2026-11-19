/*
    The cardboard box
    built once out of little cuboids (like the maze props) and drawn wherever a box is
    wide shape with grime, same as the preview we picked
*/

use macroquad::prelude::*;

// outside size in metres, the wide side runs along x
pub const BOX_W: f32 = 1.05;
pub const BOX_D: f32 = 0.75;
pub const BOX_H: f32 = 0.72;

// how bright each face is, so the box reads as 3D without lighting
// order: +x, -x, top, bottom, +z, -z
const FACE_SHADE: [f32; 6] = [1.0, 0.78, 0.92, 0.7, 0.86, 0.64];

thread_local! {
    // built the first time a box gets drawn, then reused forever
    static BOX_MESH: Mesh = build_box();
    static SHADOW: Texture2D = shadow_texture();
}

// black in the middle, fading to nothing at the edges
fn shadow_texture() -> Texture2D {
    const N: u16 = 64;
    let mut img = Image::gen_image_color(N, N, Color::new(0.0, 0.0, 0.0, 0.0));
    for y in 0..N as u32 {
        for x in 0..N as u32 {
            let d = vec2(x as f32 + 0.5, y as f32 + 0.5) / (N as f32 / 2.0) - Vec2::ONE;
            let a = ((1.0 - d.length()) / 0.6).clamp(0.0, 1.0);
            img.set_pixel(x, y, Color::new(0.0, 0.0, 0.0, 0.75 * a));
        }
    }
    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Linear);
    tex
}

/// Draw one box with its bottom at `position`, turned to `yaw`, nose tipped up/down by `rock`.
pub fn draw_cardboard_box(position: Vec3, yaw: f32, rock: f32) {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(
        Mat4::from_translation(position) * Mat4::from_rotation_y(-yaw) * Mat4::from_rotation_z(rock),
    );
    BOX_MESH.with(draw_mesh);
    gl.quad_gl.pop_model_matrix();
}

/// Soft dark blob on the floor under a box, fainter the higher the box is.
pub fn draw_box_shadow(floor: Vec3, yaw: f32, height: f32) {
    let k = (height / 0.15).clamp(0.0, 1.0);
    let alpha = 1.0 - k * 0.45;
    let grow = 1.0 + k * 0.1;
    let (hw, hd) = (BOX_W * 0.65 * grow, BOX_D * 0.65 * grow);
    let (s, c) = (-yaw).sin_cos();
    let corner = |x: f32, z: f32| floor + vec3(x * c + z * s, 0.005, -x * s + z * c);
    let tint = Color::new(1.0, 1.0, 1.0, alpha);
    let mesh = Mesh {
        vertices: vec![
            Vertex::new2(corner(-hw, -hd), vec2(0.0, 0.0), tint),
            Vertex::new2(corner(hw, -hd), vec2(1.0, 0.0), tint),
            Vertex::new2(corner(hw, hd), vec2(1.0, 1.0), tint),
            Vertex::new2(corner(-hw, hd), vec2(0.0, 1.0), tint),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(SHADOW.with(|t| t.clone())),
    };
    draw_mesh(&mesh);
}

// 0xRRGGBB, times `mul` (1.0 = as is, lower = darker)
fn tone(hex: u32, mul: f32) -> Color {
    let ch = |shift: u32| ((hex >> shift) & 0xff) as f32 / 255.0 * mul;
    Color::new(ch(16), ch(8), ch(0), 1.0)
}

struct Builder {
    mesh: Mesh,
}

impl Builder {
    // one cuboid: 6 faces, each shaded a bit differently
    fn cuboid(&mut self, size: Vec3, at: Vec3, color: Color) {
        let h = size / 2.0;
        let faces = [
            [vec3(h.x, -h.y, -h.z), vec3(h.x, h.y, -h.z), vec3(h.x, h.y, h.z), vec3(h.x, -h.y, h.z)],
            [vec3(-h.x, -h.y, h.z), vec3(-h.x, h.y, h.z), vec3(-h.x, h.y, -h.z), vec3(-h.x, -h.y, -h.z)],
            [vec3(-h.x, h.y, -h.z), vec3(-h.x, h.y, h.z), vec3(h.x, h.y, h.z), vec3(h.x, h.y, -h.z)],
            [vec3(-h.x, -h.y, h.z), vec3(-h.x, -h.y, -h.z), vec3(h.x, -h.y, -h.z), vec3(h.x, -h.y, h.z)],
            [vec3(-h.x, -h.y, h.z), vec3(h.x, -h.y, h.z), vec3(h.x, h.y, h.z), vec3(-h.x, h.y, h.z)],
            [vec3(h.x, -h.y, -h.z), vec3(-h.x, -h.y, -h.z), vec3(-h.x, h.y, -h.z), vec3(h.x, h.y, -h.z)],
        ];
        for (corners, shade) in faces.iter().zip(FACE_SHADE) {
            let c = Color::new(color.r * shade, color.g * shade, color.b * shade, 1.0);
            let base = self.mesh.vertices.len() as u16;
            for p in corners {
                self.mesh.vertices.push(Vertex::new2(at + *p, Vec2::ZERO, c));
            }
            self.mesh.indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        }
    }
}

// same tiny random generator as the preview, so the grime lands in the same spots
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0 * 16807 % 2147483647;
        self.0 as f32 / 2147483647.0
    }
}

fn build_box() -> Mesh {
    let mut b = Builder { mesh: Mesh { vertices: Vec::new(), indices: Vec::new(), texture: None } };
    let (w, d, h, t) = (BOX_W, BOX_D, BOX_H, 0.02);
    let card = tone(0xa47a4c, 1.0);
    let dark = tone(0x6e5032, 1.0);
    let tape = tone(0xbfae86, 1.0);
    let ink = tone(0x111111, 1.0);

    // the four walls
    b.cuboid(vec3(w, h, t), vec3(0.0, h / 2.0, d / 2.0), card);
    b.cuboid(vec3(w, h, t), vec3(0.0, h / 2.0, -d / 2.0), card);
    b.cuboid(vec3(t, h, d), vec3(w / 2.0, h / 2.0, 0.0), card);
    b.cuboid(vec3(t, h, d), vec3(-w / 2.0, h / 2.0, 0.0), card);

    // two top flaps with a seam between them, taped shut and over the sides
    b.cuboid(vec3(w + 0.01, 0.018, d / 2.0), vec3(0.0, h + 0.005, d / 4.0), card);
    b.cuboid(vec3(w + 0.01, 0.018, d / 2.0), vec3(0.0, h + 0.005, -d / 4.0), tone(0xa47a4c, 1.06));
    b.cuboid(vec3(w + 0.02, 0.02, 0.004), vec3(0.0, h + 0.012, 0.0), dark);
    b.cuboid(vec3(w + 0.03, 0.022, 0.15), vec3(0.0, h + 0.012, 0.0), tape);
    b.cuboid(vec3(0.006, 0.18, 0.15), vec3(w / 2.0 + 0.013, h - 0.09, 0.0), tape);
    b.cuboid(vec3(0.006, 0.18, 0.15), vec3(-w / 2.0 - 0.013, h - 0.09, 0.0), tape);

    // marker stripe on the front, hand holes on the sides
    b.cuboid(vec3(0.42, 0.045, 0.006), vec3(0.0, h * 0.76, d / 2.0 + 0.012), ink);
    for s in [1.0, -1.0] {
        b.cuboid(vec3(0.006, 0.055, 0.18), vec3(s * (w / 2.0 + 0.012), h * 0.68, 0.0), ink);
    }

    // darker corner edges
    for (x, z) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        b.cuboid(vec3(0.024, h + 0.01, 0.024), vec3(x * w / 2.0, h / 2.0, z * d / 2.0), dark);
    }

    add_grime(&mut b, card);
    add_labels(&mut b, card);
    b.mesh
}

// scuffs, water damage and worn corners on each wall
fn add_grime(b: &mut Builder, card: Color) {
    let (w, d, h) = (BOX_W, BOX_D, BOX_H);
    let e = 0.0125; // sit just outside the wall so nothing flickers
    let mut r = Rng(7);
    // (which way the wall faces, how wide it is, how far out it sits)
    let walls = [
        (vec3(0.0, 0.0, 1.0), w, d / 2.0),
        (vec3(0.0, 0.0, -1.0), w, d / 2.0),
        (vec3(1.0, 0.0, 0.0), d, w / 2.0),
        (vec3(-1.0, 0.0, 0.0), d, w / 2.0),
    ];
    for (n, wall_w, out) in walls {
        // a flat patch on this wall: `u` is sideways along it, `y` is height
        let put = |b: &mut Builder, u: f32, y: f32, pw: f32, ph: f32, color: Color| {
            let o = out + e;
            let (x, z) = if n.z != 0.0 { (u, n.z * o) } else { (n.x * o, u) };
            let size = if n.z != 0.0 { vec3(pw, ph, 0.004) } else { vec3(0.004, ph, pw) };
            b.cuboid(size, vec3(x, y, z), color);
        };
        // dirty band along the bottom
        put(b, 0.0, 0.035, wall_w - 0.03, 0.07, tone(0x5a4128, 1.0));
        // darker scuffs
        for _ in 0..4 {
            let (pw, ph) = (0.08 + r.next() * 0.18, 0.05 + r.next() * 0.16);
            let u = (r.next() - 0.5) * (wall_w - pw - 0.06);
            let y = 0.12 + r.next() * (h - 0.3);
            let mul = 0.72 + r.next() * 0.12;
            put(b, u, y, pw, ph, Color::new(card.r * mul, card.g * mul, card.b * mul, 1.0));
        }
        // drip streaks
        for _ in 0..3 {
            let u = (r.next() - 0.5) * (wall_w - 0.1);
            let y = h * (0.25 + r.next() * 0.6);
            put(b, u, y, 0.01, 0.06 + r.next() * 0.12, tone(0x7d5c3a, 1.0));
        }
        // two worn, lighter corners
        put(b, wall_w / 2.0 - 0.03, h - 0.06, 0.04, 0.1, tone(0xc49a6c, 1.0));
        put(b, -wall_w / 2.0 + 0.03, 0.12, 0.035, 0.08, tone(0xc49a6c, 1.0));
    }
}

// shipping label with a barcode on +x, "this way up" arrows and a glass icon on -x
fn add_labels(b: &mut Builder, card: Color) {
    let (w, d, h) = (BOX_W, BOX_D, BOX_H);
    let paper = tone(0xe8e4d8, 1.0);
    let ink = tone(0x151515, 1.0);
    let grey = tone(0x777777, 1.0);
    let print = tone(0x2a2a2a, 1.0);

    let (fx, lz) = (w / 2.0 + 0.016, -d * 0.08);
    b.cuboid(vec3(0.004, 0.17, 0.26), vec3(fx, h * 0.55, lz), paper);
    for i in 0..14 {
        let bar = [0.004, 0.008, 0.006][i % 3];
        b.cuboid(vec3(0.004, 0.06, bar), vec3(fx + 0.002, h * 0.55 - 0.045, lz - 0.1 + i as f32 * 0.015), ink);
    }
    for i in 0..3 {
        let i = i as f32;
        b.cuboid(
            vec3(0.004, 0.012, 0.14 - i * 0.03),
            vec3(fx + 0.002, h * 0.55 + 0.055 - i * 0.025, lz - 0.05 + i * 0.015),
            grey,
        );
    }
    b.cuboid(vec3(0.004, 0.035, 0.035), vec3(fx + 0.002, h * 0.55 + 0.05, lz + 0.095), tone(0xd04a2a, 1.0));

    let ix = -w / 2.0 - 0.016;
    for z in [-0.12, 0.0] {
        b.cuboid(vec3(0.004, 0.12, 0.004), vec3(ix, h * 0.7, z - 0.02), print);
        b.cuboid(vec3(0.004, 0.12, 0.004), vec3(ix, h * 0.7, z + 0.02), print);
        b.cuboid(vec3(0.004, 0.02, 0.07), vec3(ix, h * 0.78, z), print);
    }
    b.cuboid(vec3(0.004, 0.09, 0.09), vec3(ix, h * 0.42, 0.13), print);
    b.cuboid(
        vec3(0.006, 0.075, 0.075),
        vec3(ix - 0.002, h * 0.42, 0.13),
        Color::new(card.r * 0.9, card.g * 0.9, card.b * 0.9, 1.0),
    );
    b.cuboid(vec3(0.008, 0.06, 0.012), vec3(ix - 0.003, h * 0.42, 0.13), print);

    // packing slip on the lid
    b.cuboid(vec3(0.12, 0.006, 0.08), vec3(w * 0.25, h + 0.03, -d * 0.28), paper);
}

#[cfg(test)]
mod tests {
    use super::*;

    // macroquad draws at most 5,000 indices in one go, more than that and the box gets cut
    #[test]
    fn box_fits_in_one_draw() {
        let m = build_box();
        assert!(m.indices.len() <= 5000, "{} indices", m.indices.len());
    }
}
