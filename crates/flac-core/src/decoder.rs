//! Top-level decode: read metadata, then every frame, applying inter-channel
//! decorrelation, into planar PCM.

use crate::bitreader::BitReader;
use crate::error::Result;
use crate::frame::{read_frame_header, ChannelAssignment};
use crate::metadata::{read_until_audio, StreamInfo};
use crate::subframe::decode_subframe;

/// Fully decoded FLAC audio: the stream parameters plus one `i32` sample vector
/// per channel (planar). Use [`interleaved`](FlacAudio::interleaved) to get
/// channel-interleaved samples.
#[derive(Debug, Clone)]
pub struct FlacAudio {
    /// Stream parameters from STREAMINFO.
    pub info: StreamInfo,
    /// One sample vector per channel (all the same length).
    pub channels: Vec<Vec<i32>>,
}

impl FlacAudio {
    /// Number of interchannel samples (frames).
    pub fn frame_count(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    /// Channel-interleaved samples: `ch0[0], ch1[0], …, ch0[1], ch1[1], …`.
    pub fn interleaved(&self) -> Vec<i32> {
        let n = self.frame_count();
        let nch = self.channels.len();
        let mut out = Vec::with_capacity(n * nch);
        for i in 0..n {
            for ch in &self.channels {
                out.push(ch[i]);
            }
        }
        out
    }
}

/// Decode a complete in-memory FLAC stream to planar PCM.
pub fn decode(data: &[u8]) -> Result<FlacAudio> {
    let mut r = BitReader::new(data);
    let info = read_until_audio(&mut r)?;

    let mut channels: Vec<Vec<i32>> = (0..info.channels).map(|_| Vec::new()).collect();
    let mut decoded: u64 = 0;

    loop {
        r.align_to_byte();

        if info.total_samples > 0 && decoded >= info.total_samples {
            break;
        }
        // Need at least a sync code to attempt another frame.
        if r.bit_pos() + 14 > r.bit_len() {
            break;
        }

        // A bad sync or EOF here means we've reached the end / padding.
        let header = match read_frame_header(&mut r, &info) {
            Ok(h) => h,
            Err(_) => break,
        };

        let bs = header.block_size as usize;
        let bps = header.bits_per_sample;

        let frame_channels: Vec<Vec<i32>> = match header.channels {
            ChannelAssignment::Independent(nch) => {
                let mut v = Vec::with_capacity(nch as usize);
                for _ in 0..nch {
                    v.push(decode_subframe(&mut r, bs, bps)?);
                }
                v
            }
            ChannelAssignment::LeftSide => {
                let left = decode_subframe(&mut r, bs, bps)?;
                let side = decode_subframe(&mut r, bs, bps + 1)?;
                let right: Vec<i32> = (0..bs).map(|i| left[i] - side[i]).collect();
                vec![left, right]
            }
            ChannelAssignment::RightSide => {
                let side = decode_subframe(&mut r, bs, bps + 1)?;
                let right = decode_subframe(&mut r, bs, bps)?;
                let left: Vec<i32> = (0..bs).map(|i| right[i] + side[i]).collect();
                vec![left, right]
            }
            ChannelAssignment::MidSide => {
                let mid = decode_subframe(&mut r, bs, bps)?;
                let side = decode_subframe(&mut r, bs, bps + 1)?;
                let mut left = Vec::with_capacity(bs);
                let mut right = Vec::with_capacity(bs);
                for i in 0..bs {
                    let s = side[i] as i64;
                    // Recover the dropped LSB of the mid channel.
                    let m = ((mid[i] as i64) << 1) | (s & 1);
                    left.push(((m + s) >> 1) as i32);
                    right.push(((m - s) >> 1) as i32);
                }
                vec![left, right]
            }
        };

        for (dst, src) in channels.iter_mut().zip(frame_channels) {
            dst.extend(src);
        }
        decoded += bs as u64;

        // Byte-align and consume the 16-bit frame CRC.
        r.align_to_byte();
        if r.bit_pos() + 16 <= r.bit_len() {
            let _crc16 = r.read(16)?;
        }
    }

    Ok(FlacAudio { info, channels })
}
