use macroquad::prelude::*;
use macroquad::rand::gen_range;

use std::collections::HashMap;

use crate::core::GameConfig;
use crate::net::protocols::{CardboardBox, BOX_ENTER_RANGE};
use crate::physics::collision::PLAYER_RADIUS;
use crate::types::animation_state::MoveDir;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::world::lobby::ray_test_lobby;
use crate::world::render::cardboard_box::{BoxLook, BOX_D, BOX_H, BOX_W};

use super::PlayingState;

pub const BOX_SPEED: f32 = 1.8;
const CAM_DIST: f32 = 2.6;
const CAM_PIVOT: f32 = 1.1;
// the third-person eye stays this far above your feet and this far under the maze wall tops
const CAM_FLOOR: f32 = 0.3;
const CAM_UNDER_TOP: f32 = 0.15;
const SLIT_EYE: f32 = 0.5;
const LIFT: f32 = 0.06;
const TWEEN_TIME: f32 = 0.25;
const TWEEN_HEIGHT: f32 = 0.6;
// give up waiting for the server after this
const TWEEN_TIMEOUT: f32 = 1.0;
const BURST_PIECES: usize = 18;
const BURST_LIFE: f32 = 1.5;
// a box we broke stays broken even if an older list still has it
const BROKEN_MEMORY: f64 = 1.0;

pub struct BoxBurst {
    pieces: Vec<BurstPiece>,
    age: f32,
}

struct BurstPiece {
    pos: Vec3,
    vel: Vec3,
    size: Vec3,
    color: Color,
    floor: f32,
}

/// Everything boxes need kept between frames.
#[derive(Default)]
pub struct BoxState {
    // the server's last list, and which world it's for
    pub boxes: Vec<CardboardBox>,
    pub world: WorldId,
    pub in_box: Option<u16>,
    pub pending_enter: Option<u16>,
    pub pending_exit: bool,
    pub tween: Option<BoxTween>,
    pub feel: HashMap<u16, BoxFeel>,
    pub cam_blend: f32,
    pub bursts: Vec<BoxBurst>,
    pub broken: Vec<(u16, f64)>,
    pub pending_shots: Vec<u16>,
    pub look: BoxLook,
    // the box we just got out of, not solid until we've walked off it
    pub ghost: Option<u16>,
}

/// Where every box we could shoot is right now: (id, bottom centre, yaw).
pub type BoxTargets = Vec<(u16, Vec3, f32)>;

/// The closest box a ray hits, if any.
pub fn ray_test_boxes(targets: &BoxTargets, eye: Vec3, dir: Vec3, max_dist: f32) -> Option<(u16, f32)> {
    let half = vec3(BOX_W / 2.0, BOX_H / 2.0, BOX_D / 2.0);
    targets
        .iter()
        .filter_map(|&(id, at, yaw)| {
            // into the box's own space, where it's a plain axis-aligned block
            let turn = Quat::from_rotation_y(yaw);
            let hit = crate::weapons::combat::ray_hit_aabb(turn * (eye - at), turn * dir, vec3(0.0, BOX_H / 2.0, 0.0), half, max_dist);
            hit.map(|d| (id, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// A box flying onto you (getting in) or hopping off you (getting out), shown before the server answers.
pub struct BoxTween {
    id: u16,
    from: Vec3,
    to: Vec3,
    entering: bool,
    age: f32,
}

/// Per box: how far it's lifted, and its own walk clock for the bob.
#[derive(Default)]
pub struct BoxFeel {
    lift: f32,
    clock: f32,
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

impl PlayingState {
    // the server decides, we find out from the next box list
    pub(super) fn process_box_key(&mut self) {
        if is_key_pressed(KeyCode::F6) {
            self.cardboard.look = self.cardboard.look.next();
            // name what gets drawn, a model that isn't loaded falls back to the code-built box
            let look = self.cardboard.look;
            let label = match look.model_key() {
                Some(k) if !self.models.contains_key(k) => format!("code-built ({} not loaded)", look.name()),
                _ => look.name().to_string(),
            };
            self.lobby_console_message = Some(format!("Box look: {label}"));
            self.lobby_console_message_timer = 2.0;
        }
        if !is_key_pressed(KeyCode::E) || self.cardboard.world != self.current_world {
            return;
        }
        if self.cardboard.tween.is_some() {
            return;
        }
        if let Some(id) = self.cardboard.in_box {
            self.cardboard.pending_exit = true;
            let here = self.position;
            self.cardboard.tween = Some(BoxTween { id, from: here, to: here, entering: false, age: 0.0 });
            return;
        }
        if let Some(b) = self.box_in_reach() {
            let (id, to) = (b.id, vec3(b.position.0, self.position.y, b.position.2));
            self.cardboard.pending_enter = Some(id);
            self.cardboard.tween = Some(BoxTween { id, from: self.position, to, entering: true, age: 0.0 });
        }
    }

    fn box_in_reach(&self) -> Option<&CardboardBox> {
        if self.cardboard.world != self.current_world {
            return None;
        }
        let flat_dist = |p: (f32, f32, f32)| vec2(p.0 - self.position.x, p.2 - self.position.z).length();
        self.cardboard.boxes
            .iter()
            .filter(|b| b.carrier == 0 && flat_dist(b.position) <= BOX_ENTER_RANGE)
            .min_by(|a, b| flat_dist(a.position).total_cmp(&flat_dist(b.position)))
    }

    pub(super) fn is_boxed(&self, player_id: u32) -> bool {
        self.cardboard.world == self.current_world && self.cardboard.boxes.iter().any(|b| b.carrier == player_id)
    }

    // a carried box sits on its carrier, the server's copy only catches up every few seconds
    pub(super) fn box_pose(&self, b: &CardboardBox) -> (Vec3, f32) {
        if let Some(tw) = self.cardboard.tween.as_ref().filter(|tw| tw.id == b.id && !tw.entering) {
            let t = ease_out((tw.age / TWEEN_TIME).min(1.0));
            let arc = (t * std::f32::consts::PI).sin() * TWEEN_HEIGHT * 0.5;
            return (tw.from + vec3(0.0, arc, 0.0), self.yaw);
        }
        if self.cardboard.in_box == Some(b.id) {
            return (self.position, self.yaw);
        }
        if b.carrier != 0 {
            if let Some(p) = self.other_players.iter().find(|p| p.id == b.carrier) {
                return (p.render_position(), p.render_yaw);
            }
        }
        (vec3(b.position.0, b.position.1, b.position.2), b.yaw)
    }

    /// `box_pose` plus lift, bob and roll, and the floor under it for the shadow.
    pub(super) fn box_draw(&self, b: &CardboardBox) -> (Vec3, f32, f32, Vec3) {
        let (at, yaw) = self.box_pose(b);
        let (lift, clock) = self.cardboard.feel.get(&b.id).map_or((0.0, 0.0), |f| (f.lift, f.clock));
        let k = lift / LIFT;
        let bob = (clock * 7.0).sin().abs() * 0.015 * k;
        let roll = (clock * 7.0).sin() * 0.02 * k;
        let floor = vec3(at.x, self.box_floor_y(b), at.z);
        (at + vec3(0.0, lift + bob, 0.0), yaw, roll, floor)
    }

    fn box_floor_y(&self, b: &CardboardBox) -> f32 {
        if self.cardboard.in_box == Some(b.id) {
            return self.position.y;
        }
        let carrier = self.other_players.iter().find(|p| b.carrier != 0 && p.id == b.carrier);
        carrier.map_or(b.position.1, |p| p.render_y)
    }

    fn box_moving(&self, b: &CardboardBox) -> bool {
        if self.cardboard.in_box == Some(b.id) {
            return vec2(self.movement_state.velocity.x, self.movement_state.velocity.z).length() > 0.3;
        }
        b.carrier != 0 && self.other_players.iter().any(|p| p.id == b.carrier && p.last_move_dir != MoveDir::None)
    }

    /// The server's new list: drop boxes we already broke, burst any that vanished, see if we're in one.
    pub fn take_box_list(&mut self, world: WorldId, mut boxes: Vec<CardboardBox>, player_id: u32) {
        let now = get_time();
        self.cardboard.broken.retain(|(_, at)| now - at < BROKEN_MEMORY);
        boxes.retain(|b| !self.cardboard.broken.iter().any(|(id, _)| *id == b.id));
        if world == self.cardboard.world {
            let gone: Vec<Vec3> = self
                .cardboard
                .boxes
                .iter()
                .filter(|old| !boxes.iter().any(|b| b.id == old.id))
                .map(|old| self.box_pose(old).0)
                .collect();
            for at in gone {
                self.spawn_box_burst(at);
            }
        }
        let was_in = self.cardboard.in_box;
        self.cardboard.in_box = boxes.iter().find(|b| b.carrier == player_id).map(|b| b.id);
        if self.cardboard.in_box.is_none() && was_in.is_some() {
            self.cardboard.ghost = was_in;
        }
        self.cardboard.world = world;
        self.cardboard.boxes = boxes;
    }

    pub(super) fn box_targets(&self) -> BoxTargets {
        if self.cardboard.world != self.current_world {
            return Vec::new();
        }
        self.cardboard.boxes
            .iter()
            .filter(|b| self.cardboard.in_box != Some(b.id))
            .map(|b| {
                let (at, yaw) = self.box_pose(b);
                (b.id, at, yaw)
            })
            .collect()
    }

    /// Every box is solid, empty or not, so bumping into one gives nothing away.
    pub(super) fn resolve_box_collision(&mut self) {
        let skip = self.cardboard.tween.as_ref().map(|tw| tw.id);
        let (hw, hd) = (BOX_W / 2.0 + PLAYER_RADIUS, BOX_D / 2.0 + PLAYER_RADIUS);
        let mut still_in_ghost = false;
        for (id, at, yaw) in self.box_targets() {
            if Some(id) == skip {
                continue;
            }
            let turn = Quat::from_rotation_y(yaw);
            let local = turn * (self.position - at);
            if local.x.abs() >= hw || local.z.abs() >= hd {
                continue;
            }
            // the box we just got out of, you walk off it freely
            if self.cardboard.ghost == Some(id) {
                still_in_ghost = true;
                continue;
            }
            let (px, pz) = (hw - local.x.abs(), hd - local.z.abs());
            let side = if px < pz { vec3(local.x.signum(), 0.0, 0.0) } else { vec3(0.0, 0.0, local.z.signum()) };
            let normal = turn.inverse() * side;
            self.position += normal * px.min(pz);
            let v = &mut self.movement_state.velocity;
            let into = v.x * normal.x + v.z * normal.z;
            if into < 0.0 {
                v.x -= into * normal.x;
                v.z -= into * normal.z;
            }
        }
        if !still_in_ghost {
            self.cardboard.ghost = None;
        }
    }

    /// Break it here right away and tell the server.
    pub(super) fn break_box(&mut self, id: u16) {
        let Some(i) = self.cardboard.boxes.iter().position(|b| b.id == id) else {
            return;
        };
        let at = self.box_pose(&self.cardboard.boxes[i]).0;
        self.cardboard.boxes.remove(i);
        if self.cardboard.in_box == Some(id) {
            self.cardboard.in_box = None;
            self.cardboard.tween = None;
        }
        self.cardboard.broken.push((id, get_time()));
        self.cardboard.pending_shots.push(id);
        self.spawn_box_burst(at);
    }

    fn spawn_box_burst(&mut self, at: Vec3) {
        let pieces = (0..BURST_PIECES)
            .map(|_| {
                let a = gen_range(0.0, std::f32::consts::TAU);
                let out = vec3(a.cos(), 0.0, a.sin());
                let shade = gen_range(0.75, 1.05);
                BurstPiece {
                    pos: at + out * gen_range(0.1, 0.45) + vec3(0.0, gen_range(0.1, BOX_H), 0.0),
                    vel: out * gen_range(1.0, 3.0) + vec3(0.0, gen_range(1.5, 4.0), 0.0),
                    size: vec3(gen_range(0.08, 0.26), 0.012, gen_range(0.08, 0.2)),
                    color: Color::new(0.64 * shade, 0.48 * shade, 0.3 * shade, 1.0),
                    floor: at.y,
                }
            })
            .collect();
        self.cardboard.bursts.push(BoxBurst { pieces, age: 0.0 });
    }

    pub(super) fn draw_box_bursts(&self) {
        for burst in &self.cardboard.bursts {
            // shrink away over the last 0.3 s
            let size = ((BURST_LIFE - burst.age) / 0.3).clamp(0.0, 1.0);
            for p in &burst.pieces {
                draw_cube(p.pos + vec3(0.0, p.size.y / 2.0, 0.0), p.size * size, None, p.color);
            }
        }
    }

    pub(super) fn update_box_feel(&mut self, delta: f32) {
        for burst in &mut self.cardboard.bursts {
            burst.age += delta;
            for p in &mut burst.pieces {
                p.vel.y -= 9.8 * delta;
                p.pos += p.vel * delta;
                if p.pos.y < p.floor {
                    p.pos.y = p.floor;
                    p.vel = Vec3::ZERO;
                }
            }
        }
        self.cardboard.bursts.retain(|b| b.age < BURST_LIFE);

        // a few dozen boxes at most, so a scan beats building a map every frame
        let boxes = &self.cardboard.boxes;
        self.cardboard.feel.retain(|id, _| boxes.iter().any(|b| b.id == *id));
        for i in 0..self.cardboard.boxes.len() {
            let b = &self.cardboard.boxes[i];
            let (id, is_moving) = (b.id, self.box_moving(b));
            let f = self.cardboard.feel.entry(id).or_default();
            let (target, time) = if is_moving { (LIFT, 0.12) } else { (0.0, 0.05) };
            f.lift += (target - f.lift) * (delta / time).min(1.0);
            f.clock = if is_moving { f.clock + delta } else { 0.0 };
        }

        if let Some(tw) = &mut self.cardboard.tween {
            tw.age += delta;
            if tw.entering {
                let t = ease_out((tw.age / TWEEN_TIME).min(1.0));
                let at = tw.from.lerp(tw.to, t);
                self.position.x = at.x;
                self.position.z = at.z;
            }
            let answered = if tw.entering { self.cardboard.in_box == Some(tw.id) } else { self.cardboard.in_box.is_none() };
            if (tw.age >= TWEEN_TIME && answered) || tw.age >= TWEEN_TIMEOUT {
                self.cardboard.tween = None;
            }
        }

        self.cardboard.cam_blend = if self.cardboard.in_box.is_some() { (self.cardboard.cam_blend + delta / TWEEN_TIME).min(1.0) } else { 0.0 };
    }

    pub(super) fn box_eye_slot(&self, config: &GameConfig) -> bool {
        self.cardboard.in_box.is_some() && !self.chat.is_open() && config.bind_aim.is_down()
    }

    pub(super) fn box_camera(&self, config: &GameConfig, maze: Option<&Maze>) -> Option<Camera3D> {
        self.cardboard.in_box?;
        let forward = vec3(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos());
        let (eye, target) = if self.box_eye_slot(config) {
            let eye = self.position + vec3(0.0, SLIT_EYE, 0.0);
            (eye, eye + forward)
        } else {
            let pivot = self.position + vec3(0.0, CAM_PIVOT, 0.0);
            let eye = pivot - forward * self.camera_room(pivot, -forward, maze);
            // swing out from where your head was, the head sits over the same spot as the
            // pivot and the walls are full height, so the swing never crosses one either
            let head = self.position + vec3(0.0, self.movement_state.height, 0.0);
            let t = ease_out(self.cardboard.cam_blend);
            (head.lerp(eye, t), (head + forward).lerp(pivot + forward, t))
        };
        Some(Camera3D {
            position: eye,
            target,
            up: vec3(0.0, 1.0, 0.0),
            fovy: config.raw_fov().to_radians(),
            aspect: None,
            z_near: 0.01,
            z_far: crate::core::VIEW_DISTANCE,
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        })
    }

    // stop short of walls so the camera never ends up inside one, and keep it under the wall
    // tops (and over your feet) first, so the ray tests exactly the line the eye ends up on
    fn camera_room(&self, from: Vec3, dir: Vec3, maze: Option<&Maze>) -> f32 {
        let low = self.position.y + CAM_FLOOR;
        let (max, hit) = match (self.current_world, maze) {
            // no maze yet, nothing to test against, stay on top of the box
            (WorldId::Maze, None) => return 0.0,
            (WorldId::Maze, Some(m)) => {
                let max = cam_dist_in_height(from.y, dir.y, low, m.wall_height - CAM_UNDER_TOP, CAM_DIST);
                let hits = [m.ray_test_walls(from, dir, max).map(|(_, d)| d), m.ray_test_props(from, dir, max)];
                (max, hits.into_iter().flatten().reduce(f32::min))
            }
            // no ceiling in the lobby, the ground is in its ray test
            (WorldId::Lobby, _) => {
                let max = cam_dist_in_height(from.y, dir.y, low, f32::INFINITY, CAM_DIST);
                (max, ray_test_lobby(from, dir, max))
            }
        };
        hit.map_or(max, |d| (d - 0.2).max(0.0))
    }

    pub(super) fn lock_box_movement(&mut self) {
        if self.cardboard.tween.is_some() {
            self.move_dir = Vec3::ZERO;
            self.movement_state.velocity = Vec3::ZERO;
        }
        if self.cardboard.in_box.is_none() && self.cardboard.tween.is_none() {
            return;
        }
        let forward = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        let along = self.move_dir.dot(forward);
        self.move_dir = if along.abs() > 1e-4 { forward * along.signum() } else { Vec3::ZERO };
        self.movement_state.is_sprinting = false;
        self.movement_state.is_slow_walking = false;
        self.movement_state.is_crouching = false;
        self.movement_state.is_prone = false;
        self.movement_state.is_sliding = false;
        self.jump_requested = false;
    }
}

/// How far back from the pivot the eye can go along a direction whose up part is 'dir_y'
/// before its height leaves low..high, capped at 'max'. The pivot sits between the two.
fn cam_dist_in_height(pivot_y: f32, dir_y: f32, low: f32, high: f32, max: f32) -> f32 {
    let room = if dir_y < 0.0 {
        (pivot_y - low) / -dir_y
    } else if dir_y > 0.0 {
        (high - pivot_y) / dir_y
    } else {
        max
    };
    room.clamp(0.0, max)
}

pub(super) fn draw_box_slit() {
    let (w, h) = (screen_width(), screen_height());
    let (sw, sh) = (w * 0.45, h * 0.12);
    let (x0, y0) = ((w - sw) / 2.0, (h - sh) / 2.0);
    let inside = Color::new(0.05, 0.035, 0.02, 1.0);
    draw_rectangle(0.0, 0.0, w, y0, inside);
    draw_rectangle(0.0, y0 + sh, w, h - y0 - sh, inside);
    draw_rectangle(0.0, y0, x0, sh, inside);
    draw_rectangle(x0 + sw, y0, w - x0 - sw, sh, inside);
}

impl PlayingState {
    pub(super) fn draw_box_prompt(&self) {
        use crate::ui::fonts::{measure_game_text, text_params};
        if self.cardboard.tween.is_some() {
            return;
        }
        let text = if self.cardboard.in_box.is_some() {
            "[E]  GET OUT"
        } else if self.box_in_reach().is_some() {
            "[E]  HIDE"
        } else {
            return;
        };
        let tw = measure_game_text(text, 18);
        let (sx, sy) = (screen_width() / 2.0 - tw / 2.0, screen_height() / 2.0 + 120.0);
        draw_text_ex(text, sx + 1.0, sy + 1.0, text_params(18, Color::new(0.0, 0.0, 0.0, 0.5)));
        draw_text_ex(text, sx, sy, text_params(18, Color::new(1.0, 0.42, 0.21, 1.0)));
    }
}

#[cfg(test)]
mod tests;
