export const WEEKDAYS = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU'] as const
export type Weekday = (typeof WEEKDAYS)[number]

export type SimpleRule =
  | { kind: 'weekly'; interval: number; weekdays: Weekday[] }
  | { kind: 'monthly_day'; interval: number; day: number }
  | { kind: 'monthly_nth'; interval: number; nth: number; weekday: Weekday }

const isWeekday = (s: string): s is Weekday => (WEEKDAYS as readonly string[]).includes(s)

/** RRULE body (no DTSTART, no `RRULE:` prefix) for a simple rule. */
export function buildRrule(rule: SimpleRule): string {
  const parts: string[] = []
  const interval = rule.interval > 1 ? `;INTERVAL=${rule.interval}` : ''
  if (rule.kind === 'weekly') {
    const days = WEEKDAYS.filter((d) => rule.weekdays.includes(d))
    parts.push(`FREQ=WEEKLY${interval}`)
    if (days.length) parts.push(`BYDAY=${days.join(',')}`)
  } else if (rule.kind === 'monthly_day') {
    parts.push(`FREQ=MONTHLY${interval}`, `BYMONTHDAY=${rule.day}`)
  } else {
    parts.push(`FREQ=MONTHLY${interval}`, `BYDAY=${rule.nth}${rule.weekday}`)
  }
  return parts.join(';')
}

function posInt(s: string | undefined): number | null {
  if (s === undefined || !/^\d+$/.test(s)) return null
  const n = Number(s)
  return n >= 1 ? n : null
}

/** Parse an RRULE body into a simple rule, or `null` if the UI cannot represent it. */
export function parseSimple(text: string): SimpleRule | null {
  const body = text
    .trim()
    .replace(/^RRULE:/i, '')
    .toUpperCase()
  if (!body) return null
  const kv = new Map<string, string>()
  for (const part of body.split(';')) {
    const [k, v, ...rest] = part.split('=')
    if (!k || v === undefined || rest.length || kv.has(k)) return null
    kv.set(k, v)
  }
  const freq = kv.get('FREQ')
  let interval = 1
  if (kv.has('INTERVAL')) {
    const n = posInt(kv.get('INTERVAL'))
    if (n === null) return null
    interval = n
  }
  const allowed = (...keys: string[]) => [...kv.keys()].every((k) => keys.includes(k))
  if (freq === 'WEEKLY') {
    if (!allowed('FREQ', 'INTERVAL', 'BYDAY')) return null
    const days: Weekday[] = []
    const by = kv.get('BYDAY')
    if (by !== undefined) {
      for (const d of by.split(',')) {
        if (!isWeekday(d) || days.includes(d)) return null
        days.push(d)
      }
    }
    return { kind: 'weekly', interval, weekdays: WEEKDAYS.filter((d) => days.includes(d)) }
  }
  if (freq === 'MONTHLY') {
    if (!allowed('FREQ', 'INTERVAL', 'BYDAY', 'BYMONTHDAY')) return null
    const md = kv.get('BYMONTHDAY')
    const by = kv.get('BYDAY')
    if (md !== undefined && by === undefined) {
      if (!/^-?\d+$/.test(md)) return null
      const day = Number(md)
      if (day !== -1 && (day < 1 || day > 31)) return null
      return { kind: 'monthly_day', interval, day }
    }
    if (by !== undefined && md === undefined) {
      const m = /^(-1|[1-4])(MO|TU|WE|TH|FR|SA|SU)$/.exec(by)
      if (!m) return null
      return { kind: 'monthly_nth', interval, nth: Number(m[1]), weekday: m[2] as Weekday }
    }
  }
  return null
}
