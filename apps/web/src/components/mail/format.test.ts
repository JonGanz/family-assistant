import { describe, expect, it } from 'vitest'
import { fmtDay, fromLocalInput, toLocalInput } from './format'

describe('mail format', () => {
  it('round-trips a local wall time in a fixed tz', () => {
    const tz = 'America/New_York'
    const iso = fromLocalInput('2026-07-04T09:30', tz)
    expect(iso).toBe('2026-07-04T13:30:00.000Z')
    expect(toLocalInput(iso, tz)).toBe('2026-07-04T09:30')
    expect(fromLocalInput('2026-01-10T09:30', tz)).toBe('2026-01-10T14:30:00.000Z')
  })
  it('formats a plain day without a tz shift', () => {
    expect(fmtDay('2026-03-01', 'en-US')).toContain('Mar 1, 2026')
  })
})
