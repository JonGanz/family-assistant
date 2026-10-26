import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  api,
  type BillPatch,
  type ContactApproval,
  type EventApproval,
  type InboxCounts,
  type InboxItem,
  type InboxKind,
  type TaskApproval,
} from '../api/client'

export const emptyCounts = (): InboxCounts => ({ event: 0, bill: 0, task: 0, contact: 0, total: 0 })

export function itemId(i: InboxItem): string {
  switch (i.kind) {
    case 'event':
      return i.event.id
    case 'bill':
      return i.bill.id
    case 'task':
      return i.task.id
    case 'contact':
      return i.contact.id
  }
}

export type Approval = EventApproval | BillPatch | TaskApproval | ContactApproval

export const useInboxStore = defineStore('inbox', () => {
  const items = ref<InboxItem[]>([])
  const counts = ref<InboxCounts>(emptyCounts())
  const loaded = ref(false)
  const error = ref<string | null>(null)
  const total = computed(() => counts.value.total)

  async function load(): Promise<void> {
    try {
      const v = await api.inbox()
      items.value = v.items
      counts.value = v.counts
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  /** Counts only change on the server's say-so; a refetch is cheap. Never loaded (children) → no-op. */
  async function invalidate(kind: string): Promise<void> {
    if (loaded.value && (kind === 'inbox.updated' || kind === 'resync')) await load()
  }

  /** Removes the item at once; on failure puts it back and returns false. */
  async function decide(kind: InboxKind, id: string, approval: Approval | null): Promise<boolean> {
    const prevItems = items.value
    const prevCounts = counts.value
    items.value = prevItems.filter((i) => !(i.kind === kind && itemId(i) === id))
    counts.value = {
      ...prevCounts,
      [kind]: Math.max(0, prevCounts[kind] - 1),
      total: Math.max(0, prevCounts.total - 1),
    }
    try {
      if (approval) await api.approveInbox(kind, id, approval)
      else await api.rejectInbox(kind, id)
      error.value = null
      return true
    } catch (e) {
      items.value = prevItems
      counts.value = prevCounts
      error.value = e instanceof Error ? e.message : String(e)
      return false
    }
  }

  const approve = (kind: InboxKind, id: string, edits: Approval = {}) => decide(kind, id, edits)
  const reject = (kind: InboxKind, id: string) => decide(kind, id, null)

  return {
    items,
    counts,
    total,
    loaded,
    error,
    load,
    invalidate,
    approve,
    reject,
  }
})
