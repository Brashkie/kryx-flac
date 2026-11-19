# Roadmap — kryx-flac

A fully-owned FLAC codec (zero third-party, pure Rust), grown by proven need.

## M1 — Decoder core ✅
Bit reader, STREAMINFO, frames, all subframes (CONSTANT/VERBATIM/FIXED/LPC),
partitioned Rice residual, all decorrelation modes. Bit-exact vs FFmpeg.

## M2 — Bindings + SDK ✅
N-API crate (`flac-node`) + dual ESM/CJS TypeScript SDK published as
`@kryxjs/codecs-flac`. `decode()` returns interleaved `Int32Array` PCM + stream
info; 7 prebuilt platforms; integration suite bit-exact vs FFmpeg through the SDK.

## M3 — Robustness
Frame CRC-16 + header CRC-8 verification, metadata blocks (VORBIS_COMMENT tags,
PICTURE), seek table / seeking by sample.

## M4 — Encoder + stable
FLAC **encoder** (FIXED first, then LPC with a measured order search), then a
stable 0.1.0: CI, bilingual docs, release workflow — and benchmarks vs libFLAC
(the point of owning it: measure, then prove).

## Later
Streaming/incremental decode, 24/32-bit paths exercised, WASM target, fuzzing +
the official FLAC test corpus.
