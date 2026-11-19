//! FLAC frame header parsing.

use crate::bitreader::BitReader;
use crate::error::{FlacError, Result};
use crate::metadata::StreamInfo;

/// How the two stereo channels were decorrelated (or that channels are stored
/// independently).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelAssignment {
    /// Each channel stored independently; holds the channel count (1..=8).
    Independent(u8),
    /// Left + side (side = left − right).
    LeftSide,
    /// Right + side (side = left − right).
    RightSide,
    /// Mid + side.
    MidSide,
}

/// A decoded frame header.
#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    /// Block size in interchannel samples.
    pub block_size: u32,
    /// Bits per sample for this frame.
    pub bits_per_sample: u32,
    /// Channel layout / decorrelation.
    pub channels: ChannelAssignment,
}

const BLOCK_SIZE_TABLE: [u32; 16] = [
    0, 192, 576, 1152, 2304, 4608, 0, 0, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];
const BPS_TABLE: [u32; 8] = [0, 8, 12, 0, 16, 20, 24, 0];

/// Read a FLAC/UTF-8-style coded number (used for the frame/sample number).
fn read_utf8_number(r: &mut BitReader) -> Result<u64> {
    let first = r.read(8)?;
    if first < 0x80 {
        return Ok(first);
    }
    // Count leading ones to find the continuation-byte count.
    let mut lead = 0u32;
    let mut t = first;
    while t & 0x80 != 0 {
        lead += 1;
        t = (t << 1) & 0xFF;
    }
    let mut val = first & (0x7F >> lead);
    for _ in 0..lead - 1 {
        val = (val << 6) | (r.read(8)? & 0x3F);
    }
    Ok(val)
}

/// Parse a frame header. The reader must be positioned at the 14-bit sync code.
pub fn read_frame_header(r: &mut BitReader, info: &StreamInfo) -> Result<FrameHeader> {
    // 14-bit frame sync code: 0b11111111111110.
    let sync = r.read(14)?;
    if sync != 0x3FFE {
        return Err(FlacError::BadSync);
    }
    let _reserved = r.read(1)?;
    let _blocking_strategy = r.read(1)?;
    let bs_code = r.read(4)? as usize;
    let sr_code = r.read(4)?;
    let ch_code = r.read(4)?;
    let ss_code = r.read(3)? as usize;
    let _reserved2 = r.read(1)?;

    let _number = read_utf8_number(r)?;

    let block_size = match bs_code {
        6 => r.read(8)? as u32 + 1,
        7 => r.read(16)? as u32 + 1,
        _ => BLOCK_SIZE_TABLE[bs_code],
    };

    // Sample rate from the header, when the code says so (we don't need the
    // value, but must consume the bits).
    match sr_code {
        12 => {
            r.read(8)?;
        }
        13 | 14 => {
            r.read(16)?;
        }
        _ => {}
    }

    let _crc8 = r.read(8)?;

    let bits_per_sample = if ss_code == 0 {
        info.bits_per_sample as u32
    } else {
        BPS_TABLE[ss_code]
    };

    let channels = match ch_code {
        0..=7 => ChannelAssignment::Independent((ch_code + 1) as u8),
        8 => ChannelAssignment::LeftSide,
        9 => ChannelAssignment::RightSide,
        10 => ChannelAssignment::MidSide,
        _ => return Err(FlacError::Reserved("channel assignment")),
    };

    Ok(FrameHeader {
        block_size,
        bits_per_sample,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_single_byte() {
        let mut r = BitReader::new(&[0x42]);
        assert_eq!(read_utf8_number(&mut r).unwrap(), 0x42);
    }

    #[test]
    fn utf8_multi_byte() {
        // 0xC2 0x80 → U+0080 = 128
        let mut r = BitReader::new(&[0xC2, 0x80]);
        assert_eq!(read_utf8_number(&mut r).unwrap(), 128);
    }
}
