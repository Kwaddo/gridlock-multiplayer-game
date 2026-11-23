use std::cell::Cell;

use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::types::player_anim::{PoseDetail, FAR_POSE_DISTANCE};
use crate::types::world::WorldId;
use crate::world::render::Frustum;

use macroquad::prelude::*;

use super::PlayingState;

/*
    which bodies the camera may see this frame, worked out once per body after the frame's movement
    a body nobody can see only gets the bones its hitboxes are built from (same numbers as the full pose, see
    PoseDetail::Hitboxes) and isn't drawn, a far one gets its full pose at a lower rate (see BodyView)
    every test here leans to "seen": a wrong "seen" costs a full pose, a wrong "hidden" would be a missing body
*/

// the box around render_position() the draw culls with (rendering/players.rs), grown a bit:
// the render's camera moves a hair after this (head bob, the zoom easing), and a lying corpse reaches further
const BODY_BELOW: Vec3 = Vec3::new(2.0, 2.7, 2.0);
const BODY_ABOVE: Vec3 = Vec3::new(2.0, 3.1, 2.0);
// the frustum here is this much wider than the drawn one, same reason
const FOV_SLACK: f32 = 1.1;
// closer than this a body always counts as seen, metres
const ALWAYS_SEEN_WITHIN: f32 = 2.5;
// how far a body reaches sideways from its middle: arms, the gun, a corpse lying down, metres
const BODY_REACH: f32 = 1.2;
// the sight lines aim this high above render_position(), anywhere under the walls' top does
const SIGHT_HEIGHT: f32 = 1.0;
// sight lines to a body are at most this far apart where it stands, metres
const SIGHT_STEP: f32 = 0.25;

/// Bodies this frame, for the dev overlay: animation clocks moved, poses
/// worked out in full and for hitboxes only, bodies drawn (main view and mirror).
#[derive(Clone, Copy, Default, Debug)]
pub struct BodyStats {
    pub ticked: u32,
    pub posed_full: u32,
    pub posed_hitboxes: u32,
    pub drawn: u32,
}

thread_local! {
    static STATS: Cell<BodyStats> = Cell::new(BodyStats::default());
    // set when nothing 3D is drawn this frame (the loadout screen), every body counts as hidden
    static NOTHING_DRAWN: Cell<bool> = const { Cell::new(false) };
}

pub(in crate::screens::playing) fn count(f: impl FnOnce(&mut BodyStats)) {
    STATS.with(|s| {
        let mut stats = s.get();
        f(&mut stats);
        s.set(stats);
    });
}

pub(super) fn reset_stats() {
    STATS.with(|s| s.set(BodyStats::default()));
}

impl PlayingState {
    /// This frame's body counts so far, for the dev overlay.
    pub fn body_stats(&self) -> BodyStats {
        STATS.with(Cell::get)
    }

    /// Call before `simulate` on a frame that draws no 3D (the loadout
    /// screen): no body is posed in full or drawn that frame. Hitboxes
    /// are still worked out for every body.
    pub fn draws_no_world_this_frame(&self) {
        NOTHING_DRAWN.with(|n| n.set(true));
    }

    /// After the frame's movement: which bodies the camera may see, and the
    /// full pose now for each seen body that only got its hitbox bones this
    /// frame (it was hidden last frame, or became due).
    pub(super) fn prepare_bodies(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        let nothing_drawn = NOTHING_DRAWN.with(|n| n.replace(false));
        let mut camera = self.build_camera(config);
        camera.fovy = (camera.fovy * FOV_SLACK).min(3.0);
        let frustum = Frustum::new(camera.matrix());
        let eye = camera.position;
        // mirrors (the lobby's) show bodies behind you and around corners, so with one around every body counts
        let everyone = !self.mirrors.is_empty();
        let walls = maze.filter(|m| self.current_world == WorldId::Maze && sees_from_inside(m, eye));
        let seen_at = |at: Vec3| {
            everyone
                || (frustum.may_see(at - BODY_BELOW, at + BODY_ABOVE)
                    && !walls.is_some_and(|m| hidden_by_walls(m, eye, at)))
        };

        let models = &self.animated_models;
        let bodies = self.other_players.iter_mut().chain(self.practice_bots.iter_mut().map(|b| &mut b.player));
        for p in bodies {
            let at = p.render_position();
            let far = (at - eye).length() > FAR_POSE_DISTANCE;
            let seen = !nothing_drawn && seen_at(at);
            p.anim.view.set(seen, far, p.id);
            if p.anim.needs_full_pose() {
                p.anim.evaluate(models, PoseDetail::Full);
                count(|s| s.posed_full += 1);
            }
        }
    }
}

// the eye is in an open cell under the walls' top, so walls between it and a body hide the body
// (a death camera backed into a wall or above the walls sees past them, then every wall test says seen)
fn sees_from_inside(m: &Maze, eye: Vec3) -> bool {
    let (cx, cy) = m.cell_from_world(eye);
    eye.y < m.wall_height && m.is_open(cx, cy)
}

// whether walls block every sight line from the eye to the body: its middle and points either side
// out to where sight lines graze a disc of BODY_REACH around it, no more than SIGHT_STEP apart
// (an opening seen at an angle shows a strip much narrower than a cell, so a cell's spacing misses it)
// walls are full height boxes and the eye is under their top, so this is a top down question
fn hidden_by_walls(m: &Maze, eye: Vec3, at: Vec3) -> bool {
    let flat = vec2(at.x - eye.x, at.z - eye.z);
    let d = flat.length();
    if d < ALWAYS_SEEN_WITHIN {
        return false;
    }
    let side = vec2(-flat.y, flat.x) / d;
    // the tangent to the disc meets the body's side line this far out (d > ALWAYS_SEEN_WITHIN > BODY_REACH)
    let reach = BODY_REACH * d / (d * d - BODY_REACH * BODY_REACH).sqrt();
    let steps = (reach / SIGHT_STEP).ceil() as i32;
    let middle = at + vec3(0.0, SIGHT_HEIGHT, 0.0);
    (-steps..=steps).all(|k| {
        let off = side * (reach * k as f32 / steps as f32);
        let target = middle + vec3(off.x, 0.0, off.y);
        let to = target - eye;
        let dist = to.length();
        // the same test the name tags use (rendering/hud.rs)
        dist >= 0.01 && m.ray_hits_wall_before(eye, to / dist, dist, dist - 0.1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    // a 7x7 maze, open everywhere but the cells listed as walls
    fn maze(walls: &[(u8, u8)]) -> Maze {
        let mut cells = vec![1u8; 49];
        for &(x, y) in walls {
            cells[y as usize * 7 + x as usize] = 0;
        }
        Maze {
            width: 7,
            height: 7,
            cells,
            cell_size: 1.8,
            wall_height: 2.4,
            teleporter: (0, 0),
            rooms: Vec::new(),
            props: Vec::new(),
            ceiling_lights: Vec::new(),
            breakable_walls: HashMap::new(),
            broken_wall_scars: HashSet::new(),
        }
    }

    fn cell(x: f32, y: f32) -> Vec3 {
        vec3(x * 1.8 + 0.9, 0.0, y * 1.8 + 0.9)
    }

    #[test]
    fn a_wall_line_hides_a_body_behind_it() {
        let m = maze(&[(3, 0), (3, 1), (3, 2), (3, 3), (3, 4), (3, 5), (3, 6)]);
        let eye = cell(1.0, 3.0) + vec3(0.0, 1.6, 0.0);
        assert!(hidden_by_walls(&m, eye, cell(5.0, 3.0)));
        assert!(!hidden_by_walls(&m, eye, cell(2.0, 1.0))); // same side
    }

    // a body round a corner with only an arm's reach showing past the wall's edge still counts as seen
    #[test]
    fn a_body_just_round_a_corner_is_seen() {
        let m = maze(&[(3, 3), (3, 4), (3, 5), (3, 6)]);
        let eye = cell(1.0, 4.0) + vec3(0.0, 1.6, 0.0);
        // its middle is behind the wall's end, but it reaches past it
        let body = cell(5.0, 0.5);
        assert!(m.ray_hits_wall_before(eye, (body + vec3(0.0, 1.0, 0.0) - eye).normalize(), 20.0, 8.0));
        assert!(!hidden_by_walls(&m, eye, body));
        // well behind the wall, nothing shows
        assert!(hidden_by_walls(&m, eye, cell(5.0, 5.0)));
    }

    // seen at an angle through a one-cell opening, a body shows only a narrow strip, off its middle:
    // any strip 0.3 m wide must count as seen (checked against sight lines 1 cm apart)
    #[test]
    fn a_strip_seen_through_an_opening_at_an_angle_is_seen() {
        let m = maze(&[(3, 0), (3, 1), (3, 2), (3, 4), (3, 5), (3, 6)]);
        let eye = cell(1.0, 1.0) + vec3(0.0, 1.6, 0.0);
        let blocked = |target: Vec3| {
            let to = target - eye;
            let dist = to.length();
            m.ray_hits_wall_before(eye, to / dist, dist, dist - 0.1)
        };
        let mut old_misses = 0;
        for i in 0..60 {
            for j in 0..120 {
                let at = vec3(3.0 * 1.8 + 1.0 + i as f32 * 0.04, 0.0, j as f32 * 0.105);
                let flat = vec2(at.x - eye.x, at.z - eye.z);
                let side = vec2(-flat.y, flat.x).normalize();
                let point = |o: f32| at + vec3(side.x * o, SIGHT_HEIGHT, side.y * o);
                let (mut run, mut widest) = (0, 0);
                for k in -120..=120 {
                    run = if blocked(point(k as f32 * 0.01)) { 0 } else { run + 1 };
                    widest = widest.max(run);
                }
                if widest >= 30 {
                    assert!(!hidden_by_walls(&m, eye, at), "body at {at:?} shows {widest} cm");
                    if [-BODY_REACH, 0.0, BODY_REACH].iter().all(|&o| blocked(point(o))) {
                        old_misses += 1;
                    }
                }
            }
        }
        assert!(old_misses > 0, "the sweep should cover strips three sight lines miss");
    }

    #[test]
    fn close_bodies_and_open_sight_lines_are_seen() {
        let m = maze(&[]);
        let eye = cell(0.0, 0.0) + vec3(0.0, 1.6, 0.0);
        assert!(!hidden_by_walls(&m, eye, cell(6.0, 6.0)));
        let walled = maze(&[(1, 0)]);
        assert!(!hidden_by_walls(&walled, eye, eye + vec3(2.0, -1.6, 0.0))); // within ALWAYS_SEEN_WITHIN
    }

    #[test]
    fn an_eye_in_a_wall_or_above_them_sees_past_walls() {
        let m = maze(&[(2, 2)]);
        assert!(sees_from_inside(&m, cell(1.0, 1.0) + vec3(0.0, 1.6, 0.0)));
        assert!(!sees_from_inside(&m, cell(2.0, 2.0) + vec3(0.0, 1.6, 0.0)));
        assert!(!sees_from_inside(&m, cell(1.0, 1.0) + vec3(0.0, 3.0, 0.0)));
    }
}
