import { describe, expect, it } from 'vitest'
import { fitWithin } from './imageResize'

describe('fitWithin', () => {
  it('downscales the long edge to 2000 keeping the ratio', () => {
    expect(fitWithin(4000, 3000)).toEqual({ width: 2000, height: 1500 })
    expect(fitWithin(3000, 6000)).toEqual({ width: 1000, height: 2000 })
  })
  it('never upscales', () => {
    expect(fitWithin(800, 600)).toEqual({ width: 800, height: 600 })
  })
  it('keeps at least one pixel', () => {
    expect(fitWithin(100000, 10)).toEqual({ width: 2000, height: 1 })
  })
})
