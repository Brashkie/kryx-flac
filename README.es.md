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
```

## Hoja de ruta

- **M1** — núcleo decodificador. _hecho_
- **M2** — bindings N-API + SDK TypeScript (`@kryxjs/codecs-flac`). _hecho_
- **M3** — verificación de CRC de frame, metadatos (VORBIS_COMMENT / imágenes),
  búsqueda por muestra.
- **M4** — codificador; 0.1.0 estable con benchmarks vs libFLAC.

Ver [ROADMAP.md](./ROADMAP.md). Parte del ecosistema Kryx · Apache-2.0
