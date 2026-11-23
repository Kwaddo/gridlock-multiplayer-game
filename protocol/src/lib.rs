//! Wire protocol shared by client and server.

pub mod codec;
mod messages;

pub use messages::*;

/// Wire format version. Bump it whenever any message changes shape: a client
/// and a host only play together when theirs match (see `ClientMsg::Join`,
/// `ServerMsg::Welcome`). Builds from before the check have no version on
/// the wire and count as 1.
// 3: pitch added to MSG_STATE (one byte) and MSG_POSITION, so a 2 cant read a 3's state packets
pub const PROTOCOL_VERSION: u16 = 6; // 4 added cardboard shit, 5 added smoke break, 6 added the S&D bomb_dropped byte

/// What the server tells a client of another version once it has let it in
/// far enough to be heard: a chat notice is all an old build can show.
pub const VERSION_MISMATCH_NOTICE: &str =
    "This game runs a different version of Gridlock. Update your game to join it.";

// MESSAGE TYPES

// Message types - first byte of every packet
pub const MSG_JOIN: u8 = 1; // client -> server: username
pub const MSG_WELCOME: u8 = 2; // server -> client: player id, username
pub const MSG_POSITION: u8 = 3; // client -> server: x, y, yaw
pub const MSG_STATE: u8 = 4; // server -> client: all player positions
pub const MSG_SHOOT: u8 = 5; // client -> server: I fired, tracer end point (relayed to others as `MSG_SHOTS`)
pub const MSG_HIT: u8 = 6; // server -> client: you got hit
pub const MSG_DISCONNECT: u8 = 7; // client -> server: leaving
pub const MSG_MAZE: u8 = 8; // server -> client: maze layout
pub const MSG_WORLD_CHANGE: u8 = 9; // server -> client: bi-directional world transition
                                    // Format: [MSG_WORLD_CHANGE][world:1][spawn_x:4][spawn_y:4][spawn_z:4]
                                    //         [optional:mode:1][optional:freeze_secs:1][optional:match_timer_secs:1]
                                    // The additional bytes are only included when transitioning into the maze (match start)
                                    // so clients can synchronise timers even if packets reorder.  match_timer_secs uses the
                                    // same clamped-to-255 value as MSG_MATCH_TIMER.

// client -> server: active weapon changed (UTF‑8 name length + bytes)
pub const MSG_WEAPON_CHANGE: u8 = 33;
pub const MSG_LOBBY_LEADER: u8 = 34; // server -> all: host player id [MSG_LOBBY_LEADER][id:4] (0 = none); the first human to
                                    // join, unchanged while a match runs: starts and ends matches
pub const MSG_CHAT: u8 = 35; // client -> server: [text:str8]
                            // server -> all: [from_id:4][name:str8][text:str8] (from_id 0 = server notice)
pub const MSG_PLAYER_JOINED: u8 = 36; // server -> others: a player connected [id:4][name:str8]
pub const MSG_PLAYER_LEFT: u8 = 37; // server -> all: a player left [id:4][name:str8][reason:1] (see LEAVE_*)
pub const MSG_END_MATCH: u8 = 38; // client -> server: host ends the running match, everyone returns to the lobby

pub const MSG_MATCH_STATUS: u8 = 39; // server -> all: match phase [status:1] (see MATCH_STATUS_*), on change and every few seconds

pub const MSG_TEAM_SCORE: u8 = 40; // server -> all, team modes: [red:u16][blue:u16][to_win:u16] (big-endian),
                                  // kills each team has scored this match; sent on change and every few seconds
pub const MSG_LOBBY_SETTINGS: u8 = 41; // both ways, see `LobbySettings`: the host -> server when it changes
                                       // its lobby console; server -> all on change and to each joiner
pub const MSG_PLAYER_INFO: u8 = 42; // server -> client: what rarely changes about players (see `PlayerInfo`):
                                    // the changed ones as they change, everyone to a joiner and every few seconds

// MSG_MATCH_STATUS values
pub const MATCH_STATUS_NONE: u8 = 0; // lobby only: the host can start a match
pub const MATCH_STATUS_STARTING: u8 = 1; // warmup countdown running
pub const MATCH_STATUS_RUNNING: u8 = 2; // match on: lobby players can join it
pub const MATCH_STATUS_ENDING: u8 = 3; // winner shown, everyone returns to the lobby shortly

// MSG_START_DENIED reasons (no byte = match running and full)
pub const START_DENIED_NOT_HOST: u8 = 1;
pub const START_DENIED_STARTING: u8 = 2;
pub const START_DENIED_ENDING: u8 = 3;
pub const START_DENIED_TOO_FEW: u8 = 4; // smoke break needs 2 humans

// MSG_PLAYER_LEFT reasons
pub const LEAVE_QUIT: u8 = 0;
pub const LEAVE_TIMED_OUT: u8 = 1;
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
pub const MODE_SMOKE_BREAK: u8 = 4;

/// Friendly fire is off in the team modes: a hit on someone of your own
/// team (both Red = 1, or both Blue = 2) does nothing. Gun Game puts
/// everyone on no team (0), so every hit there counts.
pub fn is_friendly_fire(mode: u8, attacker_team: u8, victim_team: u8) -> bool {
    matches!(mode, MODE_TEAM_DEATHMATCH | MODE_SEARCH_AND_DESTROY)
        && matches!(attacker_team, 1 | 2)
        && attacker_team == victim_team
}

// Auto-fill bot limits for MSG_START_GAME, shared so the host's lobby console
// offers exactly what the server accepts
pub const MAX_BOTS_PER_TEAM: u8 = 8; // team modes, per team
pub const MAX_FFA_BOTS: u8 = 15; // Gun Game total (one per bot personality)
pub const MATCH_MAX_PLAYERS: usize = 16; // humans plus bots in a running match

/// Where `total` auto-fill bots go in a team mode with `red_humans` and
/// `blue_humans` already placed: each bot joins whichever team is smaller
/// (Red on a tie), so team sizes never differ by more than one once they
/// are evened out. Returns the teams in spawn order, so a match too full
/// for every bot drops the last ones and stays balanced.
pub fn balanced_bot_order(red_humans: usize, blue_humans: usize, total: usize) -> Vec<bool> {
    let (mut red, mut blue) = (red_humans, blue_humans);
    (0..total)
        .map(|_| {
            let to_red = red <= blue;
            if to_red { red += 1 } else { blue += 1 }
            to_red
        })
        .collect()
}
pub const MSG_START_DENIED: u8 = 20; // server -> client: start request rejected (match running / full)
pub const MSG_MATCH_JOINED: u8 = 21; // server -> all: announce player joined a running match (id, name)
pub const MSG_MATCH_END: u8 = 22; // server -> all: announce match end (winner_id, name, countdown_secs)
pub const MSG_PLAYER_LIST: u8 = 23; // server -> client: full connected player list (id,name,kills,deaths,world,is_alive,gun_level,team,weapon,is_bot)
pub const MSG_MATCH_COUNTDOWN: u8 = 24; // server -> all: match start countdown (seconds)
pub const MSG_MATCH_TIMER: u8 = 25; // server -> all: authoritative match time remaining (seconds)
                                    // format: [MSG_MATCH_TIMER][secs_hi:1][secs_lo:1] (big-endian u16)
                                    // value 0 means time is up; max representable duration is 65535s (~18h).
pub const MSG_SET_TEAM: u8 = 26; // client -> server: set preferred team (team:u8)
pub const MSG_SND_ROUND_STATE: u8 = 27; // server -> all: S&D round state (round:1, max_rounds:1, red_wins:1, blue_wins:1, phase:1, bomb_carrier_id:4, bomb_x:f32, bomb_y:f32, bomb_z:f32, bomb_site_x:f32, bomb_site_y:f32, bomb_site_z:f32, bomb_planted:1, plant_progress:f32, defuse_progress:f32, phase_timer:f32, bomb_dropped:1)
pub const MSG_SND_BOMB_PICKUP: u8 = 28; // client -> server: request to pick up dropped bomb
pub const MSG_SND_PLANT_START: u8 = 29; // client -> server: start planting bomb
pub const MSG_SND_PLANT_STOP: u8 = 30; // client -> server: stop planting bomb
pub const MSG_SND_DEFUSE_START: u8 = 31; // client -> server: start defusing bomb
pub const MSG_SND_DEFUSE_STOP: u8 = 32; // client -> server: stop defusing bomb

// carboard yeet
pub const MSG_BOXES: u8 = 43; // server -> client, every box in one world
pub const MSG_BOX_ENTER: u8 = 44; // client -> server, let me in the box 
pub const MSG_BOX_EXIT: u8 = 45; // client -> server, let me out of the box and leave it here
pub const MSG_BOX_SHOT: u8 = 46; // client -> server, I shot this box (mine from inside, or anyone's), its gone

pub const BOX_ENTER_RANGE: f32 = 2.0; // flat metres, server checks it, client picks a box with it

// smoke break
pub const MSG_SMOKE_ROUND: u8 = 47; // server -> each player, the round as they see it, see 'SmokeRound'
pub const MSG_SMOKE_COINS: u8 = 48; // server -> hiders only, every coin still out, hunters never get this
pub const MSG_SMOKE_SCORES: u8 = 49; // server -> all, everyones points so far
pub const MSG_SMOKE_COIN_TAKEN: u8 = 54; // server -> all, someone grabbed a coin here, play the sound
pub const MSG_SMOKE_GOLD_COIN: u8 = 55; // server -> hiders only, where the golden coin is and how long it has left, or that it's gone
pub const MSG_SMOKE_GOLDEN: u8 = 56; // server -> all, which players are gold right now so their box glows
pub const MSG_SMOKE_DOTS: u8 = 57; // server -> hunters only, minimap dots on gold hiders and where they were last seen
pub const MSG_SMOKE_GO_GOLDEN: u8 = 58; // client -> server, i'm going golden
pub const MSG_SMOKE_FEED: u8 = 59; // server -> all, someone got found or someone shot a decoy, for the kill feed

pub const SMOKE_PHASE_BLIND: u8 = 0; // hunters frozen and blind, hiders spread out
pub const SMOKE_PHASE_RELEASED: u8 = 1; // hunters out, no coins yet
pub const SMOKE_PHASE_COINS: u8 = 2; // coins out, the real round
pub const SMOKE_PHASE_RESULTS: u8 = 3; // round over, scores up for a few seconds, then the next round or the end

pub const SMOKE_SIDE_HIDER: u8 = 0;
pub const SMOKE_SIDE_HUNTER: u8 = 1;
pub const SMOKE_NO_WINNER: u8 = 255; // the round isn't over yet

// bot bulshit
pub const MSG_BOT_SPAWN: u8 = 50;
pub const MSG_BOT_DESPAWN_ALL: u8 = 51;
pub const MSG_SHOTS: u8 = 52; // server -> client: shots fired by bots (one bot tick) or another player, see `Shot`
pub const MSG_BOT_ALERT: u8 = 60; // server -> the spotted player only: [bot_id:4][level:1] (BOT_ALERT_*); clients without it ignore it
pub const BOT_ALERT_SUSPICIOUS: u8 = 1; // "?": the bot noticed something of you
pub const BOT_ALERT_ENGAGED: u8 = 2; // "!": the bot has spotted you and fights

pub const FLAG_IS_BOT: u8 = 1 << 6; // player list flag indicating this player is a bot
pub const FLAG_SILENT: u8 = 1 << 7; // no footstep sounds

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

#[cfg(test)]
mod bot_split_tests {
    use super::balanced_bot_order;

    #[test]
    fn bots_even_out_the_teams() {
        // 2 v 1 humans, 3 bots: Blue first, then alternating
        assert_eq!(balanced_bot_order(2, 1, 3), vec![false, true, false]);
        // already even: Red first on a tie
        assert_eq!(balanced_bot_order(1, 1, 2), vec![true, false]);
        // lopsided humans: every bot goes to the small team until it catches up
        assert_eq!(balanced_bot_order(3, 0, 3), vec![false, false, false]);
    }
}

#[cfg(test)]
mod friendly_fire_tests {
    use super::*;

    #[test]
    fn same_team_hits_are_friendly_fire_only_in_team_modes() {
        for mode in [MODE_TEAM_DEATHMATCH, MODE_SEARCH_AND_DESTROY] {
            assert!(is_friendly_fire(mode, 1, 1));
            assert!(is_friendly_fire(mode, 2, 2));
            assert!(!is_friendly_fire(mode, 1, 2));
            assert!(!is_friendly_fire(mode, 0, 0));
        }
        // Gun Game (no teams) and no mode at all: every hit counts
        assert!(!is_friendly_fire(MODE_GUN_GAME, 1, 1));
        assert!(!is_friendly_fire(MODE_GUN_GAME, 0, 0));
        assert!(!is_friendly_fire(0, 1, 1));
    }
}
