//! @kryxjs/codecs-flac — a dependency-free FLAC decoder.
//
// Pure-Rust decoder (no third-party, no vendored C) exposed through N-API.
// `decode(bytes)` returns the full file's interleaved PCM plus stream info.

import { type DecodedFlac, native } from './native.js'

export type { DecodedFlac } from './native.js'

/**
 * Decode a complete FLAC file to interleaved `Int32Array` PCM.
 *
 * Accepts a `Buffer` or any `Uint8Array` (e.g. from `fs.readFile` or `fetch`).
 */
export function decode(data: Uint8Array): DecodedFlac {
  const buf = Buffer.isBuffer(data)
    ? data
    : Buffer.from(data.buffer, data.byteOffset, data.byteLength)
  return native.decode(buf)
}

/** Version of the `@kryxjs/codecs-flac` package. */
export const VERSION = '0.1.0'
