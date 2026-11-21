import { describe, expect, it } from 'vitest'

// Encoder integration: encode PCM through the real addon, then decode it back
// and confirm the audio is bit-identical (lossless).
import { decode, encode } from '../../src/index.js'

describe('encode (integration, real addon)', () => {
  it('round-trips interleaved stereo PCM losslessly', () => {
    const n = 4000
    const samples = new Int32Array(n * 2)
    for (let i = 0; i < n; i++) {
      samples[i * 2] = ((i * 7) % 2000) - 1000 // L: ramp
      samples[i * 2 + 1] = Math.round(Math.sin(i * 0.05) * 8000) // R: tone
    }

    const flac = encode(samples, 2, { sampleRate: 44100, bitsPerSample: 16 })
    expect(Buffer.isBuffer(flac)).toBe(true)
    expect(flac.subarray(0, 4).toString('ascii')).toBe('fLaC')

    const back = decode(flac)
    expect(back.channels).toBe(2)
    expect(back.sampleRate).toBe(44100)
    expect(back.frameCount).toBe(n)
    expect([...back.samples]).toEqual([...samples])
  })

  it('embeds tags passed to the encoder', () => {
    const samples = new Int32Array(500) // mono silence
    const flac = encode(samples, 1, {
      sampleRate: 8000,
      bitsPerSample: 16,
      tags: [{ key: 'TITLE', value: 'Kryx' }],
    })
    const back = decode(flac)
    expect(back.tags.find((t) => t.key === 'TITLE')?.value).toBe('Kryx')
  })
})
