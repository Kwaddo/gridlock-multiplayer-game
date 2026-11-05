use crate::net::hosting::LocalHost;
use crate::net::{ClientMsg, ServerMsg};
use transport::{Backlogged, Link, LinkStatus};
use platform::{Duration, Instant};

/// How long to wait for `Welcome` once the link is open. A host still loading
/// its assets answers between files, so allow for a slow one.
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);
/// How long WebRTC negotiation may take before the link opens.
const LINK_TIMEOUT: Duration = Duration::from_secs(15);
/// Join packets can be lost, so resend until the server answers.
const JOIN_RESEND: Duration = Duration::from_millis(500);

/// A joined game: the link to its server, and the server itself when this
/// client is the one hosting it. Dropping a hosted session ends the game.
pub struct Session {
    pub link: Box<dyn Link>,
    pub player_id: u32,
    pub username: String,
    /// What we connected to; keys the asset cache.
    pub address: String,
    pub host: Option<LocalHost>,
}

/// Join handshake that runs across frames instead of blocking the UI.
pub struct Connector {
    link: Box<dyn Link>,
    address: String,
    host: Option<LocalHost>,
    join: Vec<u8>,
    started: Instant,
    opened: Option<Instant>,
    last_sent: Option<Instant>,
}

pub enum ConnectPoll {
    Pending(Connector),
    Connected(Session),
    Failed(&'static str),
}

impl Connector {
    /// Join someone else's game at `target` (see [`join_target`]).
    pub fn join(target: &str, username: &str) -> Result<Self, &'static str> {
        let link = transport::open(target).map_err(|_| "Invalid server address.")?;
        Ok(Self::new(link, target.to_string(), None, username))
    }

    /// Join the game we just started hosting.
    pub fn join_own(mut host: LocalHost, username: &str) -> Result<Self, &'static str> {
        let link = host.self_link()?;
        Ok(Self::new(link, host.address(), Some(host), username))
    }

    fn new(link: Box<dyn Link>, address: String, host: Option<LocalHost>, username: &str) -> Self {
        Connector {
            link,
            address,
            host,
            join: ClientMsg::Join { username: username.to_string() }.encode(),
            started: Instant::now(),
            opened: None,
            last_sent: None,
        }
    }

    /// Call once per frame until it stops returning `Pending`.
    pub fn poll(mut self) -> ConnectPoll {
        // A host we are joining is not in a session yet, so step it from here.
        if let Some(host) = &mut self.host {
            host.update();
        }
        const FAILED: ConnectPoll = ConnectPoll::Failed("Failed to connect to server.");
        match self.link.status() {
            LinkStatus::Closed => return FAILED,
            LinkStatus::Connecting if self.started.elapsed() >= LINK_TIMEOUT => return FAILED,
            LinkStatus::Connecting => {}
            LinkStatus::Open => {
                let opened = *self.opened.get_or_insert_with(Instant::now);
                let mut packets = self.link.recv().into_iter();
                while let Some(packet) = packets.next() {
                    if let Some(ServerMsg::Welcome { id, username }) = ServerMsg::decode(&packet) {
                        // Whatever followed the Welcome (assets, state) belongs to the game.
                        let link = Box::new(Backlogged::new(self.link, packets.collect()));
                        return ConnectPoll::Connected(Session {
                            link,
                            player_id: id,
                            username,
                            address: self.address,
                            host: self.host,
                        });
                    }
                }
                if opened.elapsed() >= JOIN_TIMEOUT {
                    return FAILED;
                }
                if self.last_sent.map_or(true, |t| t.elapsed() >= JOIN_RESEND) {
                    self.link.send(&self.join);
                    self.last_sent = Some(Instant::now());
                }
            }
        }
        ConnectPoll::Pending(self)
    }
}

/// Turn what a player typed into something [`Connector::join`] accepts:
/// `host:port` on native builds; in the browser, a room code (or a full room URL).
pub fn join_target(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    #[cfg(target_arch = "wasm32")]
    if !input.contains("://") {
        let base = crate::net::registry::registry_url()?;
        return Some(crate::net::registry::room_url(&base, &input.to_uppercase()));
    }
    Some(input.to_string())
}
