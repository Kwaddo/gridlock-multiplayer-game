use crate::types::animation_state::{AnimState, MoveDir};
use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, EntityKind, Hitbox};
use macroquad::prelude::*;

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
}

impl PlayerSkin {
    pub const ALL: [PlayerSkin; 26] = [
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
    ];

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
            PlayerSkin::Slide => Some("anim_slide"),
            PlayerSkin::ProneIdle => Some("anim_prone_idle"),
            PlayerSkin::ProneForward => Some("anim_prone_forward"),
            PlayerSkin::ProneTurnRight => Some("anim_prone_turn_right"),
            PlayerSkin::ProneTurnLeft => Some("anim_prone_turn_left"),
            PlayerSkin::StandDeath => Some("anim_stand_death"),
            PlayerSkin::CrouchDeath => Some("anim_crouch_death"),
            PlayerSkin::ProneDeath => Some("anim_prone_death"),
            PlayerSkin::Twerk => Some("anim_twerk"),
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
            PlayerSkin::Twerk => PlayerSkin::RifleIdle,
        }
    }
}

pub struct OtherPlayer {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub render_x: f32,
    pub render_y: f32,
    pub render_z: f32,
    pub render_yaw: f32,
    has_snapshot: bool,
    pub username: String,
    pub skin: Option<Texture2D>,
    pub skin_type: PlayerSkin,
    pub is_sprinting: bool,
    pub is_crouching: bool,
    pub is_prone: bool,
    pub is_sliding: bool,
    pub is_ads: bool,
    pub is_airborne: bool,
    pub evt_jump_start: bool,
    pub evt_landed: bool,
    pub color: Color,
    pub world: WorldId,
    pub hp: f32,
    pub max_hp: f32,
    pub hit_timer: f32,
    pub hitboxes: Vec<Hitbox>,
    pub kills: u32,
    pub sprint_grace: f32,
    // smoothing / hysteresis for rapid move-direction changes (prevents animation flicker)
    pub last_move_dir: MoveDir,
    pub pending_move_dir: Option<MoveDir>,
    pub pending_move_timer: f32,
    // previous snapshot position used for animation delta calculation
    pub prev_x: f32,
    pub prev_z: f32,
    // grace timer that keeps current walk/run animation briefly after player stops
    pub move_anim_grace: f32,
    // timestamp of the last frame where position actually changed (for idle detection)
    pub last_move_time: f64,

    // Gun Game progression: 1 = first weapon slot
    pub gun_level: u8,
    pub team: u8,

    // Current weapon name (e.g. "AK-47", "M4A1") — used to draw world-model gun
    pub current_weapon: String,

    // bot
    pub is_bot: bool,
    pub is_silent: bool, // shadow bot: no footstep sounds

    // footstep audio
    pub footstep_timer: f32,
}

impl OtherPlayer {
    const POSITION_SMOOTH_RATE: f32 = 22.0;
    const YAW_SMOOTH_RATE: f32 = 18.0;
    const SNAP_DISTANCE: f32 = 5.0;
    /// how long we continue treating a player as "sprinting" after the last sprint flag
    /// (helps hide missing/late network packets and stops walk/sprint flicker)
    pub const SPRINT_GRACE: f32 = 0.18; // seconds
    /// how long we wait before accepting a new movement direction (reduces walk/side animation flicker)
    pub const MOVE_DIR_GRACE: f32 = 0.12; // seconds

    pub fn color_for_id(id: u32) -> Color {
        let r = ((id * 73 + 29) % 256) as u8;
        let g = ((id * 97 + 53) % 256) as u8;
        let b = ((id * 61 + 19) % 256) as u8;
        Color::from_rgba(r, g, b, 255)
    }

    pub fn new(id: u32, username: String, world: WorldId) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            render_x: 0.0,
            render_y: 0.0,
            render_z: 0.0,
            render_yaw: 0.0,
            has_snapshot: false,
            username,
            skin: None,
            skin_type: PlayerSkin::RifleIdle,
            is_sprinting: false,
            is_crouching: false,
            is_prone: false,
            is_sliding: false,
            is_ads: false,
            is_airborne: false,
            evt_jump_start: false,
            evt_landed: false,
            color: Self::color_for_id(id),
            world,
            team: 0,
            hp: 100.0,
            max_hp: 100.0,
            hit_timer: 0.0,
            hitboxes: default_player_hitboxes(),
            kills: 0,
            sprint_grace: 0.0,
            last_move_dir: MoveDir::None,
            pending_move_dir: None,
            pending_move_timer: 0.0,
            prev_x: 0.0,
            prev_z: 0.0,
            move_anim_grace: 0.0,
            last_move_time: 0.0,
            gun_level: 1,
            current_weapon: String::new(),
            is_bot: false,
            is_silent: false,
            footstep_timer: 0.0,
        }
    }

    pub fn apply_snapshot(&mut self, x: f32, y: f32, z: f32, yaw: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.yaw = yaw;

        if !self.has_snapshot {
            self.render_x = x;
            self.render_y = y;
            self.render_z = z;
            self.render_yaw = yaw;
            self.has_snapshot = true;
            return;
        }

        let dx = self.render_x - x;
        let dy = self.render_y - y;
        let dz = self.render_z - z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq > Self::SNAP_DISTANCE * Self::SNAP_DISTANCE {
            self.render_x = x;
            self.render_y = y;
            self.render_z = z;
            self.render_yaw = yaw;
        }
    }

    pub fn render_position(&self) -> Vec3 {
        vec3(self.render_x, self.render_y, self.render_z)
    }

    pub fn smooth_render_state(&mut self, delta: f32) {
        if !self.has_snapshot {
            return;
        }

        let pos_alpha = 1.0 - (-Self::POSITION_SMOOTH_RATE * delta).exp();
        self.render_x += (self.x - self.render_x) * pos_alpha;
        self.render_y += (self.y - self.render_y) * pos_alpha;
        self.render_z += (self.z - self.render_z) * pos_alpha;

        let yaw_alpha = 1.0 - (-Self::YAW_SMOOTH_RATE * delta).exp();
        self.render_yaw = lerp_angle(self.render_yaw, self.yaw, yaw_alpha);
    }
}

impl CombatEntity for OtherPlayer {
    fn entity_kind(&self) -> EntityKind {
        EntityKind::Player
    }

    fn entity_id(&self) -> Option<u32> {
        Some(self.id)
    }

    fn entity_position(&self) -> Vec3 {
        self.render_position()
    }

    fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }

    fn take_damage(&mut self, dmg: f32) {
        self.hp = (self.hp - dmg).max(0.0);
        self.hit_timer = 0.3; // flash red for 300ms
    }

    fn is_alive(&self) -> bool {
        self.hp > 0.0
    }

    fn update(&mut self, delta: f32) {
        self.smooth_render_state(delta);
        if self.hit_timer > 0.0 {
            self.hit_timer = (self.hit_timer - delta).max(0.0);
        }
        if self.sprint_grace > 0.0 {
            self.sprint_grace = (self.sprint_grace - delta).max(0.0);
        }
        if self.move_anim_grace > 0.0 {
            self.move_anim_grace = (self.move_anim_grace - delta).max(0.0);
        }

        if self.pending_move_timer > 0.0 {
            self.pending_move_timer = (self.pending_move_timer - delta).max(0.0);
            if self.pending_move_timer == 0.0 {
                if let Some(pd) = self.pending_move_dir.take() {
                    self.last_move_dir = pd;
                }
            }
        }

        // Transition to idle if no position change has been seen for a short
        // grace period. This runs per-frame so it's independent of broadcast
        // frequency, and avoids flicker caused by broadcasts where this
        // player's position didn't change.
        if self.last_move_dir != MoveDir::None {
            let now = macroquad::time::get_time();
            if now - self.last_move_time > 0.12 {
                self.last_move_dir = MoveDir::None;
                self.pending_move_dir = None;
                self.pending_move_timer = 0.0;
                let anim = AnimState::from_movement(
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    self.yaw,
                    false,
                    self.is_ads,
                    self.is_crouching,
                    self.is_prone,
                    self.is_sliding,
                    false,
                );
                self.skin_type = AnimState::skin_from_state(anim, None);
                // snap render position so model doesn't slide
                self.render_x = self.x;
                self.render_z = self.z;
            }
        }
    }

    fn hit_timer(&self) -> f32 {
        self.hit_timer
    }

    fn entity_color(&self) -> Color {
        self.color
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut delta = to - from;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    from + delta * t
}
