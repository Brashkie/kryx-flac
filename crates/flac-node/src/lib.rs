//! N-API bindings for @kryxjs/codecs-flac.
//!
//! A thin bridge over the pure-Rust `kryx-flac` decoder: decode a whole file or
//! a sample range to interleaved `Int32Array` PCM, or read just the metadata
//! (tags, cover art, seek table). JS↔native crossing is coarse (one call each).

use napi::bindgen_prelude::{Buffer, Int32Array};
use napi_derive::napi;

/// A VORBIS_COMMENT tag.
#[napi(object)]
pub struct FlacTag {
    /// Tag key, uppercased (e.g. `TITLE`, `ARTIST`).
    pub key: String,
    /// Tag value.
    pub value: String,
}

/// An attached picture (cover art).
#[napi(object)]
pub struct FlacPicture {
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
    /// Raw encoded image bytes.
    pub data: Buffer,
}

/// A seek point from the SEEKTABLE.
#[napi(object)]
pub struct FlacSeekPoint {
    /// First interchannel sample in the target frame.
    pub sample: f64,
    /// Byte offset of the target frame from the first audio frame.
    pub offset: f64,
    /// Number of samples in the target frame.
    pub frame_samples: u32,
}

/// The result of decoding a FLAC stream (whole file or a range).
#[napi(object)]
pub struct DecodedFlac {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Bits per sample declared by the stream.
    pub bits_per_sample: u32,
    /// Total interchannel samples declared in STREAMINFO (0 if unknown).
    pub total_samples: f64,
    /// Number of interchannel samples actually returned.
    pub frame_count: u32,
    /// Interleaved samples: `ch0[0], ch1[0], …, ch0[1], …` as signed i32.
    pub samples: Int32Array,
    /// VORBIS_COMMENT tags.
    pub tags: Vec<FlacTag>,
    /// Attached pictures (cover art, etc.).
    pub pictures: Vec<FlacPicture>,
}

/// Stream metadata without any decoded audio.
#[napi(object)]
pub struct FlacMetadata {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Bits per sample declared by the stream.
    pub bits_per_sample: u32,
    /// Total interchannel samples declared in STREAMINFO (0 if unknown).
    pub total_samples: f64,
    /// VORBIS_COMMENT vendor string.
    pub vendor: String,
    /// VORBIS_COMMENT tags.
    pub tags: Vec<FlacTag>,
    /// Attached pictures (cover art, etc.).
    pub pictures: Vec<FlacPicture>,
    /// Seek points declared in the SEEKTABLE (placeholders removed).
    pub seek_points: Vec<FlacSeekPoint>,
}

fn map_tags(tags: Vec<(String, String)>) -> Vec<FlacTag> {
    tags.into_iter()
        .map(|(key, value)| FlacTag { key, value })
        .collect()
}

fn map_pictures(pics: Vec<kryx_flac::Picture>) -> Vec<FlacPicture> {
    pics.into_iter()
        .map(|p| FlacPicture {
            picture_type: p.picture_type,
            mime: p.mime,
            description: p.description,
            width: p.width,
            height: p.height,
            depth: p.depth,
            colors: p.colors,
            data: p.data.into(),
        })
        .collect()
}

fn into_decoded(audio: kryx_flac::FlacAudio) -> DecodedFlac {
    let frame_count = audio.frame_count() as u32;
    let sample_rate = audio.info.sample_rate;
    let channels = audio.info.channels as u32;
    let bits_per_sample = audio.info.bits_per_sample as u32;
    let total_samples = audio.info.total_samples as f64;
    let interleaved = audio.interleaved();
    DecodedFlac {
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        frame_count,
        samples: Int32Array::new(interleaved),
        tags: map_tags(audio.tags),
        pictures: map_pictures(audio.pictures),
    }
}

/// Decode a complete FLAC file to interleaved PCM (CRC-verified).
#[napi]
pub fn decode(input: Buffer) -> napi::Result<DecodedFlac> {
    let audio =
        kryx_flac::decode(input.as_ref()).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(into_decoded(audio))
}

/// Decode only the interchannel samples `[start, start + count)`, using the
/// seek table when present. `start` and `count` are sample indices.
#[napi]
pub fn decode_range(input: Buffer, start: f64, count: f64) -> napi::Result<DecodedFlac> {
    let audio = kryx_flac::decode_range(input.as_ref(), start as u64, count as u64)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(into_decoded(audio))
}

/// Read the stream's metadata (tags, pictures, seek table) without decoding
/// audio. Cheap: stops at the first audio frame.
#[napi]
pub fn read_metadata(input: Buffer) -> napi::Result<FlacMetadata> {
    let m = kryx_flac::read_metadata(input.as_ref())
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(FlacMetadata {
        sample_rate: m.stream_info.sample_rate,
        channels: m.stream_info.channels as u32,
        bits_per_sample: m.stream_info.bits_per_sample as u32,
        total_samples: m.stream_info.total_samples as f64,
        vendor: m.vendor,
        tags: map_tags(m.tags),
        pictures: map_pictures(m.pictures),
        seek_points: m
            .seek_points
            .into_iter()
            .map(|p| FlacSeekPoint {
                sample: p.sample as f64,
                offset: p.offset as f64,
                frame_samples: p.frame_samples as u32,
            })
            .collect(),
    })
}

/// Version of the native `kryx-flac` core crate.
#[napi]
pub fn core_version() -> String {
    kryx_flac::VERSION.to_string()
}
