# Changelog

## [0.3.0] — 2026-11-21

Fourth milestone (M4): a lossless FLAC **encoder**.

### Added
- `encode(channels, opts)` / `encode_interleaved(...)` in the core, and
  `encode(samples, channels, opts)` in the SDK — produce a complete, valid FLAC
  stream that FFmpeg decodes bit-exact.
- Per-subframe search over CONSTANT, FIXED (orders 0–4) and LPC (up to order 12,
  Welch-windowed autocorrelation + Levinson-Durbin + libFLAC-style coefficient
  quantization), falling back to VERBATIM, with a partitioned-Rice parameter
  search (5-bit params).
- Per-frame stereo decorrelation: independent / left-side / right-side /
  mid-side, chosen by coded size.
- Optional VORBIS_COMMENT tags via `EncodeOptions.tags`.
- `BitWriter`, `crc` reused for header CRC-8 / frame CRC-16 on write.
- Tests: encode→decode round-trips (lossless) over real + synthetic audio, tag
  round-trip; validated bit-exact through FFmpeg and competitive with libFLAC at
  `-compression_level 8` on the test signals.

## [0.2.0] — 2026-11-19

Third milestone (M3): robustness — CRC verification, metadata, and seeking.

### Added
- **CRC verification**: every frame's header CRC-8 and footer CRC-16 are checked
  while decoding; `decode` now returns `CrcMismatch` on a corrupt stream.
- **Metadata**: VORBIS_COMMENT tags, PICTURE (cover art) and SEEKTABLE are parsed
  and surfaced on `FlacAudio` / the decode result (`tags`, `pictures`).
- `readMetadata(bytes)` — read tags, pictures and seek points without decoding
  any audio.
- `decodeRange(bytes, start, count)` — decode only a sample window, using the
  seek table to jump when present.
- Core API: `read_metadata`, `decode_range`, `FlacMetadata`, `Picture`,
  `SeekPoint`, `FlacAudio::tag()`.
- Conformance fixtures: a tagged file with embedded PNG cover art, a file with a
  real (CRC-validated) seek table, and a corrupted file — tags/pictures/seeking
  verified and CRC corruption detected, at both the Rust and SDK layers.

## [0.1.0] — 2026-11-19

Second milestone (M2): the decoder shipped to Node.js.

### Added
- N-API crate (`flac-node`) bridging the pure-Rust core; `decode(Buffer)`
  returns `{ sampleRate, channels, bitsPerSample, totalSamples, frameCount,
  samples }` with `samples` an interleaved `Int32Array`, plus `coreVersion()`.
- TypeScript SDK `@kryxjs/codecs-flac` (dual ESM + CJS): `decode(Uint8Array |
  Buffer) -> DecodedFlac` and `VERSION`.
- Prebuilt binaries for 7 platforms (win x64/arm64, mac x64/arm64, linux
  x64-gnu/musl, arm64-gnu); CI + release workflows (toolchain pinned 1.90.0).
- Tests: unit suite (native mocked) + integration suite decoding real FLAC
  through the SDK, **bit-exact vs FFmpeg**. Bilingual README.

## [0.1.0-M1] — 2026-11-19

First milestone: a pure-Rust FLAC **decoder** core, zero third-party deps.

### Added
- `decode(&[u8]) -> FlacAudio` — full in-memory FLAC → planar `i32` PCM, with
  `FlacAudio::interleaved()` and `frame_count()`.
- `StreamInfo` (sample rate, channels, bits per sample, total samples, block sizes).
- Bit reader (MSB-first), STREAMINFO parsing, frame headers (UTF-8 coded number),
  CONSTANT/VERBATIM/FIXED/LPC subframes, partitioned Rice residual (+ escape),
  and left/side, right/side, mid/side decorrelation.
- Conformance tests: three real FLAC files decoded **bit-exact vs FFmpeg**
  (CONSTANT, high-entropy mono, stereo with decorrelation) + bit-reader/UTF-8 units.

### Deferred
- Encoder, frame-CRC verification, metadata beyond STREAMINFO, seeking,
  streaming/incremental decode (later milestones).
