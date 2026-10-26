import { dayKey, formatTime } from './time'
import { parseSimple, type SimpleRule, type Weekday } from './rrule'

export type TaskStatus = 'open' | 'done' | 'skipped' | 'suggested'
export type PriorityFlag = 'normal' | 'high'
export type CalendarDisplay = 'hidden' | 'until_done' | 'always'
export type TaskOrigin = 'manual' | 'system' | 'weather' | 'email'

export interface MemberRef {
  id: string
  name: string
  color: string
  avatarUrl?: string | null
}

/** Structural subset of `TaskView`; wave 5 maps generated DTOs onto it. */
export interface TaskItemModel {
  id: string
  title: string
  dueAt?: string | null
  status: TaskStatus
  priorityFlag: PriorityFlag
  assignees: MemberRef[]
  stepsDone: number
  stepsTotal: number
  claimedBy?: MemberRef | null
  origin: TaskOrigin
}

/** Structural subset of `TaskStepView`. */
export interface StepModel {
  id: string
  text: string
  done: boolean
}

export type ScheduleModel =
  { kind: 'after_done'; interval_days: number } | { kind: 'rrule'; rrule: string }

const DAY_NAMES: Record<Weekday, string> = {
  MO: 'Monday',
  TU: 'Tuesday',
  WE: 'Wednesday',
  TH: 'Thursday',
  FR: 'Friday',
  SA: 'Saturday',
  SU: 'Sunday',
}

const dayDiff = (a: string, b: string) =>
  Math.round((Date.parse(`${a}T00:00:00Z`) - Date.parse(`${b}T00:00:00Z`)) / 86_400_000)

export function isOverdue(dueAt: string | null | undefined, now: Date): boolean {
  return !!dueAt && new Date(dueAt).getTime() < now.getTime()
}

/** "today 18:00", "tomorrow", "Sat", "Nov 3", "overdue 2 d". */
export function dueLabel(dueAt: string | null | undefined, now: Date, tz: string): string {
  if (!dueAt) return ''
  const today = dayKey(now, tz)
  const key = dayKey(dueAt, tz)
  const diff = dayDiff(key, today)
  if (diff === 0) return `today ${formatTime(dueAt, tz, 'en-GB')}`
  if (diff < 0) return `overdue ${-diff} d`
  if (diff === 1) return 'tomorrow'
  const [y, m, d] = key.split('-').map(Number)
  const noon = new Date(Date.UTC(y!, m! - 1, d!, 12))
  const opts: Intl.DateTimeFormatOptions =
    diff < 7 ? { weekday: 'short' } : { month: 'short', day: 'numeric' }
  return new Intl.DateTimeFormat('en-US', { timeZone: 'UTC', ...opts }).format(noon)
}

export function calendarDisplayLabel(c: CalendarDisplay): string {
  return { hidden: 'Hidden', until_done: 'Until done', always: 'Key date' }[c]
}

function ordinal(n: number): string {
  const v = n % 100
  if (v >= 11 && v <= 13) return `${n}th`
  return `${n}${({ 1: 'st', 2: 'nd', 3: 'rd' } as Record<number, string>)[n % 10] ?? 'th'}`
}

export function ruleSummary(rule: SimpleRule): string {
  if (rule.kind === 'weekly') {
    const days = rule.weekdays
    if (days.length === 7 && rule.interval === 1) return 'Every day'
    const names =
      days.length === 1 ? DAY_NAMES[days[0]!] : days.map((d) => DAY_NAMES[d].slice(0, 3)).join(', ')
    const every = rule.interval === 1 ? 'Every' : `Every ${rule.interval} weeks on`
    return days.length
      ? `${every} ${names}`
      : rule.interval === 1
        ? 'Every week'
        : `Every ${rule.interval} weeks`
  }
  const months = rule.interval === 1 ? 'month' : `${rule.interval} months`
  if (rule.kind === 'monthly_day') {
    const day = rule.day === -1 ? 'Last day' : ordinal(rule.day)
    return `${day} of every ${months}`
  }
  const nth = rule.nth === -1 ? 'Last' : ordinal(rule.nth)
  return `${nth} ${DAY_NAMES[rule.weekday]} of every ${months}`
}

export function scheduleSummary(s: ScheduleModel): string {
  if (s.kind === 'after_done') {
    return s.interval_days === 1
      ? 'Every day after done'
      : `Every ${s.interval_days} days after done`
  }
  const simple = parseSimple(s.rrule)
  return simple ? ruleSummary(simple) : s.rrule
}

/** Valid "MM-DD" (Feb 29 allowed). */
export function seasonValid(md: string): boolean {
  const m = /^(\d{2})-(\d{2})$/.exec(md)
  if (!m) return false
  const month = Number(m[1])
  const day = Number(m[2])
  if (month < 1 || month > 12 || day < 1) return false
  return day <= new Date(Date.UTC(2024, month, 0)).getUTCDate()
}

export function progress(
  done: number,
  total: number,
): { fraction: number; percent: number; complete: boolean } {
  if (total <= 0) return { fraction: 0, percent: 0, complete: false }
  const fraction = Math.min(1, Math.max(0, done / total))
  return { fraction, percent: Math.round(fraction * 100), complete: done >= total }
}
