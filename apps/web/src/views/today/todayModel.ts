import type { TaskView } from '../../api/client'
import type { MealPlanEntryView } from '../../api/food'
import { dayKey } from '../../lib/time'
import { columns, num, row } from '../../stores/weather'

export interface SlotRef {
  id: string
  name: string
}
export interface MealItem {
  id: string
  title: string
  recipeId: string | null
}
export interface MealSlotCard {
  slot: string
  items: MealItem[]
}

/** The day's plan entries grouped per visible slot (slot order); slots with nothing are left out. */
export function mealCards(
  entries: MealPlanEntryView[],
  date: string,
  slots: SlotRef[],
): MealSlotCard[] {
  const out: MealSlotCard[] = []
  for (const s of slots) {
    const items = entries
      .filter((e) => e.date === date && e.slot_id === s.id)
      .sort((a, b) => a.ord - b.ord)
      .map((e) => ({ id: e.id, title: e.title, recipeId: e.recipe_id ?? null }))
    if (items.length) out.push({ slot: s.name, items })
  }
  return out
}

/** Open tasks due on the calendar day `day` in `tz`. */
export function tasksDueOn(tasks: TaskView[], day: string, tz: string): TaskView[] {
  return tasks.filter((t) => t.status === 'open' && !!t.due_at && dayKey(t.due_at, tz) === day)
}

export interface HourRow {
  time: string
  code: number
  temp: number | undefined
  rain: number | undefined
}
/** Hourly rows of one local day (Open-Meteo returns household-local `YYYY-MM-DDTHH:MM`). */
export function hoursOf(hourly: unknown, day: string): HourRow[] {
  const h = columns(hourly)
  const out: HourRow[] = []
  ;(h.time ?? []).forEach((t, i) => {
    if (!t.startsWith(day)) return
    const r = row(h, i)
    out.push({
      time: t.slice(11, 16),
      code: Number(r.weather_code ?? 0),
      temp: num(r.temperature_2m),
      rain: num(r.precipitation_probability),
    })
  })
  return out
}

export interface DayRow {
  date: string
  code: number
  hi: number | undefined
  lo: number | undefined
  rain: number | undefined
}
export function dailyRows(daily: unknown): DayRow[] {
  const d = columns(daily)
  return (d.time ?? []).map((t, i) => {
    const r = row(d, i)
    return {
      date: t.slice(0, 10),
      code: Number(r.weather_code ?? 0),
      hi: num(r.temperature_2m_max),
      lo: num(r.temperature_2m_min),
      rain: num(r.precipitation_probability_max),
    }
  })
}
