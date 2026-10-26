/** Date and time in the household time zone, browser locale. */
export function fmt(instant: string | Date, tz: string, locale?: string): string {
  return new Intl.DateTimeFormat(locale, {
    timeZone: tz,
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(instant))
}

/** `YYYY-MM-DD` shown as a plain date (no tz shift). */
export function fmtDay(day: string, locale?: string): string {
  const [y, m, d] = day.split('-').map(Number)
  return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', dateStyle: 'medium' }).format(
    Date.UTC(y!, m! - 1, d!, 12),
  )
}

/** RFC 3339 instant to the value of `<input type="datetime-local">` in `tz`. */
export function toLocalInput(instant: string, tz: string): string {
  const p = Object.fromEntries(
    new Intl.DateTimeFormat('en-CA', {
      timeZone: tz,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hourCycle: 'h23',
    })
      .formatToParts(new Date(instant))
      .map((x) => [x.type, x.value]),
  )
  return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}`
}

/** `datetime-local` value in `tz` to an RFC 3339 instant. */
export function fromLocalInput(value: string, tz: string): string {
  const [date, time] = value.split('T')
  const [y, mo, d] = date!.split('-').map(Number)
  const [h, mi] = time!.split(':').map(Number)
  const asUtc = Date.UTC(y!, mo! - 1, d!, h, mi)
  // Offset of tz at that wall time; refine once for DST edges.
  let guess = asUtc
  for (let i = 0; i < 2; i++) {
    const seen = toLocalInput(new Date(guess).toISOString(), tz)
    const [sd, st] = seen.split('T')
    const [sy, sm, sdd] = sd!.split('-').map(Number)
    const [sh, smi] = st!.split(':').map(Number)
    guess += asUtc - Date.UTC(sy!, sm! - 1, sdd!, sh, smi)
  }
  return new Date(guess).toISOString()
}
