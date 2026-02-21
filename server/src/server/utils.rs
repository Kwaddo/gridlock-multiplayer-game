use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::player::Player;

/// Returns a unique, valid username based on the requested string and existing players.
pub fn handle_username(players: &HashMap<SocketAddr, Player>, requested: &str) -> String {
    let mut candidate = if requested.is_empty() || requested.len() > 20 {
        println!("Invalid username '{}'. Assigning default.", requested);
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
pub fn send_assets(socket: &UdpSocket, addr: SocketAddr, manifest_opt: Option<&serde_json::Value>) {
    let manifest = match manifest_opt {
        Some(m) => m,
        None => {
            println!("Warning: could not read assets/manifest.json");
            return;
        }
    };

    let worlds = [("global", 255u8), ("lobby", 0u8), ("maze", 1u8)];

    for (world_name, world_id) in &worlds {
        if let Some(assets) = manifest.get(world_name).and_then(|v| v.as_object()) {
            if assets.is_empty() {
                continue;
            }
            let mut msg = Vec::with_capacity(256);
            msg.push(crate::protocols::MSG_ASSETS);
            msg.push(*world_id);
            msg.push(assets.len() as u8);
            for (key, path) in assets {
                if let Some(path_str) = path.as_str() {
                    let key_bytes = key.as_bytes();
                    let path_bytes = path_str.as_bytes();
                    msg.push(key_bytes.len() as u8);
                    msg.extend_from_slice(key_bytes);
                    msg.push(path_bytes.len() as u8);
                    msg.extend_from_slice(path_bytes);
                }
            }
            println!(
                "Sending {} asset(s) for world '{}'",
                assets.len(),
                world_name
            );
            let _ = socket.send_to(&msg, addr);
        }
    }
}
