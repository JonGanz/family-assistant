import { describe, expect, it } from 'vitest'
import { buildRrule, parseSimple, type SimpleRule } from './rrule'

describe('rrule helpers', () => {
  const cases: [SimpleRule, string][] = [
    [{ kind: 'weekly', interval: 1, weekdays: ['SA'] }, 'FREQ=WEEKLY;BYDAY=SA'],
    [{ kind: 'weekly', interval: 2, weekdays: ['MO', 'WE'] }, 'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE'],
    [{ kind: 'monthly_day', interval: 1, day: 1 }, 'FREQ=MONTHLY;BYMONTHDAY=1'],
    [{ kind: 'monthly_day', interval: 3, day: -1 }, 'FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=-1'],
    [{ kind: 'monthly_nth', interval: 1, nth: 2, weekday: 'TU' }, 'FREQ=MONTHLY;BYDAY=2TU'],
    [{ kind: 'monthly_nth', interval: 1, nth: -1, weekday: 'FR' }, 'FREQ=MONTHLY;BYDAY=-1FR'],
  ]
  it.each(cases)('round-trips %j', (rule, text) => {
    expect(buildRrule(rule)).toBe(text)
    expect(parseSimple(text)).toEqual(rule)
  })
  it('sorts weekdays when building', () => {
    expect(buildRrule({ kind: 'weekly', interval: 1, weekdays: ['SU', 'MO'] })).toBe(
      'FREQ=WEEKLY;BYDAY=MO,SU',
    )
  })
  it('accepts a prefix, case and any key order', () => {
    expect(parseSimple('rrule:interval=2;freq=weekly;byday=th')).toEqual({
      kind: 'weekly',
      interval: 2,
      weekdays: ['TH'],
    })
  })
  it.each([
    '',
    'FREQ=DAILY',
    'FREQ=YEARLY;BYMONTH=3',
    'FREQ=WEEKLY;COUNT=3',
    'FREQ=WEEKLY;BYDAY=XX',
    'FREQ=WEEKLY;INTERVAL=0',
    'FREQ=MONTHLY',
    'FREQ=MONTHLY;BYMONTHDAY=40',
    'FREQ=MONTHLY;BYMONTHDAY=1;BYDAY=MO',
    'FREQ=MONTHLY;BYDAY=5MO',
    'FREQ=MONTHLY;BYDAY=MO,TU',
    'garbage',
  ])('returns null for %j', (text) => {
    expect(parseSimple(text)).toBeNull()
  })
})
