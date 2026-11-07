use macroquad::prelude::*;
use transport::Link;

use crate::net::protocols::{
    ClientMsg, LobbySettings, ServerMsg, LEAVE_TIMED_OUT, START_DENIED_ENDING, START_DENIED_NOT_HOST,
    START_DENIED_STARTING, EVT_JUMP_START, EVT_LANDED, FLAG_ADS, FLAG_AIRBORNE, FLAG_CROUCHING,
    FLAG_IS_BOT, FLAG_PRONE, FLAG_SILENT, FLAG_SLIDING, FLAG_SPRINTING,
};
use crate::types::maze::{CeilingLightInfo, Maze, Prop, PropKind, Room};
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;

pub const MAZE_CELL_SIZE: f32 = 1.8;
pub const MAZE_WALL_HEIGHT: f32 = 2.4;

pub struct NetUpdate {
    pub maze: Option<(Maze, Vec3)>,
    pub players: Option<Vec<OtherPlayer>>,
    // world_change: when a player is teleported between worlds. the tuple contains
    // (world, spawn_coordinates, freeze_secs, optional_authoritative_match_timer)
    // timer is encoded as big-endian u16 seconds
    pub world_change: Option<(WorldId, Vec3, Option<u8>, Option<u16>)>,
    pub broken_walls: Option<Vec<(u8, u8)>>,
    pub damage_taken: Option<(f32, Option<u32>, bool)>, // (damage amount, attacker_id)
    pub kill_feed: Option<Vec<(u32, u32, String)>>, // (killer_id, victim_id, weapon)
    pub respawn_pos: Option<Vec3>,                // (x, y, z)
    pub respawn_fallback: Option<bool>,           // true if server fell back to teleporter
    pub lobby_console_message: Option<String>,
    pub match_join_announce: Option<String>,
    pub match_start_countdown: Option<u8>,
    pub match_time_left: Option<u16>,
    pub match_end_winner: Option<String>,
    pub match_end_countdown: Option<u8>,
    pub player_list: Option<Vec<(u32, String, u32, u32, WorldId, bool, u8, u8, String, bool)>>, // (id, name, kills, deaths, world, alive, gun_level, team, weapon, is_bot)
    pub self_kills: Option<u32>,
    pub self_gun_level: Option<u8>,
    pub self_hp: Option<f32>,
    pub self_max_hp: Option<f32>,
    pub match_mode: Option<u8>,
    pub self_freeze_secs: Option<u8>,
    pub bot_shots: Option<Vec<(Vec3, Vec3)>>, // (start, end)
    pub lobby_leader_id: Option<u32>,         // host player ID (0 = none)
    pub lobby_settings: Option<LobbySettings>, // the host's lobby console choices
    pub match_status: Option<u8>,             // MATCH_STATUS_*
    pub team_score: Option<TeamScore>,        // team modes: kills per team
    pub death: bool,                          // You died
    pub death_killer_id: Option<u32>,         // who killed us (from MSG_DEATH)
    pub death_weapon: Option<String>,         // weapon used to kill us (from MSG_DEATH)

    // S&D round state
    pub snd_round_state: Option<SndRoundState>,

    // chat lines and join/leave notices, oldest first
    pub chat: Vec<ChatEvent>,

    // true if poll_updates read at least one packet from the socket during this call
    pub received_any: bool,
}

/// Kills each team has scored this match, and how many win it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeamScore {
    pub red: u16,
    pub blue: u16,
    pub to_win: u16,
}

/// A chat-feed entry decoded from the server.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatEvent {
    /// A player's line.
    Line { name: String, text: String },
    /// From the server: joins, leaves, match events.
    Notice(String),
}

/// Parsed S&D round state from the server
pub struct SndRoundState {
    pub round: u8,
    pub max_rounds: u8,
    pub red_wins: u8,
    pub blue_wins: u8,
    pub phase: u8,
    pub attacking_team: u8,
    pub bomb_carrier_id: u32,
    pub bomb_dropped: Option<(f32, f32, f32)>,
    pub bomb_site: (f32, f32, f32),
    pub bomb_planted: bool,
    pub plant_progress: f32,
    pub defuse_progress: f32,
    pub phase_timer: f32,
}

pub fn send_position(link: &mut dyn Link, position: Vec3, yaw: f32) {
    let pos_msg = ClientMsg::Position {
        position: (position.x, position.y, position.z),
        yaw,
        state_flags: None,
        event_flags: None,
    }
    .encode();
    link.send(&pos_msg);
}

pub fn poll_updates(link: &mut dyn Link, player_id: u32) -> NetUpdate {
    let mut update = NetUpdate {
        maze: None,
        players: None,
        world_change: None,
        broken_walls: None,
        damage_taken: None,
        kill_feed: None,
        respawn_pos: None,
        respawn_fallback: None,
        lobby_console_message: None,
        match_join_announce: None,
        match_start_countdown: None,
        match_time_left: None,
        match_end_winner: None,
        match_end_countdown: None,
        player_list: None,
        self_kills: None,
        self_gun_level: None,
        self_hp: None,
        self_max_hp: None,
        match_mode: None,
        death: false,
        death_killer_id: None,
        death_weapon: None,
        snd_round_state: None,
        received_any: false,
        self_freeze_secs: None,
        bot_shots: None,
        lobby_leader_id: None,
        lobby_settings: None,
        match_status: None,
        team_score: None,
        chat: Vec::new(),
    };

    for packet in link.recv() {
        // mark that we received at least one packet during this poll
        update.received_any = true;

        let Some(msg) = ServerMsg::decode(&packet) else {
            continue;
        };
        match msg {
            ServerMsg::Maze(layout) => {
                let rooms = layout
                    .rooms
                    .iter()
                    .map(|r| Room { x: r.x, y: r.y, w: r.w, h: r.h })
                    .collect();
                let props = layout
                    .props
                    .iter()
                    .filter_map(|p| PropKind::from_u8(p.kind).map(|kind| Prop { kind, x: p.x, y: p.y }))
                    .collect();
                let ceiling_lights = layout
                    .lights
                    .iter()
                    .map(|l| CeilingLightInfo { x: l.x, y: l.y, variant: l.variant })
                    .collect();
                let mut maze = Maze {
                    width: layout.width,
                    height: layout.height,
                    cells: layout.cells,
                    cell_size: MAZE_CELL_SIZE,
                    wall_height: MAZE_WALL_HEIGHT,
                    teleporter: layout.teleporter,
                    rooms,
                    props,
                    ceiling_lights,
                    breakable_walls: std::collections::HashMap::new(),
                    broken_wall_scars: std::collections::HashSet::new(),
                };
                maze.spawn_breakable_walls();
                let spawn = maze.teleporter_world();
                update.maze = Some((maze, spawn));
            }
            ServerMsg::State(entries) => {
                let mut new_players = Vec::new();
                for e in entries {
                    if e.id != player_id {
                        let world = WorldId::from_u8(e.world).unwrap_or(WorldId::Lobby);
                        let state_flags = e.state_flags;
                        let event_flags = e.event_flags;
                        let mut other_player = OtherPlayer::new(e.id, e.name, world);
                        other_player.x = e.position.0;
                        other_player.y = e.position.1;
                        other_player.z = e.position.2;
                        other_player.yaw = e.yaw;
                        other_player.is_sprinting = (state_flags & FLAG_SPRINTING) != 0;
                        other_player.is_crouching = (state_flags & FLAG_CROUCHING) != 0;
                        other_player.is_prone = (state_flags & FLAG_PRONE) != 0;
                        other_player.is_sliding = (state_flags & FLAG_SLIDING) != 0;
                        other_player.is_ads = (state_flags & FLAG_ADS) != 0;
                        other_player.is_airborne = (state_flags & FLAG_AIRBORNE) != 0;
                        other_player.is_bot = (state_flags & FLAG_IS_BOT) != 0;
                        other_player.is_silent = (state_flags & FLAG_SILENT) != 0;
                        other_player.evt_jump_start = (event_flags & EVT_JUMP_START) != 0;
                        other_player.evt_landed = (event_flags & EVT_LANDED) != 0;
                        other_player.kills = e.kills;
                        other_player.gun_level = e.gun_level;
                        other_player.hp = e.health;
                        other_player.max_hp = e.max_health;
                        other_player.is_alive = e.alive;
                        // Set team and weapon before the alive check so they are always
                        // applied to this player and never bleed onto another slot.
                        other_player.team = e.team;
                        if !e.weapon.is_empty() {
                            other_player.current_weapon = e.weapon;
                        }
                        if !e.alive {
                            other_player.hp = 0.0;
                        }
                        new_players.push(other_player);
                    } else {
                        // update own-kills value for local UI/scoreboard
                        update.self_kills = Some(e.kills);
                        update.self_gun_level = Some(e.gun_level);
                        update.self_hp = Some(e.health);
                        update.self_max_hp = Some(e.max_health);
                        // If the server reports we are dead in the state packet,
                        // mark death so UI/state paths handle it even if MSG_DEATH
                        // was missed.
                        if !e.alive {
                            update.death = true;
                        }
                        // record authoritative post-teleport freeze so client can
                        // apply movement/weapon locks even if the MSG_WORLD_CHANGE
                        // packet was lost while joining the match.
                        update.self_freeze_secs = Some(e.freeze_secs);
                    }
                }
                update.players = Some(new_players);
            }
            ServerMsg::WorldChange { world, spawn, mode, freeze_secs, match_timer } => {
                let world = WorldId::from_u8(world).unwrap_or(WorldId::Lobby);
                // mode/freeze/timer only appear when entering the maze; the timer is an
                // authoritative starting value so the UI is correct even if
                // MSG_MATCH_TIMER broadcasts are dropped or reordered.
                if mode.is_some() {
                    update.match_mode = mode;
                }
                update.world_change =
                    Some((world, vec3(spawn.0, spawn.1, spawn.2), freeze_secs, match_timer));
            }
            ServerMsg::StartDenied { reason } => {
                let msg = match reason {
                    Some(START_DENIED_NOT_HOST) => "Only the host can start the game",
                    Some(START_DENIED_STARTING) => "Match is starting: you will join it",
                    Some(START_DENIED_ENDING) => "Match is ending: wait for the next one",
                    _ => "Match already running and full",
                };
                update.lobby_console_message = Some(msg.to_string());
            }
            ServerMsg::LobbyLeader { id } => {
                update.lobby_leader_id = Some(id);
            }
            ServerMsg::MatchStatus { status } => {
                update.match_status = Some(status);
            }
            ServerMsg::LobbySettings(settings) => {
                update.lobby_settings = Some(settings);
            }
            ServerMsg::TeamScore { red, blue, to_win } => {
                update.team_score = Some(TeamScore { red, blue, to_win });
            }
            ServerMsg::MatchJoined { name, .. } => {
                update.match_join_announce = Some(format!("{} joined the match", name));
            }
            ServerMsg::MatchCountdown { secs, mode } => {
                update.match_start_countdown = Some(secs);
                if mode.is_some() {
                    update.match_mode = mode;
                }
            }
            ServerMsg::MatchTimer { secs } => {
                update.match_time_left = Some(secs);
            }
            ServerMsg::MatchEnd { winner_name, countdown, .. } => {
                update.match_end_winner = Some(winner_name);
                if countdown.is_some() {
                    update.match_end_countdown = countdown;
                }
            }
            ServerMsg::PlayerList(entries) => {
                let list = entries
                    .into_iter()
                    .map(|e| {
                        let world = WorldId::from_u8(e.world).unwrap_or(WorldId::Lobby);
                        (
                            e.id, e.name, e.kills, e.deaths, world, e.alive, e.gun_level, e.team,
                            e.weapon, e.is_bot,
                        )
                    })
                    .collect();
                update.player_list = Some(list);
            }
            // Older hosts still send their asset list; the game loads from its
            // own manifest (app::asset_loader::load_manifest) instead.
            ServerMsg::Assets { .. } => {}
            ServerMsg::WallBroken { x, y } => {
                if let Some(ref mut walls) = update.broken_walls {
                    walls.push((x, y));
                } else {
                    update.broken_walls = Some(vec![(x, y)]);
                }
            }
            ServerMsg::Damage { amount, attacker_id, stun } => {
                update.damage_taken = Some((amount, Some(attacker_id), stun));
            }
            ServerMsg::KillFeed { killer_id, victim_id, weapon } => {
                if let Some(ref mut feed) = update.kill_feed {
                    feed.push((killer_id, victim_id, weapon));
                } else {
                    update.kill_feed = Some(vec![(killer_id, victim_id, weapon)]);
                }
            }
            ServerMsg::Death { killer_id, weapon } => {
                update.death = true;
                update.death_killer_id = Some(killer_id);
                update.death_weapon = Some(weapon);
            }
            ServerMsg::Respawn { position, kind } => {
                update.respawn_pos = Some(vec3(position.0, position.1, position.2));
                if let Some(kind) = kind {
                    update.respawn_fallback = Some(kind == 1);
                }
            }
            ServerMsg::SndRoundState(s) => {
                let bomb_dropped = if s.bomb_carrier_id == 0 && !s.bomb_planted {
                    Some(s.bomb_pos)
                } else {
                    None
                };

                update.snd_round_state = Some(SndRoundState {
                    round: s.round,
                    max_rounds: s.max_rounds,
                    red_wins: s.red_wins,
                    blue_wins: s.blue_wins,
                    phase: s.phase,
                    attacking_team: s.attacking_team,
                    bomb_carrier_id: s.bomb_carrier_id,
                    bomb_dropped,
                    bomb_site: s.bomb_site,
                    bomb_planted: s.bomb_planted,
                    plant_progress: s.plant_progress,
                    defuse_progress: s.defuse_progress,
                    phase_timer: s.phase_timer,
                });
            }
            ServerMsg::Chat { from_id, name, text } => {
                update.chat.push(if from_id == 0 {
                    ChatEvent::Notice(text)
                } else {
                    ChatEvent::Line { name, text }
                });
            }
            ServerMsg::PlayerJoined { name, .. } => {
                update.chat.push(ChatEvent::Notice(format!("{} joined the game", name)));
            }
            ServerMsg::PlayerLeft { name, reason, .. } => {
                let how = if reason == LEAVE_TIMED_OUT { "timed out" } else { "left the game" };
                update.chat.push(ChatEvent::Notice(format!("{} {}", name, how)));
            }
            ServerMsg::BotShot { start, end, .. } => {
                let start = vec3(start.0, start.1, start.2);
                let end = vec3(end.0, end.1, end.2);
                if let Some(ref mut shots) = update.bot_shots {
                    shots.push((start, end));
                } else {
                    update.bot_shots = Some(vec![(start, end)]);
                }
            }
            _ => {}
        }
    }

    update
}
