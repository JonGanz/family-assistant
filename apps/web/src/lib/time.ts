import type { EventLike } from './types'

const fmtCache = new Map<string, Intl.DateTimeFormat>()
function fmt(tz: string, opts: Intl.DateTimeFormatOptions, locale?: string): Intl.DateTimeFormat {
  const key = `${locale ?? ''}|${tz}|${JSON.stringify(opts)}`
  let f = fmtCache.get(key)
  if (!f) {
    f = new Intl.DateTimeFormat(locale, { timeZone: tz, ...opts })
    fmtCache.set(key, f)
  }
  return f
}

interface Parts {
  y: number
  m: number
  d: number
  h: number
  mi: number
  s: number
}

function parts(ms: number, tz: string): Parts {
  const p: Record<string, number> = {}
  for (const x of fmt(
    tz,
    {
      hourCycle: 'h23',
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    },
    'en-US',
  ).formatToParts(ms)) {
    if (x.type !== 'literal') p[x.type] = Number(x.value)
  }
  return { y: p.year!, m: p.month!, d: p.day!, h: p.hour!, mi: p.minute!, s: p.second! }
}

const pad = (n: number, w = 2) => String(n).padStart(w, '0')

/** Calendar day (YYYY-MM-DD) of an instant in the given tz. */
export function dayKey(instant: Date | string | number, tz: string): string {
  const p = parts(new Date(instant).getTime(), tz)
  return `${pad(p.y, 4)}-${pad(p.m)}-${pad(p.d)}`
}

/** Add days to a YYYY-MM-DD key (pure calendar arithmetic). */
export function addDays(key: string, n: number): string {
  const [y, m, d] = key.split('-').map(Number)
  const t = new Date(Date.UTC(y!, m! - 1, d! + n))
  return `${pad(t.getUTCFullYear(), 4)}-${pad(t.getUTCMonth() + 1)}-${pad(t.getUTCDate())}`
}

/** Offset (ms) of tz from UTC at the given instant. */
function offsetMs(ms: number, tz: string): number {
  const p = parts(ms, tz)
  return Date.UTC(p.y, p.m - 1, p.d, p.h, p.mi, p.s) - Math.floor(ms / 1000) * 1000
}

/** The instant at which the given calendar day (YYYY-MM-DD) starts in tz. */
export function startOfDay(tz: string, date: string | Date): Date {
  const key = typeof date === 'string' ? date : dayKey(date, tz)
  const [y, m, d] = key.split('-').map(Number)
  const wall = Date.UTC(y!, m! - 1, d!)
  // Two-pass fixed point handles offsets that change within the day.
  let guess = wall - offsetMs(wall, tz)
  guess = wall - offsetMs(guess, tz)
  // If midnight does not exist (DST gap at 00:00), guess lands after; step back to the earliest instant of the day.
  const check = dayKey(guess - 1, tz)
  if (check === key) {
    // midnight exists earlier than our guess; walk back
    let g = guess
    while (dayKey(g - 3600_000, tz) === key) g -= 3600_000
    return new Date(g)
  }
  return new Date(guess)
}

export function msUntilNextMidnight(tz: string, now: Date = new Date()): number {
  const next = startOfDay(tz, addDays(dayKey(now, tz), 1))
  return next.getTime() - now.getTime()
}

/** Group events by the days they touch, for `days` days starting at `from` (YYYY-MM-DD). */
export function groupByDay<E extends EventLike>(
  events: E[],
  tz: string,
  from: string,
  days: number,
): Map<string, E[]> {
  const out = new Map<string, E[]>()
  for (let i = 0; i < days; i++) out.set(addDays(from, i), [])
  for (const ev of events) {
    let first: string
    let last: string
    if (ev.all_day && ev.start_date) {
      first = ev.start_date
      const endEx =
        ev.end_date && ev.end_date > ev.start_date ? ev.end_date : addDays(ev.start_date, 1)
      last = addDays(endEx, -1)
    } else {
      first = dayKey(ev.start_at, tz)
      const endMs = new Date(ev.end_at).getTime()
      const startMs = new Date(ev.start_at).getTime()
      // end is exclusive: an event ending exactly at midnight does not touch the next day
      last = dayKey(endMs > startMs ? endMs - 1 : endMs, tz)
    }
    for (let k = first < from ? from : first; k <= last; k = addDays(k, 1)) {
      const bucket = out.get(k)
      if (!bucket) {
        if (k > addDays(from, days - 1)) break
        continue
      }
      bucket.push(ev)
    }
  }
  for (const list of out.values()) {
    list.sort(
      (a, b) => Number(b.all_day) - Number(a.all_day) || a.start_at.localeCompare(b.start_at),
    )
  }
  return out
}

export function formatTime(instant: Date | string, tz: string, locale?: string): string {
  return fmt(tz, { hour: 'numeric', minute: '2-digit' }, locale).format(new Date(instant))
}

export function formatDayHeading(
  key: string,
  _tz: string,
  todayKey?: string,
  locale?: string,
): string {
  const [y, m, d] = key.split('-').map(Number)
  const noon = Date.UTC(y!, m! - 1, d!, 12)
  const base = fmt('UTC', { weekday: 'long', month: 'short', day: 'numeric' }, locale).format(noon)
  if (todayKey) {
    if (key === todayKey) return `Today, ${base}`
    if (key === addDays(todayKey, 1)) return `Tomorrow, ${base}`
  }
  return base
}

export function formatRange(
  ev: Pick<EventLike, 'start_at' | 'end_at' | 'all_day'>,
  tz: string,
  locale?: string,
): string {
  if (ev.all_day) return 'All day'
  const a = formatTime(ev.start_at, tz, locale)
  const b = formatTime(ev.end_at, tz, locale)
  return a === b ? a : `${a} – ${b}`
}
