// Loads the compiled native addon through napi's generated loader (../index.js).
import * as addon from '../index.js'

/** Decoded FLAC audio as returned by the native addon. */
export interface DecodedFlac {
  /** Sample rate in Hz. */
  sampleRate: number
  /** Number of channels. */
  channels: number
  /** Bits per sample declared by the stream. */
  bitsPerSample: number
  /** Total interchannel samples declared in STREAMINFO (0 if unknown). */
  totalSamples: number
  /** Number of interchannel samples actually decoded. */
  frameCount: number
  /** Interleaved signed 32-bit samples: ch0[0], ch1[0], …, ch0[1], … */
  samples: Int32Array
}

/** The native addon surface. */
export interface NativeBindings {
  decode(input: Buffer): DecodedFlac
  coreVersion(): string
}

export const native = addon as unknown as NativeBindings
