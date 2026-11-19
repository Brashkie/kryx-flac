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
```

## Roadmap

- **M1** — decoder core. _done_
- **M2** — N-API bindings + TypeScript SDK (`@kryxjs/codecs-flac`). _done_
- **M3** — frame CRC verification, metadata (VORBIS_COMMENT / pictures), seeking.
- **M4** — encoder; stable 0.1.0 with benchmarks vs libFLAC.

See [ROADMAP.md](./ROADMAP.md). Part of the Kryx ecosystem · Apache-2.0
