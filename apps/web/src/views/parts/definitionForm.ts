import type { TaskDefinitionInput, TaskDefinitionPatch, TaskDefinitionView } from '../../api/client'
import { seasonValid } from '../../lib/tasks'

export type ScheduleKind = 'after_done' | 'rrule'
export type OnMissed = TaskDefinitionInput['on_missed'] & string
export type Strategy = TaskDefinitionInput['strategy']

export interface DefinitionForm {
  list: string
  title: string
  notes: string
  /** One step per line. */
  steps: string
  scheduleKind: ScheduleKind
  intervalDays: number
  rrule: string
  startDate: string
  dueTime: string
  leadDays: number
  onMissed: OnMissed
  strategy: Strategy
  /** Ordered. */
  candidates: string[]
  advanceOn: TaskDefinitionInput['advance_on']
  skipAdvances: boolean
  priority: TaskDefinitionInput['priority_flag']
  seasonOn: boolean
  seasonFrom: string
  seasonUntil: string
  condition: string
  paused: boolean
}

/** D-038 and D-049: an "after done" chore rolls over, a fixed-date repeat skips. */
export function defaultOnMissed(kind: ScheduleKind): OnMissed {
  return kind === 'after_done' ? 'roll_over' : 'auto_skip'
}

export const ON_MISSED_HELP: Record<OnMissed, string> = {
  roll_over: 'The task stays open and overdue until someone does it; no new one is made meanwhile.',
  auto_skip: 'A missed task is skipped automatically and the next one appears on schedule.',
  pile_up: 'Every occurrence is kept, so missed ones pile up next to the new one.',
}

export const STRATEGY_HELP: Record<Strategy, string> = {
  fixed: 'Always the same person.',
  rotation: 'Takes turns in the order below, skipping anyone who is away.',
  pool: 'Anyone in the group can do it, and can claim it first.',
}

export function emptyForm(list: string, today: string): DefinitionForm {
  return {
    list,
    title: '',
    notes: '',
    steps: '',
    scheduleKind: 'rrule',
    intervalDays: 7,
    rrule: 'FREQ=WEEKLY;BYDAY=SA',
    startDate: today,
    dueTime: '',
    leadDays: 0,
    onMissed: defaultOnMissed('rrule'),
    strategy: 'pool',
    candidates: [],
    advanceOn: 'any_completion',
    skipAdvances: true,
    priority: 'normal',
    seasonOn: false,
    seasonFrom: '04-01',
    seasonUntil: '10-31',
    condition: '',
    paused: false,
  }
}

export function formFromView(v: TaskDefinitionView): DefinitionForm {
  return {
    list: v.list,
    title: v.title,
    notes: v.notes ?? '',
    steps: v.steps_template.join('\n'),
    scheduleKind: v.schedule.kind,
    intervalDays: v.schedule.kind === 'after_done' ? v.schedule.interval_days : 7,
    rrule: v.schedule.kind === 'rrule' ? v.schedule.rrule : 'FREQ=WEEKLY;BYDAY=SA',
    startDate: v.start_date,
    dueTime: v.due_time ? v.due_time.slice(0, 5) : '',
    leadDays: v.lead_days,
    onMissed: v.on_missed,
    strategy: v.strategy,
    candidates: [...v.candidates],
    advanceOn: v.advance_on,
    skipAdvances: v.skip_advances,
    priority: v.priority_flag,
    seasonOn: !!v.season,
    seasonFrom: v.season?.from ?? '04-01',
    seasonUntil: v.season?.until ?? '10-31',
    condition: v.season_condition ?? '',
    paused: v.paused,
  }
}

const stepsOf = (s: string) =>
  s
    .split('\n')
    .map((x) => x.trim())
    .filter(Boolean)

function schedule(f: DefinitionForm): TaskDefinitionInput['schedule'] {
  return f.scheduleKind === 'after_done'
    ? { kind: 'after_done', interval_days: Math.max(1, Math.floor(f.intervalDays) || 1) }
    : { kind: 'rrule', rrule: f.rrule.trim() }
}

/** Client-side checks the server repeats (it stays the authority). */
export function formProblems(f: DefinitionForm): string[] {
  const out: string[] = []
  if (!f.title.trim()) out.push('Give it a title.')
  if (!f.list) out.push('Pick a list.')
  if (!f.startDate) out.push('Pick a start date.')
  if (f.scheduleKind === 'rrule' && !f.rrule.trim()) out.push('Choose how it repeats.')
  if (f.strategy === 'fixed' && f.candidates.length !== 1) out.push('Pick exactly one person.')
  if (f.strategy !== 'fixed' && f.candidates.length === 0) out.push('Pick at least one person.')
  if (f.seasonOn && !(seasonValid(f.seasonFrom) && seasonValid(f.seasonUntil)))
    out.push('Season dates must look like 04-01 (month-day).')
  return out
}

export function toInput(f: DefinitionForm): TaskDefinitionInput {
  return {
    list: f.list,
    title: f.title.trim(),
    notes: f.notes.trim() || null,
    steps_template: stepsOf(f.steps),
    schedule: schedule(f),
    start_date: f.startDate,
    due_time: f.dueTime || null,
    lead_days: Math.max(0, Math.floor(f.leadDays) || 0),
    on_missed: f.onMissed,
    strategy: f.strategy,
    candidates: f.candidates,
    advance_on: f.advanceOn,
    skip_advances: f.skipAdvances,
    priority_flag: f.priority,
    season: f.seasonOn ? { from: f.seasonFrom, until: f.seasonUntil } : null,
    season_condition: f.condition.trim() || null,
    paused: f.paused,
  }
}

export function toPatch(f: DefinitionForm): TaskDefinitionPatch {
  const i = toInput(f)
  return {
    list: i.list,
    title: i.title,
    notes: i.notes,
    steps_template: i.steps_template,
    schedule: i.schedule,
    start_date: i.start_date,
    due_time: i.due_time,
    lead_days: i.lead_days,
    on_missed: i.on_missed,
    strategy: i.strategy,
    candidates: i.candidates,
    advance_on: i.advance_on,
    skip_advances: i.skip_advances,
    priority_flag: i.priority_flag,
    season: i.season,
    season_condition: i.season_condition,
    paused: i.paused,
  }
}
