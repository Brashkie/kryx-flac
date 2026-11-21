//! A big-endian (MSB-first) bit reader over a byte slice — the FLAC bitstream
//! order.

use crate::error::{FlacError, Result};

/// Reads bits MSB-first from a borrowed byte slice.
pub struct BitReader<'a> {
    data: &'a [u8],
    /// Absolute bit position from the start of `data`.
    pos: usize,
}

impl<'a> BitReader<'a> {
    /// A reader positioned at the first bit of `data`.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current bit position.
    pub fn bit_pos(&self) -> usize {
        self.pos
    }

    /// Total number of bits in the stream.
    pub fn bit_len(&self) -> usize {
        self.data.len() * 8
    }

    /// Current byte position (only meaningful on a byte boundary).
    pub fn byte_pos(&self) -> usize {
        self.pos / 8
    }

    /// Borrow the raw bytes in `[start, end)` (used to compute frame CRCs).
    pub fn bytes(&self, start: usize, end: usize) -> Result<&'a [u8]> {
        self.data.get(start..end).ok_or(FlacError::UnexpectedEof)
    }

    /// Read `n` bytes as a big-endian unsigned value. The reader must be byte
    /// aligned; advances by `n` bytes.
    pub fn read_bytes_be(&mut self, n: u32) -> Result<u64> {
        self.read(n * 8)
    }

    /// Read `n` bytes as a little-endian unsigned value (VORBIS_COMMENT fields
    /// are little-endian). The reader must be byte aligned.
    pub fn read_bytes_le(&mut self, n: u32) -> Result<u64> {
        let mut v: u64 = 0;
        for i in 0..n {
            let byte = self.read(8)?;
            v |= byte << (8 * i);
        }
        Ok(v)
    }

    /// Read `n` raw bytes into an owned vector. The reader must be byte aligned.
    pub fn read_vec(&mut self, n: usize) -> Result<Vec<u8>> {
        let start = self.byte_pos();
        let slice = self.bytes(start, start + n)?.to_vec();
        self.pos += n * 8;
        Ok(slice)
    }

    /// Seek to an absolute bit position.
    pub fn seek_bit(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Advance to the next byte boundary (no-op if already aligned).
    pub fn align_to_byte(&mut self) {
        if self.pos % 8 != 0 {
            self.pos = (self.pos + 7) & !7;
        }
    }

    /// Read `n` bits (0..=64) as an unsigned value, MSB-first.
    ///
    /// Consumes whole chunks of each byte at a time rather than bit by bit.
    pub fn read(&mut self, n: u32) -> Result<u64> {
        if n == 0 {
            return Ok(0);
        }
        let end = self.pos + n as usize;
        if end > self.data.len() * 8 {
            return Err(FlacError::UnexpectedEof);
        }
        let mut v: u64 = 0;
        let mut pos = self.pos;
        let mut need = n;
        while need > 0 {
            let byte = self.data[pos >> 3] as u64;
            let bit_off = (pos & 7) as u32; // bits already consumed in this byte
            let avail = 8 - bit_off; // bits left in this byte
            let take = need.min(avail);
            // Drop the `avail - take` low bits, then keep the low `take` bits.
            let chunk = (byte >> (avail - take)) & ((1u64 << take) - 1);
            v = (v << take) | chunk;
            pos += take as usize;
            need -= take;
        }
        self.pos = pos;
        Ok(v)
    }

    /// Read `n` bits as a two's-complement signed value.
    pub fn read_signed(&mut self, n: u32) -> Result<i64> {
        if n == 0 {
            return Ok(0);
        }
        let v = self.read(n)?;
        // Sign-extend from bit n-1.
        let shift = 64 - n;
        Ok(((v << shift) as i64) >> shift)
    }

    /// Read a unary-coded value: the count of 0 bits before the terminating 1.
    ///
    /// Scans a byte at a time, using `leading_zeros` to count runs of zeros.
    pub fn read_unary(&mut self) -> Result<u32> {
        let mut q = 0u32;
        loop {
            let byte = *self
                .data
                .get(self.pos >> 3)
                .ok_or(FlacError::UnexpectedEof)?;
            let bit_off = (self.pos & 7) as u32;
            // Shift the current bit up to the MSB; zeros above it don't count.
            let window = (byte as u32) << (24 + bit_off);
            if window == 0 {
                // The rest of this byte is all zeros; move to the next byte.
                let avail = 8 - bit_off;
                q += avail;
                self.pos += avail as usize;
            } else {
                let lead = window.leading_zeros();
                q += lead;
                self.pos += (lead + 1) as usize; // zeros + the terminating 1
                return Ok(q);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bits_msb_first() {
        // 0b1011_0010, 0b1100_0000
        let mut r = BitReader::new(&[0b1011_0010, 0b1100_0000]);
        assert_eq!(r.read(3).unwrap(), 0b101);
        assert_eq!(r.read(5).unwrap(), 0b1_0010);
        assert_eq!(r.read(2).unwrap(), 0b11);
    }

    #[test]
    fn signed_sign_extends() {
        // 0b1111 as 4-bit signed = -1; 0b0111 = 7
        let mut r = BitReader::new(&[0b1111_0111]);
        assert_eq!(r.read_signed(4).unwrap(), -1);
        assert_eq!(r.read_signed(4).unwrap(), 7);
    }

    #[test]
    fn unary_counts_zeros() {
        // 0b0001_xxxx → three zeros then a one → 3
        let mut r = BitReader::new(&[0b0001_0000]);
        assert_eq!(r.read_unary().unwrap(), 3);
    }

    #[test]
    fn align_moves_to_byte_boundary() {
        let mut r = BitReader::new(&[0xFF, 0x00]);
        r.read(3).unwrap();
        r.align_to_byte();
        assert_eq!(r.bit_pos() % 8, 0);
        assert_eq!(r.bit_pos(), 8);
    }

    #[test]
    fn eof_errors() {
        let mut r = BitReader::new(&[0x00]);
        assert_eq!(r.read(9), Err(FlacError::UnexpectedEof));
    }
}
