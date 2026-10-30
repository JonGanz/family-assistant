import { describe, expect, it } from 'vitest'
import { weekLabel, weekStartKey } from './weekModel'

describe('week model', () => {
  // 2026-10-29 is a Thursday.
  it('starts the week on the configured day', () => {
    expect(weekStartKey('2026-10-29', 'monday')).toBe('2026-10-26')
    expect(weekStartKey('2026-10-29', 'sunday')).toBe('2026-10-25')
  })
  it('a Sunday belongs to the week that starts before it for Monday weeks', () => {
    expect(weekStartKey('2026-11-01', 'monday')).toBe('2026-10-26')
    expect(weekStartKey('2026-11-01', 'sunday')).toBe('2026-11-01')
  })
  it('moves by whole weeks', () => {
    expect(weekStartKey('2026-10-29', 'monday', 1)).toBe('2026-11-02')
    expect(weekStartKey('2026-10-29', 'sunday', -1)).toBe('2026-10-18')
  })
  it('labels a week, naming both months when it spans two', () => {
    expect(weekLabel('2026-10-19', 'en-US')).toBe('Oct 19 – 25')
    expect(weekLabel('2026-10-26', 'en-US')).toBe('Oct 26 – Nov 1')
  })
})
