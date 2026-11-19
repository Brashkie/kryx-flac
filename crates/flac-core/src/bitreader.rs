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
    pub fn read(&mut self, n: u32) -> Result<u64> {
        let mut v: u64 = 0;
        for _ in 0..n {
            let byte = *self
                .data
                .get(self.pos >> 3)
                .ok_or(FlacError::UnexpectedEof)?;
            let bit = (byte >> (7 - (self.pos & 7))) & 1;
            v = (v << 1) | bit as u64;
            self.pos += 1;
        }
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
    pub fn read_unary(&mut self) -> Result<u32> {
        let mut q = 0;
        while self.read(1)? == 0 {
            q += 1;
        }
        Ok(q)
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
