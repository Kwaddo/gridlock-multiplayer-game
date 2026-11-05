use std::collections::HashMap;
use crate::net::PeerId;

use crate::player::Player;

/// Returns a unique, valid username based on the requested string and existing players.
pub fn handle_username(players: &HashMap<PeerId, Player>, requested: &str) -> String {
    let mut candidate = if requested.is_empty() || requested.len() > 20 {
        log!("Invalid username '{}'. Assigning default.", requested);
        "Player".to_string()
    } else {
        requested.to_string()
    };

    if !players.values().any(|p| p.username == candidate) {
        return candidate;
    }

    let mut i = 0;
    loop {
        candidate = format!("quad{}", i);
        if !players.values().any(|p| p.username == candidate) {
            return candidate;
        }
        i += 1;
    }
}

