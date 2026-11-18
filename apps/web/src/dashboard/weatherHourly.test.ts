import { describe, expect, it } from 'vitest'
import { localHourKey, nextHours, type HourPoint } from './weatherModel'

const mk = (n: number): HourPoint[] =>
  Array.from({ length: n }, (_, i) => {
    const day = 17 + Math.floor((i + 10) / 24)
    return {
      date: `2026-11-${day}`,
      label: `${String((i + 10) % 24).padStart(2, '0')}:00`,
      temp: i,
      code: 0,
      pop: null,
    }
  })

describe('nextHours', () => {
  it('starts at the current hour, caps at 24 and crosses midnight', () => {
    const r = nextHours(mk(48), '2026-11-17T12')
    expect(r).toHaveLength(24)
    expect(r[0]!.label).toBe('12:00')
    expect(r.some((h) => h.date === '2026-11-18')).toBe(true)
  })
  it('returns fewer when fewer are available', () => {
    expect(nextHours(mk(20), '2026-11-17T12')).toHaveLength(18)
    expect(nextHours(mk(5), '2026-12-01T00')).toEqual([])
  })
  it('localHourKey uses the household timezone', () => {
    expect(localHourKey(new Date('2026-11-17T23:30:00Z'), 'Europe/Berlin')).toBe('2026-11-18T00')
  })
})
