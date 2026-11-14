// The Meal plan agenda's day range: a continuous list from two days before today to the planning horizon.
import { addDays } from '../../lib/time'

/** Days shown before today. */
export const AGENDA_DAYS_BACK = 2

export interface AgendaRange {
  from: string
  /** Last day, inclusive: today plus the horizon (the last day the plan may be written). */
  to: string
  days: string[]
}

/** `YYYY-MM-DD` keys from `today - 2` through `today + horizonWeeks * 7`, inclusive. */
export function agendaRange(today: string, horizonWeeks: number): AgendaRange {
  const from = addDays(today, -AGENDA_DAYS_BACK)
  const to = addDays(today, Math.max(0, Math.round(horizonWeeks)) * 7)
  const days: string[] = []
  for (let d = from; d <= to; d = addDays(d, 1)) days.push(d)
  return { from, to, days }
}
