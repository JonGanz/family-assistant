import { describe, expect, it } from 'vitest'
import type { TaskDefinitionView } from '../../api/client'
import {
  defaultOnMissed,
  emptyForm,
  formFromView,
  formProblems,
  toInput,
  toPatch,
} from './definitionForm'

describe('definition form mapping', () => {
  it('defaults the missed behaviour per schedule kind (D-038, D-049)', () => {
    expect(defaultOnMissed('after_done')).toBe('roll_over')
    expect(defaultOnMissed('rrule')).toBe('auto_skip')
    expect(emptyForm('l', '2026-10-25').onMissed).toBe('auto_skip')
  })

  it('maps "every N days after done" to the after_done schedule', () => {
    const f = {
      ...emptyForm('l', '2026-10-25'),
      title: ' Vacuum ',
      scheduleKind: 'after_done' as const,
      intervalDays: 10,
    }
    const i = toInput(f)
    expect(i.schedule).toEqual({ kind: 'after_done', interval_days: 10 })
    expect(i.title).toBe('Vacuum')
  })

  it('maps "Repeats" to an rrule and drops empty optional fields', () => {
    const i = toInput({
      ...emptyForm('l', '2026-10-25'),
      title: 'Bins',
      rrule: ' FREQ=WEEKLY;BYDAY=TU ',
    })
    expect(i.schedule).toEqual({ kind: 'rrule', rrule: 'FREQ=WEEKLY;BYDAY=TU' })
    expect(i.due_time).toBeNull()
    expect(i.season).toBeNull()
    expect(i.season_condition).toBeNull()
    expect(i.steps_template).toEqual([])
  })

  it('maps steps, season and condition', () => {
    const i = toInput({
      ...emptyForm('l', '2026-10-25'),
      title: 'Mow',
      steps: 'Fuel\n\n  Mow  \n',
      seasonOn: true,
      seasonFrom: '04-01',
      seasonUntil: '10-31',
      condition: ' sum(precip_mm, today) < 5 ',
      dueTime: '17:30',
    })
    expect(i.steps_template).toEqual(['Fuel', 'Mow'])
    expect(i.season).toEqual({ from: '04-01', until: '10-31' })
    expect(i.season_condition).toBe('sum(precip_mm, today) < 5')
    expect(i.due_time).toBe('17:30')
  })

  it('round-trips a view through the form into a patch', () => {
    const v = {
      id: 'd',
      list: 'l',
      title: 'Water plants',
      notes: null,
      steps_template: ['Fill can'],
      schedule: { kind: 'after_done', interval_days: 3 },
      start_date: '2026-10-01',
      due_time: '18:00',
      lead_days: 1,
      on_missed: 'roll_over',
      strategy: 'rotation',
      candidates: ['a', 'b'],
      advance_on: 'assignee_completion',
      skip_advances: false,
      priority_flag: 'high',
      season: { from: '05-01', until: '09-30' },
      season_condition: null,
      paused: true,
    } as unknown as TaskDefinitionView
    const p = toPatch(formFromView(v))
    expect(p).toMatchObject({
      schedule: { kind: 'after_done', interval_days: 3 },
      due_time: '18:00',
      lead_days: 1,
      on_missed: 'roll_over',
      candidates: ['a', 'b'],
      advance_on: 'assignee_completion',
      skip_advances: false,
      season: { from: '05-01', until: '09-30' },
      paused: true,
    })
  })

  it('reports problems before saving', () => {
    const f = emptyForm('l', '2026-10-25')
    expect(formProblems(f)).toContain('Give it a title.')
    expect(formProblems({ ...f, title: 'x', strategy: 'fixed', candidates: ['a', 'b'] })).toContain(
      'Pick exactly one person.',
    )
    expect(
      formProblems({ ...f, title: 'x', candidates: ['a'], seasonOn: true, seasonFrom: '13-01' }),
    ).toContain('Season dates must look like 04-01 (month-day).')
    expect(formProblems({ ...f, title: 'x', candidates: ['a'] })).toEqual([])
  })
})
