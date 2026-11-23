//! Browser networking through the `gridlock_web` miniquad JS plugin
//! (`web/gridlock_web.js`): WebRTC data channels and fetch, both polled.

use std::cell::RefCell;
use std::collections::HashMap;
use std::thread::LocalKey;
use std::time::Duration;

use platform::Instant;

use crate::rtc_event::for_each_event;
pub use crate::rtc_event::RtcEvent;
use crate::{Link, LinkStatus, Packet};

mod ffi {
    extern "C" {
        pub fn gridlock_rtc_open(url: *const u8, url_len: u32, is_host: u32) -> u32;
        pub fn gridlock_rtc_send_many(handle: u32, peers: *const u32, n: u32, data: *const u8, len: u32);
        pub fn gridlock_rtc_drain(handle: u32, buf: *mut u8, cap: u32, need_out: *mut u32) -> u32;
        pub fn gridlock_rtc_close(handle: u32);
        pub fn gridlock_rtc_prewarm(url: *const u8, url_len: u32);
        pub fn gridlock_rtc_preconnect(url: *const u8, url_len: u32);

        pub fn gridlock_http_start(
            method: *const u8,
            method_len: u32,
            url: *const u8,
            url_len: u32,
            body: *const u8,
            body_len: u32,
            auth: *const u8,
            auth_len: u32,
        ) -> u32;
        pub fn gridlock_http_poll(id: u32, status_out: *mut u32, len_out: *mut u32) -> u32;
        pub fn gridlock_http_take(id: u32, buf: *mut u8);
        pub fn gridlock_http_cancel(id: u32);
    }
}

/// What [`RtcSocket::poll`] drains into at first; it grows to fit an event
/// bigger than that.
const DRAIN_BUF: usize = 16 * 1024;

/// A WebRTC endpoint in a signalling room: the host (many peers) or a player
/// (one peer, the host).
pub struct RtcSocket {
    handle: u32,
    /// Where events are drained to; kept, so polling allocates nothing.
    buf: Vec<u8>,
}

impl RtcSocket {
    pub fn host(room_url: &str) -> Self {
        Self::open(room_url, true)
    }

    pub fn join(room_url: &str) -> Self {
        Self::open(room_url, false)
    }

    fn open(room_url: &str, host: bool) -> Self {
        let handle = unsafe { ffi::gridlock_rtc_open(room_url.as_ptr(), room_url.len() as u32, host as u32) };
        RtcSocket { handle, buf: Vec::new() }
    }

    pub fn send(&mut self, peer: u32, packet: &[u8]) {
        self.send_many(&[peer], packet);
    }

    /// Send the same packet to each of `peers`, in one call into JS.
    pub fn send_many(&mut self, peers: &[u32], packet: &[u8]) {
        let (n, len) = (peers.len() as u32, packet.len() as u32);
        unsafe { ffi::gridlock_rtc_send_many(self.handle, peers.as_ptr(), n, packet.as_ptr(), len) }
    }

    /// Hand every event since the last poll to `on`, in order. They come
    /// over from JS in batches, as many as fit in one buffer per call,
    /// rather than one call (and one allocation) per packet.
    pub fn poll(&mut self, mut on: impl FnMut(RtcEvent<'_>)) {
        if self.buf.is_empty() {
            self.buf.resize(DRAIN_BUF, 0);
        }
        loop {
            let mut need = 0u32;
            let cap = self.buf.len() as u32;
            let used = unsafe { ffi::gridlock_rtc_drain(self.handle, self.buf.as_mut_ptr(), cap, &mut need) };
            for_each_event(&self.buf[..(used as usize).min(self.buf.len())], &mut on);
            if need == 0 {
                break;
            }
            if need as usize > self.buf.len() {
                self.buf.resize(need as usize, 0);
            }
        }
    }
}

impl Drop for RtcSocket {
    fn drop(&mut self) {
        unsafe { ffi::gridlock_rtc_close(self.handle) }
    }
}

/// How long a URL handed to [`prewarm`] or [`preconnect`] is not handed over
/// again. Callers repeat every frame; the JS side keeps what they started for
/// 10-15 s, so passing on each repeat only re-parses the URL to find it.
// short enough that a socket JS dropped is re-warmed well before a join
const REPEAT_AFTER: Duration = Duration::from_millis(250);

thread_local! {
    static PREWARMED: RefCell<HashMap<String, Instant>> = RefCell::new(HashMap::new());
    static PRECONNECTED: RefCell<HashMap<String, Instant>> = RefCell::new(HashMap::new());
}

/// Whether `url` was last handed over at least [`REPEAT_AFTER`] ago (or never);
/// if so, it counts as handed over now.
fn due(sent: &'static LocalKey<RefCell<HashMap<String, Instant>>>, url: &str) -> bool {
    sent.with(|sent| {
        let mut sent = sent.borrow_mut();
        let now = Instant::now();
        let fresh = |at: &Instant| now.saturating_duration_since(*at) < REPEAT_AFTER;
        if sent.get(url).is_some_and(fresh) {
            return false;
        }
        sent.retain(|_, at| fresh(at));
        sent.insert(url.to_string(), now);
        true
    })
}

/// Get ready to join the room at `room_url` (the player points at it): its
/// host's spare offer and the ICE servers are fetched ahead. Cheap to call
/// every frame; repeats are ignored for a while.
pub fn prewarm(room_url: &str) {
    if due(&PREWARMED, room_url) {
        unsafe { ffi::gridlock_rtc_prewarm(room_url.as_ptr(), room_url.len() as u32) }
    }
}

/// Connect to the room at `room_url` now, ahead of a join: the next
/// [`RtcLink::open`] for it takes over the connection, open already. Call
/// every frame while the join is likely (the room is listed, or pointed
/// at); one not asked for in a few seconds is closed. The host learns the
/// player's address from the connection, as it would from a join.
pub fn preconnect(room_url: &str) {
    if due(&PRECONNECTED, room_url) {
        unsafe { ffi::gridlock_rtc_preconnect(room_url.as_ptr(), room_url.len() as u32) }
    }
}

/// A player's link to the host of a signalling room.
pub struct RtcLink {
    socket: RtcSocket,
    host: Option<u32>,
    closed: bool,
    // packets polled but not yet handed out by recv()
    inbox: Vec<Packet>,
}

impl RtcLink {
    pub fn open(room_url: &str) -> Self {
        RtcLink { socket: RtcSocket::join(room_url), host: None, closed: false, inbox: Vec::new() }
    }

    fn update(&mut self) {
        let (host, closed, inbox) = (&mut self.host, &mut self.closed, &mut self.inbox);
        self.socket.poll(|event| match event {
            // The room only ever connects a player to its host.
            RtcEvent::PeerJoined(peer) if host.is_none() => *host = Some(peer),
            RtcEvent::PeerLeft(peer) if *host == Some(peer) => *closed = true,
            RtcEvent::Packet(peer, data) if *host == Some(peer) => inbox.push(data.into()),
            RtcEvent::Closed => *closed = true,
            _ => {}
        });
    }
}

impl Link for RtcLink {
    fn send(&mut self, packet: &[u8]) {
        if let (false, Some(host)) = (self.closed, self.host) {
            self.socket.send(host, packet);
        }
    }

    fn recv(&mut self) -> Vec<Packet> {
        self.update();
        std::mem::take(&mut self.inbox)
    }

    fn status(&mut self) -> LinkStatus {
        self.update();
        match (self.closed, self.host) {
            (true, _) => LinkStatus::Closed,
            (false, Some(_)) => LinkStatus::Open,
            (false, None) => LinkStatus::Connecting,
        }
    }
}

/// One HTTP request made with the browser's fetch().
pub struct HttpRequest {
    id: Option<u32>,
}

impl HttpRequest {
    pub fn start(method: &str, url: &str, body: &[u8], authorization: Option<&str>) -> Self {
        let auth = authorization.unwrap_or("");
        let id = unsafe {
            ffi::gridlock_http_start(
                method.as_ptr(),
                method.len() as u32,
                url.as_ptr(),
                url.len() as u32,
                body.as_ptr(),
                body.len() as u32,
                auth.as_ptr(),
                auth.len() as u32,
            )
        };
        HttpRequest { id: Some(id) }
    }

    /// `None` while in flight, then once `(status, body)`; status 0 means the
    /// request never reached a server.
    pub fn poll(&mut self) -> Option<(u16, Vec<u8>)> {
        let id = self.id?;
        let (mut status, mut len) = (0u32, 0u32);
        if unsafe { ffi::gridlock_http_poll(id, &mut status, &mut len) } == 0 {
            return None;
        }
        let mut body = vec![0u8; len as usize];
        unsafe { ffi::gridlock_http_take(id, body.as_mut_ptr()) };
        self.id = None;
        Some((status as u16, body))
    }
}

impl Drop for HttpRequest {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            unsafe { ffi::gridlock_http_cancel(id) }
        }
    }
}
