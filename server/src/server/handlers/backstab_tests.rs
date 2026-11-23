//! A melee hit on a match bot that never noticed the attacker: a backstab
//! only from behind it or from outside its view cone, never to its face.

use super::*;
use crate::bot::awareness::sense::half_fov;
use crate::bot::manager::BotManager;
use crate::bot::personality::BotPersonalityId;

const KNIFE_DAMAGE: u32 = 10;

/// Knife a fresh (unaware) Red bot standing at the origin facing +x, from 1 m
/// away at the angle off its facing (radians) that `angle_from` picks given
/// half the bot's field of view. Whether the bot died.
fn knife_unaware_bot(angle_from: impl Fn(f32) -> f32, claim_backstab: bool) -> bool {
    let mut manager = BotManager::new();
    let mut players = HashMap::new();
    let bot = manager.spawn_personality_bot(&mut players, BotPersonalityId::Heater, Team::Red, "M4A1", (0.0, 0.0, 0.0));
    let human = PeerId(1);
    let mut p = Player::new(1, "h".into());
    p.world = WorldId::Maze;
    p.team = Team::Blue;
    players.insert(human, p);
    let angle = angle_from(half_fov(&manager.bot_states_mut()[&bot]));
    {
        let b = players.get_mut(&bot).unwrap();
        b.post_teleport_freeze_until = None;
        b.yaw = 0.0;
    }
    players.get_mut(&human).unwrap().position = (angle.cos(), 0.0, angle.sin());
    let bot_id = players[&bot].id;
    assert!(manager.bot_states_mut()[&bot].awareness.unaware_of(1));
    apply_damage(
        &Outbox::default(),
        &mut players,
        human,
        bot_id,
        KNIFE_DAMAGE,
        "Knife",
        claim_backstab,
        false,
        &mut None,
        MODE_TEAM_DEATHMATCH,
        manager.bot_states_mut(),
        None,
        &mut game::TeamScore::default(),
    );
    !players[&bot].is_alive
}

#[test]
fn a_frontal_knife_on_an_unaware_bot_is_no_backstab() {
    assert!(!knife_unaware_bot(|_| 0.0, false), "knifed to its face, it died");
    assert!(!knife_unaware_bot(|_| 0.0, true), "a client's backstab claim from the front counted");
    // just inside its view cone: still seen coming
    assert!(!knife_unaware_bot(|half| half - 0.1, false));
}

#[test]
fn an_unaware_bot_knifed_from_behind_or_outside_its_view_dies() {
    assert!(knife_unaware_bot(|_| std::f32::consts::PI, false), "from behind");
    assert!(knife_unaware_bot(|half| half + 0.1, false), "from the side, outside its view");
    assert!(knife_unaware_bot(|half| -(half + 0.1), false), "from the other side");
}
