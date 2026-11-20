# Roadmap — kryx-flac

A fully-owned FLAC codec (zero third-party, pure Rust), grown by proven need.

## M1 — Decoder core ✅
Bit reader, STREAMINFO, frames, all subframes (CONSTANT/VERBATIM/FIXED/LPC),
partitioned Rice residual, all decorrelation modes. Bit-exact vs FFmpeg.

## M2 — Bindings + SDK ✅
N-API crate (`flac-node`) + dual ESM/CJS TypeScript SDK published as
`@kryxjs/codecs-flac`. `decode()` returns interleaved `Int32Array` PCM + stream
info; 7 prebuilt platforms; integration suite bit-exact vs FFmpeg through the SDK.

## M3 — Robustness ✅
Header CRC-8 + frame CRC-16 verification (decode rejects corrupt streams).
Metadata: VORBIS_COMMENT tags, PICTURE cover art, SEEKTABLE. `read_metadata`
(audio-free) and `decode_range` (sample-window decode with seek-table jump).
Surfaced through the SDK as `readMetadata` / `decodeRange` + `tags`/`pictures`.

## M4 — Encoder + stable
FLAC **encoder** (FIXED first, then LPC with a measured order search), then a
stable release: and benchmarks vs libFLAC (the point of owning it: measure,
then prove).

## M5 — Performance
SIMD / Zig (or ASM) kernels on the hot loops — LPC prediction and Rice
decode — each measured against libFLAC before it lands. The "beat the
incumbent" phase.

## M6 — Streaming + full conformance
Incremental/streaming decode (frame-by-frame, no full-file buffer), 24/32-bit
paths exercised, WASM target, fuzzing + the official FLAC test corpus.
