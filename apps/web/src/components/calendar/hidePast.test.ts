import { beforeEach, describe, expect, it } from 'vitest'
import type { EventLike } from '../../lib/types'
import { eventEnded, HIDE_PAST_KEY, readHidePast, writeHidePast } from './hidePast'

const now = new Date('2026-11-10T15:00:00Z')
const tz = 'UTC'
const e = (x: Partial<EventLike>) =>
  ({ start_at: '', end_at: '', all_day: false, members: [], color: 'red', ...x }) as EventLike

describe('eventEnded', () => {
  it('timed events end at their end', () => {
    expect(eventEnded(e({ end_at: '2026-11-10T14:00:00Z' }), now, tz)).toBe(true)
    expect(eventEnded(e({ end_at: '2026-11-10T16:00:00Z' }), now, tz)).toBe(false)
  })
  it('all-day events end after their last day', () => {
    const d = (s: string, x?: string) => e({ all_day: true, start_date: s, end_date: x })
    expect(eventEnded(d('2026-11-09'), now, tz)).toBe(true)
    expect(eventEnded(d('2026-11-10'), now, tz)).toBe(false)
    expect(eventEnded(d('2026-11-08', '2026-11-11'), now, tz)).toBe(false)
  })
})
describe('storage', () => {
  beforeEach(() => localStorage.clear())
  it('defaults off and round-trips', () => {
    expect(readHidePast()).toBe(false)
    writeHidePast(true)
    expect(localStorage.getItem(HIDE_PAST_KEY)).toBe('1')
    expect(readHidePast()).toBe(true)
  })
})
