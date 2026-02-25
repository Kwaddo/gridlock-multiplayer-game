use std::collections::HashMap;
use std::net::SocketAddr;
use crate::player::{Player, Team, WorldId};
use crate::bot::ai::BotState;
use crate::weapons_stats::get_weapon_stats;


#[derive(Clone, Copy)]
pub struct BotShootingConfig {
    pub fire_rate: f32, // seconds between bot-triggered shots
    pub angular_offset: f32, // max angle offset in degrees for inaccuracy
    pub reaction_time: f32, // time in seconds before bot starts shooting after seeing target
}

pub struct BotShootingState {
    pub fire_rate: f32, // time until next shot can be fired
    pub has_target: bool, // whether bot currently has a valid target in sight
    pub reaction_remaining: f32, // time remaining before bot will start shooting at target
}

pub fn try_shoot(
    bot_addr: SocketAddr,
    players: &HashMap<SocketAddr, Player>,
    bot_state: &mut BotState,
    dt: f32,
) -> Option<(u32, u32)> {

    if bot_state.shooting.fire_rate > 0.0 {
        bot_state.shooting.fire_rate = (bot_state.shooting.fire_rate - dt).max(0.0);
        return None; // still cooling down, can't shoot
    }

    if bot_state.shooting.reaction_remaining > 0.0 {
        bot_state.shooting.reaction_remaining = (bot_state.shooting.reaction_remaining - dt).max(0.0);
        return None; // still reacting, can't shoot yet
    }

    // resolve bot player from players using bot_addr
    let bot = match players.get(&bot_addr) {
        Some(p) => p,
        None => return None, // bot player not found (shouldn't happen)
    };
    let weapon_stats = get_weapon_stats(&bot.current_weapon);

    // find target
    
    let target = players.values()
        .filter(|p| {
            p.id != bot.id 
            && p.is_alive 
            && p.world == bot.world 
            && distance(bot.position, p.position) <= weapon_stats.range
            && (
                if bot.world == WorldId::Lobby {
                    !p.is_bot
                } else {
                    p.team != bot.team && p.team != Team::Spectator
                }
            )
        }).min_by(|a, b| {
            let da = distance(bot.position, a.position);
            let db = distance(bot.position, b.position);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        let Some(target) = target else {
            bot_state.target_id = None;
            bot_state.shooting.has_target = false;
            return None; // no valid targets
        };

        // handle target change
        let new_target = target.id;

        if bot_state.target_id != Some(new_target) {
            bot_state.target_id = Some(new_target);
            bot_state.shooting.has_target = true;
            bot_state.shooting.reaction_remaining = bot_state.shooting_config.reaction_time;
            return None; // just acquired target, start reaction timer
        }

        // has target and reaction time elapsed, attempt to shoot
        if bot_state.shooting.reaction_remaining > 0.0 {
            return None; // still reacting, can't shoot yet
        }

        if bot_state.shooting.fire_rate > 0.0 {
            return None; // still cooling down, can't shoot
        }
        
        let weapon_min_interval = if weapon_stats.shots_per_sec > 0.0 {
            1.0 / weapon_stats.shots_per_sec
        } else {
            bot_state.shooting_config.fire_rate
        };
        let interval = bot_state.shooting_config.fire_rate.max(weapon_min_interval);
        bot_state.shooting.fire_rate = interval;
        let dmg = weapon_stats.damage;
        Some((new_target, dmg))
}


fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
