//! Top-level decode: read metadata, then every frame, verifying CRCs and
//! applying inter-channel decorrelation, into planar PCM. Also exposes a
//! metadata-only read and a sample-range (seeking) decode.

use crate::bitreader::BitReader;
use crate::crc::{crc16, crc8};
use crate::error::{FlacError, Result};
use crate::frame::{read_frame_header, ChannelAssignment};
use crate::metadata::{read_until_audio, FlacMetadata, Picture, SeekPoint, StreamInfo};
use crate::subframe::decode_subframe;

/// Fully decoded FLAC audio: the stream parameters, metadata, and one `i32`
/// sample vector per channel (planar). Use
/// [`interleaved`](FlacAudio::interleaved) to get channel-interleaved samples.
#[derive(Debug, Clone)]
pub struct FlacAudio {
    /// Stream parameters from STREAMINFO.
    pub info: StreamInfo,
    /// One sample vector per channel (all the same length).
    pub channels: Vec<Vec<i32>>,
    /// VORBIS_COMMENT tags as `(KEY, value)` pairs (keys uppercased).
    pub tags: Vec<(String, String)>,
    /// Attached pictures (cover art, etc.).
    pub pictures: Vec<Picture>,
    /// Seek points declared in the SEEKTABLE (placeholders removed).
    pub seek_points: Vec<SeekPoint>,
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

    /// First value of a tag, case-insensitive on the key (e.g. `"TITLE"`).
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// Read only the metadata region (STREAMINFO, tags, pictures, seek table)
/// without decoding any audio. Cheap: stops at the first audio frame.
pub fn read_metadata(data: &[u8]) -> Result<FlacMetadata> {
    let mut r = BitReader::new(data);
    read_until_audio(&mut r)
}

/// Decode a complete in-memory FLAC stream to planar PCM, verifying every
/// frame's header CRC-8 and footer CRC-16.
pub fn decode(data: &[u8]) -> Result<FlacAudio> {
    let mut r = BitReader::new(data);
    let meta = read_until_audio(&mut r)?;
    let info = meta.stream_info;

    let mut channels: Vec<Vec<i32>> = (0..info.channels).map(|_| Vec::new()).collect();
    let mut decoded: u64 = 0;

    loop {
        if info.total_samples > 0 && decoded >= info.total_samples {
            break;
        }
        match decode_one_frame(&mut r, &info)? {
            None => break,
            Some((bs, frame_channels)) => {
                for (dst, src) in channels.iter_mut().zip(frame_channels) {
                    dst.extend(src);
                }
                decoded += bs as u64;
            }
        }
    }

    Ok(FlacAudio {
        info,
        channels,
        tags: meta.tags,
        pictures: meta.pictures,
        seek_points: meta.seek_points,
    })
}

/// Decode only the interchannel samples `[start, start + count)`.
///
/// Uses the SEEKTABLE (when present) to jump close to `start` instead of
/// decoding from the beginning; otherwise decodes forward from the first frame.
/// The returned [`FlacAudio`] holds just the requested window.
pub fn decode_range(data: &[u8], start: u64, count: u64) -> Result<FlacAudio> {
    let mut r = BitReader::new(data);
    let meta = read_until_audio(&mut r)?;
    let info = meta.stream_info;
    let end = start.saturating_add(count);

    // Jump to the latest seek point at or before `start`, if any.
    let mut cursor: u64 = 0;
    if let Some(sp) = meta.seek_points.iter().rfind(|p| p.sample <= start) {
        let byte = meta.audio_offset as u64 + sp.offset;
        r.seek_bit(byte as usize * 8);
        cursor = sp.sample;
    }

    let mut channels: Vec<Vec<i32>> = (0..info.channels).map(|_| Vec::new()).collect();

    loop {
        if cursor >= end {
            break;
        }
        if info.total_samples > 0 && cursor >= info.total_samples {
            break;
        }
        let (bs, frame_channels) = match decode_one_frame(&mut r, &info)? {
            None => break,
            Some(v) => v,
        };
        let frame_start = cursor;
        let frame_end = cursor + bs as u64;
        cursor = frame_end;

        // Keep only the overlap of this frame with [start, end).
        let lo = start.max(frame_start);
        let hi = end.min(frame_end);
        if lo < hi {
            let a = (lo - frame_start) as usize;
            let b = (hi - frame_start) as usize;
            for (dst, src) in channels.iter_mut().zip(&frame_channels) {
                dst.extend_from_slice(&src[a..b]);
            }
        }
    }

    Ok(FlacAudio {
        info,
        channels,
        tags: meta.tags,
        pictures: meta.pictures,
        seek_points: meta.seek_points,
    })
}

/// Decode a single frame at the reader's current position, verifying CRCs.
/// Returns `Ok(None)` when a frame can't be started (bad sync / EOF / padding).
fn decode_one_frame(r: &mut BitReader, info: &StreamInfo) -> Result<Option<(u32, Vec<Vec<i32>>)>> {
    r.align_to_byte();
    let frame_start = r.byte_pos();

    // Need at least a sync code to attempt another frame.
    if r.bit_pos() + 14 > r.bit_len() {
        return Ok(None);
    }

    let header = match read_frame_header(r, info) {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };

    // Verify the header CRC-8 (over the header bytes, excluding the CRC byte).
    let after_header = r.byte_pos();
    let header_bytes = r.bytes(frame_start, after_header - 1)?;
    let stored_crc8 = r.bytes(after_header - 1, after_header)?[0];
    if crc8(header_bytes) != stored_crc8 {
        return Err(FlacError::CrcMismatch("header"));
    }

    let bs = header.block_size as usize;
    let bps = header.bits_per_sample;

    let frame_channels: Vec<Vec<i32>> = match header.channels {
        ChannelAssignment::Independent(nch) => {
            let mut v = Vec::with_capacity(nch as usize);
            for _ in 0..nch {
                v.push(decode_subframe(r, bs, bps)?);
            }
            v
        }
        ChannelAssignment::LeftSide => {
            let left = decode_subframe(r, bs, bps)?;
            let side = decode_subframe(r, bs, bps + 1)?;
            let right: Vec<i32> = (0..bs).map(|i| left[i] - side[i]).collect();
            vec![left, right]
        }
        ChannelAssignment::RightSide => {
            let side = decode_subframe(r, bs, bps + 1)?;
            let right = decode_subframe(r, bs, bps)?;
            let left: Vec<i32> = (0..bs).map(|i| right[i] + side[i]).collect();
            vec![left, right]
        }
        ChannelAssignment::MidSide => {
            let mid = decode_subframe(r, bs, bps)?;
            let side = decode_subframe(r, bs, bps + 1)?;
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

    // Byte-align, then verify the frame CRC-16 (over the whole frame) and
    // consume the stored value.
    r.align_to_byte();
    let frame_end = r.byte_pos();
    let frame_bytes = r.bytes(frame_start, frame_end)?;
    let computed = crc16(frame_bytes);
    let stored_crc16 = r.read(16)? as u16;
    if computed != stored_crc16 {
        return Err(FlacError::CrcMismatch("frame"));
    }

    Ok(Some((header.block_size, frame_channels)))
}
