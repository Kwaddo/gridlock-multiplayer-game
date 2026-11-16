//! Team-mode rules: the weapon a player respawns with.

use super::tests::{start_match, two_players};
use super::*;
use crate::player::Team;

fn player(team: Team, gun_level: u32, weapon: &str) -> Player {
    let mut p = Player::new(1, "p".into());
    p.team = team;
    p.gun_level = gun_level;
    p.current_weapon = weapon.into();
    p
}

fn respawn_weapon(p: &Player, mode: u8) -> String {
    game::respawn_weapon_for(&HashMap::new(), p, mode)
}

#[test]
fn gun_game_respawns_with_the_weapon_of_the_level() {
    let last = game::GUN_GAME_PROGRESSION.len() as u32;
    let p = |level| player(Team::Spectator, level, "AK-47");
    assert_eq!(respawn_weapon(&p(1), MODE_GUN_GAME), game::GUN_GAME_PROGRESSION[0]);
    assert_eq!(respawn_weapon(&p(0), MODE_GUN_GAME), game::GUN_GAME_PROGRESSION[0]);
    assert_eq!(respawn_weapon(&p(3), MODE_GUN_GAME), game::GUN_GAME_PROGRESSION[2]);
    assert_eq!(respawn_weapon(&p(last), MODE_GUN_GAME), game::GUN_GAME_PROGRESSION[last as usize - 1]);
    // past the last level: Punch
    assert_eq!(respawn_weapon(&p(last + 1), MODE_GUN_GAME), "Punch");
}

#[test]
fn team_modes_keep_the_weapon_held() {
    for mode in [MODE_TEAM_DEATHMATCH, MODE_SEARCH_AND_DESTROY] {
        // gun_level is 1 for everyone: it must not matter outside Gun Game
        assert_eq!(respawn_weapon(&player(Team::Red, 1, "M4A1"), mode), "M4A1");
        assert_eq!(respawn_weapon(&player(Team::Blue, 1, "SPAS-12"), mode), "SPAS-12");
        // nothing held yet: the first of the team's loadout
        assert_eq!(respawn_weapon(&player(Team::Red, 1, ""), mode), game::TEAM_LOADOUT_RED[0]);
        assert_eq!(respawn_weapon(&player(Team::Blue, 1, ""), mode), game::TEAM_LOADOUT_BLUE[0]);
        // no team: Knife
        assert_eq!(respawn_weapon(&player(Team::Spectator, 1, "M4A1"), mode), "Knife");
    }
    assert_eq!(respawn_weapon(&player(Team::Red, 1, "M4A1"), 0), "Knife");
}

fn unfreeze(server: &mut Server) {
    for p in server.players.values_mut() {
        p.post_teleport_freeze_until = None;
    }
}

fn hit(server: &mut Server, from: PeerId, victim: PeerId, damage: f32) {
    let victim_id = server.players[&victim].id;
    let msg = ClientMsg::Hit { victim_id, damage, weapon: Some("AK-47".into()), backstab: None, headshot: None };
    server.handle_packet(from, &msg.encode());
}

#[test]
fn tdm_respawn_request_keeps_the_weapon_held() {
    let (mut server, a, b) = two_players();
    start_match(&mut server, a);
    unfreeze(&mut server);
    server.handle_packet(a, &ClientMsg::WeaponChange { name: "M4A1".into() }.encode());
    assert_eq!(server.players[&a].gun_level, 1, "everyone starts at level 1");

    hit(&mut server, b, a, 500.0);
    assert!(!server.players[&a].is_alive);
    server.handle_packet(a, &ClientMsg::RespawnRequest.encode());
    assert!(server.players[&a].is_alive);
    assert_eq!(server.players[&a].current_weapon, "M4A1");
}
