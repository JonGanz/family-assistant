import { addDays } from '../lib/time'

export type WeekStart = 'monday' | 'sunday'

/** The first day of the week shown, `offset` weeks from the week holding `today` (YYYY-MM-DD). */
export function weekStartKey(today: string, weekStart: WeekStart, offset = 0): string {
  const dow = new Date(`${today}T12:00:00Z`).getUTCDay() // 0 = Sunday
  const back = weekStart === 'sunday' ? dow : (dow + 6) % 7
  return addDays(today, offset * 7 - back)
}

/** "Oct 25 – 31", or "Oct 28 – Nov 3" when the week spans two months. */
export function weekLabel(start: string, locale?: string): string {
  const fmt = (key: string, o: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(locale, { timeZone: 'UTC', ...o }).format(new Date(`${key}T12:00:00Z`))
  const end = addDays(start, 6)
  const sameMonth = start.slice(0, 7) === end.slice(0, 7)
  const a = fmt(start, { month: 'short', day: 'numeric' })
  const b = sameMonth ? fmt(end, { day: 'numeric' }) : fmt(end, { month: 'short', day: 'numeric' })
  return `${a} – ${b}`
}
