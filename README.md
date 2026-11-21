# kryx-flac · `@kryxjs/codecs-flac`

A **dependency-free FLAC decoder** — the first fully-owned codec of the
[Kryx](https://github.com/Brashkie) multimedia ecosystem. Pure-Rust core, no
third-party crates, no vendored C: the FLAC format implemented straight from the
spec, exposed to Node.js through a thin N-API binding and a dual ESM + CJS SDK.

Validated **bit-exact against FFmpeg** on real files, at both layers (Rust
conformance tests and the TypeScript integration suite).

> 🇪🇸 [Léeme en español](./README.es.md)

## Install

```sh
npm install @kryxjs/codecs-flac
```

Prebuilt binaries ship for Windows (x64/arm64), macOS (x64/arm64) and Linux
(x64 gnu/musl, arm64 gnu). No build step, no system FLAC library.

## Usage

```ts
import { readFileSync } from 'node:fs'
import { decode } from '@kryxjs/codecs-flac'

const audio = decode(readFileSync('song.flac'))
// audio.sampleRate · audio.channels · audio.bitsPerSample
// audio.frameCount  · audio.totalSamples
// audio.samples → Int32Array, interleaved: ch0[0], ch1[0], ch0[1], …
```

`decode` accepts a `Buffer` or any `Uint8Array`. CommonJS works the same way:

```js
const { decode } = require('@kryxjs/codecs-flac')
```

### Metadata, seeking, CRC

```ts
import { readMetadata, decodeRange } from '@kryxjs/codecs-flac'

// Tags + cover art, without decoding any audio:
const meta = readMetadata(buf)
meta.tags            // [{ key: 'TITLE', value: '…' }, …]
meta.pictures        // [{ mime, width, height, data: Buffer }, …]
meta.seekPoints      // [{ sample, offset, frameSamples }, …]

// Decode only a sample window (uses the seek table when present):
const slice = decodeRange(buf, 44_100, 1_000) // 1000 samples from 44 100
```

Every frame's header CRC-8 and footer CRC-16 are verified while decoding;
`decode` throws on a corrupt stream.

### Encoding

```ts
import { encode } from '@kryxjs/codecs-flac'

// interleaved i32 PCM: ch0[0], ch1[0], ch0[1], …
const flac = encode(samples, 2, { sampleRate: 44100, bitsPerSample: 16 })
```

A lossless encoder: per frame it chooses the best stereo decorrelation
(left/side, right/side, mid/side) and the cheapest subframe (CONSTANT, FIXED,
LPC, VERBATIM) with a partitioned-Rice search. On the test signals its output
is slightly smaller than FFmpeg/libFLAC at `-compression_level 8`, and decodes
bit-exact in FFmpeg. Optional `tags` are embedded as VORBIS_COMMENT.

## What it decodes

- `fLaC` marker + STREAMINFO (other metadata blocks skipped)
- Frame headers (sync, block size, sample rate, channel assignment, bit depth,
  UTF-8 coded number, CRC-8 parsed)
- All four subframes: **CONSTANT, VERBATIM, FIXED, LPC**
- Partitioned **Rice** residual (both parameter methods + escape)
- All inter-channel decorrelation modes: independent, **left/side, right/side,
  mid/side**

## Rust core

The decoder is usable directly as the `kryx-flac` crate:

```rust
let bytes = std::fs::read("song.flac")?;
let audio = kryx_flac::decode(&bytes)?;
let interleaved = audio.interleaved(); // Vec<i32>
```

## Build & test

```sh
npm run build          # native addon (release) + dual TS bundle
npm run build:debug    # faster, unoptimized native
npm test               # rust core tests + vitest (unit + integration)
npm run lint           # biome
cargo bench -p kryx-flac   # encode + decode throughput (Criterion)
```

### Performance

Measured with `cargo bench` on a 1 s 44.1 kHz stereo signal. The encoder's
partitioned-Rice search is computed in a single pass with an analytic parameter
estimate (libFLAC-style), which cut encode time from ~215 ms to ~19 ms (~11×)
with no change to the output. Decode uses a chunked bit reader. Next up (M5
phase 2): explicit SIMD / Zig kernels on the LPC and residual loops, measured
against the portable baseline.

## Roadmap

- **M1** — decoder core. _done_
- **M2** — N-API bindings + TypeScript SDK (`@kryxjs/codecs-flac`). _done_
- **M3** — CRC verification, metadata (tags / cover art / seek table), seeking. _done_
- **M4** — lossless encoder (FIXED + LPC + stereo decorrelation). _done_
- **M5** — performance: SIMD / Zig kernels on the hot loops, measured vs libFLAC.
- **M6** — streaming/incremental decode, WASM, fuzzing + the official FLAC corpus.

See [ROADMAP.md](./ROADMAP.md). Part of the Kryx ecosystem · Apache-2.0
