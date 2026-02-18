use super::player::PlayerSkin;
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveDir {
    None,
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimState {
    pub move_dir: MoveDir,
    pub sprinting: bool,
    pub ads: bool,
    pub crouched: bool,
    pub prone: bool,
    pub slide: bool,
    pub jump_started: bool,
}

impl AnimState {
    pub fn from_movement(
        prev_x: f32,
        prev_z: f32,
        x: f32,
        z: f32,
        yaw: f32,
        sprinting: bool,
        ads: bool,
        crouched: bool,
        prone: bool,
        slide: bool,
        jump_started: bool
    ) -> Self {
        let dx = x - prev_x;
        let dz = z - prev_z;
        let move_len_sq = dx * dx + dz * dz;

        let move_dir = if move_len_sq <= 0.0001 {
            MoveDir::None
        } else {
            let v = vec2(dx, dz).normalize();
            let forward = vec2(yaw.cos(), yaw.sin());
            let right = vec2(-forward.y, forward.x);
            let f_dot = v.dot(forward);
            let r_dot = v.dot(right);

            if f_dot > 0.4 {
                MoveDir::Forward
            } else if f_dot < -0.4 {
                MoveDir::Backward
            } else if r_dot >= 0.0 {
                MoveDir::Right
            } else {
                MoveDir::Left
            }
        };

        Self {
            move_dir,
            sprinting,
            ads,
            crouched,
            prone,
            slide,
            jump_started,
        }
    }

    pub fn skin_from_state(state: AnimState, override_skin: Option<PlayerSkin>) -> PlayerSkin {
        if let Some(skin) = override_skin {
            return skin;
        }

        if state.jump_started {
            return PlayerSkin::Jump;
        }

        if state.slide {
            return PlayerSkin::Slide;
        }

        if state.prone {
            return match state.move_dir {
                MoveDir::Forward => PlayerSkin::ProneForward,
                MoveDir::Left => PlayerSkin::ProneTurnLeft,
                MoveDir::Right => PlayerSkin::ProneTurnRight,
                _ => PlayerSkin::ProneIdle,
            };
        }
        if state.crouched {
            if state.ads && matches!(state.move_dir, MoveDir::None) {
                return PlayerSkin::CrouchAim;
            }
            return match state.move_dir {
                MoveDir::Forward => PlayerSkin::CrouchForward,
                MoveDir::Backward => PlayerSkin::CrouchBack,
                MoveDir::Left => PlayerSkin::CrouchLeft,
                MoveDir::Right => PlayerSkin::CrouchRight,
                MoveDir::None => PlayerSkin::Crouch,
            };
        }

        if state.ads {
            return PlayerSkin::RifleAimIdle;
        }

        if state.sprinting {
            return match state.move_dir {
                MoveDir::Forward => PlayerSkin::SprintForward,
                MoveDir::Left => PlayerSkin::SprintLeft,
                MoveDir::Right => PlayerSkin::SprintRight,
                MoveDir::Backward => PlayerSkin::WalkBack,
                MoveDir::None => PlayerSkin::RifleIdle,
            };
        }

        match state.move_dir {
            MoveDir::Forward => PlayerSkin::RifleWalk,
            MoveDir::Backward => PlayerSkin::WalkBack,
            MoveDir::Left => PlayerSkin::WalkLeft,
            MoveDir::Right => PlayerSkin::WalkRight,
            MoveDir::None => PlayerSkin::RifleIdle,
        }
    }
}

