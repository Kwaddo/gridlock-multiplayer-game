//! Byte-level helpers for the wire format: big-endian numbers and
//! `u8`-length-prefixed UTF-8 strings.

pub type Vec3 = (f32, f32, f32);

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

    pub fn str8(&mut self) -> Option<String> {
        let len = self.u8()? as usize;
        self.bytes(len).map(|b| String::from_utf8_lossy(b).into_owned())
    }
}
