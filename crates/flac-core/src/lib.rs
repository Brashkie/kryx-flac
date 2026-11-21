//! # kryx-flac — a dependency-free FLAC decoder in pure Rust
//!
//! The first fully-owned codec of the Kryx ecosystem (`@kryxjs/codecs-flac`):
//! no third-party crates, no vendored C — just the FLAC spec implemented
//! directly. It decodes a complete in-memory FLAC stream to planar `i32` PCM.
//!
//! ```no_run
//! let bytes = std::fs::read("song.flac").unwrap();
//! let audio = kryx_flac::decode(&bytes).unwrap();
//! println!("{} Hz, {} ch, {} bits, {} frames",
//!     audio.info.sample_rate, audio.info.channels,
//!     audio.info.bits_per_sample, audio.frame_count());
//! let interleaved = audio.interleaved();
//! ```
//!
//! ## Scope
//!
//! Decoder: `fLaC` marker + STREAMINFO, frame headers, the four subframe types
//! (CONSTANT, VERBATIM, FIXED, LPC), partitioned Rice residual, and all
//! inter-channel decorrelation modes (independent, left/side, right/side,
//! mid/side). Validated bit-exact against FFmpeg on real files.
//!
//! Also: header CRC-8 + frame CRC-16 verification, metadata (VORBIS_COMMENT
//! tags, PICTURE cover art, SEEKTABLE), a metadata-only read
//! ([`read_metadata`]), and sample-range decoding with seeking
//! ([`decode_range`]).
//!
//! Deferred: encoder, incremental/streaming decode (later milestones).

#![forbid(unsafe_code)]

mod bitreader;
mod bitwriter;
mod crc;
mod decoder;
mod encoder;
mod error;
mod frame;
mod metadata;
mod subframe;

pub use decoder::{decode, decode_range, read_metadata, FlacAudio};
pub use encoder::{encode, encode_interleaved, EncodeOptions};
pub use error::{FlacError, Result};
pub use frame::ChannelAssignment;
pub use metadata::{FlacMetadata, Picture, SeekPoint, StreamInfo};

/// Version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
