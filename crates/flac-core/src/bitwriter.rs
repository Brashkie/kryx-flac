//! A big-endian (MSB-first) bit writer — the mirror of [`BitReader`], used by
//! the encoder to assemble the FLAC bitstream.
//!
//! [`BitReader`]: crate::bitreader::BitReader

/// Accumulates bits MSB-first into a byte buffer.
#[derive(Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    /// Bit accumulator holding `nbits` valid low bits.
    acc: u64,
    nbits: u32,
}

impl BitWriter {
    /// An empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append the low `n` bits of `val` (0..=32), MSB-first.
    pub fn write(&mut self, val: u64, n: u32) {
        debug_assert!(n <= 32);
        if n == 0 {
            return;
        }
        let mask = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
        self.acc = (self.acc << n) | (val & mask);
        self.nbits += n;
        while self.nbits >= 8 {
            self.nbits -= 8;
            self.bytes.push((self.acc >> self.nbits) as u8);
        }
    }

    /// Append `n` bits (0..=64), splitting into <=32-bit chunks.
    pub fn write_wide(&mut self, val: u64, n: u32) {
        if n > 32 {
            let hi = n - 32;
            self.write(val >> 32, hi);
            self.write(val & 0xFFFF_FFFF, 32);
        } else {
            self.write(val, n);
        }
    }

    /// Append the low `n` bits of a two's-complement signed value.
    pub fn write_signed(&mut self, val: i64, n: u32) {
        let mask = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
        self.write_wide((val as u64) & mask, n);
    }

    /// Write a unary code: `q` zero bits then a terminating `1`.
    pub fn write_unary(&mut self, q: u32) {
        let mut left = q;
        while left >= 32 {
            self.write(0, 32);
            left -= 32;
        }
        // `left` zeros followed by a 1 == the value 1 in `left + 1` bits.
        self.write(1, left + 1);
    }

    /// Pad with zero bits up to the next byte boundary.
    pub fn align_byte(&mut self) {
        if self.nbits % 8 != 0 {
            let pad = 8 - (self.nbits % 8);
            self.write(0, pad);
        }
    }

    /// Number of whole bytes flushed so far (valid only when byte aligned).
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// The flushed bytes so far (valid only when byte aligned).
    pub fn bytes_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Finish and return the buffer (pads to a byte boundary first).
    pub fn finish(mut self) -> Vec<u8> {
        self.align_byte();
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_msb_first() {
        let mut w = BitWriter::new();
        w.write(0b101, 3);
        w.write(0b1_0010, 5);
        assert_eq!(w.finish(), vec![0b1011_0010]);
    }

    #[test]
    fn unary_roundtrips_with_reader() {
        use crate::bitreader::BitReader;
        let mut w = BitWriter::new();
        for q in [0u32, 1, 7, 40, 100] {
            w.write_unary(q);
        }
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        for q in [0u32, 1, 7, 40, 100] {
            assert_eq!(r.read_unary().unwrap(), q);
        }
    }

    #[test]
    fn signed_roundtrips() {
        use crate::bitreader::BitReader;
        let mut w = BitWriter::new();
        for v in [-1i64, 7, -16, 15, 0] {
            w.write_signed(v, 5);
        }
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        for v in [-1i64, 7, -16, 15, 0] {
            assert_eq!(r.read_signed(5).unwrap(), v);
        }
    }
}
