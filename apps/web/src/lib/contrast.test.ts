import { describe, expect, it } from 'vitest'
import { contrastRatio, textColorFor } from './contrast'

describe('contrast', () => {
  it('picks black on light and white on dark', () => {
    expect(textColorFor('#ffffff')).toBe('#000000')
    expect(textColorFor('#000000')).toBe('#ffffff')
    expect(textColorFor('#ffeb3b')).toBe('#000000')
    expect(textColorFor('#1a237e')).toBe('#ffffff')
  })
  it('always reaches 4.5:1 over a colour sweep', () => {
    for (let r = 0; r < 256; r += 51)
      for (let g = 0; g < 256; g += 51)
        for (let b = 0; b < 256; b += 51) {
          const bg = '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')
          expect(contrastRatio(bg, textColorFor(bg))).toBeGreaterThanOrEqual(4.5)
        }
  })
  it('accepts shorthand', () => {
    expect(textColorFor('#fff')).toBe('#000000')
  })
})
