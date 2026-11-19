//! N-API bindings for @kryxjs/codecs-flac.
//!
//! A thin bridge over the pure-Rust `kryx-flac` decoder: takes a FLAC file as a
//! `Buffer` and returns the decoded PCM (interleaved `Int32Array`) plus the
//! stream parameters. JS↔native crossing is coarse (one call per file).

use napi::bindgen_prelude::{Buffer, Int32Array};
use napi_derive::napi;

/// The result of decoding a FLAC stream.
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
    /// Number of interchannel samples actually decoded.
    pub frame_count: u32,
    /// Interleaved samples: `ch0[0], ch1[0], …, ch0[1], …` as signed i32.
    pub samples: Int32Array,
}

/// Decode a complete FLAC file to interleaved PCM.
#[napi]
pub fn decode(input: Buffer) -> napi::Result<DecodedFlac> {
    let audio = kryx_flac::decode(input.as_ref())
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    let frame_count = audio.frame_count() as u32;
    let interleaved = audio.interleaved();

    Ok(DecodedFlac {
        sample_rate: audio.info.sample_rate,
        channels: audio.info.channels as u32,
        bits_per_sample: audio.info.bits_per_sample as u32,
        total_samples: audio.info.total_samples as f64,
        frame_count,
        samples: Int32Array::new(interleaved),
    })
}

/// Version of the native `kryx-flac` core crate.
#[napi]
pub fn core_version() -> String {
    kryx_flac::VERSION.to_string()
}
