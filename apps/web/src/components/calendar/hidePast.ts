import { addDays, dayKey } from '../../lib/time'
import type { EventLike } from '../../lib/types'

export const HIDE_PAST_KEY = 'fa.calendar.week.hidePast'

export function readHidePast(): boolean {
  try {
    return localStorage.getItem(HIDE_PAST_KEY) === '1'
  } catch {
    return false
  }
}

export function writeHidePast(v: boolean): void {
  try {
    localStorage.setItem(HIDE_PAST_KEY, v ? '1' : '0')
  } catch {
    // Blocked storage: the choice is simply not remembered.
  }
}

/** Over by now: timed events by their end, all-day events once their last day is behind us. */
export function eventEnded(e: EventLike, now: Date, tz: string): boolean {
  if (e.all_day && e.start_date) {
    const endExclusive =
      e.end_date && e.end_date > e.start_date ? e.end_date : addDays(e.start_date, 1)
    return endExclusive <= dayKey(now, tz)
  }
  return Date.parse(e.end_at) <= now.getTime()
}
