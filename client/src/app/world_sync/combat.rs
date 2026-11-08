use macroquad::prelude::*;

use client::screens::PlayingState;
use client::types::world::WorldId;

/// Applies damage the server says we took, with a hit announcement.
pub(super) fn apply_damage(
    playing_state: &mut PlayingState,
    damage: f32,
    attacker_id: Option<u32>,
    is_stun: bool,
) {
    playing_state.take_damage(damage, attacker_id);
    if is_stun {
        playing_state.movement_state.stamina = 0.0;
    }
    // also show a short announcement so hits appear in the feed
    let ann = if let Some(aid) = attacker_id {
        playing_state
            .connected_players
            .iter()
            .find(|(id, ..)| *id == aid)
            .map(|(_, name, ..)| format!("Hit by {}", name))
            .unwrap_or_else(|| "Hit!".to_string())
    } else {
        "Hit!".to_string()
    };
    playing_state.announcements.push((ann, 2.0));
}

/// Spawns tracers and muzzle flashes for shots fired by bots.
pub(super) fn spawn_bot_shots(playing_state: &mut PlayingState, bot_shots: Vec<(Vec3, Vec3)>) {
    for (start, end) in bot_shots {
        playing_state
            .tracers
            .push(client::weapons::effects::BulletTracer {
                start,
                end,
                lifetime: 0.09,
                speed: 120.0,
                progress: 0.0,
                tracer_length: 0.5,
                tracer_color: [255, 170, 90, 255],
                tracer_thickness: 0.03,
            });
        playing_state.world_muzzle_flashes.push(
            client::weapons::effects::WorldMuzzleFlash {
                position: start,
                lifetime: client::weapons::effects::WorldMuzzleFlash::DURATION,
            },
        );
    }
}

/// Adds kill-feed entries and starts death animations for the victims.
pub(super) fn apply_kill_feed(
    playing_state: &mut PlayingState,
    kills: Vec<(u32, u32, String)>,
    player_id: u32,
    username: &str,
) {
    for (killer_id, victim_id, weapon) in kills {
        // resolve names now so they persist even after player models are removed
        let killer_name = if killer_id == player_id {
            username.to_string()
        } else {
            playing_state
                .connected_players
                .iter()
                .find(|(id, ..)| *id == killer_id)
                .map(|(_, name, ..)| name.clone())
                .or_else(|| {
                    playing_state
                        .other_players
                        .iter()
                        .find(|p| p.id == killer_id)
                        .map(|p| p.username.clone())
                })
                .unwrap_or_else(|| "Player".to_string())
        };
        let victim_name = if victim_id == player_id {
            username.to_string()
        } else {
            playing_state
                .connected_players
                .iter()
                .find(|(id, ..)| *id == victim_id)
                .map(|(_, name, ..)| name.clone())
                .or_else(|| {
                    playing_state
                        .other_players
                        .iter()
                        .find(|p| p.id == victim_id)
                        .map(|p| p.username.clone())
                })
                .unwrap_or_else(|| "Player".to_string())
        };

        // resolve teams from connected_players
        let killer_team = playing_state
            .connected_players
            .iter()
            .find(|(id, ..)| *id == killer_id)
            .map(|(_, _, _, _, _, team, _, _)| *team)
            .unwrap_or(0);
        let victim_team = playing_state
            .connected_players
            .iter()
            .find(|(id, ..)| *id == victim_id)
            .map(|(_, _, _, _, _, team, _, _)| *team)
            .unwrap_or(0);

        // push a kill-feed entry for UI
        playing_state
            .kill_feed
            .push(client::screens::playing::KillFeedEntry {
                killer_id,
                victim_id,
                killer_name,
                victim_name,
                killer_team,
                victim_team,
                weapon,
                timer: 5.0,
            });

        // kill confirmation effect for local player
        if killer_id == player_id {
            playing_state.kill_confirm_timer = 0.3;
            // TDM: resupply ammo on kill
            if playing_state.match_mode == Some(client::net::protocols::MODE_TEAM_DEATHMATCH) {
                playing_state.weapons.resupply_active(15);
            }
        }

        // mark victim as dying so they play a death animation before removal.
        // the server will eventually stop sending state updates for the dead
        // player, but the kill-feed packet arrives earlier and gives us an
        // opportunity to trigger the animation right away.
        if victim_id != player_id {
            if let Some(victim) = playing_state.other_players.iter_mut().find(|p| p.id == victim_id) {
                // pick death animation based on current stance
                victim.skin_type = if victim.is_prone {
                    client::types::player::PlayerSkin::ProneDeath
                } else if victim.is_crouching {
                    client::types::player::PlayerSkin::CrouchDeath
                } else {
                    client::types::player::PlayerSkin::StandDeath
                };
                victim.death_timer = Some(2.0);
                // reset animation playback so it starts from the beginning
                if let Some(model_key) = victim.skin_type.model_key() {
                    if let Some(anim) = playing_state.animated_models.get_mut(model_key) {
                        anim.play_from_start(false);
                    }
                }
            }
        }
    }
}

/// Handles the server telling us we died.
pub(super) fn apply_death(
    playing_state: &mut PlayingState,
    death_killer_id: Option<u32>,
    death_weapon: &Option<String>,
) {
    if playing_state.current_world == WorldId::Lobby {
        // Lobby safety: never enter death state in lobby; server-side also clamps.
        playing_state.is_dead = false;
        playing_state.hp = playing_state.hp.max(1.0);
    } else {
        // Only trigger death logic when transitioning from alive -> dead.
        // Servers send periodic MSG_STATE entries marking a player as
        // dead until they respawn; calling `die()` every poll would keep
        // resetting the death timer and prevent auto-respawn.
        if !playing_state.is_dead {
            // resolve killer name from the connected_players list
            if let Some(killer_id) = death_killer_id {
                playing_state.death_killer_name = playing_state
                    .connected_players
                    .iter()
                    .find(|(id, ..)| *id == killer_id)
                    .map(|(_, name, ..)| name.clone());
            } else {
                playing_state.death_killer_name = None;
            }
            playing_state.death_weapon_name = death_weapon.clone();
            playing_state.die(None);
        }
    }
}
