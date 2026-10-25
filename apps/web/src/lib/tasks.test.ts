import { describe, expect, it } from 'vitest'
import {
  calendarDisplayLabel,
  dueLabel,
  isOverdue,
  progress,
  scheduleSummary,
  seasonValid,
} from './tasks'

const tz = 'Europe/Berlin'
const now = new Date('2026-10-24T10:00:00Z') // Saturday 12:00 local

describe('dueLabel', () => {
  it('formats relative to now', () => {
    expect(dueLabel('2026-10-24T16:00:00Z', now, tz)).toBe('today 18:00')
    expect(dueLabel('2026-10-25T10:00:00Z', now, tz)).toBe('tomorrow')
    expect(dueLabel('2026-10-27T10:00:00Z', now, tz)).toBe('Tue')
    expect(dueLabel('2026-11-03T10:00:00Z', now, tz)).toBe('Nov 3')
    expect(dueLabel('2026-10-22T10:00:00Z', now, tz)).toBe('overdue 2 d')
    expect(dueLabel(null, now, tz)).toBe('')
  })
  it('uses household-local days', () => {
    expect(dueLabel('2026-10-24T22:30:00Z', now, tz)).toBe('tomorrow')
  })
})

describe('isOverdue', () => {
  it('compares instants', () => {
    expect(isOverdue('2026-10-24T09:00:00Z', now)).toBe(true)
    expect(isOverdue('2026-10-24T11:00:00Z', now)).toBe(false)
    expect(isOverdue(undefined, now)).toBe(false)
  })
})

describe('labels', () => {
  it('calendar display', () => {
    expect(calendarDisplayLabel('hidden')).toBe('Hidden')
    expect(calendarDisplayLabel('until_done')).toBe('Until done')
    expect(calendarDisplayLabel('always')).toBe('Key date')
  })
  it('schedule summary', () => {
    expect(scheduleSummary({ kind: 'after_done', interval_days: 7 })).toBe(
      'Every 7 days after done',
    )
    expect(scheduleSummary({ kind: 'rrule', rrule: 'FREQ=WEEKLY;BYDAY=SA' })).toBe('Every Saturday')
    expect(scheduleSummary({ kind: 'rrule', rrule: 'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH' })).toBe(
      'Every 2 weeks on Mon, Thu',
    )
    expect(scheduleSummary({ kind: 'rrule', rrule: 'FREQ=MONTHLY;BYMONTHDAY=1' })).toBe(
      '1st of every month',
    )
    expect(scheduleSummary({ kind: 'rrule', rrule: 'FREQ=MONTHLY;BYDAY=-1FR' })).toBe(
      'Last Friday of every month',
    )
    expect(scheduleSummary({ kind: 'rrule', rrule: 'FREQ=YEARLY;BYMONTH=3' })).toBe(
      'FREQ=YEARLY;BYMONTH=3',
    )
  })
})

describe('seasonValid', () => {
  it('validates MM-DD', () => {
    for (const ok of ['01-01', '02-29', '12-31', '11-30']) expect(seasonValid(ok)).toBe(true)
    for (const bad of ['13-01', '00-10', '04-31', '2-3', '1101', '', '02-30'])
      expect(seasonValid(bad)).toBe(false)
  })
})

describe('progress', () => {
  it('handles edges', () => {
    expect(progress(0, 0)).toEqual({ fraction: 0, percent: 0, complete: false })
    expect(progress(1, 3)).toMatchObject({ percent: 33, complete: false })
    expect(progress(3, 3)).toMatchObject({ percent: 100, complete: true })
    expect(progress(5, 3).fraction).toBe(1)
  })
})
