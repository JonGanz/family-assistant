import { describe, expect, it } from 'vitest'
import { fitCell } from './dayCellFit'

// 16px rem: a slot is 51.2px and a gap 4px, so 4 rows need about 4 * 55.2 - 4 = 216.8px.
describe('fitCell', () => {
  it('draws everything that fits in one column', () => {
    expect(fitCell(3, 200, 170, 16)).toEqual({ shown: 3, more: 0, cols: 1 })
  })
  it('puts "+N" in the last slot when they do not fit', () => {
    expect(fitCell(5, 200, 170, 16)).toEqual({ shown: 2, more: 3, cols: 1 })
  })
  it('splits into two columns only when one column is not enough', () => {
    expect(fitCell(6, 500, 120, 16)).toEqual({ shown: 3, more: 3, cols: 2 })
    expect(fitCell(4, 500, 120, 16)).toEqual({ shown: 4, more: 0, cols: 2 })
  })
  it('keeps one column while the items fit, even in a wide cell', () => {
    expect(fitCell(2, 500, 120, 16)).toEqual({ shown: 2, more: 0, cols: 1 })
  })
  it('keeps full width when a half would be too narrow', () => {
    expect(fitCell(6, 200, 120, 16).cols).toBe(1)
  })
  it('splits a cell the width of a typical five-person dashboard', () => {
    expect(fitCell(4, 330, 120, 16).cols).toBe(2)
  })
  it('shows only "+N" when not even one slot fits beside it', () => {
    expect(fitCell(3, 200, 40, 16)).toEqual({ shown: 0, more: 3, cols: 1 })
  })
  it('draws everything until the size is known', () => {
    expect(fitCell(9, 0, 0, 16)).toEqual({ shown: 9, more: 0, cols: 1 })
  })
})
