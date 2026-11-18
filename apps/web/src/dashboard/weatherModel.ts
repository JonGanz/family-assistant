import type { WeatherView } from './types'

type Columns = Record<string, unknown[]>

function columns(v: unknown): Columns {
  return v && typeof v === 'object' ? (v as Columns) : {}
}
const num = (a: unknown[] | undefined, i: number): number | null => {
  const v = a?.[i]
  return typeof v === 'number' ? v : null
}
const str = (a: unknown[] | undefined, i: number): string => {
  const v = a?.[i]
  return typeof v === 'string' ? v : ''
}

export interface CurrentWx {
  temp: number | null
  feels: number | null
  code: number
  night: boolean
  wind: number | null
}
export interface HourPoint {
  /** Local date of the hour, YYYY-MM-DD. */
  date: string
  /** Local wall-clock hour, e.g. "14:00" (Open-Meteo returns household-local times). */
  label: string
  temp: number | null
  code: number
  pop: number | null
}
export interface DayPoint {
  date: string
  code: number
  max: number | null
  min: number | null
  pop: number | null
}

export function currentOf(w: WeatherView): CurrentWx {
  const c = w.current as Record<string, unknown> | null
  const n = (k: string) => (typeof c?.[k] === 'number' ? (c[k] as number) : null)
  return {
    temp: n('temperature_2m'),
    feels: n('apparent_temperature'),
    code: n('weather_code') ?? 0,
    night: n('is_day') === 0,
    wind: n('wind_speed_10m'),
  }
}

export function hourlyOf(w: WeatherView, count = 12): HourPoint[] {
  const h = columns(w.hourly)
  const times = h['time'] ?? []
  return times.slice(0, count).map((_, i) => ({
    date: str(times, i).slice(0, 10),
    label: str(times, i).slice(11, 16),
    temp: num(h['temperature_2m'], i),
    code: num(h['weather_code'], i) ?? 0,
    pop: num(h['precipitation_probability'], i),
  }))
}

export function dailyOf(w: WeatherView, count = 7): DayPoint[] {
  const d = columns(w.daily)
  const times = d['time'] ?? []
  return times.slice(0, count).map((_, i) => ({
    date: str(times, i).slice(0, 10),
    code: num(d['weather_code'], i) ?? 0,
    max: num(d['temperature_2m_max'], i),
    min: num(d['temperature_2m_min'], i),
    pop: num(d['precipitation_probability_max'], i),
  }))
}

/** Local wall-clock "YYYY-MM-DDTHH" of `now` in `tz`, comparable with Open-Meteo hour strings. */
export function localHourKey(now: Date, tz: string): string {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: tz,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    hourCycle: 'h23',
  }).formatToParts(now)
  const g = (t: string) => parts.find((p) => p.type === t)?.value ?? ''
  return `${g('year')}-${g('month')}-${g('day')}T${g('hour')}`
}

/**
 * The next `max` (default 24) hours starting at the current local hour, crossing midnight.
 * Hours are assumed chronological; fewer are returned when fewer remain.
 */
export function nextHours(points: HourPoint[], nowKey: string, max = 24): HourPoint[] {
  const key = (h: HourPoint) => `${h.date}T${h.label.slice(0, 2)}`
  const start = points.findIndex((h) => key(h) >= nowKey)
  return start < 0 ? [] : points.slice(start, start + max)
}
