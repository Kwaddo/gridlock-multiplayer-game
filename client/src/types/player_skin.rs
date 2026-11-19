#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerSkin {
    Texture,
    RifleIdle,
    RifleAimIdle,
    RifleWalk,
    WalkBack,
    WalkLeft,
    WalkRight,
    SprintForward,
    SprintLeft,
    SprintRight,
    Crouch,
    CrouchAim,
    CrouchForward,
    CrouchBack,
    CrouchLeft,
    CrouchRight,
    Jump,
    Slide,
    ProneIdle,
    ProneForward,
    ProneTurnRight,
    ProneTurnLeft,
    StandDeath,
    CrouchDeath,
    ProneDeath,
    Twerk,
    Breakdance,
    RunningMan,
    Gangnam,
    Moonwalk,
    // new clips (2026-11-09): crawl back, slow walk, the 3 part jump, deaths by direction and headshot
    ProneBack,
    SlowWalkForward,
    JumpStart,
    FallLoop,
    Land,
    DeathFront,
    DeathBack,
    DeathSide,
    DeathFrontHeadshot,
    DeathBackHeadshot,
    RifleFire, // upper body only, layered over whatever the legs do
}

impl PlayerSkin {
    pub const ALL: [PlayerSkin; 41] = [
        PlayerSkin::Texture,
        PlayerSkin::RifleIdle,
        PlayerSkin::RifleAimIdle,
        PlayerSkin::RifleWalk,
        PlayerSkin::WalkBack,
        PlayerSkin::WalkLeft,
        PlayerSkin::WalkRight,
        PlayerSkin::SprintForward,
        PlayerSkin::SprintLeft,
        PlayerSkin::SprintRight,
        PlayerSkin::Crouch,
        PlayerSkin::CrouchAim,
        PlayerSkin::CrouchForward,
        PlayerSkin::CrouchBack,
        PlayerSkin::CrouchLeft,
        PlayerSkin::CrouchRight,
        PlayerSkin::Jump,
        PlayerSkin::Slide,
        PlayerSkin::ProneIdle,
        PlayerSkin::ProneForward,
        PlayerSkin::ProneTurnRight,
        PlayerSkin::ProneTurnLeft,
        PlayerSkin::StandDeath,
        PlayerSkin::CrouchDeath,
        PlayerSkin::ProneDeath,
        PlayerSkin::Twerk,
        PlayerSkin::Breakdance,
        PlayerSkin::RunningMan,
        PlayerSkin::Gangnam,
        PlayerSkin::Moonwalk,
        PlayerSkin::ProneBack,
        PlayerSkin::SlowWalkForward,
        PlayerSkin::JumpStart,
        PlayerSkin::FallLoop,
        PlayerSkin::Land,
        PlayerSkin::DeathFront,
        PlayerSkin::DeathBack,
        PlayerSkin::DeathSide,
        PlayerSkin::DeathFrontHeadshot,
        PlayerSkin::DeathBackHeadshot,
        PlayerSkin::RifleFire,
    ];

    /// Emote wheel clips, wire id is index + 1 (0 = none).
    pub const EMOTES: [PlayerSkin; 5] = [
        PlayerSkin::Twerk,
        PlayerSkin::Breakdance,
        PlayerSkin::RunningMan,
        PlayerSkin::Gangnam,
        PlayerSkin::Moonwalk,
    ];

    pub fn emote(id: u8) -> Option<PlayerSkin> {
        (id as usize).checked_sub(1).and_then(|i| Self::EMOTES.get(i)).copied()
    }

    pub fn is_emote(self) -> bool {
        Self::EMOTES.contains(&self)
    }

    pub const fn is_model_based(self) -> bool {
        !matches!(self, PlayerSkin::Texture)
    }

    pub const fn model_key(self) -> Option<&'static str> {
        match self {
            PlayerSkin::Texture => None,
            PlayerSkin::RifleIdle => Some("anim_rifle_idle"),
            PlayerSkin::RifleAimIdle => Some("anim_rifle_aim_idle"),
            PlayerSkin::RifleWalk => Some("anim_walk_forward"),
            PlayerSkin::WalkBack => Some("anim_walk_back"),
            PlayerSkin::WalkLeft => Some("anim_walk_left"),
            PlayerSkin::WalkRight => Some("anim_walk_right"),
            PlayerSkin::SprintForward => Some("anim_sprint_forward"),
            PlayerSkin::SprintLeft => Some("anim_sprint_left"),
            PlayerSkin::SprintRight => Some("anim_sprint_right"),
            PlayerSkin::Crouch => Some("anim_crouch"),
            PlayerSkin::CrouchAim => Some("anim_crouch_aim"),
            PlayerSkin::CrouchForward => Some("anim_crouch_forward"),
            PlayerSkin::CrouchBack => Some("anim_crouch_back"),
            PlayerSkin::CrouchLeft => Some("anim_crouch_left"),
            PlayerSkin::CrouchRight => Some("anim_crouch_right"),
            PlayerSkin::Jump => Some("anim_jump"),
            // no slide clip anymore (slide was cut), sprint stands in, before this it was anim_slide
            PlayerSkin::Slide => Some("anim_sprint_forward"),
            PlayerSkin::ProneIdle => Some("anim_prone_idle"),
            PlayerSkin::ProneForward => Some("anim_prone_forward"),
            // prone turn clips were dropped (a/d is off while prone), old clients still send these
            PlayerSkin::ProneTurnRight | PlayerSkin::ProneTurnLeft => Some("anim_prone_forward"),
            // the old single stand death is gone, the front death stands in until death choosers pick by direction
            PlayerSkin::StandDeath => Some("anim_death_front"),
            PlayerSkin::CrouchDeath => Some("anim_crouch_death"),
            PlayerSkin::ProneDeath => Some("anim_prone_death"),
            PlayerSkin::Twerk => Some("anim_emote_twerk"),
            PlayerSkin::Breakdance => Some("anim_emote_breakdance"),
            PlayerSkin::RunningMan => Some("anim_emote_running_man"),
            PlayerSkin::Gangnam => Some("anim_emote_gangnam"),
            PlayerSkin::Moonwalk => Some("anim_emote_moonwalk"),
            PlayerSkin::ProneBack => Some("anim_prone_back"),
            PlayerSkin::SlowWalkForward => Some("anim_slow_walk_forward"),
            PlayerSkin::JumpStart => Some("anim_jump_start"),
            PlayerSkin::FallLoop => Some("anim_fall_loop"),
            PlayerSkin::Land => Some("anim_land"),
            PlayerSkin::DeathFront => Some("anim_death_front"),
            PlayerSkin::DeathBack => Some("anim_death_back"),
            // no left death clip, the right one covers both sides (ahmed's call)
            PlayerSkin::DeathSide => Some("anim_death_right"),
            PlayerSkin::DeathFrontHeadshot => Some("anim_death_front_headshot"),
            PlayerSkin::DeathBackHeadshot => Some("anim_death_back_headshot"),
            PlayerSkin::RifleFire => Some("anim_rifle_fire"),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            PlayerSkin::Texture => "Texture",
            PlayerSkin::RifleIdle => "Rifle Idle",
            PlayerSkin::RifleAimIdle => "Rifle Aim Idle",
            PlayerSkin::RifleWalk => "Walk Forward",
            PlayerSkin::WalkBack => "Walk Back",
            PlayerSkin::WalkLeft => "Walk Left",
            PlayerSkin::WalkRight => "Walk Right",
            PlayerSkin::SprintForward => "Sprint Forward",
            PlayerSkin::SprintLeft => "Sprint Left",
            PlayerSkin::SprintRight => "Sprint Right",
            PlayerSkin::Crouch => "Crouch",
            PlayerSkin::CrouchAim => "Crouch Aim",
            PlayerSkin::CrouchForward => "Crouch Forward",
            PlayerSkin::CrouchBack => "Crouch Back",
            PlayerSkin::CrouchLeft => "Crouch Left",
            PlayerSkin::CrouchRight => "Crouch Right",
            PlayerSkin::Jump => "Jump",
            PlayerSkin::Slide => "Slide",
            PlayerSkin::ProneIdle => "Prone Idle",
            PlayerSkin::ProneForward => "Prone Forward",
            PlayerSkin::ProneTurnRight => "Prone Turn Right",
            PlayerSkin::ProneTurnLeft => "Prone Turn Left",
            PlayerSkin::StandDeath => "Stand Death",
            PlayerSkin::CrouchDeath => "Crouch Death",
            PlayerSkin::ProneDeath => "Prone Death",
            PlayerSkin::Twerk => "Twerk",
            PlayerSkin::Breakdance => "Breakdance",
            PlayerSkin::RunningMan => "Running Man",
            PlayerSkin::Gangnam => "Gangnam",
            PlayerSkin::Moonwalk => "Moonwalk",
            PlayerSkin::ProneBack => "Prone Back",
            PlayerSkin::SlowWalkForward => "Slow Walk Forward",
            PlayerSkin::JumpStart => "Jump Start",
            PlayerSkin::FallLoop => "Fall Loop",
            PlayerSkin::Land => "Land",
            PlayerSkin::DeathFront => "Death Front",
            PlayerSkin::DeathBack => "Death Back",
            PlayerSkin::DeathSide => "Death Side",
            PlayerSkin::DeathFrontHeadshot => "Death Front Headshot",
            PlayerSkin::DeathBackHeadshot => "Death Back Headshot",
            PlayerSkin::RifleFire => "Rifle Fire",
        }
    }

    pub fn next(self) -> Self {
        // Hardcoded test flow:
        // Walk -> Sprint -> Jump -> StandDeath
        // Crouch -> CrouchDeath
        // Prone -> ProneDeath
        // Extras
        // Edit this match when you want a different manual animation sequence.
        match self {
            PlayerSkin::Texture => PlayerSkin::RifleIdle,

            // Walk / sprint / jump -> stand death
            PlayerSkin::RifleIdle => PlayerSkin::RifleAimIdle,
            PlayerSkin::RifleAimIdle => PlayerSkin::RifleWalk,
            PlayerSkin::RifleWalk => PlayerSkin::WalkBack,
            PlayerSkin::WalkBack => PlayerSkin::WalkLeft,
            PlayerSkin::WalkLeft => PlayerSkin::WalkRight,
            PlayerSkin::WalkRight => PlayerSkin::SprintForward,
            PlayerSkin::SprintForward => PlayerSkin::SprintLeft,
            PlayerSkin::SprintLeft => PlayerSkin::SprintRight,
            PlayerSkin::SprintRight => PlayerSkin::Jump,
            PlayerSkin::Jump => PlayerSkin::StandDeath,
            PlayerSkin::StandDeath => PlayerSkin::Crouch,

            // Crouch -> crouch death
            PlayerSkin::Crouch => PlayerSkin::CrouchAim,
            PlayerSkin::CrouchAim => PlayerSkin::CrouchForward,
            PlayerSkin::CrouchForward => PlayerSkin::CrouchBack,
            PlayerSkin::CrouchBack => PlayerSkin::CrouchLeft,
            PlayerSkin::CrouchLeft => PlayerSkin::CrouchRight,
            PlayerSkin::CrouchRight => PlayerSkin::CrouchDeath,
            PlayerSkin::CrouchDeath => PlayerSkin::ProneIdle,

            // Prone -> prone death
            PlayerSkin::ProneIdle => PlayerSkin::ProneForward,
            PlayerSkin::ProneForward => PlayerSkin::ProneTurnRight,
            PlayerSkin::ProneTurnRight => PlayerSkin::ProneTurnLeft,
            PlayerSkin::ProneTurnLeft => PlayerSkin::ProneDeath,

            // Extras
            PlayerSkin::ProneDeath => PlayerSkin::Slide,
            PlayerSkin::Slide => PlayerSkin::Twerk,
            PlayerSkin::Twerk => PlayerSkin::Breakdance,
            PlayerSkin::Breakdance => PlayerSkin::RunningMan,
            PlayerSkin::RunningMan => PlayerSkin::Gangnam,
            PlayerSkin::Gangnam => PlayerSkin::Moonwalk,
            PlayerSkin::Moonwalk => PlayerSkin::ProneBack,
            PlayerSkin::ProneBack => PlayerSkin::SlowWalkForward,
            PlayerSkin::SlowWalkForward => PlayerSkin::JumpStart,
            PlayerSkin::JumpStart => PlayerSkin::FallLoop,
            PlayerSkin::FallLoop => PlayerSkin::Land,
            PlayerSkin::Land => PlayerSkin::DeathFront,
            PlayerSkin::DeathFront => PlayerSkin::DeathBack,
            PlayerSkin::DeathBack => PlayerSkin::DeathSide,
            PlayerSkin::DeathSide => PlayerSkin::DeathFrontHeadshot,
            PlayerSkin::DeathFrontHeadshot => PlayerSkin::DeathBackHeadshot,
            PlayerSkin::DeathBackHeadshot => PlayerSkin::RifleFire,
            PlayerSkin::RifleFire => PlayerSkin::RifleIdle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emote_ids_map_to_clips() {
        assert_eq!(PlayerSkin::emote(0), None);
        assert_eq!(PlayerSkin::emote(6), None);
        assert_eq!(PlayerSkin::emote(255), None);
        assert_eq!(PlayerSkin::emote(1), Some(PlayerSkin::Twerk));
        assert_eq!(PlayerSkin::emote(5), Some(PlayerSkin::Moonwalk));
    }

    #[test]
    fn emotes_use_emote_models() {
        for s in PlayerSkin::EMOTES {
            assert!(s.model_key().unwrap().starts_with("anim_emote_"));
        }
    }
}
