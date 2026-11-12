import { describe, expect, it } from 'vitest'
import { kioskRootPx, kioskScale, MIN_SCALE } from './fit'

describe('kioskScale', () => {
  it('is 1 at the design size in either orientation, and above it', () => {
    expect(kioskScale(1920, 1080)).toBe(1)
    expect(kioskScale(1080, 1920)).toBe(1)
    expect(kioskScale(3840, 2160)).toBe(1)
  })
  it('shrinks with the tighter side', () => {
    expect(kioskScale(1280, 800)).toBeCloseTo(1280 / 1920)
    expect(kioskScale(1920, 980)).toBeCloseTo(980 / 1080)
    expect(kioskScale(800, 1280)).toBeCloseTo(1280 / 1920)
    expect(kioskRootPx(960, 540)).toBe(8)
  })
  it('never goes below the floor and survives an empty viewport', () => {
    expect(kioskScale(320, 200)).toBe(MIN_SCALE)
    expect(kioskScale(0, 0)).toBe(1)
  })
})
