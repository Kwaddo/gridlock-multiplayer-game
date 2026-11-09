//! Applying `MSG_STATE` and `MSG_PLAYER_INFO` to the other players we draw.

use client::net::protocols::{
    PlayerInfo, PlayerState, EVT_JUMP_START, EVT_LANDED, FLAG_ADS, FLAG_AIRBORNE, FLAG_CROUCHING,
    FLAG_IS_BOT, FLAG_PRONE, FLAG_SILENT, FLAG_SLIDING, FLAG_SPRINTING,
};
use client::net::StateSnapshot;
use client::screens::PlayingState;
use client::types::animation_state::{AnimState, MoveDir};
use client::types::player::PlayerSkin;
use client::types::world::WorldId;
use client::ui::OtherPlayer;

/// Merges the server's player snapshots into the other players we draw.
/// Only a player we have not seen yet gets a new `OtherPlayer`; the rest are
/// updated in place.
pub(super) fn merge_players(playing_state: &mut PlayingState, snapshot: StateSnapshot) {
    let StateSnapshot { world, players } = snapshot;
    // track ids we saw this frame so we can prune removed players later
    playing_state.seen_player_ids.clear();
    // reserve enough space in case it shrinks
    playing_state.seen_player_ids.reserve(players.len());

    for incoming in &players {
        playing_state.seen_player_ids.insert(incoming.id);
        let flags = Flags::of(incoming);
        if let Some(&idx) = playing_state.player_index.get(&incoming.id) {
            // update existing player by index
            let current = &mut playing_state.other_players[idx];

            // skip updating dying players — they're playing a death animation
            if current.death_timer.is_some() {
                continue;
            }

            let (x, y, z) = incoming.position;
            // Check if this player's position actually changed.
            // The server broadcasts state on every position packet from
            // ANY player, so most broadcasts carry the same position for
            // players who didn't move. Only recompute animations when
            // the position genuinely changed.
            let dx = x - current.x;
            let dz = z - current.z;
            let pos_changed = dx * dx + dz * dz > 1.0e-4;
            let state_changed = flags.sprinting != current.is_sprinting
                || flags.ads != current.is_ads
                || flags.crouching != current.is_crouching
                || flags.prone != current.is_prone
                || flags.sliding != current.is_sliding
                || flags.jump_start;

            if pos_changed || state_changed {
                let effective_sprinting =
                    flags.sprinting || current.is_sprinting || current.sprint_grace > 0.0;
                let anim_state = AnimState::from_movement(
                    current.x,
                    current.z,
                    x,
                    z,
                    incoming.yaw,
                    effective_sprinting,
                    flags.ads,
                    flags.crouching,
                    flags.prone,
                    flags.sliding,
                    flags.jump_start,
                );

                let candidate_dir = anim_state.move_dir;
                if candidate_dir != MoveDir::None {
                    current.last_move_dir = candidate_dir;
                }
                current.skin_type = AnimState::skin_from_state(anim_state, None);
                current.last_move_time = macroquad::time::get_time();
            }
            // When position is unchanged we do nothing — the per-frame
            // update() on OtherPlayer will transition to idle after a
            // short grace period using last_move_time.

            current.world = world;
            if flags.sprinting {
                current.sprint_grace = OtherPlayer::SPRINT_GRACE;
            }
            current.is_sprinting = flags.sprinting;
            current.is_ads = flags.ads;
            current.is_crouching = flags.crouching;
            current.is_prone = flags.prone;
            current.is_sliding = flags.sliding;
            current.apply_snapshot(x, y, z, incoming.yaw);
            // name, team, weapon, kills and max health: see apply_player_info
            current.hp = if incoming.alive { incoming.health as f32 } else { 0.0 };
            current.is_alive = incoming.alive;

            if !incoming.alive {
                current.skin_type = death_skin(current);
                current.death_timer = Some(2.0);
                // no rewind needed, the death clip starts this body's own clock at 0
                // before this it rewound the one shared death clip for everyone
            }
        } else {
            // new player
            let mut joined = new_player(incoming, &flags, world, playing_state.player_info.get(&incoming.id));
            let anim_state = AnimState::from_movement(
                joined.x,
                joined.z,
                joined.x,
                joined.z,
                joined.yaw,
                joined.is_sprinting,
                joined.is_ads,
                joined.is_crouching,
                joined.is_prone,
                joined.is_sliding,
                joined.evt_jump_start,
            );
            joined.skin_type = AnimState::skin_from_state(anim_state, None);
            joined.last_move_dir = anim_state.move_dir;
            joined.apply_snapshot(joined.x, joined.y, joined.z, joined.yaw);
            if !joined.is_alive {
                joined.skin_type = death_skin(&joined);
                joined.death_timer = Some(2.0);
            }
            playing_state.player_index.insert(joined.id, playing_state.other_players.len());
            playing_state.other_players.push(joined);
        }
    }

    // remove any players the server no longer reports
    // (keep dying players until their death animation timer expires)
    if !playing_state.seen_player_ids.is_empty() {
        let before = playing_state.other_players.len();
        let seen = &playing_state.seen_player_ids;
        playing_state.other_players.retain(|p| seen.contains(&p.id) || p.death_timer.is_some());
        // the index only goes stale when someone left
        if playing_state.other_players.len() != before {
            playing_state.rebuild_player_index();
        }
    }
}

fn death_skin(p: &OtherPlayer) -> PlayerSkin {
    if p.is_prone {
        PlayerSkin::ProneDeath
    } else if p.is_crouching {
        PlayerSkin::CrouchDeath
    } else {
        PlayerSkin::StandDeath
    }
}

/// A state entry's flag bytes, unpacked.
struct Flags {
    sprinting: bool,
    crouching: bool,
    prone: bool,
    sliding: bool,
    ads: bool,
    airborne: bool,
    bot: bool,
    silent: bool,
    jump_start: bool,
    landed: bool,
}

impl Flags {
    fn of(e: &PlayerState) -> Self {
        let (s, ev) = (e.state_flags, e.event_flags);
        Flags {
            sprinting: s & FLAG_SPRINTING != 0,
            crouching: s & FLAG_CROUCHING != 0,
            prone: s & FLAG_PRONE != 0,
            sliding: s & FLAG_SLIDING != 0,
            ads: s & FLAG_ADS != 0,
            airborne: s & FLAG_AIRBORNE != 0,
            bot: s & FLAG_IS_BOT != 0,
            silent: s & FLAG_SILENT != 0,
            jump_start: ev & EVT_JUMP_START != 0,
            landed: ev & EVT_LANDED != 0,
        }
    }
}

/// An `OtherPlayer` for someone who just showed up in our world.
fn new_player(e: &PlayerState, flags: &Flags, world: WorldId, info: Option<&PlayerInfo>) -> OtherPlayer {
    let mut p = OtherPlayer::new(e.id, String::new(), world);
    (p.x, p.y, p.z) = e.position;
    p.yaw = e.yaw;
    p.is_sprinting = flags.sprinting;
    p.is_crouching = flags.crouching;
    p.is_prone = flags.prone;
    p.is_sliding = flags.sliding;
    p.is_ads = flags.ads;
    p.is_airborne = flags.airborne;
    p.is_bot = flags.bot;
    p.is_silent = flags.silent;
    p.evt_jump_start = flags.jump_start;
    p.evt_landed = flags.landed;
    if p.is_sprinting {
        p.sprint_grace = OtherPlayer::SPRINT_GRACE;
    }
    p.is_alive = e.alive;
    p.hp = if e.alive { e.health as f32 } else { 0.0 };
    if let Some(info) = info {
        copy_info(&mut p, info);
    }
    p
}

/// Name, team, weapon, kills, gun level and max health, from `MSG_PLAYER_INFO`.
fn copy_info(p: &mut OtherPlayer, info: &PlayerInfo) {
    if p.username != info.name {
        p.username.clone_from(&info.name);
    }
    // copy the weapon verbatim: an empty one means none, so no stale gun is drawn
    if p.current_weapon != info.weapon {
        p.current_weapon.clone_from(&info.weapon);
    }
    p.team = info.team;
    p.kills = info.kills;
    p.gun_level = info.gun_level;
    p.max_hp = info.max_health as f32;
}

/// `MSG_PLAYER_INFO` changed: bring the players we draw up to date (all but
/// the dying, whom state updates leave alone too).
pub(super) fn apply_player_info(playing_state: &mut PlayingState) {
    let info = &playing_state.player_info;
    for p in playing_state.other_players.iter_mut().filter(|p| p.death_timer.is_none()) {
        if let Some(info) = info.get(&p.id) {
            copy_info(p, info);
        }
    }
}
