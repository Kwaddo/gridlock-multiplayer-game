use crate::net::hosting::LocalHost;
use crate::net::{ClientMsg, ServerMsg};
use transport::{Backlogged, Link, LinkStatus};
use platform::{Duration, Instant};

/// How long to wait for `Welcome` once the link is open. A host still loading
/// its assets answers between files, so allow for a slow one.
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);
/// How long WebRTC negotiation may take before the link opens.
const LINK_TIMEOUT: Duration = Duration::from_secs(25);
/// Join packets can be lost, so resend until the server answers. A resend
/// is one small packet and the server answers a repeat with Welcome again,
/// so resend often: a lost Join or Welcome then costs a moment, not half a second.
const JOIN_RESEND: Duration = Duration::from_millis(150);

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
    /// Join requests sent so far.
    attempts: u32,
    /// What the server sent before its Welcome arrived (a Welcome can be
    /// lost while what followed it gets through, e.g. the maze on its
    /// reliable channel); it belongs to the game as much as what follows.
    early: Vec<Box<[u8]>>,
}

/// Where a join stands, for the "Connecting" screen.
pub struct ConnectProgress<'a> {
    /// What is happening now.
    pub stage: &'static str,
    /// 1-based step out of `steps`.
    pub step: u8,
    pub steps: u8,
    /// 0..1 bar fill. Estimated from time spent against the timeouts, since
    /// neither step reports real progress; it eases out so it never stalls
    /// at a hard edge and never reaches full before the join succeeds.
    pub fraction: f32,
    pub elapsed: Duration,
    /// Seconds left before this step gives up.
    pub gives_up_in: Duration,
    pub join_attempts: u32,
    pub address: &'a str,
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
            attempts: 0,
            early: Vec::new(),
        }
    }

    pub fn progress(&self) -> ConnectProgress<'_> {
        // eased 0..1 over `limit`: quick at first, slowing as the timeout nears
        let ease = |t: Duration, limit: Duration| {
            let x = (t.as_secs_f32() / limit.as_secs_f32()).min(1.0);
            1.0 - (-5.0 * x).exp()
        };
        // linking takes the first 60% of the bar, the join request the rest
        const LINK_SHARE: f32 = 0.6;
        let (stage, step, fraction, gives_up_in) = match self.opened {
            None => {
                let stage = if self.host.is_some() {
                    "Starting your server"
                } else if cfg!(target_arch = "wasm32") {
                    "Opening peer connection"
                } else {
                    "Reaching server"
                };
                let t = self.started.elapsed();
                (stage, 1, 0.02 + (LINK_SHARE - 0.02) * ease(t, LINK_TIMEOUT), LINK_TIMEOUT.saturating_sub(t))
            }
            Some(opened) => {
                let t = opened.elapsed();
                (
                    "Joining game",
                    2,
                    LINK_SHARE + (0.97 - LINK_SHARE) * ease(t, JOIN_TIMEOUT),
                    JOIN_TIMEOUT.saturating_sub(t),
                )
            }
        };
        ConnectProgress {
            stage,
            step,
            steps: 2,
            fraction,
            elapsed: self.started.elapsed(),
            gives_up_in,
            join_attempts: self.attempts,
            address: &self.address,
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
                        // Whatever came around the Welcome (assets, state) belongs to the game.
                        let mut backlog = std::mem::take(&mut self.early);
                        backlog.extend(packets);
                        let link = Box::new(Backlogged::new(self.link, backlog));
                        return ConnectPoll::Connected(Session {
                            link,
                            player_id: id,
                            username,
                            address: self.address,
                            host: self.host,
                        });
                    }
                    self.early.push(packet);
                }
                if opened.elapsed() >= JOIN_TIMEOUT {
                    return FAILED;
                }
                if self.last_sent.map_or(true, |t| t.elapsed() >= JOIN_RESEND) {
                    self.link.send(&self.join);
                    self.last_sent = Some(Instant::now());
                    self.attempts += 1;
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
