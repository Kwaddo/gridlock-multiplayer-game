use std::collections::HashMap;
use std::net::SocketAddr;
use crate::player::{Player, Team};
use crate::bot::ai::BotState;


pub struct BotShootingConfig {
    pub fire_rate: f32, // shots per second
    pub angular_offset: f32, // max angle offset in degrees for inaccuracy
    pub damage: f32,
    pub range: f32,
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

    // find target
    
    let target = players.values()
        .filter(|p| {
            p.id != bot.id 
            && p.is_alive 
            && p.team != bot.team 
            && p.world == bot.world 
            && p.team != Team::Spectator
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
        
        bot_state.shooting.fire_rate = 1.0 / bot_state.shooting_config.fire_rate;
        let dmg = bot_state.shooting_config.damage.round().max(0.0) as u32;
        Some((new_target, dmg))
}


fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
