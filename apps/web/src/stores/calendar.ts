import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api, type EventView } from '../api/client'
import { invalidates, type LiveEventData, type LiveEventKind } from '../composables/useLiveStream'
import { startOfDay } from '../lib/time'

interface Held {
  key: string
  /** RFC 3339 instants; `to` exclusive. */
  from: string
  to: string
  member?: string
  ids: string[]
}

const keyOf = (from: string, to: string, member?: string) => `${from}|${to}|${member ?? ''}`

export const useCalendarStore = defineStore('calendar', () => {
  const byId = ref<Record<string, EventView>>({})
  const held = ref<Held[]>([])
  const error = ref<string | null>(null)
  const pending = ref(0)
  const loading = computed(() => pending.value > 0)
  const tzRef = ref('UTC')

  async function fetchHeld(h: Held): Promise<void> {
    pending.value++
    try {
      const list = await api.events(h.from, h.to, h.member)
      for (const id of h.ids) delete byId.value[id]
      for (const ev of list) byId.value[ev.id] = ev
      h.ids = list.map((e) => e.id)
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      pending.value--
    }
  }

  /** Loads (once) the events of day keys `[fromDay, toDay)` in the household tz. */
  async function ensureRange(
    fromDay: string,
    toDay: string,
    tz: string,
    member?: string,
  ): Promise<void> {
    tzRef.value = tz
    const from = startOfDay(tz, fromDay).toISOString()
    const to = startOfDay(tz, toDay).toISOString()
    const key = keyOf(from, to, member)
    if (held.value.some((x) => x.key === key)) return
    held.value.push({ key, from, to, member, ids: [] })
    // Use the reactive proxy from the array, not the raw object, so `ids` updates re-render.
    await fetchHeld(held.value[held.value.length - 1]!)
  }

  /** The events of a loaded range, sorted by start. Empty before it is loaded. */
  function eventsIn(fromDay: string, toDay: string, tz: string, member?: string): EventView[] {
    const key = keyOf(
      startOfDay(tz, fromDay).toISOString(),
      startOfDay(tz, toDay).toISOString(),
      member,
    )
    const h = held.value.find((x) => x.key === key)
    if (!h) return []
    return h.ids
      .map((id) => byId.value[id])
      .filter((e): e is EventView => !!e)
      .sort((a, b) => a.start_at.localeCompare(b.start_at))
  }

  /** Refetches only the held ranges that overlap the change (`resync` refetches all). */
  async function invalidate(kind: LiveEventKind, data?: LiveEventData): Promise<void> {
    const hit = held.value.filter((h) =>
      invalidates(kind, data, { kind: 'calendar', from: h.from, to: h.to }),
    )
    await Promise.all(hit.map(fetchHeld))
  }

  function reset(): void {
    byId.value = {}
    held.value = []
  }

  return { byId, held, error, loading, ensureRange, eventsIn, invalidate, reset }
})
