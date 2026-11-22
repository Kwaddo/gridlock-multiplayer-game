use super::*;

#[test]
fn one_hunter_per_six_and_only_humans_hunt() {
    assert_eq!(hunter_count(2, 2), 1);
    assert_eq!(hunter_count(6, 6), 1);
    assert_eq!(hunter_count(7, 7), 2);
    assert_eq!(hunter_count(13, 13), 3);
    // you alone with bot hiders, you hunt
    assert_eq!(hunter_count(5, 1), 1);
    // nobody to hide, or nobody human to hunt
    assert_eq!(hunter_count(1, 1), 0);
    assert_eq!(hunter_count(4, 0), 0);
}

#[test]
fn coins_are_four_a_hider_but_never_under_twenty() {
    assert_eq!(coin_count(1), 10);
    assert_eq!(coin_count(2), 10);
    assert_eq!(coin_count(3), 12);
    assert_eq!(coin_count(10), 40);
}

#[test]
fn phases_run_blind_released_coins_vote() {
    let mut s = SmokeState::new();
    s.new_round();
    assert_eq!((s.round, s.phase), (1, SmokePhase::Blind));
    // a second short of the end is still blind
    s.tick(SMOKE_BLIND_TIME - 1.0);
    assert_eq!(s.phase, SmokePhase::Blind);
    s.tick(1.0);
    assert_eq!(s.phase, SmokePhase::Released);
    assert_eq!(s.phase_timer, SMOKE_RELEASED_TIME);
    s.tick(SMOKE_RELEASED_TIME);
    assert_eq!(s.phase, SmokePhase::Coins);
    s.tick(SMOKE_ROUND_TIME);
    assert_eq!(s.phase, SmokePhase::Results);
}

#[test]
fn ticks_that_skip_past_zero_still_move_on() {
    // the clock goes 0.02 -> -0.01, it never lands on exactly 0
    let mut s = SmokeState::new();
    s.new_round();
    s.tick(SMOKE_BLIND_TIME - 0.02);
    s.tick(0.03);
    assert_eq!(s.phase, SmokePhase::Released);
}

#[test]
fn a_new_round_wipes_round_things_but_keeps_match_points() {
    let mut s = SmokeState::new();
    let me = PeerId(1);
    s.new_round();
    s.add_points(me, 40);
    s.multipliers.insert(me, 0.5);
    s.alive_clock = 3.0;
    s.tick(SMOKE_BLIND_TIME);
    s.new_round();
    assert_eq!((s.round, s.phase, s.phase_timer), (2, SmokePhase::Blind, SMOKE_BLIND_TIME));
    assert_eq!(s.points[&me], 40, "match points stay");
    assert!(s.round_points.is_empty() && s.multipliers.is_empty());
    assert_eq!(s.alive_clock, 0.0);
}

#[test]
fn points_never_go_under_zero_and_gains_use_the_multiplier() {
    let mut s = SmokeState::new();
    let me = PeerId(1);
    s.add_points(me, SMOKE_POINTS_DECOY);
    assert_eq!(s.points[&me], 0, "a decoy at 0 points leaves you at 0");
    s.multipliers.insert(me, 0.7);
    s.add_points(me, SMOKE_POINTS_FIND);
    assert_eq!(s.points[&me], 35, "a find at x0.7 is worth 35");
    s.add_points(me, SMOKE_POINTS_DECOY);
    assert_eq!(s.points[&me], 25, "losses don't get multiplied");
    // the round counts only what really changed: 0, +35, -10
    assert_eq!(s.round_points[&me], 25);
}

#[test]
fn hiders_earn_while_hidden_but_not_while_blind() {
    let mut s = SmokeState::new();
    let hider = PeerId(2);
    s.new_round();
    s.hiders.insert(hider);
    // blind pays nothing, hunters can't do anything yet
    s.tick(SMOKE_BLIND_TIME);
    assert_eq!(s.points.get(&hider), None);
    // released: 5 s = 1 point
    s.tick(SMOKE_ALIVE_EVERY);
    assert_eq!(s.points[&hider], SMOKE_POINTS_ALIVE);
    // a big hitch pays every 5 s it covered, not just one
    s.tick(SMOKE_ALIVE_EVERY * 2.0 + 0.5);
    assert_eq!(s.points[&hider], SMOKE_POINTS_ALIVE * 3);
}

#[test]
fn clock_running_out_means_hiders_win() {
    let mut s = SmokeState::new();
    let (hunter, hider) = (PeerId(1), PeerId(2));
    s.new_round();
    s.hunters.insert(hunter);
    s.starting_hunters.insert(hunter);
    s.hiders.insert(hider);
    s.tick(SMOKE_BLIND_TIME);
    s.tick(SMOKE_RELEASED_TIME);
    assert_eq!(s.phase, SmokePhase::Coins);
    let before = s.points[&hider];
    s.tick(SMOKE_ROUND_TIME);
    assert_eq!(s.phase, SmokePhase::Results);
    assert_eq!(s.phase_timer, SMOKE_RESULTS_TIME);
    // the 3 min of alive points, the win bonus and the mvp (only one earning anything)
    let alive = (SMOKE_ROUND_TIME / SMOKE_ALIVE_EVERY) as i32 * SMOKE_POINTS_ALIVE;
    assert_eq!(s.points[&hider], before + alive + SMOKE_POINTS_WIN + SMOKE_POINTS_MVP);
    assert_eq!(s.points.get(&hunter), None, "losing hunters get nothing");
}

#[test]
fn starting_hunters_get_the_win_but_found_hiders_dont() {
    let mut s = SmokeState::new();
    let (hunter, found) = (PeerId(1), PeerId(2));
    s.new_round();
    s.hunters.insert(hunter);
    s.starting_hunters.insert(hunter);
    // found mid round, hunting now but didn't start as one
    s.hunters.insert(found);
    s.add_points(found, SMOKE_POINTS_COIN);
    s.end_round(SMOKE_SIDE_HUNTER);
    assert_eq!(s.points[&hunter], SMOKE_POINTS_WIN);
    // their coin made them the mvp, but no win bonus
    assert_eq!(s.points[&found], SMOKE_POINTS_COIN + SMOKE_POINTS_MVP);
}

#[test]
fn mvp_is_picked_before_the_win_bonus() {
    let mut s = SmokeState::new();
    let (hunter, hider) = (PeerId(1), PeerId(2));
    s.new_round();
    s.starting_hunters.insert(hunter);
    s.hiders.insert(hider);
    // the hunter did more this round, the hiders still win
    s.add_points(hunter, SMOKE_POINTS_FIND);
    s.add_points(hider, SMOKE_POINTS_COIN);
    s.end_round(SMOKE_SIDE_HIDER);
    assert_eq!(s.points[&hunter], SMOKE_POINTS_FIND + SMOKE_POINTS_MVP);
    assert_eq!(s.points[&hider], SMOKE_POINTS_COIN + SMOKE_POINTS_WIN);
}
