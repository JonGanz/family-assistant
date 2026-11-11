import { describe, expect, it } from 'vitest'
import { capacity, visibleCount } from './monthGrid'

describe('capacity', () => {
  it('counts whole lines below the header', () => {
    expect(capacity(100, 20, 20)).toBe(4)
    expect(capacity(99, 20, 20)).toBe(3)
  })
  it('never goes negative and guards a zero line height', () => {
    expect(capacity(10, 20, 20)).toBe(0)
    expect(capacity(100, 0, 20)).toBe(0)
  })
})

describe('visibleCount', () => {
  it('shows everything that fits', () => expect(visibleCount(3, 3)).toBe(3))
  it('leaves a line for "+N" when overflowing', () => {
    expect(visibleCount(5, 3)).toBe(2)
    expect(visibleCount(5, 1)).toBe(0)
    expect(visibleCount(5, 0)).toBe(0)
  })
})
