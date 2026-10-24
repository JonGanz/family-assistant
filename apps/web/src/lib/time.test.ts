import { describe, expect, it } from 'vitest'
import { addDays, dayKey, formatRange, groupByDay, msUntilNextMidnight, startOfDay } from './time'
import type { EventLike } from './types'

const NY = 'America/New_York'
const LON = 'Europe/London'
const ev = (o: Partial<EventLike>): EventLike => ({
  start_at: '',
  end_at: '',
  all_day: false,
  members: [],
  color: '#fff',
  visibility: 'full',
  ...o,
})

describe('dayKey / startOfDay', () => {
  it('uses the household tz, not UTC', () => {
    expect(dayKey('2026-03-08T03:00:00Z', NY)).toBe('2026-03-07')
    expect(dayKey('2026-03-08T05:00:00Z', NY)).toBe('2026-03-08')
  })
  it('handles NY spring forward (23h day)', () => {
    expect(startOfDay(NY, '2026-03-08').toISOString()).toBe('2026-03-08T05:00:00.000Z')
    expect(startOfDay(NY, '2026-03-09').toISOString()).toBe('2026-03-09T04:00:00.000Z')
  })
  it('handles NY fall back (25h day)', () => {
    expect(startOfDay(NY, '2026-11-01').toISOString()).toBe('2026-11-01T04:00:00.000Z')
    expect(startOfDay(NY, '2026-11-02').toISOString()).toBe('2026-11-02T05:00:00.000Z')
  })
  it('handles London', () => {
    expect(startOfDay(LON, '2026-03-29').toISOString()).toBe('2026-03-29T00:00:00.000Z')
    expect(startOfDay(LON, '2026-03-30').toISOString()).toBe('2026-03-29T23:00:00.000Z')
    expect(startOfDay(LON, '2026-10-25').toISOString()).toBe('2026-10-24T23:00:00.000Z')
    expect(startOfDay(LON, '2026-10-26').toISOString()).toBe('2026-10-26T00:00:00.000Z')
  })
  it('addDays crosses month ends', () => {
    expect(addDays('2026-02-28', 1)).toBe('2026-03-01')
    expect(addDays('2026-01-01', -1)).toBe('2025-12-31')
  })
})

describe('msUntilNextMidnight', () => {
  it('is 23h on NY spring-forward day', () => {
    const now = new Date('2026-03-08T05:00:00Z') // local midnight
    expect(msUntilNextMidnight(NY, now)).toBe(23 * 3600_000)
  })
  it('is 25h on NY fall-back day', () => {
    const now = new Date('2026-11-01T04:00:00Z')
    expect(msUntilNextMidnight(NY, now)).toBe(25 * 3600_000)
  })
  it('London late evening', () => {
    const now = new Date('2026-03-28T23:30:00Z')
    expect(msUntilNextMidnight(LON, now)).toBe(30 * 60_000)
  })
})

describe('groupByDay', () => {
  it('splits timed events by local day and ends exclusively at midnight', () => {
    const e1 = ev({ start_at: '2026-03-08T01:00:00Z', end_at: '2026-03-08T02:00:00Z' }) // Mar 7 20:00-21:00 NY
    const e2 = ev({ start_at: '2026-03-08T20:00:00Z', end_at: '2026-03-09T04:00:00Z' }) // Mar 8 16:00 - Mar 9 00:00 NY
    const g = groupByDay([e1, e2], NY, '2026-03-07', 3)
    expect(g.get('2026-03-07')).toEqual([e1])
    expect(g.get('2026-03-08')).toEqual([e2])
    expect(g.get('2026-03-09')).toEqual([])
  })
  it('spans multi-day all-day events with exclusive end', () => {
    const a = ev({ all_day: true, start_date: '2026-11-01', end_date: '2026-11-03' })
    const g = groupByDay([a], NY, '2026-10-31', 5)
    expect([...g].filter(([, l]) => l.length).map(([k]) => k)).toEqual(['2026-11-01', '2026-11-02'])
  })
  it('puts all-day first and clips to the window', () => {
    const a = ev({ all_day: true, start_date: '2026-10-20', end_date: '2026-10-30' })
    const t = ev({ start_at: '2026-10-26T14:00:00Z', end_at: '2026-10-26T15:00:00Z' })
    const g = groupByDay([t, a], LON, '2026-10-26', 2)
    expect(g.get('2026-10-26')).toEqual([a, t])
    expect(g.get('2026-10-27')).toEqual([a])
  })
  it('keeps a multi-day timed event across DST change in the right days', () => {
    const e = ev({ start_at: '2026-10-24T22:00:00Z', end_at: '2026-10-25T22:00:00Z' }) // London
    const g = groupByDay([e], LON, '2026-10-24', 3)
    expect(g.get('2026-10-24')).toEqual([e]) // 23:00 BST
    expect(g.get('2026-10-25')).toEqual([e])
    expect(g.get('2026-10-26')).toEqual([])
  })
})

describe('formatRange', () => {
  it('all day and timed', () => {
    expect(formatRange(ev({ all_day: true }), NY)).toBe('All day')
    const s = formatRange(
      ev({ start_at: '2026-03-08T15:00:00Z', end_at: '2026-03-08T16:30:00Z' }),
      NY,
      'en-US',
    )
    expect(s).toMatch(/11:00.*AM.*12:30.*PM/)
  })
})
