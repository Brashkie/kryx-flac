//! Errors surfaced while decoding a FLAC stream.

use std::fmt;

/// A decode error.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlacError {
    /// The stream doesn't start with the `fLaC` marker.
    BadMagic,
    /// The stream ended before a value could be fully read.
    UnexpectedEof,
    /// A STREAMINFO metadata block was expected but not found.
    MissingStreamInfo,
    /// A reserved or unsupported bit pattern was encountered (what/where).
    Reserved(&'static str),
    /// A subframe used an unknown type code.
    BadSubframe(u8),
    /// The frame sync code didn't match where a frame was expected.
    BadSync,
    /// A frame CRC didn't match (header CRC-8 or frame CRC-16): corrupt stream.
    CrcMismatch(&'static str),
    /// A metadata block was malformed (e.g. a bad VORBIS_COMMENT / PICTURE).
    BadMetadata(&'static str),
}

impl fmt::Display for FlacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlacError::BadMagic => write!(f, "not a FLAC stream (missing fLaC marker)"),
            FlacError::UnexpectedEof => write!(f, "unexpected end of stream"),
            FlacError::MissingStreamInfo => write!(f, "missing STREAMINFO metadata block"),
            FlacError::Reserved(w) => write!(f, "reserved/unsupported value: {w}"),
            FlacError::BadSubframe(t) => write!(f, "unknown subframe type {t}"),
            FlacError::BadSync => write!(f, "bad frame sync code"),
            FlacError::CrcMismatch(w) => write!(f, "CRC mismatch ({w}): corrupt stream"),
            FlacError::BadMetadata(w) => write!(f, "malformed metadata block: {w}"),
        }
    }
}

impl std::error::Error for FlacError {}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, FlacError>;
