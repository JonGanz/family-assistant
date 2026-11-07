import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type BillPatch, type BillView } from '../api/client'
import { isConflict } from '../api/http'

export type BillFilter = 'proposed' | 'open' | 'paid' | 'dismissed'

/** Money from minor units with the bill's own currency. */
export function formatMoney(amountMinor: number, currency: string, locale?: string): string {
  let digits: number
  try {
    digits =
      new Intl.NumberFormat(locale, { style: 'currency', currency }).resolvedOptions()
        .maximumFractionDigits ?? 2
  } catch {
    return `${(amountMinor / 100).toFixed(2)} ${currency}`
  }
  return new Intl.NumberFormat(locale, { style: 'currency', currency }).format(
    amountMinor / 10 ** digits,
  )
}

export const useBillsStore = defineStore('bills', () => {
  const items = ref<BillView[]>([])
  /** The chosen states (D-105); empty means All. One is sent to the server, several filter in the view. */
  const filter = ref<BillFilter[]>([])
  const statusParam = () => (filter.value.length === 1 ? filter.value[0] : undefined)
  const loading = ref(false)
  const error = ref<string | null>(null)
  /** Cursor of the next page; null when everything is loaded. */
  const nextBefore = ref<string | null>(null)

  /** The server pages newest-created first; the list shows the soonest due first (undated last). */
  function byDue(a: BillView, b: BillView): number {
    if (a.due_on && b.due_on) return a.due_on < b.due_on ? -1 : a.due_on > b.due_on ? 1 : 0
    if (a.due_on) return -1
    if (b.due_on) return 1
    return 0
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      const shown = items.value.length
      let page = await api.bills(statusParam())
      const all = [...page.items]
      while (page.next_before && all.length < shown) {
        page = await api.bills(statusParam(), page.next_before)
        all.push(...page.items)
      }
      items.value = all.sort(byDue)
      nextBefore.value = page.next_before ?? null
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function loadMore(): Promise<void> {
    if (!nextBefore.value) return
    loading.value = true
    try {
      const page = await api.bills(statusParam(), nextBefore.value)
      items.value = [...items.value, ...page.items].sort(byDue)
      nextBefore.value = page.next_before ?? null
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  /** Returns false on a stale version (the list is reloaded). */
  async function patch(b: BillView, p: BillPatch): Promise<boolean> {
    try {
      await api.patchBill(b.id, b.version, p)
      await load()
      return true
    } catch (e) {
      if (isConflict(e)) {
        await load()
        return false
      }
      throw e
    }
  }

  return { items, filter, loading, error, nextBefore, load, loadMore, patch }
})
