// Loads the compiled native addon through napi's generated loader (../index.js).
import * as addon from '../index.js'

/** A VORBIS_COMMENT tag. */
export interface FlacTag {
  /** Tag key, uppercased (e.g. `TITLE`, `ARTIST`). */
  key: string
  /** Tag value. */
  value: string
}

/** An attached picture (cover art). */
export interface FlacPicture {
  /** Picture type per the FLAC/ID3 APIC list (3 = front cover, …). */
  pictureType: number
  /** MIME type, e.g. `image/jpeg`. */
  mime: string
  /** UTF-8 description (often empty). */
  description: string
  /** Width in pixels (0 if unknown). */
  width: number
  /** Height in pixels (0 if unknown). */
  height: number
  /** Color depth in bits per pixel (0 if unknown). */
  depth: number
  /** Number of colors used, for indexed images (0 otherwise). */
  colors: number
  /** Raw encoded image bytes. */
  data: Buffer
}

/** A seek point from the SEEKTABLE. */
export interface FlacSeekPoint {
  /** First interchannel sample in the target frame. */
  sample: number
  /** Byte offset of the target frame from the first audio frame. */
  offset: number
  /** Number of samples in the target frame. */
  frameSamples: number
}

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
  /** Number of interchannel samples actually returned. */
  frameCount: number
  /** Interleaved signed 32-bit samples: ch0[0], ch1[0], …, ch0[1], … */
  samples: Int32Array
  /** VORBIS_COMMENT tags. */
  tags: FlacTag[]
  /** Attached pictures (cover art, etc.). */
  pictures: FlacPicture[]
}

/** Stream metadata without any decoded audio. */
export interface FlacMetadata {
  sampleRate: number
  channels: number
  bitsPerSample: number
  totalSamples: number
  /** VORBIS_COMMENT vendor string. */
  vendor: string
  tags: FlacTag[]
  pictures: FlacPicture[]
  seekPoints: FlacSeekPoint[]
}

/** Options for {@link NativeBindings.encode}. */
export interface EncodeOptions {
  /** Sample rate in Hz. */
  sampleRate: number
  /** Bits per sample (4..=32). */
  bitsPerSample: number
  /** Samples per block per channel (default 4096). */
  blockSize?: number
  /** VORBIS_COMMENT tags to embed. */
  tags?: FlacTag[]
}

/** The native addon surface. */
export interface NativeBindings {
  decode(input: Buffer): DecodedFlac
  decodeRange(input: Buffer, start: number, count: number): DecodedFlac
  readMetadata(input: Buffer): FlacMetadata
  encode(samples: Int32Array, channels: number, opts: EncodeOptions): Buffer
  coreVersion(): string
}

export const native = addon as unknown as NativeBindings
