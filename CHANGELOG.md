# Changelog

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
