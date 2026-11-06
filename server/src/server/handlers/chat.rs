use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

use crate::net::{Outbox, PeerId};
use crate::player::Player;
use crate::protocols::*;
use crate::server::broadcast;

/// Longest chat line kept, in characters; the rest is cut off.
pub const CHAT_MAX_CHARS: usize = 120;
/// A player's next chat line is dropped if it comes sooner than this.
pub const CHAT_MIN_INTERVAL: Duration = Duration::from_millis(400);

/// Chat text as it is shown to everyone: control characters removed, trimmed,
/// cut to `CHAT_MAX_CHARS`. `None` when nothing printable is left.
pub fn sanitize_chat(text: &str) -> Option<String> {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control())
        .take(CHAT_MAX_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Handler for `MSG_CHAT`: relay a player's line to every human player.
pub fn handle_chat(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    last_chat: &mut HashMap<PeerId, Instant>,
    src_addr: PeerId,
    text: &str,
) {
    let Some(sender) = players.get(&src_addr).filter(|p| !p.is_bot) else {
        return;
    };
    let Some(text) = sanitize_chat(text) else {
        return;
    };
    let now = Instant::now();
    if last_chat
        .get(&src_addr)
        .is_some_and(|t| now.duration_since(*t) < CHAT_MIN_INTERVAL)
    {
        return;
    }
    last_chat.insert(src_addr, now);

    log!("[chat] {}: {}", sender.username, text);
    let msg = ServerMsg::Chat { from_id: sender.id, name: sender.username.clone(), text }.encode();
    broadcast::to_humans(socket, players, &msg);
}

/// A line from the server itself (shown without a sender name).
pub fn broadcast_notice(socket: &Outbox, players: &HashMap<PeerId, Player>, text: &str) {
    let msg = ServerMsg::Chat { from_id: 0, name: String::new(), text: text.to_string() }.encode();
    broadcast::to_humans(socket, players, &msg);
}

/// Tell everyone but the newcomer that `joined` connected.
pub fn announce_joined(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    joined_addr: PeerId,
) {
    let Some(joined) = players.get(&joined_addr) else {
        return;
    };
    let msg = ServerMsg::PlayerJoined { id: joined.id, name: joined.username.clone() }.encode();
    for (addr, p) in players {
        if *addr != joined_addr && !p.is_bot {
            let _ = socket.send_to(&msg, *addr);
        }
    }
}

/// Tell the remaining players that `left` (already removed) is gone.
pub fn announce_left(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    left: &Player,
    reason: u8,
) {
    if left.is_bot {
        return;
    }
    let msg = ServerMsg::PlayerLeft { id: left.id, name: left.username.clone(), reason }.encode();
    broadcast::to_humans(socket, players, &msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_text_is_cleaned() {
        assert_eq!(sanitize_chat("  hi  "), Some("hi".to_string()));
        assert_eq!(sanitize_chat("a\nb\u{7f}c"), Some("abc".to_string()));
        assert_eq!(sanitize_chat(" \t\r\n"), None);
        assert_eq!(sanitize_chat(&"é".repeat(500)).unwrap().chars().count(), CHAT_MAX_CHARS);
    }
}
