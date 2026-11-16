//! Bot decisions that used to stall or flicker (issue #21): finished hunts,
//! S&D freeze time, push/hold flips, endless retreats, aim vs. broadcast yaw.

use std::collections::HashMap;
use std::time::Duration;

use platform::Instant;

use crate::bot::ai::{self, BotBehavior};
use crate::bot::behavior::decide_behavior;
use crate::bot::manager::{aim_at, BotManager};
use crate::bot::personality::BotPersonalityId;
use crate::maze::MazeData;
use crate::net::{Outbox, PeerId};
use crate::physics::PhysicsConfig;
use crate::player::{Player, Team, WorldId};
use crate::server::game::{self, maze_side_for_players, TeamScore, MAZE_CELL_SIZE, MAZE_MAX_SIDE, MAZE_MIN_SIDE};

fn maze_sizes() -> [u8; 3] {
    [MAZE_MIN_SIDE, maze_side_for_players(8), MAZE_MAX_SIDE]
}

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

#[test]
fn a_bot_done_hunting_with_no_target_picks_something_new() {
    for side in maze_sizes() {
        let maze = MazeData::new(side, side);
        for id in [BotPersonalityId::Heater, BotPersonalityId::Ghost, BotPersonalityId::Viper] {
            let mut manager = BotManager::new();
            let mut players = HashMap::new();
            let addr = spawn(&mut manager, &mut players, &maze, id, Team::Red);
            let state = manager.bot_states_mut().get_mut(&addr).unwrap();
            state.behavior = BotBehavior::Hunt { path: Vec::new(), path_idx: 0, target_player_id: 0, repath_timer: 1.0 };
            // every decision, not just a lucky one: a stalled hunt is what teleported bots
            for _ in 0..20 {
                let state = &manager.bot_states_mut()[&addr];
                match decide_behavior(&players[&addr], state, &players, &maze, &maze.analysis) {
                    None => panic!("{id:?} stayed on a finished hunt"),
                    Some(BotBehavior::Hunt { .. }) => panic!("{id:?} hunted nobody"),
                    Some(_) => {}
                }
            }
        }
    }
}

#[test]
fn bots_stand_still_in_snd_freeze_time() {
    let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let mut manager = BotManager::new();
    let mut players = HashMap::new();
    let addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Heater, Team::Red);
    // somewhere to walk to
    let state = &manager.bot_states_mut()[&addr];
    let plan = (0..20)
        .find_map(|_| decide_behavior(&players[&addr], state, &players, &maze, &maze.analysis))
        .expect("a plan");
    manager.bot_states_mut().get_mut(&addr).unwrap().behavior = plan;
    let start = players[&addr].position;

    let outbox = Outbox::default();
    let tick = |manager: &mut BotManager, players: &mut HashMap<PeerId, Player>, frozen: bool| {
        // past the manager's wall-clock tick gate
        std::thread::sleep(Duration::from_millis(40));
        let mut score = TeamScore::default();
        manager.tick(&outbox, players, &mut None, 0, Some(&maze), &mut score, frozen);
    };
    for _ in 0..6 {
        tick(&mut manager, &mut players, true);
        assert_eq!(players[&addr].position, start, "moved during freeze time");
    }
    // and it would have moved: freeze time over, it walks
    let moved = (0..20).any(|_| {
        tick(&mut manager, &mut players, false);
        players[&addr].position != start
    });
    assert!(moved, "never moved after freeze time");
}

#[test]
fn a_bot_set_on_holding_keeps_to_it_and_rolls_rarely() {
    let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let mut manager = BotManager::new();
    let mut players = HashMap::new();
    let addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Heater, Team::Red);
    let enemy_addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Nexto, Team::Blue);
    let enemy_id = players[&enemy_addr].id;

    let decide = |manager: &mut BotManager, roll: f32| {
        let state = manager.bot_states_mut().get_mut(&addr).unwrap();
        state.personality.push_weight = 1.0;
        state.personality.flank_weight = 0.0;
        state.personality.hold_angle_weight = 1.0;
        state.tactics.stance_roll = roll;
        state.target_id = Some(enemy_id); // in sight
        state.behavior = BotBehavior::Reposition { path: vec![(1, 1)], path_index: 0, hold_face: Some((1.0, 0.0)) };
        let state = &manager.bot_states_mut()[&addr];
        decide_behavior(&players[&addr], state, &players, &maze, &maze.analysis)
    };
    // on the way to hold an angle and set on holding: an enemy in sight doesn't change that
    for _ in 0..20 {
        assert!(decide(&mut manager, 0.9).is_none(), "gave up walking to its hold spot");
    }
    // set on pushing: it goes for them
    assert!(decide(&mut manager, 0.1).is_some(), "kept walking to a hold spot while pushing");

    // the stance is rolled every few seconds, not every tick
    let state = manager.bot_states_mut().get_mut(&addr).unwrap();
    state.target_id = None;
    state.tactics.stance_timer = 0.0;
    let bot = players.get_mut(&addr).unwrap();
    ai::tick_bot(bot, state, 1.0 / 30.0, &PhysicsConfig::default(), None, None);
    let roll = state.tactics.stance_roll;
    for _ in 0..80 {
        ai::tick_bot(bot, state, 1.0 / 30.0, &PhysicsConfig::default(), None, None);
        assert_eq!(state.tactics.stance_roll, roll, "stance rolled again within 3 s");
    }
}

#[test]
fn a_wounded_bot_holds_after_retreating_and_heals() {
    let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let mut manager = BotManager::new();
    let mut players = HashMap::new();
    let addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Heater, Team::Red);
    spawn(&mut manager, &mut players, &maze, BotPersonalityId::Nexto, Team::Blue);
    let bot = players.get_mut(&addr).unwrap();
    bot.health = 1;
    let state = manager.bot_states_mut().get_mut(&addr).unwrap();
    state.personality.retreat_health_threshold = 0.5;

    // got away: hold there rather than retreat again at once
    state.behavior = BotBehavior::Retreat { path: Vec::new(), path_index: 0 };
    let state = &manager.bot_states_mut()[&addr];
    let b = decide_behavior(&players[&addr], state, &players, &maze, &maze.analysis);
    assert!(matches!(b, Some(BotBehavior::HoldAngle { .. })), "did not hold after retreating");

    // health comes back out of combat, as for players, so the retreat ends
    let bot = players.get_mut(&addr).unwrap();
    bot.regen_resume_at = Some(Instant::now() + Duration::from_secs(60));
    game::process_lobby_regen(&mut players);
    assert_eq!(players[&addr].health, 1, "healed in combat");
    players.get_mut(&addr).unwrap().regen_resume_at = None;
    game::process_lobby_regen(&mut players);
    assert_eq!(players[&addr].health, 2, "a match bot did not regenerate");

    // shooting-range bots keep their damage
    let mut range_bot = Player::new(77, "dummy".into());
    range_bot.is_bot = true;
    range_bot.world = WorldId::Lobby;
    range_bot.health = 10;
    players.insert(PeerId(77), range_bot);
    game::process_lobby_regen(&mut players);
    assert_eq!(players[&PeerId(77)].health, 10, "a range bot regenerated");
}

#[test]
fn the_broadcast_yaw_is_the_aim_yaw() {
    let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let mut manager = BotManager::new();
    let mut players = HashMap::new();
    let addr = spawn(&mut manager, &mut players, &maze, BotPersonalityId::Heater, Team::Red);
    let state = manager.bot_states_mut().get_mut(&addr).unwrap();
    let bot = players.get_mut(&addr).unwrap();
    let p = bot.position;
    for look in [(p.0 + 10.0, p.1, p.2 + 3.0), (p.0 - 2.0, p.1 + 1.0, p.2 - 8.0), p] {
        // tick_bot faced the way it walked
        bot.yaw = 2.5;
        aim_at(bot, state, Some(look), 1.0 / 30.0);
        assert_eq!(bot.yaw, state.current_yaw, "looks one way, aims another at {look:?}");
        assert_eq!(bot.pitch, state.current_pitch);
    }
    // nothing to aim at: the aim follows the body
    bot.yaw = 1.0;
    aim_at(bot, state, None, 1.0 / 30.0);
    assert_eq!(state.current_yaw, 1.0);
}
