import type { AisleView, GroceryItemView, StoreAisleView, StoreView } from '../../../api/food'

export interface AisleGroup {
  /** `null` for the trailing "Other" group. */
  aisleId: string | null
  name: string
  items: GroceryItemView[]
}

export interface StoreGroup {
  /** `null` for "Any store", or for the single group of a household without stores. */
  storeId: string | null
  /** `null` when the household has no stores: no store heading. */
  name: string | null
  aisles: AisleGroup[]
}

const byOrd = <T extends { ord: number }>(a: T, b: T) => a.ord - b.ord

/**
 * The aisles of one store in its own order (D-121): its rows first, then any aisle without a row
 * in the household order (a missing row means "household order, not hidden"). Hidden aisles are
 * returned in `hidden` so their items fall into "Other".
 */
export function aisleOrder(
  aisles: AisleView[],
  rows: StoreAisleView[] | undefined,
): { order: AisleView[]; hidden: Set<string> } {
  const household = [...aisles].sort(byOrd)
  if (!rows?.length) return { order: household, hidden: new Set() }
  const byId = new Map(household.filter((a) => a.id).map((a) => [a.id as string, a]))
  const hidden = new Set(rows.filter((r) => r.hidden).map((r) => r.aisle_id as string))
  const seen = new Set<string>()
  const order: AisleView[] = []
  for (const r of [...rows].sort(byOrd)) {
    const a = byId.get(r.aisle_id as string)
    if (a && !hidden.has(r.aisle_id as string)) {
      order.push(a)
      seen.add(r.aisle_id as string)
    }
  }
  for (const a of household) if (a.id && !seen.has(a.id) && !hidden.has(a.id)) order.push(a)
  return { order, hidden }
}

function aisleGroups(
  items: GroceryItemView[],
  aisles: AisleView[],
  rows: StoreAisleView[] | undefined,
): AisleGroup[] {
  const { order, hidden } = aisleOrder(aisles, rows)
  const out: AisleGroup[] = []
  const placed = new Set<string>()
  for (const a of order) {
    const here = items.filter((i) => i.aisle_id === a.id)
    here.forEach((i) => placed.add(i.id as string))
    if (here.length) out.push({ aisleId: a.id as string, name: a.name, items: here })
  }
  const rest = items.filter(
    (i) => !placed.has(i.id as string) || (i.aisle_id && hidden.has(i.aisle_id as string)),
  )
  if (rest.length) out.push({ aisleId: null, name: 'Other', items: rest })
  return out
}

/**
 * Open items grouped by store (in store order, "Any store" last), then by aisle in that store's
 * order. Without stores there is one group, by household aisle. `onlyStores` keeps the chosen
 * stores plus "Any store" items; empty means all.
 */
export function groupItems(
  items: GroceryItemView[],
  stores: StoreView[],
  aisles: AisleView[],
  storeAisles: Record<string, StoreAisleView[]>,
  onlyStores: string[] = [],
): StoreGroup[] {
  if (!stores.length) {
    const g = aisleGroups(items, aisles, undefined)
    return g.length ? [{ storeId: null, name: null, aisles: g }] : []
  }
  const known = new Set(stores.map((s) => s.id as string))
  const keep = (s: string | null) => !onlyStores.length || s === null || onlyStores.includes(s)
  const out: StoreGroup[] = []
  for (const s of [...stores].sort(byOrd)) {
    if (!keep(s.id as string)) continue
    const here = items.filter((i) => i.store_id === s.id)
    const g = aisleGroups(here, aisles, storeAisles[s.id as string])
    if (g.length) out.push({ storeId: s.id as string, name: s.name, aisles: g })
  }
  const anyItems = items.filter((i) => !i.store_id || !known.has(i.store_id as string))
  const g = aisleGroups(anyItems, aisles, undefined)
  if (g.length) out.push({ storeId: null, name: 'Any store', aisles: g })
  return out
}

/** "for Tue dinner, Wed lunch" from the sources; `YYYY-MM-DD` dates are read as calendar days. */
export function sourceHint(sources: { date: string; slot_name: string }[], max = 2): string {
  if (!sources.length) return ''
  const part = (s: { date: string; slot_name: string }) => {
    const [y, m, d] = s.date.split('-').map(Number)
    const day = new Date(y!, (m ?? 1) - 1, d ?? 1).toLocaleDateString(undefined, {
      weekday: 'short',
    })
    return `${day} ${s.slot_name.toLowerCase()}`
  }
  const shown = sources.slice(0, max).map(part).join(', ')
  return `for ${shown}${sources.length > max ? ` +${sources.length - max}` : ''}`
}
