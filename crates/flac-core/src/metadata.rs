//! FLAC metadata: the `fLaC` marker and the STREAMINFO block (the only one this
//! decoder needs). Other metadata blocks are skipped.

use crate::bitreader::BitReader;
use crate::error::{FlacError, Result};

/// The stream's global parameters, from the mandatory STREAMINFO block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInfo {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels (1..=8).
    pub channels: u8,
    /// Bits per sample (4..=32).
    pub bits_per_sample: u8,
    /// Total interchannel samples (0 if unknown).
    pub total_samples: u64,
    /// Minimum block size (samples).
    pub min_block_size: u16,
    /// Maximum block size (samples).
    pub max_block_size: u16,
}

/// Read the `fLaC` marker, then every metadata block, returning STREAMINFO.
/// Leaves `r` positioned at the first audio frame.
pub fn read_until_audio(r: &mut BitReader) -> Result<StreamInfo> {
    // "fLaC"
    if r.read(32)? != 0x664C6143 {
        return Err(FlacError::BadMagic);
    }

    let mut stream_info = None;
    loop {
        let last = r.read(1)? == 1;
        let block_type = r.read(7)?;
        let length = r.read(24)? as usize;
        let block_start = r.bit_pos();

        if block_type == 0 {
            // STREAMINFO
            let min_block_size = r.read(16)? as u16;
            let max_block_size = r.read(16)? as u16;
            let _min_frame = r.read(24)?;
            let _max_frame = r.read(24)?;
            let sample_rate = r.read(20)? as u32;
            let channels = (r.read(3)? + 1) as u8;
            let bits_per_sample = (r.read(5)? + 1) as u8;
            let total_samples = r.read(36)?;
            stream_info = Some(StreamInfo {
                sample_rate,
                channels,
                bits_per_sample,
                total_samples,
                min_block_size,
                max_block_size,
            });
        }

        // Skip to the end of this block regardless of type.
        r.seek_bit(block_start + length * 8);
        if last {
            break;
        }
    }

    stream_info.ok_or(FlacError::MissingStreamInfo)
}
