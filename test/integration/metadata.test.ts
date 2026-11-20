import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

// Integration tests for the M3 surface (metadata, seeking, CRC) through the
// real compiled addon and the public SDK.
import { decode, decodeRange, readMetadata } from '../../src/index.js'

const dir = join(__dirname, '..', 'fixtures')
const read = (name: string) => readFileSync(join(dir, name))

describe('readMetadata (integration, real addon)', () => {
  it('reads VORBIS_COMMENT tags and cover art without decoding audio', () => {
    const m = readMetadata(read('tagged.flac'))
    const title = m.tags.find((t) => t.key === 'TITLE')?.value
    expect(title).toBe('Kryx Test Tone')
    expect(m.tags.find((t) => t.key === 'ARTIST')?.value).toBe('Brashkie')

    expect(m.pictures).toHaveLength(1)
    expect(m.pictures[0].mime).toBe('image/png')
    expect(Buffer.isBuffer(m.pictures[0].data)).toBe(true)
    expect([...m.pictures[0].data.subarray(1, 4)]).toEqual([0x50, 0x4e, 0x47]) // "PNG"
  })

  it('exposes the seek table', () => {
    const m = readMetadata(read('seek.flac'))
    expect(m.seekPoints.length).toBeGreaterThan(0)
    expect(m.seekPoints.every((p) => p.sample < 8000)).toBe(true)
  })
})

describe('decode (tags survive)', () => {
  it('carries tags and pictures on a full decode', () => {
    const got = decode(read('tagged.flac'))
    expect(got.tags.find((t) => t.key === 'TITLE')?.value).toBe('Kryx Test Tone')
    expect(got.pictures).toHaveLength(1)
    expect(got.frameCount).toBe(8000)
  })

  it('throws on a CRC-corrupted stream', () => {
    expect(() => decode(read('corrupt.flac'))).toThrow(/CRC/i)
  })
})

describe('decodeRange (seeking)', () => {
  it('returns a window identical to the matching slice of a full decode', () => {
    const full = decode(read('seek.flac')).samples
    const ch = 2

    for (const [start, count] of [
      [0, 100],
      [600, 500],
      [4000, 1000],
      [7900, 100],
    ] as const) {
      const got = decodeRange(read('seek.flac'), start, count).samples
      const s = start * ch
      const e = Math.min(start + count, 8000) * ch
      expect([...got]).toEqual([...full.subarray(s, e)])
    }
  })
})
