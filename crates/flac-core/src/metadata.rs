//! FLAC metadata: the `fLaC` marker, STREAMINFO, and the optional blocks this
//! decoder surfaces — VORBIS_COMMENT (tags), PICTURE (cover art), and SEEKTABLE
//! (seek points). Any other block type is skipped.

use crate::bitreader::BitReader;
use crate::error::{FlacError, Result};

/// The stream's global parameters, from the mandatory STREAMINFO block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// A picture attached to the stream (PICTURE metadata block), e.g. cover art.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Picture type per the FLAC/ID3 APIC list (3 = front cover, …).
    pub picture_type: u32,
    /// MIME type, e.g. `image/jpeg`.
    pub mime: String,
    /// UTF-8 description (often empty).
    pub description: String,
    /// Width in pixels (0 if unknown).
    pub width: u32,
    /// Height in pixels (0 if unknown).
    pub height: u32,
    /// Color depth in bits per pixel (0 if unknown).
    pub depth: u32,
    /// Number of colors used, for indexed images (0 otherwise).
    pub colors: u32,
    /// The raw encoded image bytes.
    pub data: Vec<u8>,
}

/// A seek point (SEEKTABLE entry): a sample number and the byte offset of the
/// frame that contains it, relative to the first audio frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPoint {
    /// First interchannel sample in the target frame.
    pub sample: u64,
    /// Byte offset of the target frame from the first audio frame.
    pub offset: u64,
    /// Number of samples in the target frame.
    pub frame_samples: u16,
}

/// Everything parsed from the metadata region before the first audio frame.
#[derive(Debug, Clone, Default)]
pub struct FlacMetadata {
    /// Mandatory stream parameters.
    pub stream_info: StreamInfo,
    /// VORBIS_COMMENT vendor string (empty if no such block).
    pub vendor: String,
    /// VORBIS_COMMENT tags as `(KEY, value)` pairs, keys uppercased.
    pub tags: Vec<(String, String)>,
    /// Attached pictures (cover art, etc.).
    pub pictures: Vec<Picture>,
    /// Seek points, placeholders removed, sorted by sample.
    pub seek_points: Vec<SeekPoint>,
    /// Byte offset of the first audio frame from the start of the stream.
    pub audio_offset: usize,
}

impl FlacMetadata {
    /// First value of a tag, case-insensitive on the key (e.g. `"TITLE"`).
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// Read the `fLaC` marker and every metadata block, returning the parsed
/// metadata. Leaves `r` positioned at the first audio frame.
pub fn read_until_audio(r: &mut BitReader) -> Result<FlacMetadata> {
    // "fLaC"
    if r.read(32)? != 0x664C6143 {
        return Err(FlacError::BadMagic);
    }

    let mut meta = FlacMetadata::default();
    let mut have_stream_info = false;

    loop {
        let last = r.read(1)? == 1;
        let block_type = r.read(7)?;
        let length = r.read(24)? as usize;
        let block_start = r.bit_pos();

        match block_type {
            0 => {
                meta.stream_info = read_stream_info(r)?;
                have_stream_info = true;
            }
            3 => meta.seek_points = read_seek_table(r, length)?,
            4 => {
                let (vendor, tags) = read_vorbis_comment(r)?;
                meta.vendor = vendor;
                meta.tags = tags;
            }
            6 => meta.pictures.push(read_picture(r)?),
            _ => {}
        }

        // Skip to the declared end of this block regardless of how much we read.
        r.seek_bit(block_start + length * 8);
        if last {
            break;
        }
    }

    if !have_stream_info {
        return Err(FlacError::MissingStreamInfo);
    }
    meta.audio_offset = r.byte_pos();
    Ok(meta)
}

fn read_stream_info(r: &mut BitReader) -> Result<StreamInfo> {
    let min_block_size = r.read(16)? as u16;
    let max_block_size = r.read(16)? as u16;
    let _min_frame = r.read(24)?;
    let _max_frame = r.read(24)?;
    let sample_rate = r.read(20)? as u32;
    let channels = (r.read(3)? + 1) as u8;
    let bits_per_sample = (r.read(5)? + 1) as u8;
    let total_samples = r.read(36)?;
    Ok(StreamInfo {
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        min_block_size,
        max_block_size,
    })
}

/// SEEKTABLE: a sequence of 18-byte seek points. Placeholder points
/// (sample == u64::MAX) are dropped.
fn read_seek_table(r: &mut BitReader, length: usize) -> Result<Vec<SeekPoint>> {
    let count = length / 18;
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let sample = r.read(64)?;
        let offset = r.read(64)?;
        let frame_samples = r.read(16)? as u16;
        if sample != u64::MAX {
            points.push(SeekPoint {
                sample,
                offset,
                frame_samples,
            });
        }
    }
    points.sort_by_key(|p| p.sample);
    Ok(points)
}

/// VORBIS_COMMENT: little-endian length-prefixed UTF-8 strings.
fn read_vorbis_comment(r: &mut BitReader) -> Result<(String, Vec<(String, String)>)> {
    let vendor_len = r.read_bytes_le(4)? as usize;
    let vendor = utf8(r.read_vec(vendor_len)?)?;

    let count = r.read_bytes_le(4)? as usize;
    let mut tags = Vec::with_capacity(count);
    for _ in 0..count {
        let len = r.read_bytes_le(4)? as usize;
        let raw = utf8(r.read_vec(len)?)?;
        if let Some(eq) = raw.find('=') {
            let key = raw[..eq].to_ascii_uppercase();
            let value = raw[eq + 1..].to_string();
            tags.push((key, value));
        }
    }
    Ok((vendor, tags))
}

/// PICTURE: big-endian length-prefixed fields then the raw image bytes.
fn read_picture(r: &mut BitReader) -> Result<Picture> {
    let picture_type = r.read_bytes_be(4)? as u32;
    let mime_len = r.read_bytes_be(4)? as usize;
    let mime = utf8(r.read_vec(mime_len)?)?;
    let desc_len = r.read_bytes_be(4)? as usize;
    let description = utf8(r.read_vec(desc_len)?)?;
    let width = r.read_bytes_be(4)? as u32;
    let height = r.read_bytes_be(4)? as u32;
    let depth = r.read_bytes_be(4)? as u32;
    let colors = r.read_bytes_be(4)? as u32;
    let data_len = r.read_bytes_be(4)? as usize;
    let data = r.read_vec(data_len)?;
    Ok(Picture {
        picture_type,
        mime,
        description,
        width,
        height,
        depth,
        colors,
        data,
    })
}

fn utf8(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| FlacError::BadMetadata("invalid UTF-8"))
}
