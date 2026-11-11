import { LEFT_PANELS, PORTRAIT_NOW, PORTRAIT_STRIP, PORTRAIT_TOP, type Orientation } from './layout'
import type { PanelConfig } from './layout'
import type { DashboardData } from './types'

/** Today's meals in slot order (the slot's time, then the order the server sent). */
export function sortedMeals(data: Pick<DashboardData, 'meals'>) {
  return [...(data.meals ?? [])]
    .map((m, i) => ({ m, i }))
    .sort((a, b) => a.m.time.localeCompare(b.m.time) || a.i - b.i)
    .map((x) => x.m)
}

export interface MealSlotGroup {
  slot_name: string
  meals: NonNullable<DashboardData['meals']>
}

/** Today's meals grouped under their slot title, in slot order (no times shown). */
export function mealsBySlot(data: Pick<DashboardData, 'meals'>): MealSlotGroup[] {
  const out: MealSlotGroup[] = []
  for (const m of sortedMeals(data)) {
    const g = out.find((x) => x.slot_name === m.slot_name)
    if (g) g.meals.push(m)
    else out.push({ slot_name: m.slot_name, meals: [m] })
  }
  return out
}

/**
 * An empty `meals` panel is dropped, unless nothing else would share its column (then it stays and
 * says so). Its column is the right one in landscape and "coming up" in portrait.
 */
export function mealsPanelVisible(
  panel: PanelConfig,
  panels: PanelConfig[],
  data: Pick<DashboardData, 'meals'>,
  o: Orientation,
): boolean {
  if (panel.kind !== 'meals' || (data.meals ?? []).length > 0) return true
  const sameColumn = (k: string) =>
    o === 'portrait'
      ? ![...PORTRAIT_TOP, ...PORTRAIT_STRIP, ...PORTRAIT_NOW].includes(k)
      : !LEFT_PANELS.has(k)
  return !panels.some((p) => p !== panel && p.kind !== 'meals' && sameColumn(p.kind))
}
