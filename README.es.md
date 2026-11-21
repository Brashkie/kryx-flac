# kryx-flac · `@kryxjs/codecs-flac`

Un **decodificador FLAC sin dependencias** — el primer códec totalmente propio
del ecosistema multimedia [Kryx](https://github.com/Brashkie). Núcleo en Rust
puro, sin crates de terceros ni C incrustado: el formato FLAC implementado
directo desde la especificación, expuesto a Node.js mediante un binding N-API
delgado y un SDK dual ESM + CJS.

Validado **bit a bit contra FFmpeg** en archivos reales, en ambas capas (tests
de conformidad en Rust y la suite de integración en TypeScript).

> 🇬🇧 [Read me in English](./README.md)

## Instalación

```sh
npm install @kryxjs/codecs-flac
```

Se publican binarios precompilados para Windows (x64/arm64), macOS (x64/arm64) y
Linux (x64 gnu/musl, arm64 gnu). Sin paso de compilación y sin librería FLAC del
sistema.

## Uso

```ts
import { readFileSync } from 'node:fs'
import { decode } from '@kryxjs/codecs-flac'

const audio = decode(readFileSync('cancion.flac'))
// audio.sampleRate · audio.channels · audio.bitsPerSample
// audio.frameCount  · audio.totalSamples
// audio.samples → Int32Array, intercalado: ch0[0], ch1[0], ch0[1], …
```

`decode` acepta un `Buffer` o cualquier `Uint8Array`. En CommonJS es igual:

```js
const { decode } = require('@kryxjs/codecs-flac')
```

### Metadatos, seeking, CRC

```ts
import { readMetadata, decodeRange } from '@kryxjs/codecs-flac'

// Tags + carátula, sin decodificar audio:
const meta = readMetadata(buf)
meta.tags            // [{ key: 'TITLE', value: '…' }, …]
meta.pictures        // [{ mime, width, height, data: Buffer }, …]
meta.seekPoints      // [{ sample, offset, frameSamples }, …]

// Decodificar solo una ventana de muestras (usa el seek table si existe):
const slice = decodeRange(buf, 44_100, 1_000) // 1000 muestras desde 44 100
```

Se verifica el CRC-8 de cabecera y el CRC-16 de cada frame al decodificar;
`decode` lanza error si el stream está corrupto.

### Codificación

```ts
import { encode } from '@kryxjs/codecs-flac'

// PCM i32 intercalado: ch0[0], ch1[0], ch0[1], …
const flac = encode(samples, 2, { sampleRate: 44100, bitsPerSample: 16 })
```

Encoder lossless: por frame elige la mejor decorrelación estéreo (left/side,
right/side, mid/side) y el subframe más barato (CONSTANT, FIXED, LPC, VERBATIM)
con búsqueda de Rice particionado. En las señales de prueba su salida es algo
menor que FFmpeg/libFLAC con `-compression_level 8`, y FFmpeg la decodifica
bit a bit. Los `tags` opcionales se insertan como VORBIS_COMMENT.

## Qué decodifica

- Marca `fLaC` + STREAMINFO (otros bloques de metadatos se omiten)
- Cabeceras de frame (sync, tamaño de bloque, frecuencia, asignación de canales,
  profundidad de bits, número codificado UTF-8, CRC-8 leído)
- Los cuatro subframes: **CONSTANT, VERBATIM, FIXED, LPC**
- Residual **Rice** particionado (ambos métodos de parámetro + escape)
- Todos los modos de decorrelación entre canales: independiente, **left/side,
  right/side, mid/side**

## Núcleo en Rust

El decodificador se usa directamente como el crate `kryx-flac`:

```rust
let bytes = std::fs::read("cancion.flac")?;
let audio = kryx_flac::decode(&bytes)?;
let interleaved = audio.interleaved(); // Vec<i32>
```

## Compilar y probar

```sh
npm run build          # addon nativo (release) + bundle TS dual
npm run build:debug    # nativo sin optimizar, más rápido
npm test               # tests del núcleo Rust + vitest (unit + integración)
npm run lint           # biome
cargo bench -p kryx-flac   # throughput de encode + decode (Criterion)
```

### Rendimiento

Medido con `cargo bench` sobre 1 s de estéreo a 44.1 kHz. La búsqueda de Rice
particionado del encoder se calcula en una sola pasada con estimación analítica
del parámetro (estilo libFLAC), lo que bajó el encode de ~215 ms a ~19 ms (~11×)
sin cambiar la salida. El decode usa un lector de bits por chunks. Sigue (M5
fase 2): kernels SIMD / Zig en los bucles de LPC y residual, medidos contra esta
base portable.

## Hoja de ruta

- **M1** — núcleo decodificador. _hecho_
- **M2** — bindings N-API + SDK TypeScript (`@kryxjs/codecs-flac`). _hecho_
- **M3** — verificación de CRC, metadatos (tags / carátula / seek table),
  decodificación por rango. _hecho_
- **M4** — codificador lossless (FIXED + LPC + decorrelación estéreo). _hecho_
- **M5** — rendimiento: kernels SIMD / Zig en los bucles calientes, medido vs libFLAC.
- **M6** — decodificación en streaming, WASM, fuzzing + el corpus oficial de FLAC.

Ver [ROADMAP.md](./ROADMAP.md). Parte del ecosistema Kryx · Apache-2.0
