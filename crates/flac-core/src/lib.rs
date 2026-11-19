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
//! ## Scope (M1)
//!
//! Decoder only: `fLaC` marker + STREAMINFO, frame headers, the four subframe
//! types (CONSTANT, VERBATIM, FIXED, LPC), partitioned Rice residual, and all
//! inter-channel decorrelation modes (independent, left/side, right/side,
//! mid/side). Validated bit-exact against FFmpeg on real files.
//!
//! Deferred: encoder, seeking, metadata beyond STREAMINFO (tags, pictures),
//! incremental/streaming decode, CRC verification (frame CRCs are parsed but not
//! checked yet).

#![forbid(unsafe_code)]

mod bitreader;
mod decoder;
mod error;
mod frame;
mod metadata;
mod subframe;

pub use decoder::{decode, FlacAudio};
pub use error::{FlacError, Result};
pub use frame::ChannelAssignment;
pub use metadata::StreamInfo;

/// Version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
