//! Bot awareness (issue #61): the model's maths, then one bot's senses over
//! time against enemies placed by hand (no maze: every sight line is clear,
//! every spot lit), then what it does with what it knows.

use std::collections::HashMap;

use super::model::{self, Alert, Boxed, Posture, SightInput, Track};
use super::sense::{sense, Noise};
use super::tuning::*;
use crate::bot::ai::{BotBehavior, BotState};
use crate::bot::behavior::decide_behavior;
use crate::bot::manager::BotManager;
use crate::bot::personality::BotPersonalityId;
use crate::maze::MazeData;
use crate::net::{Outbox, PeerId, BOT_PEER_BASE};
use crate::player::{Player, Team, WorldId};
use crate::server::game::{TeamScore, MAZE_CELL_SIZE, MAZE_MIN_SIDE};

const DT: f32 = 1.0 / 30.0;

fn plain_view(distance: f32) -> SightInput {
    SightInput {
        distance,
        view_range: 40.0,
        off_axis: 0.0,
        light: 1.0,
        posture: Posture::Standing,
        boxed: Boxed::No,
    }
}

#[test]
fn sight_is_slower_far_off_in_the_dark_at_the_edge_and_low_down() {
    let base = model::sight_rate(&plain_view(12.0), 1.0);
    assert!((base - SIGHT_RATE_AT_REF).abs() < 1e-5);
    assert!(model::sight_rate(&plain_view(3.0), 1.0) > 3.0 * base, "up close is much faster");
    assert!(model::sight_rate(&plain_view(30.0), 1.0) < 0.5 * base);
    assert_eq!(model::sight_rate(&plain_view(41.0), 1.0), 0.0, "past the view range");
    let with = |f: &dyn Fn(&mut SightInput)| {
        let mut s = plain_view(12.0);
        f(&mut s);
        model::sight_rate(&s, 1.0)
    };
    assert!((with(&|s| s.light = 0.0) - base * DARK_MULT).abs() < 1e-5);
    assert!((with(&|s| s.off_axis = 1.0) - base * EDGE_MULT).abs() < 1e-5);
    assert_eq!(with(&|s| s.off_axis = 0.3), base, "the middle of the view is all straight ahead");
    assert_eq!(with(&|s| s.off_axis = 1.01), 0.0, "outside the view cone");
    assert!(with(&|s| s.posture = Posture::Crouched) < base);
    assert!(with(&|s| s.posture = Posture::Prone) < with(&|s| s.posture = Posture::Crouched));
    assert!(with(&|s| s.posture = Posture::Sprinting) > base);
    // personality and difficulty scale it, aim untouched
    assert!((model::sight_rate(&plain_view(12.0), 1.5) - 1.5 * base).abs() < 1e-5);
}

#[test]
fn a_still_box_is_scenery_and_a_moving_one_is_slow_to_notice() {
    let mut s = plain_view(5.0);
    s.boxed = Boxed::Still;
    assert_eq!(model::sight_rate(&s, 1.0), 0.0);
    s.boxed = Boxed::Moving;
    let moving = model::sight_rate(&s, 1.0);
    assert!(moving > 0.0 && moving < model::sight_rate(&plain_view(5.0), 1.0) * 0.5);
}

#[test]
fn suppressed_shots_are_heard_much_closer_and_knives_not_at_all() {
    let loud = model::gunfire_radius("AK-47");
    let quiet = model::gunfire_radius("Suppressed Pistol");
    assert_eq!(model::gunfire_radius("AS VAL"), quiet);
    assert!(quiet * 4.0 < loud);
    assert_eq!(model::gunfire_radius("Knife"), 0.0);
    // heard at all: at least suspicious; never engaged by ear alone
    assert!(model::gunfire_floor(loud * 0.99, loud) >= SUSPICIOUS_AT);
    assert!((model::gunfire_floor(0.0, loud) - HEARING_CAP).abs() < 1e-5);
    assert_eq!(model::gunfire_floor(loud + 1.0, loud), 0.0);
    assert_eq!(model::gunfire_floor(1.0, 0.0), 0.0);
}

#[test]
fn light_falls_off_between_the_lit_and_dark_radii() {
    let lights = || [(0.0, 0.0), (100.0, 0.0)].into_iter();
    assert_eq!(model::light_level(lights(), (LIT_RADIUS, 0.0)), 1.0);
    assert_eq!(model::light_level(lights(), (DARK_RADIUS + 1.0, 0.0)), 0.0);
    let mid = model::light_level(lights(), ((LIT_RADIUS + DARK_RADIUS) / 2.0, 0.0));
    assert!((mid - 0.5).abs() < 1e-4);
    assert_eq!(model::light_level(std::iter::empty(), (0.0, 0.0)), 0.0, "no lights: dark");
}

#[test]
fn awareness_holds_then_fades_and_engaged_lets_go_late() {
    let mut t = Track::default();
    let step = |t: &mut Track, gain: f32, secs: f32| {
        let mut up = Vec::new();
        for _ in 0..(secs / DT).round() as usize {
            let before = t.alert();
            t.age(DT);
            t.add(gain * DT, ENGAGED_AT);
            up.extend(t.settle(DT, before));
        }
        up
    };
    // seen for a while: suspicious, then engaged, each reported once on the way up
    assert_eq!(step(&mut t, 0.5, 2.2), vec![Alert::Suspicious, Alert::Engaged]);
    assert!(t.engaged);
    // out of sight: nothing fades during the engaged delay
    step(&mut t, 0.0, ENGAGED_DECAY_DELAY_SECS - 0.1);
    assert_eq!(t.level, 1.0);
    // then slowly; still engaged until under DISENGAGE_BELOW
    step(&mut t, 0.0, 3.0);
    assert!(t.engaged && t.level < 1.0);
    let secs_to_release = (1.0 - DISENGAGE_BELOW) / DECAY_PER_SEC - 3.0;
    assert!(step(&mut t, 0.0, secs_to_release + 0.2).is_empty(), "nothing goes up on the way down");
    assert_eq!(t.alert(), Alert::Suspicious);
    step(&mut t, 0.0, 10.0);
    assert_eq!(t.alert(), Alert::Unaware);
    assert_eq!(t.level, 0.0);

    // a sound raises it to a floor at once; it never adds past a cap
    let mut t = Track::default();
    t.raise_to(0.6);
    assert_eq!((t.level, t.since), (0.6, 0.0));
    t.add(1.0, HEARING_CAP);
    assert_eq!(t.level, HEARING_CAP);
    t.add(1.0, 0.5);
    assert_eq!(t.level, HEARING_CAP, "a cap below the level leaves it be");
}

// ── One bot's senses over time ──────────────────────────────────────────────

struct Scene {
    manager: BotManager,
    players: HashMap<PeerId, Player>,
    bot: PeerId,
}

const ENEMY: PeerId = PeerId(1);
const ENEMY_ID: u32 = 1;

/// A bot at the origin facing +x, and one enemy human at `enemy_at`.
fn scene(id: BotPersonalityId, enemy_at: (f32, f32, f32)) -> Scene {
    let mut manager = BotManager::new();
    let mut players = HashMap::new();
    let bot = manager.spawn_personality_bot(&mut players, id, Team::Red, "M4A1", (0.0, 0.5, 0.0));
    players.get_mut(&bot).unwrap().post_teleport_freeze_until = None;
    let mut enemy = Player::new(ENEMY_ID, "sneaky".into());
    enemy.team = Team::Blue;
    enemy.world = WorldId::Maze;
    enemy.position = enemy_at;
    players.insert(ENEMY, enemy);
    Scene { manager, players, bot }
}

impl Scene {
    fn state(&mut self) -> &mut BotState {
        self.manager.bot_states_mut().get_mut(&self.bot).unwrap()
    }
    fn enemy(&mut self) -> &mut Player {
        self.players.get_mut(&ENEMY).unwrap()
    }
    /// Sense for `secs`, moving the enemy by `step` (m/s) and hearing `noises`
    /// on the first tick only. Returns every level crossed on the way up.
    fn run(&mut self, secs: f32, step: (f32, f32), noises: &[Noise]) -> Vec<(u32, Alert)> {
        let mut crossings = Vec::new();
        for tick in 0..(secs / DT).round() as usize {
            let e = self.enemy();
            e.position.0 += step.0 * DT;
            e.position.2 += step.1 * DT;
            let bot = &self.players[&self.bot];
            let state = self.manager.bot_states_mut().get_mut(&self.bot).unwrap();
            let heard = if tick == 0 { noises } else { &[] };
            sense(bot, state, &self.players, None, heard, DT);
            crossings.extend(state.awareness.take_alerts());
        }
        crossings
    }
}

#[test]
fn walking_up_behind_a_bot_goes_unnoticed_and_the_knife_is_a_backstab() {
    let mut s = scene(BotPersonalityId::Nexto, (-12.0, 0.5, 0.0));
    s.run(4.0, (2.8, 0.0), &[]); // walks to within a metre of its back
    assert!(s.enemy().position.0 > -1.0);
    assert_eq!(s.state().awareness.alert(ENEMY_ID), Alert::Unaware);

    // a knife from the side (no backstab claim) still kills an unaware bot
    let victim_id = s.players[&s.bot].id;
    let mut score = TeamScore::default();
    crate::server::handlers::other::apply_damage(
        &Outbox::default(),
        &mut s.players,
        ENEMY,
        victim_id,
        44,
        "Knife",
        false,
        false,
        &mut None,
        crate::protocols::MODE_TEAM_DEATHMATCH,
        s.manager.bot_states_mut(),
        None,
        &mut score,
    );
    assert!(!s.players[&s.bot].is_alive, "an unaware bot survived a knife");
}

#[test]
fn a_knife_on_a_bot_that_saw_you_is_just_a_knife() {
    let mut s = scene(BotPersonalityId::Nexto, (2.0, 0.5, 0.0));
    s.run(1.0, (0.0, 0.0), &[]);
    assert!(s.state().awareness.is_engaged(ENEMY_ID), "didn't notice someone a step in front");
    let victim_id = s.players[&s.bot].id;
    crate::server::handlers::other::apply_damage(
        &Outbox::default(),
        &mut s.players,
        ENEMY,
        victim_id,
        44,
        "Knife",
        false,
        false,
        &mut None,
        crate::protocols::MODE_TEAM_DEATHMATCH,
        s.manager.bot_states_mut(),
        None,
        &mut TeamScore::default(),
    );
    assert!(s.players[&s.bot].is_alive);
}

#[test]
fn sprinting_up_behind_is_heard() {
    let mut s = scene(BotPersonalityId::Nexto, (-12.0, 0.5, 0.0));
    s.enemy().is_sprinting = true;
    s.run(1.4, (8.0, 0.0), &[]);
    assert!(s.state().awareness.alert(ENEMY_ID) >= Alert::Suspicious);
}

#[test]
fn someone_in_plain_view_is_engaged_within_a_second_or_so() {
    let mut s = scene(BotPersonalityId::Nexto, (10.0, 0.5, 0.0));
    let up = s.run(1.5, (0.0, 0.0), &[]);
    assert_eq!(up, vec![(ENEMY_ID, Alert::Suspicious), (ENEMY_ID, Alert::Engaged)]);
    assert!(s.state().awareness.tracks(ENEMY_ID));

    // gone round a corner (behind the bot): it follows them a moment, then
    // only goes to where it last saw them
    let last_seen = s.enemy().position;
    s.enemy().position = (-20.0, 0.5, 0.0);
    s.run(ENGAGED_TRACK_SECS + 0.5, (0.0, 0.0), &[]);
    let aware = &s.state().awareness;
    assert!(aware.is_engaged(ENEMY_ID) && !aware.tracks(ENEMY_ID));
    assert_eq!(aware.suspicion((-30.0, 0.5, 0.0)), Some(last_seen));
}

#[test]
fn loud_shots_pull_bots_in_and_suppressed_ones_mostly_dont() {
    let at = (-20.0, 0.5, 0.0); // behind the bot: heard, not seen
    for (weapon, heard) in [("AK-47", true), ("Suppressed Pistol", false), ("Knife", false)] {
        let mut s = scene(BotPersonalityId::Nexto, at);
        let shot = Noise::gunfire(ENEMY, &s.players[&ENEMY], weapon);
        s.run(0.5, (0.0, 0.0), &[shot]);
        let aware = &s.state().awareness;
        assert_eq!(aware.alert(ENEMY_ID) == Alert::Suspicious, heard, "{weapon}");
        if heard {
            assert_eq!(aware.suspicion((0.0, 0.5, 0.0)), Some(at), "goes to check where it came from");
            assert_eq!(aware.look_at((0.0, 0.5, 0.0)), Some(at), "turns to look");
        }
    }
    // a suppressed shot right next to the bot is heard
    let mut s = scene(BotPersonalityId::Nexto, (-3.0, 0.5, 0.0));
    let shot = Noise::gunfire(ENEMY, &s.players[&ENEMY], "Suppressed Pistol");
    s.run(0.1, (0.0, 0.0), &[shot]);
    assert_eq!(s.state().awareness.alert(ENEMY_ID), Alert::Suspicious);
}

#[test]
fn a_still_box_in_plain_view_is_ignored_and_a_moving_one_draws_a_look() {
    let mut s = scene(BotPersonalityId::Nexto, (6.0, 0.5, 0.0));
    s.enemy().in_box = true;
    s.run(5.0, (0.0, 0.0), &[]);
    assert_eq!(s.state().awareness.level(ENEMY_ID), 0.0, "a still box was noticed");
    s.run(1.0, (0.0, 1.5), &[]);
    let level = s.state().awareness.level(ENEMY_ID);
    assert!(level > 0.0 && level < 1.0, "moving box: {level}");
    s.run(20.0, (0.0, 0.0), &[]);
    s.run(10.0, (0.0, 0.4), &[]);
    assert!(!s.state().awareness.is_engaged(ENEMY_ID), "nobody to fight in a box");
}

#[test]
fn smoke_break_decoys_are_never_sensed() {
    let mut s = scene(BotPersonalityId::Nexto, (-50.0, 0.5, 0.0));
    let mut decoy = Player::new(30000, String::new());
    decoy.world = WorldId::Maze;
    decoy.position = (3.0, 0.5, 0.0);
    decoy.is_bot = true;
    let decoy_peer = PeerId(BOT_PEER_BASE + 0x8000);
    assert!(crate::server::smoke_break::is_decoy(decoy_peer));
    s.players.insert(decoy_peer, decoy);
    s.run(3.0, (0.0, 0.0), &[]);
    assert_eq!(s.state().awareness.level(30000), 0.0);
}

#[test]
fn a_bot_that_senses_nobody_for_long_gets_a_rough_hint() {
    let mut s = scene(BotPersonalityId::Nexto, (-40.0, 0.5, 5.0));
    s.run(STALL_HINT_SECS - 1.0, (0.0, 0.0), &[]);
    assert_eq!(s.state().awareness.hint((0.0, 0.5, 0.0)), None);
    s.run(1.5, (0.0, 0.0), &[]);
    let hint = s.state().awareness.hint((0.0, 0.5, 0.0)).expect("no hint after a long stall");
    assert!(super::flat_dist(hint, (-40.0, 0.5, 5.0)) <= HINT_SPREAD + 1e-3);
    assert_eq!(s.state().awareness.alert(ENEMY_ID), Alert::Unaware, "a hint isn't noticing");
    // the stall goes on: each hint is tighter, down to a floor
    s.run(STALL_HINT_SECS * 3.0, (0.0, 0.0), &[]);
    let hint = s.state().awareness.hint((0.0, 0.5, 0.0)).expect("hint");
    let spread = (HINT_SPREAD * HINT_NARROWING.powi(3)).max(HINT_MIN_SPREAD);
    assert!(super::flat_dist(hint, (-40.0, 0.5, 5.0)) <= spread + 1e-3);
}

#[test]
fn a_bot_only_shoots_and_hunts_enemies_it_noticed() {
    let mut s = scene(BotPersonalityId::Heater, (6.0, 0.5, 0.0));
    let bot = s.bot;
    // in plain view, but not noticed yet: no target
    crate::bot::shooting::try_shoot(bot, &s.players, s.manager.bot_states_mut().get_mut(&bot).unwrap(), None, DT);
    assert_eq!(s.state().target_id, None);
    s.run(1.0, (0.0, 0.0), &[]);
    crate::bot::shooting::try_shoot(bot, &s.players, s.manager.bot_states_mut().get_mut(&bot).unwrap(), None, DT);
    assert_eq!(s.state().target_id, Some(ENEMY_ID));

    // on a real maze: an unnoticed enemy isn't hunted; once it shot the bot, it is
    let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let mut s = scene(BotPersonalityId::Heater, (0.0, 0.0, 0.0));
    let (x, y, z, _) = maze.random_spawn_world(MAZE_CELL_SIZE);
    s.players.get_mut(&bot).unwrap().position = (x, y, z);
    let (red, blue) = maze.team_spawn_positions(MAZE_CELL_SIZE);
    let far = if super::flat_dist(red, (x, y, z)) > super::flat_dist(blue, (x, y, z)) { red } else { blue };
    s.enemy().position = far;
    let state = s.state();
    state.personality.push_weight = 1.0;
    state.personality.flank_weight = 0.0;
    state.personality.hold_angle_weight = 0.0;
    let hunts = |s: &mut Scene| {
        (0..10).any(|_| {
            let state = &s.manager.bot_states_mut()[&bot];
            matches!(
                decide_behavior(&s.players[&bot], state, &s.players, &maze, &maze.analysis),
                Some(BotBehavior::Hunt { target_player_id: ENEMY_ID, .. })
            )
        })
    };
    assert!(!hunts(&mut s), "hunted someone it never noticed");
    s.state().alert(ENEMY_ID, far);
    s.state().tactics.alert = None; // the awareness alone, not the turn-to-the-shooter alert
    assert!(hunts(&mut s), "didn't go after someone it is fighting");
}

/// Bots still find people: a bot of each kind, alone on a maze with a human
/// standing still far away, becomes engaged with them in reasonable time,
/// through roaming, sighting and the anti-stall hint.
#[test]
fn bots_still_find_a_player_who_stays_put() {
    use crate::physics::PhysicsConfig;
    for id in [BotPersonalityId::Heater, BotPersonalityId::Ghost, BotPersonalityId::Sniper] {
        let mut worst = 0.0f32;
        for _ in 0..3 {
            let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
            let (red, blue) = maze.team_spawn_positions(MAZE_CELL_SIZE);
            let mut s = scene(id, blue);
            s.players.get_mut(&s.bot).unwrap().position = red;
            let cells = Some((maze.cells.as_slice(), maze.width, maze.height, maze.prop_cells()));
            let limit = 120.0;
            let mut found = None;
            for tick in 0..(limit / DT) as usize {
                let bot = s.bot;
                if tick % 6 == 0 {
                    let state = &s.manager.bot_states_mut()[&bot];
                    if let Some(b) = decide_behavior(&s.players[&bot], state, &s.players, &maze, &maze.analysis) {
                        s.state().behavior = b;
                    }
                }
                let state = s.manager.bot_states_mut().get_mut(&bot).unwrap();
                sense(&s.players[&bot], state, &s.players, Some(&maze), &[], DT);
                let look = state.awareness.look_at(s.players[&bot].position);
                let body = s.players.get_mut(&bot).unwrap();
                crate::bot::ai::tick_bot(body, state, DT, &PhysicsConfig::default(), cells, None);
                crate::bot::manager::aim_at(body, state, look, DT);
                if state.awareness.is_engaged(ENEMY_ID) {
                    found = Some(tick as f32 * DT);
                    break;
                }
            }
            let secs = found.unwrap_or_else(|| panic!("{id:?} never found a player standing still in {limit} s"));
            worst = worst.max(secs);
        }
        println!("{id:?}: found within {worst:.1} s");
    }
}

/// Issue #62: the spotted player (and nobody else) hears of each level once,
/// and a bot shot unseen tells its shooter it is on them.
#[test]
fn the_spotted_player_alone_gets_each_alert_once() {
    use crate::protocols::{ServerMsg, BOT_ALERT_ENGAGED, BOT_ALERT_SUSPICIOUS};
    let mut s = scene(BotPersonalityId::Nexto, (10.0, 0.5, 0.0));
    let mut bystander = Player::new(2, "watcher".into());
    bystander.world = WorldId::Maze;
    bystander.team = Team::Red;
    s.players.insert(PeerId(2), bystander);
    let bot_id = s.players[&s.bot].id;
    let outbox = Outbox::default();
    let tick = |s: &mut Scene, secs: f32| {
        for _ in 0..(secs / DT).round() as usize {
            let bot = &s.players[&s.bot];
            let state = s.manager.bot_states_mut().get_mut(&s.bot).unwrap();
            sense(bot, state, &s.players, None, &[], DT);
            super::sense::send_alerts(s.manager.bot_states_mut(), &s.players, &outbox);
        }
        outbox.drain().into_iter().map(|(to, packet)| (to, ServerMsg::decode(&packet).unwrap())).collect::<Vec<_>>()
    };
    let sent = tick(&mut s, 2.0);
    let alert = |level| (ENEMY, ServerMsg::BotAlert { bot_id, level });
    assert_eq!(sent, vec![alert(BOT_ALERT_SUSPICIOUS), alert(BOT_ALERT_ENGAGED)]);

    // out of sight and back a moment later: no repeat
    s.enemy().position = (-30.0, 0.5, 0.0);
    tick(&mut s, ENGAGED_DECAY_DELAY_SECS + (1.0 - DISENGAGE_BELOW) / DECAY_PER_SEC + 0.5);
    assert!(!s.state().awareness.is_engaged(ENEMY_ID));
    s.enemy().position = (10.0, 0.5, 0.0);
    let sent = tick(&mut s, 2.0);
    assert!(s.state().awareness.is_engaged(ENEMY_ID));
    assert_eq!(sent, vec![alert(BOT_ALERT_ENGAGED)], "a fresh spotting after the repeat window is news");

    // shot from behind by a stranger: "!" at once
    s.state().alert(2, (0.0, 0.5, -10.0));
    assert!(tick(&mut s, DT).is_empty(), "a teammate (or a bot) gets nothing");
    s.state().alert(ENEMY_ID, (10.0, 0.5, 0.0));
    assert!(tick(&mut s, DT).is_empty(), "already told");
}

#[test]
fn a_hint_is_dropped_once_the_bot_senses_someone_again() {
    let mut s = scene(BotPersonalityId::Nexto, (-40.0, 0.5, 5.0));
    s.run(STALL_HINT_SECS + 0.5, (0.0, 0.0), &[]);
    assert!(s.state().awareness.hint((0.0, 0.5, 0.0)).is_some(), "no hint after a long stall");
    // someone walks into plain view: the stall is over, and its hint with it
    s.enemy().position = (6.0, 0.5, 0.0);
    s.run(1.0, (0.0, 0.0), &[]);
    assert_ne!(s.state().awareness.alert(ENEMY_ID), Alert::Unaware);
    assert_eq!(s.state().awareness.hint((0.0, 0.5, 0.0)), None, "a stale hint outlived the stall");
}

// a hint inside a wall would route to an open cell too far from it to ever count as reached
#[test]
fn a_hint_lands_on_an_open_cell_centre_a_bot_there_has_reached() {
    use super::sense::snap_to_open_cell;
    use crate::maze::pathfinding::{cell_to_world, world_to_cell};
    let maze = MazeData::new(MAZE_MIN_SIDE, MAZE_MIN_SIDE);
    let cell_of = |p: (f32, f32, f32)| world_to_cell(p.0, p.2, MAZE_CELL_SIZE, &maze.cells, maze.width, maze.height);
    let is_centre = |p: (f32, f32, f32)| {
        cell_of(p).is_some_and(|(cx, cy)| cell_to_world(cx, cy, MAZE_CELL_SIZE) == (p.0, p.2))
    };
    // every wall cell's centre snaps to an open cell's centre
    let w = maze.width as usize;
    let walls: Vec<_> = (0..maze.cells.len()).filter(|i| maze.cells[*i] == 0).collect();
    assert!(!walls.is_empty());
    for i in walls {
        let (x, z) = cell_to_world((i % w) as u8, (i / w) as u8, MAZE_CELL_SIZE);
        let snapped = snap_to_open_cell(Some(&maze), (x, 0.5, z));
        assert!(is_centre(snapped), "{snapped:?} from wall cell {i}");
    }

    // and a stall hint in the maze is one, and counts as reached from there
    let mut s = scene(BotPersonalityId::Nexto, (0.0, 0.5, 0.0));
    // the open cell nearest the middle: a hint near it never leaves the maze
    let mid = (w / 2) as i32;
    let open = (0..maze.cells.len())
        .filter(|i| maze.cells[*i] == 1)
        .min_by_key(|i| ((i % w) as i32 - mid).pow(2) + ((i / w) as i32 - mid).pow(2))
        .unwrap();
    let (ex, ez) = cell_to_world((open % w) as u8, (open / w) as u8, MAZE_CELL_SIZE);
    s.enemy().position = (ex, 0.5, ez);
    s.players.get_mut(&s.bot).unwrap().position = (-100.0, 0.5, -100.0);
    for _ in 0..((STALL_HINT_SECS + 0.5) / DT) as usize {
        let bot = &s.players[&s.bot];
        let state = s.manager.bot_states_mut().get_mut(&s.bot).unwrap();
        sense(bot, state, &s.players, Some(&maze), &[], DT);
    }
    let hint = s.state().awareness.hint((-100.0, 0.5, -100.0)).expect("no hint after a long stall");
    assert!(is_centre(hint), "{hint:?} is no open cell centre");
    assert_eq!(s.state().awareness.hint(hint), None, "a bot at the hint hasn't reached it");
}
