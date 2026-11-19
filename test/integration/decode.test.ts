import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

// Integration tests exercise the REAL compiled addon through the public SDK
// and check the decoded PCM is bit-exact against ffmpeg-produced references.
import { decode } from '../../src/index.js'

const dir = join(__dirname, '..', 'fixtures')

/** Read a signed 16-bit little-endian PCM file into an Int32Array. */
function readPcmS16(path: string): Int32Array {
  const raw = readFileSync(path)
  const out = new Int32Array(raw.length / 2)
  for (let i = 0; i < out.length; i++) out[i] = raw.readInt16LE(i * 2)
  return out
}

describe('decode (integration, real addon)', () => {
  it('decodes a stereo file bit-exact against ffmpeg PCM', () => {
    const flac = readFileSync(join(dir, 'stereo.flac'))
    const got = decode(flac)

    expect(got.sampleRate).toBe(8000)
    expect(got.channels).toBe(2)
    expect(got.bitsPerSample).toBe(16)
    expect(got.frameCount).toBe(800)
    expect(got.totalSamples).toBe(800)

    const expected = readPcmS16(join(dir, 'stereo.pcm'))
    expect(got.samples.length).toBe(expected.length)
    expect([...got.samples]).toEqual([...expected])
  })

  it('decodes a mono noise file bit-exact against ffmpeg PCM', () => {
    const flac = readFileSync(join(dir, 'noise.flac'))
    const got = decode(flac)

    expect(got.sampleRate).toBe(8000)
    expect(got.channels).toBe(1)
    expect(got.frameCount).toBe(400)

    const expected = readPcmS16(join(dir, 'noise.pcm'))
    expect(got.samples.length).toBe(expected.length)
    expect([...got.samples]).toEqual([...expected])
  })

  it('accepts a plain Uint8Array as well as a Buffer', () => {
    const flac = readFileSync(join(dir, 'stereo.flac'))
    const asBytes = new Uint8Array(flac)
    const got = decode(asBytes)
    expect(got.frameCount).toBe(800)
  })
})
