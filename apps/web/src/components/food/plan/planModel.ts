import { addDays } from '../../../lib/time'
import type { MealPlanEntryView, MealSlotView } from '../../../api/food'

/** A visible slot with its id, as the grid and the sheets use it. */
export interface PlanSlot {
  id: string
  name: string
  time?: string
}

/** Slots from the settings store (ids are present once saved) in slot order. */
export function planSlots(slots: Pick<MealSlotView, 'id' | 'name' | 'time'>[]): PlanSlot[] {
  return slots.flatMap((s) => (s.id ? [{ id: s.id, name: s.name, time: s.time }] : []))
}

/** `weeks` rows of seven `YYYY-MM-DD` days; the first row starts on `from`. */
export function weekRows(from: string, weeks: number): string[][] {
  return Array.from({ length: Math.max(0, weeks) }, (_, w) =>
    Array.from({ length: 7 }, (_, d) => addDays(from, w * 7 + d)),
  )
}

/** The `data-testid` of a (date, slot) cell: `plan-cell-<YYYY-MM-DD>-<slot name lower>`. */
export const cellTestId = (date: string, slotName: string) =>
  `plan-cell-${date}-${slotName.toLowerCase()}`

const noon = (key: string) => new Date(`${key}T12:00:00Z`)
const fmt = (key: string, o: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(undefined, { timeZone: 'UTC', ...o }).format(noon(key))

/** "Mon" / "Mon 9" / "Mon Nov 9" for a day header. */
export function dayLabel(key: string, withMonth: boolean): string {
  const dow = fmt(key, { weekday: 'short' })
  const num = fmt(key, { day: 'numeric' })
  return withMonth ? `${dow} ${fmt(key, { month: 'short' })} ${num}` : `${dow} ${num}`
}

export const entryTitle = (e: Pick<MealPlanEntryView, 'title' | 'free_text'>) =>
  e.title || e.free_text || ''

/** What a meal or template item holds while it is being edited: a recipe or free text, plus servings. */
export interface ItemPick {
  kind: 'recipe' | 'text'
  recipe_id: string | null
  recipe_title: string
  text: string
  /** The input's text; empty means "the recipe's default". */
  servings: string
}

export const emptyPick = (): ItemPick => ({
  kind: 'recipe',
  recipe_id: null,
  recipe_title: '',
  text: '',
  servings: '',
})

export function pickFrom(c: {
  recipe_id?: string | null
  free_text?: string | null
  title?: string
  servings?: number | null
}): ItemPick {
  return c.recipe_id
    ? {
        kind: 'recipe',
        recipe_id: c.recipe_id,
        recipe_title: c.title ?? '',
        text: '',
        servings: c.servings ? String(c.servings) : '',
      }
    : {
        kind: 'text',
        recipe_id: null,
        recipe_title: '',
        text: c.free_text ?? '',
        servings: c.servings ? String(c.servings) : '',
      }
}

export const pickValid = (p: ItemPick) =>
  p.kind === 'recipe' ? !!p.recipe_id : p.text.trim().length > 0

/** The wire fields of a pick: exactly one of `recipe_id` and `free_text`, and `servings` when set. */
export function pickPayload(p: ItemPick): {
  recipe_id?: string
  free_text?: string
  servings?: number
} {
  const n = Number.parseInt(p.servings, 10)
  const servings = Number.isFinite(n) && n > 0 ? n : undefined
  return p.kind === 'recipe'
    ? { recipe_id: p.recipe_id ?? undefined, servings }
    : { free_text: p.text.trim(), servings }
}
