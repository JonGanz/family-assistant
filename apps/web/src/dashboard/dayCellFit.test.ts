import { describe, expect, it } from 'vitest'
import { adaptiveSlotRem, fitCell, LANDSCAPE, PORTRAIT } from './dayCellFit'

// Slot 3.2rem, passed explicitly so the tests do not move with the real slot size. 16px rem: a slot is 51.2px and a gap 4px, so 4 rows need about 4 * 55.2 - 4 = 216.8px.
describe('fitCell', () => {
  it('draws everything that fits in one column', () => {
    expect(fitCell(3, 200, 170, 16, 3.2)).toEqual({ shown: 3, more: 0, cols: 1 })
  })
  it('puts "+N" in the last slot when they do not fit', () => {
    expect(fitCell(5, 200, 170, 16, 3.2)).toEqual({ shown: 2, more: 3, cols: 1 })
  })
  it('splits into two columns only when one column is not enough', () => {
    expect(fitCell(6, 500, 120, 16, 3.2)).toEqual({ shown: 3, more: 3, cols: 2 })
    expect(fitCell(4, 500, 120, 16, 3.2)).toEqual({ shown: 4, more: 0, cols: 2 })
  })
  it('keeps one column while the items fit, even in a wide cell', () => {
    expect(fitCell(2, 500, 120, 16, 3.2)).toEqual({ shown: 2, more: 0, cols: 1 })
  })
  it('keeps full width when a half would be too narrow', () => {
    expect(fitCell(6, 200, 120, 16, 3.2).cols).toBe(1)
  })
  it('splits a cell the width of a typical five-person dashboard', () => {
    expect(fitCell(4, 330, 120, 16, 3.2).cols).toBe(2)
  })
  it('shows only "+N" when not even one slot fits beside it', () => {
    expect(fitCell(3, 200, 40, 16, 3.2)).toEqual({ shown: 0, more: 3, cols: 1 })
  })
  it('draws everything until the size is known', () => {
    expect(fitCell(9, 0, 0, 16, 3.2)).toEqual({ shown: 9, more: 0, cols: 1 })
  })
})

describe('adaptiveSlotRem', () => {
  it('keeps the tall slot when there is room', () => {
    expect(adaptiveSlotRem(400, 16, PORTRAIT)).toBe(4.2)
    expect(adaptiveSlotRem(0, 16, LANDSCAPE)).toBe(3.4)
  })
  it('shrinks so the wanted rows fit, but never below the text height', () => {
    for (const [r, rows, h] of [
      [PORTRAIT, 3, 170],
      [LANDSCAPE, 2, 105],
    ] as const) {
      const slot = adaptiveSlotRem(h, 16, r)
      expect(slot).toBeLessThan(r.max)
      expect(fitCell(rows, 100, h, 16, slot).shown).toBe(rows)
    }
    expect(adaptiveSlotRem(60, 16, PORTRAIT)).toBe(3.35)
    expect(adaptiveSlotRem(60, 16, LANDSCAPE)).toBe(3.1)
  })
})
