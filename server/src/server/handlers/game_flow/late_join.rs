//! A lobby player joining a match that is already running. The joiner plays
//! the running match as it is: its mode and settings never change. In team
//! modes they go to the smaller team and start at its spawn; an auto-fill
//! bot on that team gives up its place, so team sizes stay the same and the
//! match never goes over its size. In S&D they spectate until the next round;
//! in Gun Game they start at level 1 at a random point away from everyone.

use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

use crate::bot::manager::BotManager;
use crate::maze::MazeData;
use crate::net::{Outbox, PeerId};
use crate::player::{Player, Team, WorldId};
use crate::protocols::*;
use crate::server::snd::SndState;
use crate::server::{
    broadcast, game,
    game::{MATCH_MAX_PLAYERS, MATCH_POST_TELEPORT_FREEZE_SECS, MAZE_CELL_SIZE},
};

/// Move `src_addr` from the lobby into the running match. Refused with
/// `MSG_START_DENIED` when the match is full and no bot can make room.
pub(super) fn join_running_match(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    maze: &Option<MazeData>,
    bot_manager: &mut BotManager,
    snd_state: &mut Option<SndState>,
    match_mode: u8,
    match_start_time: Option<Instant>,
    match_duration: Option<Duration>,
    match_total_maze_entries: &mut usize,
    src_addr: PeerId,
) {
    if players.get(&src_addr).map(|p| p.world) != Some(WorldId::Lobby) {
        return;
    }
    let Some(current_maze) = maze.as_ref() else {
        // shouldn't happen: match_running implies a maze exists
        let _ = socket.send_to(&ServerMsg::StartDenied { reason: None }.encode(), src_addr);
        return;
    };

    // Gun Game and Smoke Break have no teams (Smoke Break makes the joiner a
    // hunter on its next tick)
    let teamless = match_mode == MODE_GUN_GAME || match_mode == MODE_SMOKE_BREAK;
    let team = if teamless { Team::Spectator } else { pick_team(players) };
    let full = players.values().filter(|p| p.world == WorldId::Maze).count() >= MATCH_MAX_PLAYERS;
    let bot = bot_to_replace(players, snd_state.as_ref(), match_mode, team, full);
    if full && bot.is_none() {
        let _ = socket.send_to(&ServerMsg::StartDenied { reason: None }.encode(), src_addr);
        log!("Start-game request rejected: match already running and full.");
        return;
    }
    if let Some(addr) = bot {
        if let Some(gone) = bot_manager.despawn_bot(players, addr) {
            if let Some(snd) = snd_state.as_mut() {
                snd.round_alive.remove(&addr);
                if snd.bomb_carrier == Some(gone.id) {
                    // the bomb stays in play where the bot stood
                    snd.drop_bomb(gone.position);
                }
            }
            log!("Bot '{}' left the match to make room for a late joiner", gone.username);
        }
    }

    let spawn = match team {
        Team::Red => current_maze.team_spawn_positions(MAZE_CELL_SIZE).0,
        Team::Blue => current_maze.team_spawn_positions(MAZE_CELL_SIZE).1,
        Team::Spectator => {
            let (x, y, z, _) = game::random_spawn_far(current_maze, players, MAZE_CELL_SIZE * 3.0);
            (x, y, z)
        }
    };
    let Some(joiner) = players.get_mut(&src_addr) else { return };
    joiner.world = WorldId::Maze;
    joiner.team = team;
    joiner.position = spawn;
    joiner.post_teleport_freeze_until =
        Some(Instant::now() + Duration::from_secs(MATCH_POST_TELEPORT_FREEZE_SECS));
    joiner.death_time = None;
    // S&D: one life per round, and this round started without them
    let spectating = match_mode == MODE_SEARCH_AND_DESTROY;
    joiner.is_alive = !spectating;
    joiner.health = if spectating { 0 } else { joiner.max_health };
    if match_mode == MODE_GUN_GAME {
        joiner.gun_level = 1;
        joiner.current_weapon = game::GUN_GAME_PROGRESSION[0].to_string();
    }

    let response = ServerMsg::WorldChange {
        world: WorldId::Maze.as_u8(),
        spawn: joiner.position,
        mode: Some(match_mode),
        freeze_secs: Some(MATCH_POST_TELEPORT_FREEZE_SECS as u8),
        // include the authoritative timer so the joiner doesn't show a default
        match_timer: Some(remaining_secs(match_start_time, match_duration)),
    }
    .encode();
    let _ = socket.send_to(&response, src_addr);
    current_maze.send(socket, src_addr);

    // tell the joiner what is happening
    let mut notices = Vec::new();
    match team {
        Team::Red => notices.push("Joining Red team"),
        Team::Blue => notices.push("Joining Blue team"),
        Team::Spectator if match_mode == MODE_SMOKE_BREAK => notices.push("Joining as a hunter"),
        Team::Spectator => notices.push("Joining at gun level 1"),
    }
    if spectating {
        notices.push("Spectating until next round");
    }
    for text in notices {
        let notice = ServerMsg::Chat { from_id: 0, name: String::new(), text: text.to_string() };
        let _ = socket.send_to(&notice.encode(), src_addr);
    }

    *match_total_maze_entries += 1;
    log!("Player '{}' joined running match on {:?}", joiner.username, team);

    let ann = ServerMsg::MatchJoined { id: joiner.id, name: joiner.username.clone() }.encode();
    for addr in players.keys() {
        let _ = socket.send_to(&ann, *addr);
    }
    broadcast::broadcast_state(socket, players);
    broadcast::broadcast_player_list(socket, players);
}

/// Seconds left in the match, `u16::MAX` for an endless one.
fn remaining_secs(match_start_time: Option<Instant>, match_duration: Option<Duration>) -> u16 {
    let Some(dur) = match_duration else { return u16::MAX };
    let elapsed = match_start_time.map_or(Duration::ZERO, |start| start.elapsed());
    dur.saturating_sub(elapsed).as_secs().min(u16::MAX as u64) as u16
}

/// The smaller team in the match, bots included. On a tie, the team with an
/// auto-fill bot to give up its place (so the sizes stay even), else Red.
fn pick_team(players: &HashMap<PeerId, Player>) -> Team {
    let count = |team: Team, bots_only: bool| {
        players
            .values()
            .filter(|p| p.world == WorldId::Maze && p.team == team && (p.is_bot || !bots_only))
            .count()
    };
    let (red, blue) = (count(Team::Red, false), count(Team::Blue, false));
    if red != blue {
        return if red < blue { Team::Red } else { Team::Blue };
    }
    if count(Team::Red, true) == 0 && count(Team::Blue, true) > 0 {
        Team::Blue
    } else {
        Team::Red
    }
}

/// The bot that gives up its place for a joiner on `team`: in team modes any
/// bot on that team, in Gun Game one only when the match is full. A dead bot
/// goes first, and the S&D bomb carrier last. Smoke Break has no auto-fill
/// bots (its decoys are bots, but they belong to the round), so never there.
fn bot_to_replace(
    players: &HashMap<PeerId, Player>,
    snd: Option<&SndState>,
    match_mode: u8,
    team: Team,
    full: bool,
) -> Option<PeerId> {
    if match_mode == MODE_SMOKE_BREAK || (match_mode == MODE_GUN_GAME && !full) {
        return None;
    }
    let carrier = snd.and_then(|s| s.bomb_carrier);
    players
        .iter()
        .filter(|(_, p)| p.is_bot && p.world == WorldId::Maze && p.team == team)
        .min_by_key(|(addr, p)| (p.is_alive, carrier == Some(p.id), p.id, addr.0))
        .map(|(addr, _)| *addr)
}
