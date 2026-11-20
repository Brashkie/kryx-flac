import { beforeEach, describe, expect, it, vi } from 'vitest'

// Mock the native addon so the unit layer tests the SDK glue in isolation
// (Buffer coercion, re-exports) without needing a compiled .node.
const decodeMock = vi.hoisted(() => vi.fn())

vi.mock('../../index.js', () => ({
  decode: decodeMock,
  coreVersion: () => '0.1.0',
}))

import { VERSION, decode } from '../../src/index.js'

const fakeResult = {
  sampleRate: 8000,
  channels: 2,
  bitsPerSample: 16,
  totalSamples: 800,
  frameCount: 800,
  samples: new Int32Array([1, -1, 2, -2]),
}

describe('decode (unit)', () => {
  beforeEach(() => {
    decodeMock.mockReset()
    decodeMock.mockReturnValue(fakeResult)
  })

  it('passes a Buffer straight through to the native binding', () => {
    const buf = Buffer.from([0x66, 0x4c, 0x61, 0x43])
    const out = decode(buf)

    expect(decodeMock).toHaveBeenCalledTimes(1)
    expect(Buffer.isBuffer(decodeMock.mock.calls[0][0])).toBe(true)
    expect(out).toBe(fakeResult)
  })

  it('wraps a plain Uint8Array in a Buffer view before calling native', () => {
    const bytes = new Uint8Array([0x66, 0x4c, 0x61, 0x43])
    decode(bytes)

    const arg = decodeMock.mock.calls[0][0]
    expect(Buffer.isBuffer(arg)).toBe(true)
    expect([...arg]).toEqual([0x66, 0x4c, 0x61, 0x43])
  })

  it('does not copy when the input offset/length are respected', () => {
    const backing = new Uint8Array([0xff, 0x66, 0x4c, 0x61, 0x43, 0xff])
    const view = backing.subarray(1, 5)
    decode(view)

    const arg = decodeMock.mock.calls[0][0]
    expect([...arg]).toEqual([0x66, 0x4c, 0x61, 0x43])
  })

  it('exposes the package VERSION', () => {
    expect(VERSION).toBe('0.2.0')
  })
})
