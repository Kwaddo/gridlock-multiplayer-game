use std::collections::HashMap;

use macroquad::prelude::*;

use crate::types::animated_model::{ AnimatedModel, Pose };
use crate::types::player::PlayerSkin;

use crate::types::blend_space::{ weights, STANDING };

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

// only the start of the fire clip plays, the kick, every shot restarts it so auto fire reads as recoil
const FIRE_KICK: f32 = 0.25;

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
const SLOW_WALK_BELOW: f32 = 3.0;
// ...for at least this long means slow walk, long enough that speeding up or coasting to a stop doesnt count
const SLOW_WALK_AFTER: f32 = 0.4;

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
        }
    }

    // looks like theyre slow walking (walking forward slowly for a while), no footsteps then
    pub fn slow_walking(&self) -> bool {
        self.slow_for >= SLOW_WALK_AFTER
    }

    pub fn pose(&self) -> Option<&Pose> {
        // no pose until first tick, falls back to own pose until then
        self.pose.as_ref()
    }

    pub fn tick(&mut self, input: &AnimInput, dt: f32, models: &HashMap<String, AnimatedModel>) {
        let (skin, pos, pitch) = (input.skin, input.pos, input.pitch);
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
            self.prev = Some(if was_space { FadeFrom::Still } else { FadeFrom::Clip(self.clip, self.time) });
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
            return; // no model for this skin, so no pose
        };

        let rate = playback_rate(skin, speed);
        // lent out of self for this tick so the upper layer can use self too, handed back at the end
        let mut owned = self.pose.take().unwrap_or_else(|| model.new_pose()); // make a pose if we dont have one yet
        if !model.pose_fits(&owned) {
            owned = model.new_pose(); // the character changed to one with a different skeleton
        }
        let pose = &mut owned;
        if now_space {
            // how they move compared to where they face, x = right, y = forward
            let (sin, cos) = input.yaw.sin_cos();
            let local = vec2(self.vel.dot(vec2(-sin, cos)), self.vel.dot(vec2(cos, sin)));
            self.sample_space(local, dt, models, pose);
        } else {
            self.time = advance(skin, self.time, dt * rate, clip_end(skin, model.duration()));
            model.sample_local(self.time, pose);
        }

        // still fading out of the old clip or the blend space's still, mix it in
        if let Some(from) = self.prev {
            self.fade += dt;
            let w = smoothstep(self.fade / FADE_TIME);
            match from {
                _ if w >= 1.0 => self.prev = None, // fade done, just the new clip now
                FadeFrom::Still => match &self.still {
                    Some(still) if model.pose_fits(still) => pose.blend_from(still, w),
                    _ => self.prev = None,
                },
                FadeFrom::Clip(prev_skin, prev_time) => match find(models, prev_skin) {
                    Some(prev_model) => {
                        let prev_time = advance(prev_skin, prev_time, dt * rate, clip_end(prev_skin, prev_model.duration()));
                        self.prev = Some(FadeFrom::Clip(prev_skin, prev_time));
                        let prev_pose = self.prev_pose.get_or_insert_with(|| prev_model.new_pose());
                        if !prev_model.pose_fits(prev_pose) {
                            *prev_pose = prev_model.new_pose();
                        }
                        prev_model.sample_local(prev_time, prev_pose);
                        pose.blend_from(prev_pose, w);
                    }
                    None => self.prev = None,
                },
            }
        }

        // a still of the blend space before the aim and look bends, leaving it fades from this
        if now_space {
            let still = self.still.get_or_insert_with(|| model.new_pose());
            if !model.pose_fits(still) {
                *still = model.new_pose();
            }
            still.copy_from(pose);
        }

        // aiming while moving, legs keep the move clip and the upper body aims, before this you glided in the aim idle
        let wants = (input.aiming && is_moving_loop(skin))
            .then_some(if input.crouching { PlayerSkin::CrouchAim } else { PlayerSkin::RifleAimIdle });
        // no upper layer while dying or prone, a kick on a body on the floor looks broken
        let layerable = !is_one_shot(skin) && !is_prone(skin);
        let (wants, fired) = if layerable { (wants, input.fired) } else { (None, false) };
        self.layer_upper(wants, fired, dt, models, model, pose);

        // look up/down, not while dying or lying prone (bending a body on the floor looks broken)
        if !is_one_shot(skin) && !is_prone(skin) {
            model.bend_upper_body(pose, pitch);
        }

        model.finish_pose(pose);
        if !model.drawn_on_gpu() {
            model.skin_pose_on_cpu(pose); // if gpu skinning is off, bend the mesh on the cpu for this pose so we can draw it
        }
        self.pose = Some(owned);
    }
}

impl PlayerAnim {
    // fade the upper layer toward 'wants' and mix it onto the upper body of 'pose'
    fn layer_upper(
        &mut self,
        wants: Option<PlayerSkin>,
        fired: bool,
        dt: f32,
        models: &HashMap<String, AnimatedModel>,
        model: &AnimatedModel,
        pose: &mut Pose,
    ) {
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
        let layer = self.upper_pose.get_or_insert_with(|| upper_model.new_pose());
        if !upper_model.pose_fits(layer) {
            *layer = upper_model.new_pose();
        }
        upper_model.sample_local(self.upper_time, layer);
        model.layer_upper_body(pose, layer, smoothstep(self.upper_weight));
    }

    // standing moves, mixes the blend space's clips by how they really move
    // every moving clip shares one stride so the feet land together, idle keeps its own clock
    fn sample_space(&mut self, local_vel: Vec2, dt: f32, models: &HashMap<String, AnimatedModel>, pose: &mut Pose) {
        let mix = weights(&STANDING, local_vel);
        if let Some(len) = stride_length(&mix, |s| find(models, s).map(|m| m.duration())) {
            self.stride = (self.stride + dt / len).fract();
        }
        if let Some(idle) = find(models, STANDING.idle) {
            self.time = advance(STANDING.idle, self.time, dt, idle.duration());
        }
        let mut total = 0.0;
        for &(skin, w) in &mix {
            let Some(m) = find(models, skin) else { continue };
            let t = if skin == STANDING.idle { self.time } else { self.stride * m.duration() };
            let first = total == 0.0;
            total += w;
            if first {
                m.sample_local(t, pose);
                continue;
            }
            let scratch = self.mix_pose.get_or_insert_with(|| m.new_pose());
            if !m.pose_fits(scratch) {
                *scratch = m.new_pose();
            }
            m.sample_local(t, scratch);
            pose.mix_in(scratch, w / total);
        }
    }
}

impl Default for PlayerAnim {
    fn default() -> Self {
        Self::new()
    }
}

// the model for a skin, missing clip falls back to idle, same as drawing does
fn find(models: &HashMap<String, AnimatedModel>, skin: PlayerSkin) -> Option<&AnimatedModel> {
    skin.model_key()
        .and_then(|k| models.get(k))
        .or_else(|| models.get("anim_rifle_idle"))
}

// move a clip's time forward, one shots stop on their last frame, everything else loops
fn advance(skin: PlayerSkin, time: f32, step: f32, end: f32) -> f32 {
    let t = time + step;
    if is_one_shot(skin) {
        t.min(end) // deaths and slides stop on their last frame
    } else if end > 0.0 {
        t.rem_euclid(end) // loop the clip, so walking and running keep going
    } else {
        0.0
    }
}

// soft start and end for the fade, 0..1 in, 0..1 out
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// clips that play once and hold, instead of looping
fn is_one_shot(skin: PlayerSkin) -> bool {
    matches!(
        skin,
        PlayerSkin::StandDeath
            | PlayerSkin::CrouchDeath
            | PlayerSkin::ProneDeath
            | PlayerSkin::Slide
            // the chooser's deaths, before this they looped so bodies kept dying
            | PlayerSkin::DeathFront
            | PlayerSkin::DeathBack
            | PlayerSkin::DeathSide
            | PlayerSkin::DeathFrontHeadshot
            | PlayerSkin::DeathBackHeadshot
            | PlayerSkin::RifleFire
    )
}

// lying on the floor, no look up/down bend here
fn is_prone(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(skin, ProneIdle | ProneForward | ProneBack | ProneTurnLeft | ProneTurnRight)
}

// standing clips the blend space drives, idle too so starting and stopping blend by speed
// before this tapping w played the walk at full stride
fn in_space(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(skin, RifleIdle | RifleWalk | WalkBack | WalkLeft | WalkRight | SprintForward | SprintLeft | SprintRight)
}

// one step's length for this mix, the moving clips' lengths by weight, None when its all idle
fn stride_length(mix: &[(PlayerSkin, f32)], length_of: impl Fn(PlayerSkin) -> Option<f32>) -> Option<f32> {
    let (mut len, mut moving) = (0.0, 0.0);
    for &(skin, w) in mix.iter().filter(|(s, _)| *s != STANDING.idle) {
        if let Some(l) = length_of(skin) {
            len += l * w;
            moving += w;
        }
    }
    (moving > 0.0 && len > 0.0).then(|| len / moving)
}

// looping clips where the feet move, switching between two of these keeps the stride
fn is_moving_loop(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(
        skin,
        RifleWalk | WalkBack | WalkLeft | WalkRight | SlowWalkForward
            | SprintForward | SprintLeft | SprintRight
            | CrouchForward | CrouchBack | CrouchLeft | CrouchRight
            | ProneForward | ProneBack | ProneTurnLeft | ProneTurnRight
    )
}

// how fast to play a moving clip, real speed / the speed the clip is made for (the movement config speeds)
// so coasting to a stop slows the steps down instead of swinging full strides, 1.0 for everything else
fn playback_rate(skin: PlayerSkin, speed: f32) -> f32 {
    use PlayerSkin::*;
    let made_for = match skin {
        RifleWalk | WalkBack | WalkLeft | WalkRight => 5.0, // walk_speed
        SlowWalkForward => 1.8, // slow_walk_speed
        SprintForward | SprintLeft | SprintRight => 8.0, // sprint_speed
        CrouchForward | CrouchBack | CrouchLeft | CrouchRight => 2.5, // crouch_speed
        ProneForward | ProneBack | ProneTurnLeft | ProneTurnRight => 1.5, // prone_speed
        _ => return 1.0,
    };
    (speed / made_for).clamp(0.5, 1.5)
}

// where a clip ends, the slide file is longer than the slide, only the first second is used, same as before
fn clip_end(skin: PlayerSkin, duration: f32) -> f32 {
    match skin {
        PlayerSkin::Slide => duration.min(1.0),
        PlayerSkin::RifleFire => duration.min(FIRE_KICK),
        _ => duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_shots_hold_and_loops_wrap() {
        assert_eq!(advance(PlayerSkin::StandDeath, 1.9, 0.5, 2.0), 2.0);
        assert!((advance(PlayerSkin::RifleWalk, 0.9, 0.3, 1.0) - 0.2).abs() < 1e-5);
    }

    #[test]
    fn walk_slows_down_when_barely_moving() {
        assert_eq!(playback_rate(PlayerSkin::RifleWalk, 5.0), 1.0);
        assert_eq!(playback_rate(PlayerSkin::RifleWalk, 0.5), 0.5); // coasting, half speed steps
        assert_eq!(playback_rate(PlayerSkin::RifleIdle, 0.0), 1.0); // idle isnt speed matched
    }

    #[test]
    fn every_death_holds_its_last_frame() {
        use PlayerSkin::*;
        for death in [StandDeath, CrouchDeath, ProneDeath, DeathFront, DeathBack, DeathSide, DeathFrontHeadshot, DeathBackHeadshot] {
            assert_eq!(advance(death, 1.9, 0.5, 2.0), 2.0, "{death:?} looped");
        }
    }

    #[test]
    fn fire_plays_just_the_kick_and_holds() {
        assert_eq!(clip_end(PlayerSkin::RifleFire, 1.2), FIRE_KICK);
        assert_eq!(clip_end(PlayerSkin::RifleFire, 0.1), 0.1); // a short clip plays whole
        assert_eq!(advance(PlayerSkin::RifleFire, 0.2, 0.2, FIRE_KICK), FIRE_KICK); // stops at the end, no loop
    }

    #[test]
    fn stride_length_mixes_the_moving_clips() {
        use PlayerSkin::*;
        let len = |s| Some(match s { RifleWalk => 1.0, SprintForward => 0.6, _ => 2.0 });
        assert_eq!(stride_length(&[(RifleIdle, 1.0)], len), None); // standing still, stride waits
        let mid = stride_length(&[(RifleWalk, 0.5), (SprintForward, 0.5)], len).unwrap();
        assert!((mid - 0.8).abs() < 1e-5);
        let slow = stride_length(&[(RifleIdle, 0.5), (RifleWalk, 0.5)], len).unwrap();
        assert!((slow - 1.0).abs() < 1e-5); // idle doesnt stretch the step
    }

    #[test]
    fn fade_starts_and_ends_soft() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-5);
    }
}
