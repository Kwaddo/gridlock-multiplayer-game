use platform::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldId {
    Lobby = 0,
    Maze = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Spectator = 0,
    Red = 1,
    Blue = 2,
}

#[allow(dead_code)]
impl Team {
    pub fn from_u8(value: u8) -> Option<Team> {
        match value {
            0 => Some(Team::Spectator),
            1 => Some(Team::Red),
            2 => Some(Team::Blue),
            _ => None,
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl WorldId {
    pub fn from_u8(value: u8) -> Option<WorldId> {
        match value {
            0 => Some(WorldId::Lobby),
            1 => Some(WorldId::Maze),
            _ => None,
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

pub struct Player {
    pub id: u32,
    pub username: String,
    pub position: (f32, f32, f32),
    pub yaw: f32,
    pub pitch: f32,
    pub health: u32,
    pub max_health: u32,
    pub is_alive: bool,
    pub kills: u32,
    pub deaths: u32,
    pub gun_level: u32,
    pub last_activity: Instant,
    pub death_time: Option<Instant>,
    pub world: WorldId, // where is this player right now? lobby, maze, etc

    // per-player post-teleport freeze (server-side): when set, player cannot move or shoot until Instant
    pub post_teleport_freeze_until: Option<Instant>,

    // anim state
    pub is_sprinting: bool,
    pub is_ads: bool,
    pub is_crouched: bool,
    pub is_airborne: bool,
    pub is_prone: bool,
    pub is_sliding: bool,

    pub evt_jump_start: bool, // for precise jump animation timing
    pub evt_landed: bool,     // for precise landing animation timing
    // team for team-based modes
    pub team: Team,
    // player's currently assigned primary weapon (knife is implicitly available)
    pub current_weapon: String,

    // bot shit
    pub is_bot: bool,
    pub is_silent: bool, // no footstep sounds
    // whether server auto-respawn loop should revive this player after death
    pub auto_respawn: bool,
    // Regen delay anchor: health won't regenerate until this instant has passed.
    pub regen_resume_at: Option<Instant>,
}

impl Player {
    pub fn new(id: u32, username: String) -> Player {
        Player {
            id,
            username,
            position: (0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            health: 125,
            max_health: 125,
            is_alive: true,
            kills: 0,
            deaths: 0,
            gun_level: 1,
            last_activity: Instant::now(),
            death_time: None,
            world: WorldId::Lobby, // start in the lobby by default
            post_teleport_freeze_until: None,

            is_sprinting: false,
            is_ads: false,
            is_crouched: false,
            is_airborne: false,
            is_prone: false,
            is_sliding: false,
            evt_jump_start: false,
            evt_landed: false,
            team: Team::Spectator,
            current_weapon: "Knife".to_string(),
            is_bot: false,
            is_silent: false,
            auto_respawn: true,
            regen_resume_at: None,
        }
    }
}
