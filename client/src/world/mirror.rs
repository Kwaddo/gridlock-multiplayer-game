use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

#[derive(Debug, Clone, Copy)]
pub struct Mirror {
    pub center: Vec3,
    pub normal: Vec3,
    pub width: f32,
    pub height: f32,
}

pub const LOBBY_MIRROR: Mirror = Mirror {
    center: vec3(2.0, 1.35, -24.9),
    normal: vec3(0.0, 0.0, 1.0),
    width: 4.0,
    height: 2.5,
};

/// How many mirrors get a live reflection at once. Each one costs a full
/// extra draw of the world, so keep it low. The rest show a plain tinted panel.
pub const MAX_LIVE_MIRRORS: usize = 1;

/// Past this distance (m) a mirror shows the plain tinted panel instead of a
/// live reflection. Tune so the switch doesn't pop.
pub const MAX_REFLECT_DISTANCE: f32 = 30.0;

/// Mirrors covering less of the screen than this (in NDC units, the whole
/// screen is 4.0) show the plain panel. 0.004 ≈ 0.1% of the screen.
const MIN_SCREEN_AREA: f32 = 0.004;

/// The mirrors that belong to a world when you enter it.
/// Generated maps will add their own on top later.
pub fn mirrors_for_world(world: crate::types::world::WorldId) -> Vec<Mirror> {
    match world {
        crate::types::world::WorldId::Lobby => vec![LOBBY_MIRROR],
        _ => Vec::new(),
    }
}

/// Distance to the closest mirror a shot hits, if any.
pub fn nearest_mirror_hit(mirrors: &[Mirror], eye: Vec3, dir: Vec3, range: f32) -> Option<f32> {
    mirrors
        .iter()
        .filter_map(|m| {
            let (center, size) = m.collider();
            crate::weapons::combat
                ::ray_hit_aabb_with_normal(eye, dir, center, size * 0.5, range)
                .map(|(d, _)| d)
        })
        .reduce(f32::min)
}

/// Which mirrors get a live reflection this frame: the ones that look
/// biggest on screen, up to MAX_LIVE_MIRRORS. Returns indexes into `mirrors`.
pub fn pick_live_mirrors(mirrors: &[Mirror], eye: Vec3, view_proj: Mat4) -> Vec<usize> {
    let mut scored: Vec<(usize, f32)> = mirrors
        .iter()
        .enumerate()
        .filter_map(|(i, m)| m.screen_score(eye, view_proj).map(|s| (i, s)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(MAX_LIVE_MIRRORS)
        .map(|(i, _)| i)
        .collect()
}

// collision things
const EDGE: f32 = 0.03;
const BACKING_Z: f32 = -0.04;
const BACKING_DEPTH: f32 = 0.06;

/// actual mirror look magic woah, silvered glass reflects ~90% with a faint cool tint. (idk how this works)
const GLASS_TINT: [f32; 4] = [0.88, 0.91, 0.94, 1.0];

const GLASS_VERTEX: &str =
    r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
}"#;

const GLASS_FRAGMENT: &str =
    r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform sampler2D Reflection;
uniform vec2 ScreenSize;
uniform vec4 Tint;

void main() {
    // This pixel's spot on screen is the same spot in the mirror texture.
    vec2 uv = gl_FragCoord.xy / ScreenSize;
    gl_FragColor = vec4(texture2D(Reflection, uv).rgb * Tint.rgb, 1.0);
}"#;

thread_local! {
    static GLASS_MATERIAL: Material = load_material(
        ShaderSource::Glsl { vertex: GLASS_VERTEX, fragment: GLASS_FRAGMENT },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("ScreenSize", UniformType::Float2),
                UniformDesc::new("Tint", UniformType::Float4)
            ],
            textures: vec!["Reflection".to_string()],
            pipeline_params: PipelineParams {
                depth_write: true,
                depth_test: Comparison::LessOrEqual,
                ..Default::default()
            },
        }
    ).expect("mirror glass shader failed to compile");
}

impl Mirror {
    /// distance from mirror plane, positive if in front of mirror, negative if behind
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p - self.center)
    }

    /// reflect a point across the mirror plane
    pub fn reflect_point(&self, p: Vec3) -> Vec3 {
        p - 2.0 * self.signed_distance(p) * self.normal
    }

    /// the matrix that flips the whole world across the mirror plane.
    /// the mirror camera is built from this later.

    pub fn reflection_matrix(&self) -> Mat4 {
        let n = self.normal;
        let d = -n.dot(self.center);

        //
        Mat4::from_cols(
            vec4(1.0 - 2.0 * n.x * n.x, -2.0 * n.x * n.y, -2.0 * n.x * n.z, 0.0),
            vec4(-2.0 * n.y * n.x, 1.0 - 2.0 * n.y * n.y, -2.0 * n.y * n.z, 0.0),
            vec4(-2.0 * n.z * n.x, -2.0 * n.z * n.y, 1.0 - 2.0 * n.z * n.z, 0.0),
            vec4(-2.0 * d * n.x, -2.0 * d * n.y, -2.0 * d * n.z, 1.0)
        )
    }

    /// direction along the glass to the viewer's right (when facing the mirror). so we can guard against seeing the back of the mirror.
    pub fn right(&self) -> Vec3 {
        vec3(0.0, 1.0, 0.0).cross(self.normal).normalize()
    }

    /// maps the mirror's local space (x along the glass, y up, z out of the
    /// glass, origin at the glass centre) into the world.
    fn local_to_world(&self) -> Mat4 {
        Mat4::from_cols(
            self.right().extend(0.0),
            vec4(0.0, 1.0, 0.0, 0.0),
            self.normal.extend(0.0),
            self.center.extend(1.0)
        )
    }

    // draws the physical mirror, no reflection yet
    pub fn draw_body(&self, reflection: Option<&Texture2D>) {
        let (w, h) = (self.width, self.height);
        let silver = Color::new(0.78, 0.8, 0.82, 1.0);
        let backing = Color::new(0.12, 0.12, 0.14, 1.0);
        let glass = Color::new(0.66, 0.72, 0.76, 1.0); // placeholder tint

        let gl = unsafe { get_internal_gl() };
        gl.quad_gl.push_model_matrix(self.local_to_world());

        // Backing panel
        draw_cube(vec3(0.0, 0.0, -0.04), vec3(w + 2.0 * EDGE, h + 2.0 * EDGE, 0.06), None, backing);
        // Glass, a hair in front of the backing.
        match reflection {
            // Live reflection: the glass shows the mirror texture.
            Some(tex) =>
                GLASS_MATERIAL.with(|material| {
                    material.set_texture("Reflection", tex.clone());
                    material.set_uniform("ScreenSize", vec2(screen_width(), screen_height()));
                    material.set_uniform("Tint", GLASS_TINT);
                    gl_use_material(material);
                    draw_cube(vec3(0.0, 0.0, 0.0), vec3(w, h, 0.005), None, WHITE);
                    gl_use_default_material();
                }),
            // No reflection this frame (behind it, too far later): plain tinted panel.
            None => draw_cube(vec3(0.0, 0.0, 0.0), vec3(w, h, 0.005), None, glass),
        }
        // Edges: top, bottom, left, right.
        let y_edge = h / 2.0 + EDGE / 2.0;
        let x_edge = w / 2.0 + EDGE / 2.0;
        draw_cube(vec3(0.0, y_edge, 0.0), vec3(w + 2.0 * EDGE, EDGE, EDGE), None, silver);
        draw_cube(vec3(0.0, -y_edge, 0.0), vec3(w + 2.0 * EDGE, EDGE, EDGE), None, silver);
        draw_cube(vec3(-x_edge, 0.0, 0.0), vec3(EDGE, h, EDGE), None, silver);
        draw_cube(vec3(x_edge, 0.0, 0.0), vec3(EDGE, h, EDGE), None, silver);

        gl.quad_gl.pop_model_matrix();
    }

    pub fn collider(&self) -> (Vec3, Vec3) {
        let hx = self.width / 2.0 + EDGE;
        let hy = self.height / 2.0 + EDGE;
        let z_back = BACKING_Z - BACKING_DEPTH / 2.0;
        let z_front = EDGE / 2.0;
        let m = self.local_to_world();
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for x in [-hx, hx] {
            for y in [-hy, hy] {
                for z in [z_back, z_front] {
                    let p = m.transform_point3(vec3(x, y, z));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
        ((min + max) / 2.0, max - min) 
    }

    /// Roughly how much of the screen this mirror covers (bigger = more
    /// worth reflecting). None if you're behind it or it's off screen.
    pub fn screen_score(&self, eye: Vec3, view_proj: Mat4) -> Option<f32> {
        let dist = self.signed_distance(eye);
        if dist < 0.05 {
            return None; // behind the glass (or inside it)
        }
        if (eye - self.center).length() > MAX_REFLECT_DISTANCE {
            return None; // too far to be worth a reflection: plain panel
        }
        // project the 4 glass corners onto the screen
        let (hx, hy) = (self.width / 2.0, self.height / 2.0);
        let m = self.local_to_world();
        let corners = [(-hx, -hy), (hx, -hy), (-hx, hy), (hx, hy)]
            .map(|(x, y)| view_proj * m.transform_point3(vec3(x, y, 0.0)).extend(1.0));
        let in_front = corners.iter().filter(|c| c.w > 0.0).count();
        if in_front == 0 {
            return None; // the whole mirror is behind you
        }
        // every corner in front: off screen if they're all past the same edge.
        // (if some corners are behind you, you're right up against it: keep it.)
        if in_front == 4 {
            let ndc = corners.map(|c| c.truncate().truncate() / c.w);
            let all = |f: fn(Vec2) -> bool| ndc.iter().all(|&p| f(p));
            if all(|p| p.x < -1.0) || all(|p| p.x > 1.0) || all(|p| p.y < -1.0) || all(|p| p.y > 1.0) {
                return None;
            }
            // tiny on screen (a sliver at a steep angle): plain panel
            let (lo, hi) = ndc.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), &p| (lo.min(p), hi.max(p)));
            let size = (hi - lo).clamp(Vec2::ZERO, Vec2::splat(2.0));
            if size.x * size.y < MIN_SCREEN_AREA {
                return None;
            }
        }
        // size / distance², facing-weighted: a big, close, head-on mirror wins
        let to_eye = (eye - self.center).normalize();
        let facing = self.normal.dot(to_eye).max(0.0);
        let d2 = (eye - self.center).length_squared().max(0.25);
        Some((self.width * self.height * facing) / d2)
    }

    ///
    /// the camera looking at the world flipped across this mirror
    pub fn camera_matrix(&self, main: &Camera3D) -> Mat4 {
        let aspect = main.aspect.unwrap_or(screen_width() / screen_height());
        let proj = Mat4::perspective_rh_gl(main.fovy, aspect, main.z_near, main.z_far);
        let view = Mat4::look_at_rh(main.position, main.target, main.up);
        self.clipped_view_proj(proj, view)
    }

    /// The plain mirrored camera without clipping; tests compare against it.
    #[cfg(test)]
    fn mirrored_view_proj(&self, proj: Mat4, view: Mat4) -> Mat4 {
        proj * view * self.reflection_matrix()
    }

    /// Like mirrored_view_proj, but anything behind the glass is clipped away.
    /// Without this, a wall behind a mirror gets flipped to the front and covers
    /// the reflection. Works by tilting the near plane onto the mirror plane
    /// (Lengyel's oblique near plane). Only depth changes; x/y stay the same.
    fn clipped_view_proj(&self, proj: Mat4, view: Mat4) -> Mat4 {
        let view = view * self.reflection_matrix();
        // the mirror plane (keep side positive), nudged 1 cm back so the
        // glass's own edge isn't shaved off
        let plane_world = self.normal.extend(-self.normal.dot(self.center) + 0.01);
        // the same plane, in the camera's view space
        let c = view.inverse().transpose() * plane_world;
        // Lengyel: replace the projection's z row so the near plane becomes `c`
        let q = proj.inverse() * vec4(c.x.signum(), c.y.signum(), 1.0, 1.0);
        let c = c * (2.0 / c.dot(q));
        let mut p = proj;
        for col in 0..4 {
            let w = p.col(col)[3];
            p.col_mut(col)[2] = c[col] - w;
        }
        p * view
    }
}

/// a camera with a ready-made matrix, so set_camera() can draw the mirrored world, optionally into an off-screen render target so that rendering can be captured for further processing or display.
pub struct MirrorCamera {
    pub matrix: Mat4,
    pub render_target: Option<RenderTarget>,
}

impl Camera for MirrorCamera {
    fn matrix(&self) -> Mat4 {
        self.matrix
    }

    fn depth_enabled(&self) -> bool {
        true
    }

    fn render_pass(&self) -> Option<RenderPass> {
        self.render_target.as_ref().map(|rt| rt.render_pass.clone())
    }

    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_matches_point_reflection() {
        let m = LOBBY_MIRROR;
        let r = m.reflection_matrix();
        for p in [vec3(0.0, 1.2, -20.0), vec3(5.0, 0.0, 3.0), vec3(-3.0, 2.0, -24.9)] {
            let a = m.reflect_point(p);
            let b = r.transform_point3(p);
            assert!((a - b).length() < 1e-4, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn reflecting_twice_returns_the_point() {
        let m = LOBBY_MIRROR;
        let p = vec3(1.0, 1.5, -18.0);
        assert!((m.reflect_point(m.reflect_point(p)) - p).length() < 1e-4);
    }

    #[test]
    fn front_is_positive_behind_is_negative() {
        let m = LOBBY_MIRROR;
        assert!(m.signed_distance(vec3(2.0, 1.2, -20.0)) > 0.0);
        assert!(m.signed_distance(vec3(2.0, 1.2, -26.0)) < 0.0);
    }

    #[test]
    fn glass_lands_on_the_same_pixel_for_both_cameras() {
        let m = LOBBY_MIRROR;
        // A player standing in front of the mirror, looking at it.
        let eye = vec3(1.0, 1.2, -20.0);
        let proj = Mat4::perspective_rh_gl(1.2, 16.0 / 9.0, 0.01, 100.0);
        let view = Mat4::look_at_rh(eye, vec3(2.0, 1.2, -24.9), vec3(0.0, 1.0, 0.0));
        let main = proj * view;
        let mirror = m.mirrored_view_proj(proj, view);

        // A point on the glass: identical screen position in both cameras.
        let on_glass = vec3(3.0, 0.5, -24.9);
        let a = main.project_point3(on_glass);
        let b = mirror.project_point3(on_glass);
        assert!((a.truncate() - b.truncate()).length() < 1e-4, "{a:?} vs {b:?}");

        // Something in front of the mirror shows up where its reflection would be.
        let in_front = vec3(1.0, 1.0, -22.0);
        let c = mirror.project_point3(in_front);
        let d = main.project_point3(m.reflect_point(in_front));
        assert!((c - d).length() < 1e-4, "{c:?} vs {d:?}");
    }

        #[test]
    fn collider_wraps_the_mirror_body() {
        let (c, s) = LOBBY_MIRROR.collider();
        assert!((c - vec3(2.0, 1.35, -24.9275)).length() < 1e-3, "{c:?}");
        assert!((s - vec3(4.06, 2.56, 0.085)).length() < 1e-3, "{s:?}");
    }

    fn view_proj(eye: Vec3, target: Vec3) -> Mat4 {
        Mat4::perspective_rh_gl(1.2, 16.0 / 9.0, 0.01, 100.0) *
            Mat4::look_at_rh(eye, target, vec3(0.0, 1.0, 0.0))
    }

    #[test]
    fn the_mirror_you_look_at_goes_live() {
        // a second mirror off to the side, facing the same way
        let side = Mirror { center: vec3(12.0, 1.35, -24.9), ..LOBBY_MIRROR };
        let mirrors = [side, LOBBY_MIRROR];
        let eye = vec3(2.0, 1.2, -20.0);
        let live = pick_live_mirrors(&mirrors, eye, view_proj(eye, LOBBY_MIRROR.center));
        assert_eq!(live, vec![1]);
    }

    #[test]
    fn no_reflection_when_behind_or_looking_away() {
        let mirrors = [LOBBY_MIRROR];
        // behind the glass
        let behind = vec3(2.0, 1.2, -26.0);
        assert!(pick_live_mirrors(&mirrors, behind, view_proj(behind, LOBBY_MIRROR.center)).is_empty());
        // in front, but facing away from it
        let eye = vec3(2.0, 1.2, -20.0);
        assert!(pick_live_mirrors(&mirrors, eye, view_proj(eye, vec3(2.0, 1.2, -10.0))).is_empty());
    }

    /// true if the point survives clipping (inside -w..w on every axis)
    fn visible(m: Mat4, p: Vec3) -> bool {
        let c = m * p.extend(1.0);
        c.w > 0.0 && c.x.abs() <= c.w && c.y.abs() <= c.w && c.z.abs() <= c.w
    }

    #[test]
    fn things_behind_the_glass_are_clipped() {
        let m = LOBBY_MIRROR;
        let eye = vec3(2.0, 1.2, -20.0);
        let proj = Mat4::perspective_rh_gl(1.2, 16.0 / 9.0, 0.01, 100.0);
        let view = Mat4::look_at_rh(eye, m.center, vec3(0.0, 1.0, 0.0));
        let clipped = m.clipped_view_proj(proj, view);
        // in front of the mirror: kept
        assert!(visible(clipped, vec3(2.0, 1.2, -22.0)));
        // behind the mirror (it would otherwise appear between you and the glass): gone
        assert!(!visible(clipped, vec3(2.0, 1.2, -26.0)));
        assert!(visible(m.mirrored_view_proj(proj, view), vec3(2.0, 1.2, -26.0)), "unclipped sees it");
        // the screen position doesn't move, only depth changes
        let a = m.mirrored_view_proj(proj, view).project_point3(vec3(1.0, 1.0, -22.0));
        let b = clipped.project_point3(vec3(1.0, 1.0, -22.0));
        assert!((a.truncate() - b.truncate()).length() < 1e-4, "{a:?} vs {b:?}");
    }

    #[test]
    fn far_or_tiny_mirrors_get_no_reflection() {
        let mirrors = [LOBBY_MIRROR];
        // 40 m away, looking straight at it
        let far = vec3(2.0, 1.2, 15.0);
        assert!(pick_live_mirrors(&mirrors, far, view_proj(far, LOBBY_MIRROR.center)).is_empty());
        // almost edge-on: a sliver
        let edge_on = vec3(25.0, 1.35, -24.8);
        assert!(pick_live_mirrors(&mirrors, edge_on, view_proj(edge_on, LOBBY_MIRROR.center)).is_empty());
    }

    #[test]
    fn a_shot_at_the_glass_hits_the_mirror() {
        let eye = vec3(2.0, 1.2, -20.0);
        let d = nearest_mirror_hit(&[LOBBY_MIRROR], eye, vec3(0.0, 0.0, -1.0), 100.0);
        assert!(d.is_some_and(|d| (d - 4.885).abs() < 1e-3), "{d:?}");
        assert!(nearest_mirror_hit(&[LOBBY_MIRROR], eye, vec3(0.0, 0.0, 1.0), 100.0).is_none());
    }
}
