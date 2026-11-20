//! @kryxjs/codecs-flac — a dependency-free FLAC decoder.
//
// Pure-Rust decoder (no third-party, no vendored C) exposed through N-API.
// `decode(bytes)` returns the full file's interleaved PCM plus stream info,
// tags and cover art; `decodeRange` seeks; `readMetadata` skips the audio.

import { type DecodedFlac, type FlacMetadata, native } from './native.js'

export type {
  DecodedFlac,
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

/** Version of the `@kryxjs/codecs-flac` package. */
export const VERSION = '0.2.0'
