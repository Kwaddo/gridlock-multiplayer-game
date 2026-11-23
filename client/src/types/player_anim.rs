use std::collections::HashMap;

use macroquad::prelude::*;

use crate::types::animated_model::{ AnimatedModel, Pose };
use crate::types::player::PlayerSkin;

use crate::types::blend_space::{ weights, MAX_MIX, STANDING };

mod clips;
mod evaluate;
#[cfg(test)]
mod tests;
mod view;

use clips::*;
pub use view::{BodyView, FAR_POSE_DISTANCE};

/*
   one players animation clock, before this every clip had one clock shared by everyone, so 2 people walking were clones
   and a second death rewound the first one. L code tbh I don't know how we let that sldie

   also blends, when the clip changes the old one keeps playing and fades out over FADE_TIME
   like ue's blend, before this every clip change was an instant cut (the pop)
*/

// how long a switch between two clips takes to fade, seconds
const FADE_TIME: f32 = 0.2;

// how long the upper body layer takes to fade in or out
const UPPER_FADE: f32 = 0.1;

// the cpu skinning fallback is slow, each body bends its mesh this often at most, same 30 fps cap the shared clips had
const CPU_SKIN_INTERVAL: f32 = 1.0 / 30.0;

// what a body tells its animation each frame
pub struct AnimInput {
    pub skin: PlayerSkin,
    pub pos: Vec3,     // where the body is drawn, for its real speed
    pub pitch: f32,    // look up/down, + = up
    pub yaw: f32,      // look left/right, + = right
    pub aiming: bool,  // ads, the upper body holds the gun up while moving
    pub crouching: bool,
    pub fired: bool,   // shot this frame, the upper body kicks
}

// other players' slow walk isnt sent (no flag bits left), so: walking forward under this speed...
// between slow walk (1.8) and the slowest normal walk (2.5, out of stamina), before this 3.0 caught tired walkers too
// and muted their footsteps
const SLOW_WALK_BELOW: f32 = 2.2;
// ...for at least this long means slow walk, long enough that speeding up or coasting to a stop doesnt count
const SLOW_WALK_AFTER: f32 = 0.4;

/// How much of a body's pose `PlayerAnim::evaluate` works out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoseDetail {
    /// Every bone and the skinning matrices, to draw the body.
    Full,
    /// Only the bones the hitboxes are built from (`AnimatedModel::hit_nodes`),
    /// bit for bit what `Full` gives them. For bodies nobody can see.
    Hitboxes,
}

#[derive(Copy, Clone)]
enum FadeFrom {
    Clip(PlayerSkin, f32), // clip and how far into it, it keeps playing while it fades
    Still, // blend space's last pose
}

pub struct PlayerAnim {
    clip: PlayerSkin,
    time: f32,
    pose: Option<Pose>, // pose for the current clip, updated every tick, used for drawing the player body
    // what we're fading out of, the old clip still playing or a still of the blend space
    prev: Option<FadeFrom>,
    prev_pose: Option<Pose>, // scratch pose for the old clip, kept so it isnt remade every switch
    fade: f32, // seconds since the last switch
    // how fast this body really moves on the ground, so walk speed can match it
    last_pos: Option<Vec3>,
    vel: Vec2, // smoothed ground velocity, for the blend space to pick clips and how much of each

    stride: f32, 
    mix_pose: Option<Pose>, // scratch pose for the blend space, kept so it isnt remade every tick
    still: Option<Pose>, // scratch pose for the blend space's last pose, kept so it isnt remade every tick

    // how long theyve been walking forward slowly, see SLOW_WALK_AFTER
    slow_for: f32,
    // upper body layer (aim now, fire next), its own clip clock and how strongly it shows
    upper_clip: Option<PlayerSkin>,
    upper_time: f32,
    upper_weight: f32,
    upper_pose: Option<Pose>,
    cpu_skin_accum: f32, // seconds since this body's mesh was last bent on the cpu

    // what advance worked out for evaluate, so the pose can be worked out again (in full) without moving the clocks
    posable: bool,   // there's a model for the clip, without one there's no pose, same as before
    local_vel: Vec2, // ground velocity against where they face, x = right, y = forward, for the blend space
    pitch: f32,      // look up/down to bend the upper body by
    bend: bool,      // the look bend and the upper layer apply, not while dying or prone
    // the hitbox bones' pose for bodies nobody sees, see PoseDetail::Hitboxes
    hit_pose: Option<Pose>,
    hit_fresh: bool, // the last evaluate went to hit_pose, so hitboxes read that, not pose
    hit_nodes: Vec<usize>, // scratch list of the hitbox nodes, kept so it isnt remade every tick
    ticks: u32,      // advances so far, wrapping
    full_at: u32,    // the advance pose was last fully worked out at
    /// Whether the camera may see this body this frame, and how often its full pose is due.
    pub view: BodyView,
}

impl PlayerAnim {
    pub fn new() -> Self {
        Self {
            clip: PlayerSkin::RifleIdle,
            time: 0.0,
            pose: None,
            prev: None,
            prev_pose: None,
            fade: 0.0,
            last_pos: None,
            vel: Vec2::ZERO,
            stride: 0.0,
            mix_pose: None,
            still: None,
            slow_for: 0.0,
            upper_clip: None,
            upper_time: 0.0,
            upper_weight: 0.0,
            upper_pose: None,
            cpu_skin_accum: 0.0,
            posable: false,
            local_vel: Vec2::ZERO,
            pitch: 0.0,
            bend: false,
            hit_pose: None,
            hit_fresh: false,
            hit_nodes: Vec::new(),
            ticks: 0,
            full_at: 0,
            view: BodyView::default(),
        }
    }

    // looks like theyre slow walking (walking forward slowly for a while), no footsteps then
    pub fn slow_walking(&self) -> bool {
        self.slow_for >= SLOW_WALK_AFTER
    }

    // the full pose, for drawing, from the last frame it was worked out in full
    pub fn pose(&self) -> Option<&Pose> {
        // no pose until first tick, falls back to own pose until then
        self.pose.as_ref()
    }

    /// The pose the hitboxes read: the latest one worked out, full or only
    /// the hitbox bones (whose joints are the same either way).
    pub fn hit_pose(&self) -> Option<&Pose> {
        if self.hit_fresh { self.hit_pose.as_ref() } else { self.pose.as_ref() }
    }

    /// Whether the camera wants this frame's full pose and it isn't worked out yet.
    pub fn needs_full_pose(&self) -> bool {
        self.posable && self.view.wants_full() && !(self.pose.is_some() && self.full_at == self.ticks)
    }

    // move the clocks and work out the whole pose, one call per frame, for bodies that are always drawn (yours)
    pub fn tick(&mut self, input: &AnimInput, dt: f32, models: &HashMap<String, AnimatedModel>) {
        self.advance(input, dt, models);
        self.evaluate(models, PoseDetail::Full);
    }

    /// Move this body's clocks on by `dt`: velocity, clip switches and fades,
    /// the blend space's stride, the upper layer. Then `evaluate` works out
    /// the pose for where the clocks are; it can run again (in full) without
    /// moving them.
    pub fn advance(&mut self, input: &AnimInput, dt: f32, models: &HashMap<String, AnimatedModel>) {
        self.ticks = self.ticks.wrapping_add(1);
        self.view.aged(dt);
        let (skin, pos) = (input.skin, input.pos);
        // smoothed ground velocity, which way and how fast, before this it was only the speed
        if let (Some(last), true) = (self.last_pos, dt > 0.0) {
            let step = vec2(pos.x - last.x, pos.z - last.z);
            // a respawn or teleport jumps metres in one frame, skip it, before this it read as a huge speed for a moment
            if step.length() < 2.0 {
                self.vel += (step / dt - self.vel) * (1.0 - (-12.0 * dt).exp());
            }
        }
        self.last_pos = Some(pos);
        let speed = self.vel.length();

        // a slow forward walk shows the slow walk clip, before this slow walking looked like a normal walk in slow motion
        let slow = skin == PlayerSkin::RifleWalk && speed > 0.3 && speed < SLOW_WALK_BELOW;
        self.slow_for = if slow { self.slow_for + dt } else { 0.0 };
        let skin = if self.slow_for >= SLOW_WALK_AFTER { PlayerSkin::SlowWalkForward } else { skin };

        // the blend space counts as one clip, walk fwd -> strafe inside it is the mix moving, not a switch
        let (was_space, now_space) = (in_space(self.clip), in_space(skin));
        let switched = if now_space { !was_space } else { skin != self.clip };
        if switched {
            // where in the step we were (0..1), so the next moving loop picks up the same step
            let phase = if was_space {
                Some(self.stride)
            } else if is_moving_loop(self.clip) {
                find(models, self.clip).map(|m| self.time / clip_end(self.clip, m.duration()).max(0.0001))
            } else {
                None
            };
            // the blend space isnt one clip so it cant keep playing while it fades, it fades from a still instead
            // a respawn cuts straight in, before this the body rose out of its corpse pose at the spawn
            self.prev = if is_death(self.clip) {
                None
            } else {
                Some(if was_space { FadeFrom::Still } else { FadeFrom::Clip(self.clip, self.time) })
            };
            self.fade = 0.0;
            if now_space {
                self.stride = phase.unwrap_or(0.0);
                self.time = 0.0; // the idle's own clock
            } else {
                self.time = match (phase, find(models, skin)) {
                    (Some(p), Some(new)) if is_moving_loop(skin) => p * clip_end(skin, new.duration()),
                    _ => 0.0,
                };
            }
        }
        self.clip = skin;

        let Some(model) = find(models, skin) else {
            self.posable = false;
            return; // no model for this skin, so no pose
        };
        self.posable = true;

        let rate = playback_rate(skin, speed);
        if now_space {
            // how they move compared to where they face, x = right, y = forward
            let (sin, cos) = input.yaw.sin_cos();
            self.local_vel = vec2(self.vel.dot(vec2(-sin, cos)), self.vel.dot(vec2(cos, sin)));
            self.advance_space(dt, models);
        } else {
            self.time = advance(skin, self.time, dt * rate, clip_end(skin, model.duration()));
        }

        // still fading out of the old clip or the blend space's still, its clock keeps going
        if let Some(from) = self.prev {
            self.fade += dt;
            let w = smoothstep(self.fade / FADE_TIME);
            match from {
                _ if w >= 1.0 => self.prev = None, // fade done, just the new clip now
                FadeFrom::Still => {
                    if !self.still.as_ref().is_some_and(|still| model.pose_fits(still)) {
                        self.prev = None;
                    }
                }
                FadeFrom::Clip(prev_skin, prev_time) => match find(models, prev_skin) {
                    Some(prev_model) => {
                        let prev_time = advance(prev_skin, prev_time, dt * rate, clip_end(prev_skin, prev_model.duration()));
                        self.prev = Some(FadeFrom::Clip(prev_skin, prev_time));
                    }
                    None => self.prev = None,
                },
            }
        }

        // aiming while moving, legs keep the move clip and the upper body aims, before this you glided in the aim idle
        let wants = (input.aiming && is_moving_loop(skin))
            .then_some(if input.crouching { PlayerSkin::CrouchAim } else { PlayerSkin::RifleAimIdle });
        // no upper layer while dying or prone, a kick on a body on the floor looks broken
        let layerable = !is_one_shot(skin) && !is_prone(skin);
        let (wants, fired) = if layerable { (wants, input.fired) } else { (None, false) };
        self.advance_upper(wants, fired, dt, models);

        // look up/down, not while dying or lying prone (bending a body on the floor looks broken)
        self.bend = layerable;
        self.pitch = input.pitch;

        if !model.drawn_on_gpu() {
            // capped per body, see evaluate, before this every body did it every frame and the fallback tanked fps
            self.cpu_skin_accum += dt;
        }
    }
}

impl PlayerAnim {
    // fade the upper layer toward 'wants', its clock and weight, evaluate mixes it onto the upper body
    fn advance_upper(&mut self, wants: Option<PlayerSkin>, fired: bool, dt: f32, models: &HashMap<String, AnimatedModel>) {
        // nothing wanted and already faded out, nothing to do, before this it looked up the fire clip every tick anyway
        // (a faded out layer never holds a clip, see the end of this, so no kick can be left playing)
        if wants.is_none() && !fired && self.upper_weight <= 0.0 {
            self.upper_clip = None;
            return;
        }
        // a shot restarts the kick, it beats aiming until it finishes, then aiming (or nothing) takes back over
        if fired {
            self.upper_clip = Some(PlayerSkin::RifleFire);
            self.upper_time = 0.0;
        }
        let kick_end = find(models, PlayerSkin::RifleFire)
            .map(|m| clip_end(PlayerSkin::RifleFire, m.duration()))
            .unwrap_or(0.0);
        let kicking = self.upper_clip == Some(PlayerSkin::RifleFire) && self.upper_time < kick_end;
        let wants = if kicking { Some(PlayerSkin::RifleFire) } else { wants };
        if wants.is_some() && wants != self.upper_clip {
            self.upper_clip = wants;
            self.upper_time = 0.0;
        }
        let target = if wants.is_some() { 1.0 } else { 0.0 };
        let step = dt / UPPER_FADE;
        self.upper_weight = if target > self.upper_weight {
            (self.upper_weight + step).min(1.0)
        } else {
            (self.upper_weight - step).max(0.0)
        };
        if self.upper_weight <= 0.0 {
            self.upper_clip = None;
            return;
        }
        let Some(clip) = self.upper_clip else { return };
        let Some(upper_model) = find(models, clip) else { return };
        self.upper_time = advance(clip, self.upper_time, dt, clip_end(clip, upper_model.duration()));
    }

    // the blend space's clocks: every moving clip shares one stride so the feet land together, idle keeps its own clock
    fn advance_space(&mut self, dt: f32, models: &HashMap<String, AnimatedModel>) {
        let mix = weights(&STANDING, self.local_vel);
        // each clip's model looked up once, before this the stride, the idle clock and the sampling each looked it up again
        let mut found: [Option<&AnimatedModel>; MAX_MIX] = [None; MAX_MIX];
        for (slot, &(skin, _)) in found.iter_mut().zip(mix.iter()) {
            *slot = find(models, skin);
        }
        let model_of = |skin: PlayerSkin| match mix.iter().position(|(s, _)| *s == skin) {
            Some(i) => found[i],
            None => find(models, skin),
        };
        if let Some(len) = stride_length(&mix, |s| model_of(s).map(|m| m.duration())) {
            self.stride = (self.stride + dt / len).fract();
        }
        if let Some(idle) = model_of(STANDING.idle) {
            self.time = advance(STANDING.idle, self.time, dt, idle.duration());
        }
    }
}

impl Default for PlayerAnim {
    fn default() -> Self {
        Self::new()
    }
}
