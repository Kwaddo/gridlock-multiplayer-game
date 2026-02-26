pub const MSG_JOIN: u8 = 1;
pub const MSG_WELCOME: u8 = 2; // server -> client: player id, username
pub const MSG_POSITION: u8 = 3;
pub const MSG_STATE: u8 = 4;
pub const MSG_SHOOT: u8 = 5;
pub const MSG_HIT: u8 = 6;
pub const MSG_DISCONNECT: u8 = 7;
pub const MSG_MAZE: u8 = 8;
pub const MSG_WORLD_CHANGE: u8 = 9; // server -> client: bi-directional world transition
pub const MSG_ASSETS: u8 = 10; // server -> client: asset manifest per world
pub const MSG_DISCOVER: u8 = 11; // client -> server: LAN discovery broadcast
pub const MSG_WALL_BROKEN: u8 = 12; // client <-> server: breakable wall destroyed (x, y)
pub const MSG_DEATH: u8 = 13; // server -> client: you died (killer_id, weapon)
pub const MSG_RESPAWN_REQUEST: u8 = 14; // client -> server: i want to respawn
pub const MSG_RESPAWN: u8 = 15; // server -> client: respawn at position (x, y, z) + optional spawn-kind byte (0=random,1=teleporter-fallback)
pub const MSG_DAMAGE: u8 = 16; // server -> client: you took damage (amount, attacker_id)
pub const MSG_KILL: u8 = 17; // server -> client: you killed someone (victim_id, weapon)
pub const MSG_KILL_FEED: u8 = 18; // server -> all: broadcast kill (killer_id, victim_id)
pub const MSG_START_GAME: u8 = 19; // client -> server: request to start a match
// format: [MSG_START_GAME][mode:1][duration_hi:1][duration_lo:1]
// duration is a big-endian u16 representing seconds, where 0=use server default,
// 65535=infinite.  only the first mode byte is required for backward
// compatibility; new clients always append two bytes.

                                   // Game mode identifiers used with MSG_START_GAME
pub const MODE_GUN_GAME: u8 = 1;
pub const MODE_TEAM_DEATHMATCH: u8 = 2;
pub const MODE_SEARCH_AND_DESTROY: u8 = 3;
pub const MSG_START_DENIED: u8 = 20; // server -> client: start request rejected (match running / full)
pub const MSG_MATCH_JOINED: u8 = 21; // server -> all: announce player joined running match (id, name)
pub const MSG_MATCH_END: u8 = 22; // server -> all: announce match end (winner_id, name, countdown_secs)
pub const MSG_PLAYER_LIST: u8 = 23; // server -> client: full connected player list (id,name,kills,world,is_alive)
pub const MSG_MATCH_COUNTDOWN: u8 = 24; // server -> all: match start countdown (seconds)
pub const MSG_MATCH_TIMER: u8 = 25; // server -> all: authoritative match time remaining (seconds)
// format: [MSG_MATCH_TIMER][secs_hi:1][secs_lo:1] (big-endian u16)
// value 0 means time is up; max representable duration is 65535s (~18h).
pub const MSG_SET_TEAM: u8 = 26; // client -> server: set preferred team (team:u8)
pub const MSG_SND_ROUND_STATE: u8 = 27; // server -> all: S&D round state
pub const MSG_SND_BOMB_PICKUP: u8 = 28; // client -> server: request to pick up dropped bomb
pub const MSG_SND_PLANT_START: u8 = 29; // client -> server: start planting bomb
pub const MSG_SND_PLANT_STOP: u8 = 30; // client -> server: stop planting bomb
pub const MSG_SND_DEFUSE_START: u8 = 31; // client -> server: start defusing bomb
pub const MSG_SND_DEFUSE_STOP: u8 = 32; // client -> server: stop defusing bomb

// when client switches active weapon; payload = [len:1][UTF8 name]
pub const MSG_WEAPON_CHANGE: u8 = 33; // client -> server: notify active weapon


// bot bulshit
pub const MSG_BOT_SPAWN: u8 = 50;
pub const MSG_BOT_DESPAWN_ALL: u8 = 51;
pub const MSG_BOT_SHOT: u8 = 52; // server -> client: bot fired (shooter_id, victim_id)

pub const FLAG_IS_BOT: u8 = 1 << 6; // player list flag indicating this player is a bot

// State flags for player movement and actions, sent in the position message
pub const FLAG_SPRINTING: u8 = 1 << 0;
pub const FLAG_CROUCHING: u8 = 1 << 1;
pub const FLAG_ADS: u8 = 1 << 2;
pub const FLAG_AIRBORNE: u8 = 1 << 3;
pub const FLAG_PRONE: u8 = 1 << 4;
pub const FLAG_SLIDING: u8 = 1 << 5;

// Event flags for the state message
pub const EVT_JUMP_START: u8 = 1 << 0;
pub const EVT_LANDED: u8 = 1 << 1;
pub const EVT_SLIDE_START: u8 = 1 << 2;
pub const EVT_SLIDE_END: u8 = 1 << 3;
