//! Turning away clients of another protocol version.
//!
//! A newer client reads the version in our `Welcome` and leaves with its own
//! message. A build from before the version check knows none of that, so it
//! is turned away with what it does understand: a `Welcome` (id 0) lets it
//! out of its connecting screen into an empty lobby, a chat notice there says
//! why, and after a while a player list without it sends it back to its menu
//! ("disconnected by server"). It is never added as a player, so it gets no
//! game traffic it would misread.

use std::time::Duration;
use platform::Instant;

use super::Server;
use crate::net::PeerId;
use crate::protocols::*;

/// How often a turned-away client is reminded why (chat lines fade after 10 s).
const NOTICE_EVERY: Duration = Duration::from_secs(3);
/// How long it sits in the empty lobby, reading the notice, before it is sent
/// back to its menu.
pub(crate) const KICK_AFTER: Duration = Duration::from_secs(10);
/// When to forget a turned-away client that went quiet.
const FORGET_AFTER: Duration = Duration::from_secs(60);

pub(super) struct Rejected {
    since: Instant,
    last_notice: Instant,
    last_heard: Instant,
}

impl Server {
    /// Answer a `Join` of another version (`None`: a build from before the check).
    pub(super) fn reject_join(&mut self, src: PeerId, username: &str, version: Option<u16>) {
        log!(
            "Turned away '{}': protocol version {:?}, this server runs {}",
            username, version, PROTOCOL_VERSION
        );
        let now = Instant::now();
        let entry = self
            .rejected
            .entry(src)
            .or_insert(Rejected { since: now, last_notice: now, last_heard: now });
        entry.last_notice = now;
        entry.last_heard = now;
        let welcome =
            ServerMsg::Welcome { id: 0, username: username.to_string(), version: Some(PROTOCOL_VERSION) };
        let _ = self.outbox.send_to(&welcome.encode(), src);
        self.send_version_notice(src);
    }

    /// A packet from a turned-away client: remind it why, then send it back to
    /// its menu. False (the packet is not used up) for anyone else, and for a
    /// `Join` (it may come from an updated client) or a discovery ping.
    pub(super) fn handle_rejected(&mut self, src: PeerId, packet: &[u8]) -> bool {
        let Some(entry) = self.rejected.get_mut(&src) else {
            return false;
        };
        match packet.first() {
            Some(&MSG_JOIN) | Some(&MSG_DISCOVER) => return false,
            Some(&MSG_DISCONNECT) => {
                self.rejected.remove(&src);
                return true;
            }
            _ => {}
        }
        let now = Instant::now();
        entry.last_heard = now;
        if entry.since.elapsed() >= KICK_AFTER {
            // not on the list: old builds take that as removed by the server
            let _ = self.outbox.send_to(&ServerMsg::PlayerList(Vec::new()).encode(), src);
        } else if entry.last_notice.elapsed() >= NOTICE_EVERY {
            entry.last_notice = now;
            self.send_version_notice(src);
        }
        true
    }

    fn send_version_notice(&self, src: PeerId) {
        let notice =
            ServerMsg::Chat { from_id: 0, name: String::new(), text: VERSION_MISMATCH_NOTICE.to_string() };
        let _ = self.outbox.send_to(&notice.encode(), src);
    }

    /// Drop turned-away clients that went quiet.
    pub(super) fn forget_rejected(&mut self) {
        self.rejected.retain(|_, r| r.last_heard.elapsed() < FORGET_AFTER);
    }

    /// Whether `peer` was turned away for running another version.
    pub fn is_rejected(&self, peer: PeerId) -> bool {
        self.rejected.contains_key(&peer)
    }

    #[cfg(test)]
    pub(super) fn age_rejection(&mut self, peer: PeerId, by: Duration) {
        if let Some(r) = self.rejected.get_mut(&peer) {
            r.since = r.since.checked_sub(by).unwrap_or(r.since);
            r.last_notice = r.last_notice.checked_sub(by).unwrap_or(r.last_notice);
        }
    }
}
