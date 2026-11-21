//! @kryxjs/codecs-flac — a dependency-free FLAC codec.
//
// Pure-Rust decoder and encoder (no third-party, no vendored C) through N-API.
// `decode` returns interleaved PCM + stream info, tags and cover art;
// `decodeRange` seeks; `readMetadata` skips the audio; `encode` writes FLAC.

import { type DecodedFlac, type EncodeOptions, type FlacMetadata, native } from './native.js'

export type {
  DecodedFlac,
  EncodeOptions,
  FlacMetadata,
  FlacPicture,
  FlacSeekPoint,
  FlacTag,
} from './native.js'

/** Coerce any byte source to a Buffer view without copying when possible. */
function toBuffer(data: Uint8Array): Buffer {
  return Buffer.isBuffer(data) ? data : Buffer.from(data.buffer, data.byteOffset, data.byteLength)
}

/**
 * Decode a complete FLAC file to interleaved `Int32Array` PCM, verifying every
 * frame's CRC. Accepts a `Buffer` or any `Uint8Array`.
 */
export function decode(data: Uint8Array): DecodedFlac {
  return native.decode(toBuffer(data))
}

/**
 * Decode only the interchannel samples `[start, start + count)`, using the
 * stream's seek table when present. `start` and `count` are sample indices.
 */
export function decodeRange(data: Uint8Array, start: number, count: number): DecodedFlac {
  return native.decodeRange(toBuffer(data), start, count)
}

/**
 * Read the stream's metadata (tags, cover art, seek table) without decoding
 * any audio. Much cheaper than {@link decode} for tag/art readers.
 */
export function readMetadata(data: Uint8Array): FlacMetadata {
  return native.readMetadata(toBuffer(data))
}

/**
 * Encode interleaved PCM (`ch0[0], ch1[0], …`) to a complete FLAC file,
 * returning the bytes as a `Buffer`. Lossless; picks per-frame stereo
 * decorrelation and the cheapest subframe (CONSTANT / FIXED / LPC / VERBATIM).
 */
export function encode(samples: Int32Array, channels: number, opts: EncodeOptions): Buffer {
  return native.encode(samples, channels, opts)
}

/** Version of the `@kryxjs/codecs-flac` package. */
export const VERSION = '0.3.0'
