//! Browser networking through the `gridlock_web` miniquad JS plugin
//! (`web/gridlock_web.js`): WebRTC data channels and fetch, both polled.

use crate::{Link, LinkStatus};

mod ffi {
    extern "C" {
        pub fn gridlock_rtc_open(url: *const u8, url_len: u32, is_host: u32) -> u32;
        pub fn gridlock_rtc_send(handle: u32, peer: u32, data: *const u8, len: u32);
        pub fn gridlock_rtc_next(handle: u32, peer_out: *mut u32, len_out: *mut u32) -> u32;
        pub fn gridlock_rtc_take(handle: u32, buf: *mut u8);
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

/// Something that happened on an [`RtcSocket`] since the last poll.
pub enum RtcEvent {
    /// A data channel to `peer` opened. Peer ids are small, unique per socket
    /// and never reused, so a host can use them as server peer ids directly.
    PeerJoined(u32),
    PeerLeft(u32),
    Packet(u32, Vec<u8>),
    /// Signalling closed (e.g. the host left, or the room was taken).
    Closed,
}

/// A WebRTC endpoint in a signalling room: the host (many peers) or a player
/// (one peer, the host).
pub struct RtcSocket {
    handle: u32,
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
        RtcSocket { handle }
    }

    pub fn send(&mut self, peer: u32, packet: &[u8]) {
        unsafe { ffi::gridlock_rtc_send(self.handle, peer, packet.as_ptr(), packet.len() as u32) }
    }

    pub fn poll(&mut self) -> Vec<RtcEvent> {
        let mut events = Vec::new();
        loop {
            let (mut peer, mut len) = (0u32, 0u32);
            let kind = unsafe { ffi::gridlock_rtc_next(self.handle, &mut peer, &mut len) };
            if kind == 0 {
                break;
            }
            let mut data = vec![0u8; len as usize];
            unsafe { ffi::gridlock_rtc_take(self.handle, data.as_mut_ptr()) };
            events.push(match kind {
                1 => RtcEvent::PeerJoined(peer),
                2 => RtcEvent::PeerLeft(peer),
                3 => RtcEvent::Packet(peer, data),
                _ => RtcEvent::Closed,
            });
        }
        events
    }
}

impl Drop for RtcSocket {
    fn drop(&mut self) {
        unsafe { ffi::gridlock_rtc_close(self.handle) }
    }
}

/// Get ready to join the room at `room_url` (the player points at it): its
/// host's spare offer and the ICE servers are fetched ahead. Cheap to call
/// every frame; repeats are ignored for a while.
pub fn prewarm(room_url: &str) {
    unsafe { ffi::gridlock_rtc_prewarm(room_url.as_ptr(), room_url.len() as u32) }
}

/// Connect to the room at `room_url` now, ahead of a join: the next
/// [`RtcLink::open`] for it takes over the connection, open already. Call
/// every frame while the join is likely (the room is listed, or pointed
/// at); one not asked for in a few seconds is closed. The host learns the
/// player's address from the connection, as it would from a join.
pub fn preconnect(room_url: &str) {
    unsafe { ffi::gridlock_rtc_preconnect(room_url.as_ptr(), room_url.len() as u32) }
}

/// A player's link to the host of a signalling room.
pub struct RtcLink {
    socket: RtcSocket,
    host: Option<u32>,
    closed: bool,
    // packets polled but not yet handed out by recv()
    inbox: Vec<Box<[u8]>>,
}

impl RtcLink {
    pub fn open(room_url: &str) -> Self {
        RtcLink { socket: RtcSocket::join(room_url), host: None, closed: false, inbox: Vec::new() }
    }

    fn update(&mut self) {
        for event in self.socket.poll() {
            match event {
                // The room only ever connects a player to its host.
                RtcEvent::PeerJoined(peer) if self.host.is_none() => self.host = Some(peer),
                RtcEvent::PeerLeft(peer) if self.host == Some(peer) => self.closed = true,
                RtcEvent::Packet(peer, data) if self.host == Some(peer) => self.inbox.push(data.into()),
                RtcEvent::Closed => self.closed = true,
                _ => {}
            }
        }
    }
}

impl Link for RtcLink {
    fn send(&mut self, packet: &[u8]) {
        if let (false, Some(host)) = (self.closed, self.host) {
            self.socket.send(host, packet);
        }
    }

    fn recv(&mut self) -> Vec<Box<[u8]>> {
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
