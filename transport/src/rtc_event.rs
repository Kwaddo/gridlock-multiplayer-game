//! WebRTC events as `web/gridlock_web.js` hands them over: drained in
//! batches into one buffer (`gridlock_rtc_drain`), not one call per event.

/// Something that happened on an `RtcSocket` since the last poll.
pub enum RtcEvent<'a> {
    /// A data channel to `peer` opened. Peer ids are small, unique per socket
    /// and never reused, so a host can use them as server peer ids directly.
    PeerJoined(u32),
    PeerLeft(u32),
    /// A packet from a peer; the bytes are only lent for the callback.
    Packet(u32, &'a [u8]),
    /// Signalling closed (e.g. the host left, or the room was taken).
    Closed,
}

/// Each drained event starts with kind, peer and payload length, as
/// little-endian u32s, then has the payload.
pub(crate) const EVENT_HEADER: usize = 12;

/// Hand the events in `drained` to `on`, in order; a truncated one ends it.
pub(crate) fn for_each_event(mut drained: &[u8], on: &mut impl FnMut(RtcEvent<'_>)) {
    let word = |b: &[u8], i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    while drained.len() >= EVENT_HEADER {
        let (kind, peer, len) = (word(drained, 0), word(drained, 4), word(drained, 8) as usize);
        let Some(data) = drained.get(EVENT_HEADER..EVENT_HEADER + len) else {
            return;
        };
        on(match kind {
            1 => RtcEvent::PeerJoined(peer),
            2 => RtcEvent::PeerLeft(peer),
            3 => RtcEvent::Packet(peer, data),
            _ => RtcEvent::Closed,
        });
        drained = &drained[EVENT_HEADER + len..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: u32, peer: u32, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for word in [kind, peer, data.len() as u32] {
            out.extend(word.to_le_bytes());
        }
        out.extend(data);
        out
    }

    fn read(drained: &[u8]) -> Vec<String> {
        let mut seen = Vec::new();
        for_each_event(drained, &mut |e| {
            seen.push(match e {
                RtcEvent::PeerJoined(p) => format!("joined {p}"),
                RtcEvent::PeerLeft(p) => format!("left {p}"),
                RtcEvent::Packet(p, data) => format!("packet {p} {data:?}"),
                RtcEvent::Closed => "closed".into(),
            })
        });
        seen
    }

    #[test]
    fn reads_every_kind_in_order() {
        let mut drained = event(1, 7, &[]);
        drained.extend(event(3, 7, &[1, 2, 3]));
        drained.extend(event(3, 9, &[]));
        drained.extend(event(2, 7, &[]));
        drained.extend(event(4, 0, &[]));
        assert_eq!(read(&drained), ["joined 7", "packet 7 [1, 2, 3]", "packet 9 []", "left 7", "closed"]);
    }

    #[test]
    fn stops_at_a_truncated_event() {
        let mut drained = event(3, 1, &[5; 4]);
        drained.extend(&event(3, 1, &[6; 10])[..15]);
        assert_eq!(read(&drained), ["packet 1 [5, 5, 5, 5]"]);
        assert!(read(&[0; 11]).is_empty());
    }
}
