import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type BillPatch, type BillView } from '../api/client'
import { isConflict } from '../api/http'

export type BillFilter = '' | 'proposed' | 'open' | 'paid' | 'dismissed'

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
  const filter = ref<BillFilter>('')
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function load(): Promise<void> {
    loading.value = true
    try {
      items.value = await api.bills(filter.value || undefined)
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

  return { items, filter, loading, error, load, patch }
})
