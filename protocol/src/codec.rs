//! Byte-level helpers for the wire format: big-endian numbers and
//! `u8`-length-prefixed UTF-8 strings.

pub type Vec3 = (f32, f32, f32);

/// Fixed-point position step on the wire: 1/64 m (15.6 mm), so a coordinate
/// comes back within 7.8 mm of what was sent. In an `i16` that spans
/// +-512 m on every axis. The worlds are far smaller: the lobby and shooting
/// range sit within about +-41 m, the biggest maze (43 cells of 1.8 m) spans
/// 0..77 m, and even the largest maze a `u8` side could describe (255 cells,
/// 459 m) fits. Anything beyond saturates at the edge instead of wrapping.
pub const POS_UNITS_PER_M: f32 = 64.0;

/// Largest coordinate magnitude that survives [`quantize_coord`] unclamped.
pub const POS_LIMIT_M: f32 = i16::MAX as f32 / POS_UNITS_PER_M;

/// Metres to wire units. Saturates out of range; NaN becomes 0.
pub fn quantize_coord(v: f32) -> i16 {
    // float -> int `as` casts saturate and map NaN to 0
    (v * POS_UNITS_PER_M).round() as i16
}

pub fn dequantize_coord(q: i16) -> f32 {
    q as f32 / POS_UNITS_PER_M
}

/// Radians to a `u16` turn fraction (65536 steps per full circle, 0.0055
/// degrees each). Any angle is accepted; non-finite ones become 0.
pub fn quantize_yaw(yaw: f32) -> u16 {
    let turns = yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
    // a turn that rounds up to 65536 is a full circle: wrap it to 0
    ((turns * 65536.0).round() as u32 & 0xFFFF) as u16
}

/// Back to radians, in `[-PI, PI)`.
pub fn dequantize_yaw(q: u16) -> f32 {
    q as i16 as f32 * (std::f32::consts::PI / 32768.0)
}

/// Look up/down angle (radians) to whole degrees in one byte, clamped to +-89,
/// plenty for bending a body, before this pitch wasnt sent at all
pub fn quantize_pitch(pitch: f32) -> u8 {
    // float -> int `as` casts saturate and map NaN to 0
    (pitch.to_degrees().round().clamp(-89.0, 89.0) as i8) as u8
}

/// Back to radians.
pub fn dequantize_pitch(q: u8) -> f32 {
    (q as i8 as f32).to_radians()
}

/// Append-only packet builder.
#[derive(Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new(msg_type: u8) -> Self {
        let mut buf = Vec::with_capacity(64);
        buf.push(msg_type);
        Writer { buf }
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }

    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(v as u8)
    }

    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn vec3(&mut self, v: Vec3) -> &mut Self {
        self.f32(v.0).f32(v.1).f32(v.2)
    }

    /// A position as three fixed-point `i16`s, see [`quantize_coord`].
    pub fn pos16(&mut self, v: Vec3) -> &mut Self {
        for c in [v.0, v.1, v.2] {
            self.u16(quantize_coord(c) as u16);
        }
        self
    }

    /// Length-prefixed string; anything past 255 bytes is cut off.
    pub fn str8(&mut self, s: &str) -> &mut Self {
        let bytes = &s.as_bytes()[..s.len().min(255)];
        self.u8(bytes.len() as u8);
        self.buf.extend_from_slice(bytes);
        self
    }

    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(b);
        self
    }

    pub fn finish(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }
}

/// Cursor over a packet payload. Every read returns `None` once the data runs out.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    pub fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let out = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(out)
    }

    pub fn rest(&mut self) -> &'a [u8] {
        let out = &self.buf[self.pos.min(self.buf.len())..];
        self.pos = self.buf.len();
        out
    }

    pub fn u8(&mut self) -> Option<u8> {
        self.bytes(1).map(|b| b[0])
    }

    pub fn bool(&mut self) -> Option<bool> {
        self.u8().map(|b| b != 0)
    }

    pub fn u16(&mut self) -> Option<u16> {
        self.bytes(2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Option<u32> {
        self.bytes(4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn f32(&mut self) -> Option<f32> {
        self.bytes(4).map(|b| f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn vec3(&mut self) -> Option<Vec3> {
        Some((self.f32()?, self.f32()?, self.f32()?))
    }

    pub fn pos16(&mut self) -> Option<Vec3> {
        let mut c = || self.u16().map(|q| dequantize_coord(q as i16));
        Some((c()?, c()?, c()?))
    }

    pub fn str8(&mut self) -> Option<String> {
        let len = self.u8()? as usize;
        self.bytes(len).map(|b| String::from_utf8_lossy(b).into_owned())
    }
}
