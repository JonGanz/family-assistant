import { dayKey, startOfDay } from '../../lib/time'

const hm = (instant: Date, tz: string): string =>
  new Intl.DateTimeFormat('en-GB', {
    timeZone: tz,
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
  }).format(instant)

/** Household-local `YYYY-MM-DD` + `HH:MM` (default 23:59 when only a date is given) to an RFC 3339 instant. */
export function toInstant(date: string, time: string, tz: string): string | null {
  if (!date) return null
  const t = /^\d{2}:\d{2}$/.test(time) ? time : '23:59'
  const [h, m] = t.split(':').map(Number)
  const start = startOfDay(tz, date).getTime()
  let ms = start + ((h ?? 0) * 60 + (m ?? 0)) * 60_000
  // Correct for a DST change inside the day.
  const got = hm(new Date(ms), tz).split(':').map(Number)
  const diff = (h ?? 0) * 60 + (m ?? 0) - ((got[0] ?? 0) * 60 + (got[1] ?? 0))
  if (diff !== 0 && Math.abs(diff) <= 120) ms += diff * 60_000
  return new Date(ms).toISOString()
}

/** Splits an instant into household-local date and time inputs. */
export function fromInstant(
  iso: string | null | undefined,
  tz: string,
): { date: string; time: string } {
  if (!iso) return { date: '', time: '' }
  return { date: dayKey(iso, tz), time: hm(new Date(iso), tz) }
}
