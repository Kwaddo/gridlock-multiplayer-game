use std::collections::HashMap;
use crate::net::{Outbox, PeerId};

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

/// Sends asset manifest entries for a given client.  Used during join and on demand.
pub fn send_assets(socket: &Outbox, addr: PeerId, manifest_opt: Option<&serde_json::Value>) {
    let manifest = match manifest_opt {
        Some(m) => m,
        None => {
            log!("Warning: could not read assets/manifest.json");
            return;
        }
    };

    let worlds = [("global", 255u8), ("lobby", 0u8), ("maze", 1u8)];

    for (world_name, world_id) in &worlds {
        if let Some(assets) = manifest.get(world_name).and_then(|v| v.as_object()) {
            if assets.is_empty() {
                continue;
            }
            let entries = assets
                .iter()
                .filter_map(|(key, path)| Some((key.clone(), path.as_str()?.to_string())))
                .collect();
            let msg = crate::protocols::ServerMsg::Assets { world: *world_id, entries }.encode();
            log!(
                "Sending {} asset(s) for world '{}'",
                assets.len(),
                world_name
            );
            let _ = socket.send_to(&msg, addr);
        }
    }
}
