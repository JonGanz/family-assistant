import { addDays, dayKey, groupByDay } from '../../lib/time'
import type { EventLike, MemberLike } from '../../lib/types'

/** The column for events linked to nobody (household-wide), shown when no member is chosen. */
export const EVERYONE = 'everyone'

export interface GridColumn {
  key: string
  name: string
  color: string
}

export interface GridRow {
  key: string
  today: boolean
  weekday: string
  dom: number
  cells: { key: string; name: string; events: EventLike[] }[]
}

export interface WeekGrid {
  columns: GridColumn[]
  rows: GridRow[]
}

/**
 * Days as rows, members as columns. `chosen` (empty = everyone) decides which member columns
 * exist at all; an event linked to several members appears in each of their columns, and events
 * linked to nobody get an "Everyone" column while no member is picked.
 */
export function buildWeekGrid(o: {
  events: EventLike[]
  members: MemberLike[]
  chosen: string[]
  /** First day (YYYY-MM-DD) of the 7 shown. */
  start: string
  tz: string
  now?: Date
  locale?: string
}): WeekGrid {
  const today = dayKey(o.now ?? new Date(), o.tz)
  const people = o.chosen.length ? o.members.filter((m) => o.chosen.includes(m.id)) : o.members
  const columns: GridColumn[] = people.map((m) => ({
    key: m.id,
    name: m.display_name,
    color: m.color,
  }))
  const hasWide = !o.chosen.length && o.events.some((e) => e.members.length === 0)
  if (hasWide) columns.push({ key: EVERYONE, name: 'Everyone', color: 'var(--ui-outline)' })
  const byDay = groupByDay(o.events, o.tz, o.start, 7)
  const rows = Array.from({ length: 7 }, (_, i): GridRow => {
    const key = addDays(o.start, i)
    const d = new Date(`${key}T12:00:00Z`)
    const list = [...(byDay.get(key) ?? [])].sort(
      (a, b) => Number(b.all_day) - Number(a.all_day) || a.start_at.localeCompare(b.start_at),
    )
    return {
      key,
      today: key === today,
      weekday: new Intl.DateTimeFormat(o.locale, { weekday: 'short', timeZone: 'UTC' }).format(d),
      dom: d.getUTCDate(),
      cells: columns.map((c) => ({
        key: c.key,
        name: c.name,
        events: list.filter((e) =>
          c.key === EVERYONE ? e.members.length === 0 : e.members.includes(c.key),
        ),
      })),
    }
  })
  return { columns, rows }
}

/**
 * Events shown in one cell; any further ones collapse into a "+N events" item. Wide screens fit
 * two two-line items plus the "+N" line; a phone fits three one-line slots, the last being "+N"
 * when the events do not all fit.
 */
export function weekVisible(total: number, phone: boolean): number {
  if (!phone) return Math.min(total, 2)
  return total <= 3 ? total : 2
}
