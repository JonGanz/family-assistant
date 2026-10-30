import { describe, expect, it } from 'vitest'
import { buildsDiffer, builtAtText } from './aboutModel'

describe('about model', () => {
  it('flags differing commits only when both are known', () => {
    expect(buildsDiffer('abc', 'abc')).toBe(false)
    expect(buildsDiffer('abc', 'def')).toBe(true)
    expect(buildsDiffer('abc-dirty', 'abc')).toBe(true)
    expect(buildsDiffer(null, 'abc')).toBe(false)
    expect(buildsDiffer('abc', '')).toBe(false)
  })
  it('formats a build time in the given zone', () => {
    expect(builtAtText('2026-10-29T12:00:00Z', 'UTC')).toMatch(/2026/)
    expect(builtAtText(null, 'UTC')).toBe('unknown')
    expect(builtAtText('not a date', 'UTC')).toBe('not a date')
  })
})
