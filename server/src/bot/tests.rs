//! Bot behaviour on real generated mazes: decisions and movement driven the
//! way `BotManager::tick` drives them, without its wall-clock tick gate.

use std::collections::HashMap;

use crate::bot::ai::{self, BotBehavior, BotState};
use crate::bot::behavior::decide_behavior;
use crate::bot::manager::BotManager;
use crate::bot::personality::BotPersonalityId;
use crate::maze::MazeData;
use crate::net::PeerId;
use crate::physics::PhysicsConfig;
use crate::player::{Player, Team, WorldId};
use crate::server::game::{maze_side_for_players, MAZE_CELL_SIZE, MAZE_MAX_SIDE, MAZE_MIN_SIDE};

/// Every maze size a match can get: smallest, a mid-size lobby, largest.
fn maze_sizes() -> [u8; 3] {
    [MAZE_MIN_SIDE, maze_side_for_players(8), MAZE_MAX_SIDE]
}

const DT: f32 = 1.0 / 30.0;

fn spawn(
    manager: &mut BotManager,
    players: &mut HashMap<PeerId, Player>,
    maze: &MazeData,
    id: BotPersonalityId,
    team: Team,
) -> PeerId {
    let (x, y, z, _) = maze.random_spawn_world(MAZE_CELL_SIZE);
    let addr = manager.spawn_personality_bot(players, id, team, "M4A1", (x, y, z));
    players.get_mut(&addr).unwrap().post_teleport_freeze_until = None;
    addr
}

/// Decide five times a second and move every tick, as the manager does.
fn run(
    manager: &mut BotManager,
    players: &mut HashMap<PeerId, Player>,
    maze: &MazeData,
    addr: PeerId,
    secs: f32,
    mut each_tick: impl FnMut(&Player, &BotState),
) {
    let ticks = (secs / DT) as usize;
    for tick in 0..ticks {
        if tick % 6 == 0 {
            let state = &manager.bot_states_mut()[&addr];
            let bot = &players[&addr];
            if let Some(b) = decide_behavior(bot, state, players, maze, &maze.analysis) {
                manager.bot_states_mut().get_mut(&addr).unwrap().behavior = b;
            }
        }
        let cells = Some((maze.cells.as_slice(), maze.width, maze.height, maze.prop_cells()));
        let state = manager.bot_states_mut().get_mut(&addr).unwrap();
        let bot = players.get_mut(&addr).unwrap();
        ai::tick_bot(bot, state, DT, &PhysicsConfig::default(), cells, None);
        each_tick(&players[&addr], &manager.bot_states_mut()[&addr]);
    }
}

fn cell_of(p: &Player) -> (i32, i32) {
    ((p.position.0 / MAZE_CELL_SIZE) as i32, (p.position.2 / MAZE_CELL_SIZE) as i32)
}

#[test]
fn bots_with_nobody_around_keep_moving() {
    for side in maze_sizes() {
        // Bots used to freeze for good once they reached the first spot they roamed to.
        for id in [BotPersonalityId::Heater, BotPersonalityId::Nexto, BotPersonalityId::Viper] {
            let maze = MazeData::new(side, side);
            let mut manager = BotManager::new();
            let mut players = HashMap::new();
            let addr = spawn(&mut manager, &mut players, &maze, id, Team::Red);

            let mut cells_per_window: Vec<std::collections::HashSet<(i32, i32)>> = vec![Default::default(); 6];
            let mut tick = 0usize;
            run(&mut manager, &mut players, &maze, addr, 60.0, |bot, _| {
                cells_per_window[(tick as f32 * DT / 10.0) as usize % 6].insert(cell_of(bot));
                tick += 1;
            });
            for (i, cells) in cells_per_window.iter().enumerate() {
                assert!(cells.len() >= 3, "{id:?} stood still in seconds {}-{}: {cells:?}", i * 10, i * 10 + 10);
            }
        }
    }
}

#[test]
fn walking_bots_never_step_backwards_into_their_own_cell_centre() {
    for side in maze_sizes() {
        let maze = MazeData::new(side, side);
        let mut manager = BotManager::new();
        let mut players = HashMap::new();
        let addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Heater, Team::Red);
        let start = cell_of(&players[&addr]);
        let state = &manager.bot_states_mut()[&addr];
        // a roam can pick the bot's own cell, which is no plan: it decides again soon
        let b = (0..20)
            .find_map(|_| decide_behavior(&players[&addr], state, &players, &maze, &maze.analysis))
            .expect("a plan");
        let first = match &b {
            BotBehavior::Reposition { path, .. } | BotBehavior::Hunt { path, .. } => path.first().copied(),
            _ => None,
        };
        if let Some(first) = first {
            assert_ne!((first.0 as i32, first.1 as i32), start, "path starts with the cell the bot is in");
        }
    }
}

#[test]
fn a_bot_shot_from_behind_goes_after_the_shooter() {
    for side in maze_sizes() {
        let maze = MazeData::new(side, side);
        let mut manager = BotManager::new();
        let mut players = HashMap::new();
        let bot_addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Ghost, Team::Red);
        let enemy_addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Nexto, Team::Blue);
        let (enemy_id, enemy_pos) = (players[&enemy_addr].id, players[&enemy_addr].position);

        let state = manager.bot_states_mut().get_mut(&bot_addr).unwrap();
        // holding an angle, the shooter unseen
        state.behavior = BotBehavior::HoldAngle { face_direction: (1.0, 0.0), hold_timer: 0.0, max_hold_time: 30.0 };
        state.alert(enemy_id, enemy_pos);
        let state = &manager.bot_states_mut()[&bot_addr];
        let b = decide_behavior(&players[&bot_addr], state, &players, &maze, &maze.analysis);
        match b {
            Some(BotBehavior::Hunt { target_player_id, .. }) => assert_eq!(target_player_id, enemy_id),
            Some(BotBehavior::Investigate { .. }) => {}
            other => panic!("kept holding: {}", other.is_some()),
        }
    }
}

#[test]
fn bots_ignore_lobby_players_and_spectators() {
    for side in maze_sizes() {
        let maze = MazeData::new(side, side);
        let mut manager = BotManager::new();
        let mut players = HashMap::new();
        let addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Heater, Team::Red);
        let mut lobby = Player::new(1, "idle".into());
        lobby.team = Team::Blue;
        lobby.world = WorldId::Lobby;
        players.insert(PeerId(1), lobby);

        let state = &manager.bot_states_mut()[&addr];
        if let Some(BotBehavior::Hunt { target_player_id, .. } | BotBehavior::Flank { target_player_id, .. }) =
            decide_behavior(&players[&addr], state, &players, &maze, &maze.analysis)
        {
            assert_ne!(target_player_id, 1, "hunted a player in the lobby");
        }
    }
}

#[test]
fn teamless_bots_fight_everyone_in_gun_game() {
    // Gun Game leaves everyone on Team::Spectator: a teamless bot must still
    // treat other teamless players as enemies, while team bots ignore them.
    let mut ffa_bot = Player::new(1, "bot".into());
    let mut ffa_player = Player::new(2, "player".into());
    ffa_bot.team = Team::Spectator;
    ffa_player.team = Team::Spectator;
    assert!(ffa_bot.is_hostile_to(&ffa_player));
    assert!(!ffa_bot.is_hostile_to(&ffa_bot));

    let mut red = Player::new(3, "red".into());
    red.team = Team::Red;
    assert!(!red.is_hostile_to(&ffa_player));
    let mut blue = Player::new(4, "blue".into());
    blue.team = Team::Blue;
    assert!(red.is_hostile_to(&blue));
}

/// The per-cell prop lookup collides exactly where the old loop over every
/// prop did, including on cell edges.
#[test]
fn prop_lookup_matches_scanning_every_prop() {
    const R: f32 = 0.28;
    let old = |maze: &MazeData, x: f32, z: f32| {
        maze.props.iter().any(|prop| {
            let px = prop.x as f32 * MAZE_CELL_SIZE;
            let pz = prop.y as f32 * MAZE_CELL_SIZE;
            x + R > px && x - R < px + MAZE_CELL_SIZE && z + R > pz && z - R < pz + MAZE_CELL_SIZE
        })
    };
    let mut hits = 0;
    for side in maze_sizes() {
        let maze = MazeData::new(side, side);
        let extent = side as f32 * MAZE_CELL_SIZE;
        // cell edges (plus and minus the radius) and points between
        let steps = side as usize * 24;
        for i in 0..=steps {
            for j in 0..=steps {
                let (x, z) = (i as f32 * extent / steps as f32, j as f32 * extent / steps as f32);
                for (x, z) in [(x, z), (x - R, z + R), (x + R, z - R)] {
                    let new = ai::footprint_hits_prop(maze.prop_cells(), maze.width, maze.height, x, z, R);
                    assert_eq!(new, old(&maze, x, z), "at ({}, {})", x, z);
                    hits += new as usize;
                }
            }
        }
    }
    assert!(hits > 0, "no props were touched at all");
}

/// The cached navigation grid is the maze with props blocked, and follows
/// broken walls.
#[test]
fn nav_cells_block_props_and_follow_broken_walls() {
    let mut maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let expected = |maze: &MazeData| {
        let mut nav = maze.cells.clone();
        for prop in &maze.props {
            nav[prop.y as usize * maze.width as usize + prop.x as usize] = 0;
        }
        nav
    };
    assert_eq!(maze.nav_cells(), expected(&maze).as_slice());
    let w = maze.width as usize;
    let wall = (w + 1..maze.cells.len() - w - 1).find(|&i| maze.cells[i] == 0 && !maze.prop_cells()[i]).expect("an inner wall");
    assert!(maze.break_wall((wall % w) as u8, (wall / w) as u8));
    assert_eq!(maze.nav_cells()[wall], 1);
    assert_eq!(maze.nav_cells(), expected(&maze).as_slice());
}
