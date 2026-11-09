use std::collections::HashMap;

use macroquad::prelude::*;

use crate::types::animated_model::{ AnimatedModel, Pose };
use crate::types::player::PlayerSkin;

/*
   one players animation clock, before this every clip had one clock shared by everyone, so 2 people walking were clones
   and a second death rewound the first one. L code tbh I don't know how we let that sldie

   also blends, when the clip changes the old one keeps playing and fades out over FADE_TIME
   like ue's blend, before this every clip change was an instant cut (the pop)
*/

// how long a switch between two clips takes to fade, seconds
const FADE_TIME: f32 = 0.2;

pub struct PlayerAnim {
    clip: PlayerSkin,
    time: f32,
    pose: Option<Pose>, // pose for the current clip, updated every tick, used for drawing the player body
    // the clip we're fading out of and how far into it, it keeps playing while it fades
    prev: Option<(PlayerSkin, f32)>,
    prev_pose: Option<Pose>, // scratch pose for the old clip, kept so it isnt remade every switch
    fade: f32, // seconds since the last switch
    // how fast this body really moves on the ground, so walk speed can match it
    last_pos: Option<Vec3>,
    speed: f32,
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
            speed: 0.0,
        }
    }

    pub fn pose(&self) -> Option<&Pose> {
        // no pose until first tick, falls back to own pose until then
        self.pose.as_ref()
    }

    // `pos` = where the body is drawn, used to measure how fast it really moves
    pub fn tick(&mut self, skin: PlayerSkin, dt: f32, models: &HashMap<String, AnimatedModel>, pos: Vec3) {
        // smoothed ground speed, before this the walk always played at full speed even when barely moving
        if let Some(last) = self.last_pos {
            if dt > 0.0 {
                let now = vec2(pos.x - last.x, pos.z - last.z).length() / dt;
                self.speed += (now - self.speed) * (1.0 - (-12.0 * dt).exp());
            }
        }
        self.last_pos = Some(pos);

        if skin != self.clip {
            // keep the old clip playing so it can fade out
            self.prev = Some((self.clip, self.time));
            self.fade = 0.0;
            // two movement loops (walk fwd -> strafe) keep the same point in the stride,
            // anything else starts from the top, before this every switch restarted the step
            self.time = match (find(models, self.clip), find(models, skin)) {
                (Some(old), Some(new)) if is_moving_loop(self.clip) && is_moving_loop(skin) => {
                    let old_end = clip_end(self.clip, old.duration()).max(0.0001);
                    (self.time / old_end) * clip_end(skin, new.duration())
                }
                _ => 0.0,
            };
            self.clip = skin;
        }

        let Some(model) = find(models, skin) else {
            return; // no model for this skin, so no pose
        };

        let rate = playback_rate(skin, self.speed);
        self.time = advance(skin, self.time, dt * rate, clip_end(skin, model.duration()));
        let pose = self.pose.get_or_insert_with(|| model.new_pose()); // make a pose if we dont have one yet, then fill it in
        model.sample_local(self.time, pose);

        // still fading out of the old clip, mix it in
        if let Some((prev_skin, prev_time)) = self.prev {
            self.fade += dt;
            let w = smoothstep(self.fade / FADE_TIME);
            match find(models, prev_skin) {
                Some(prev_model) if w < 1.0 => {
                    let prev_time = advance(prev_skin, prev_time, dt * rate, clip_end(prev_skin, prev_model.duration()));
                    self.prev = Some((prev_skin, prev_time));
                    let prev_pose = self.prev_pose.get_or_insert_with(|| prev_model.new_pose());
                    prev_model.sample_local(prev_time, prev_pose);
                    pose.blend_from(prev_pose, w);
                }
                _ => self.prev = None, // fade done (or old clip gone), just the new clip now
            }
        }

        model.finish_pose(pose);
        if !model.drawn_on_gpu() {
            model.skin_pose_on_cpu(pose); // if gpu skinning is off, bend the mesh on the cpu for this pose so we can draw it
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
        PlayerSkin::StandDeath | PlayerSkin::CrouchDeath | PlayerSkin::ProneDeath | PlayerSkin::Slide
    )
}

// looping clips where the feet move, switching between two of these keeps the stride
fn is_moving_loop(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(
        skin,
        RifleWalk | WalkBack | WalkLeft | WalkRight
            | SprintForward | SprintLeft | SprintRight
            | CrouchForward | CrouchBack | CrouchLeft | CrouchRight
            | ProneForward | ProneTurnLeft | ProneTurnRight
    )
}

// how fast to play a moving clip, real speed / the speed the clip is made for (the movement config speeds)
// so coasting to a stop slows the steps down instead of swinging full strides, 1.0 for everything else
fn playback_rate(skin: PlayerSkin, speed: f32) -> f32 {
    use PlayerSkin::*;
    let made_for = match skin {
        RifleWalk | WalkBack | WalkLeft | WalkRight => 5.0, // walk_speed
        SprintForward | SprintLeft | SprintRight => 8.0, // sprint_speed
        CrouchForward | CrouchBack | CrouchLeft | CrouchRight => 2.5, // crouch_speed
        ProneForward | ProneTurnLeft | ProneTurnRight => 1.5, // prone_speed
        _ => return 1.0,
    };
    (speed / made_for).clamp(0.5, 1.5)
}

// where a clip ends, the slide file is longer than the slide, only the first second is used, same as before
fn clip_end(skin: PlayerSkin, duration: f32) -> f32 {
    match skin {
        PlayerSkin::Slide => duration.min(1.0),
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
    fn fade_starts_and_ends_soft() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-5);
    }
}
