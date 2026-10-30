import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api, type DashboardView } from '../api/client'
import type { DashboardData, TaskView } from '../dashboard/types'
import { ApiError } from '../api/http'
import { taskApi } from '../kiosk/taskApi'
import { getDeviceToken } from '../kiosk/deviceToken'
import { connectSse, type SseHandle, type SseMessage, type SseStatus } from '../kiosk/sse'
import { addDays, dayKey, msUntilNextMidnight, startOfDay } from '../lib/time'

export const CACHE_KEY = 'fa.dashboard.snapshot'
const CACHE_VERSION = 3

interface CacheEntry {
  v: number
  snapshot: DashboardData
  savedAt: number
}

export function readCache(): CacheEntry | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY)
    if (!raw) return null
    const c = JSON.parse(raw) as Partial<CacheEntry>
    return c.v === CACHE_VERSION && c.snapshot && typeof c.savedAt === 'number'
      ? (c as CacheEntry)
      : null
  } catch {
    return null
  }
}

export function writeCache(snapshot: DashboardData, savedAt: number): void {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ v: CACHE_VERSION, snapshot, savedAt }))
  } catch {
    /* storage unavailable */
  }
}

/** Days to fetch: the agenda window, or a full week for the Week view (API caps 1..14). */
export function daysToFetch(settings: DashboardData['settings'] | undefined): number {
  const agenda = settings?.agenda_days ?? 3
  return Math.min(14, Math.max(1, agenda, 7))
}

/** Retry delay while the first load keeps failing. */
export const RETRY_MS = 10_000

export const useDashboardStore = defineStore('dashboard', () => {
  const snapshot = ref<DashboardData | null>(null)
  const savedAt = ref<number | null>(null)
  const deviceId = ref<string | null>(null)
  const connection = ref<SseStatus>('connecting')
  const lastHeartbeat = ref<string | null>(null)
  /** True while showing cached data because the API could not be reached. */
  const stale = ref(false)
  /** Why the last fetch failed (shown while there is nothing to render), or null. */
  const loadError = ref<string | null>(null)
  /** Epoch ms until which the device is elevated (PIN step-up), or null. */
  const elevatedUntil = ref<number | null>(null)
  /** The week the Week view shows (its first day), and that week's events and tasks. */
  const weekFrom = ref<string | null>(null)
  const weekData = ref<{ from: string; events: DashboardData['events']; tasks: TaskView[] } | null>(
    null,
  )
  let handle: SseHandle | null = null
  let midnight: ReturnType<typeof setTimeout> | undefined
  let inflight: Promise<void> | null = null
  let retry: ReturnType<typeof setTimeout> | undefined

  const householdName = computed(() => snapshot.value?.household.name ?? '')
  const members = computed(() => snapshot.value?.members ?? [])
  const tz = computed(() => snapshot.value?.household.tz ?? 'UTC')
  /** Shown while the stream is not open or the last fetch failed. */
  const showStaleBadge = computed(
    () => snapshot.value !== null && (stale.value || connection.value !== 'open'),
  )

  /** The instant range the held snapshot covers: [start of today, start of today + N days). */
  function heldWindow(): { from: number; to: number } | null {
    const s = snapshot.value
    if (!s) return null
    const z = s.household.tz
    const today = dayKey(new Date(), z)
    const days = daysToFetch(s.settings)
    return {
      from: startOfDay(z, today).getTime(),
      to: startOfDay(z, addDays(today, days)).getTime(),
    }
  }

  function applyCache(): void {
    const c = readCache()
    if (c && !snapshot.value) {
      snapshot.value = c.snapshot
      savedAt.value = c.savedAt
    }
  }

  /** Fields the UI derives from the snapshot (D-013 override, Now list size, addable lists). */
  function derived(data: DashboardView): Partial<DashboardData> {
    return {
      task_lists: data.task_lists ?? [],
      now_list_size: data.household.now_list_size,
      completion_enabled: data.household.completion_override,
      epoch: snapshot.value?.epoch,
    }
  }

  function patch(p: Partial<DashboardData>): void {
    if (snapshot.value) snapshot.value = { ...snapshot.value, ...p }
  }

  /** Re-render task checkboxes from the data (after a cancelled or refused completion). */
  function resetChecks(): void {
    patch({ epoch: (snapshot.value?.epoch ?? 0) + 1 })
  }

  /** Complete a task for `member`. A 403 means the household turned dashboard completion off. */
  async function complete(id: string, member: string): Promise<boolean> {
    try {
      await taskApi.complete(id, member)
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) patch({ completion_enabled: false })
      resetChecks()
      return false
    }
    await refresh()
    return true
  }

  async function reopen(id: string, member: string): Promise<void> {
    try {
      await taskApi.reopen(id, member)
    } finally {
      await refresh()
    }
  }

  /** Tick or untick a routine step as the run's member; optimistic, then reconciled. */
  async function tickStep(run: string, step: string, done: boolean): Promise<void> {
    const s = snapshot.value
    const r = s?.routines.find((x) => x.id === run)
    if (!s || !r) return
    const stamp = done ? new Date().toISOString() : null
    patch({
      routines: s.routines.map((x) =>
        x.id === run
          ? {
              ...x,
              steps: x.steps.map((st) => (st.step === step ? { ...st, done_at: stamp } : st)),
              done: x.steps.filter((st) => (st.step === step ? done : st.done_at)).length,
            }
          : x,
      ),
    })
    try {
      await taskApi.tickStep(run, step, done, r.member)
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) patch({ completion_enabled: false })
    } finally {
      await refresh()
    }
  }

  async function addTask(b: Parameters<typeof taskApi.create>[0]): Promise<boolean> {
    try {
      await taskApi.create(b)
    } catch {
      return false
    }
    await refresh()
    return true
  }

  /** Fetches the events and tasks of the week being viewed (the snapshot only starts today). */
  async function loadWeek(): Promise<void> {
    const from = weekFrom.value
    if (!from) return
    try {
      const d = await api.dashboard(7, undefined, from)
      if (weekFrom.value === from) {
        weekData.value = { from, events: d.events, tasks: d.tasks ?? [] }
      }
    } catch {
      /* keep the previous week; the next refresh retries */
    }
  }

  /** The Week view asks for a week (its first day), or null when it is not showing. */
  function showWeek(from: string | null): Promise<void> {
    weekFrom.value = from
    if (!from) weekData.value = null
    return loadWeek()
  }

  async function doRefresh(): Promise<void> {
    try {
      const days = daysToFetch(snapshot.value?.settings)
      let data = await api.dashboard(days)
      const wanted = daysToFetch(data.settings)
      if (wanted !== days) data = await api.dashboard(wanted)
      snapshot.value = {
        ...data,
        ...derived(data),
        tasks: data.tasks ?? [],
        routines: data.routines ?? [],
      }
      savedAt.value = Date.now()
      stale.value = false
      loadError.value = null
      writeCache(snapshot.value, savedAt.value)
      void loadWeek()
    } catch (e) {
      applyCache()
      stale.value = true
      loadError.value =
        e instanceof ApiError
          ? `HTTP ${e.status}: ${e.message}`
          : String((e as Error)?.message ?? e)
      // Nothing to show yet: keep trying instead of waiting for an SSE event that may never come.
      if (!snapshot.value) {
        clearTimeout(retry)
        retry = setTimeout(() => void refresh(), RETRY_MS)
      }
    }
  }

  /** Coalesces concurrent refetches (an SSE burst triggers one request, not many). */
  function refresh(): Promise<void> {
    if (!inflight) inflight = doRefresh().finally(() => (inflight = null))
    return inflight
  }

  async function loadDeviceId(): Promise<void> {
    try {
      const me = await api.me(true)
      deviceId.value = me.actor.device?.id ?? null
    } catch {
      /* offline: device.updated falls back to refetching for any id */
    }
  }

  /** Apply one SSE message by kind (Frontend.md §4.2). Returns whether it refetched. */
  function handleEvent(kind: string, payload: unknown): boolean {
    const p = (payload ?? {}) as { from?: string; to?: string; id?: string }
    switch (kind) {
      case 'calendar.updated': {
        const w = heldWindow()
        const from = p.from ? Date.parse(p.from) : NaN
        const to = p.to ? Date.parse(p.to) : NaN
        if (
          weekFrom.value === null &&
          w &&
          !Number.isNaN(from) &&
          !Number.isNaN(to) &&
          !(from < w.to && w.from < to)
        ) {
          return false
        }
        break
      }
      case 'device.updated':
        if (p.id && deviceId.value && p.id !== deviceId.value) return false
        break
      case 'tasks.updated':
      case 'routines.updated':
      case 'weather.updated':
      case 'advisory.updated':
      case 'member.updated':
      case 'household.updated':
      case 'inbox.updated':
      case 'resync':
        break
      default:
        return false
    }
    void refresh()
    return true
  }

  function onMessage(m: SseMessage, onHeartbeat?: () => void): void {
    if (m.event === 'heartbeat') {
      lastHeartbeat.value = m.data
      onHeartbeat?.()
      return
    }
    let payload: unknown
    try {
      payload = m.data ? JSON.parse(m.data) : undefined
    } catch {
      payload = undefined
    }
    handleEvent(m.event, payload)
  }

  function scheduleMidnight(): void {
    clearTimeout(midnight)
    const ms = msUntilNextMidnight(tz.value) + 1000
    midnight = setTimeout(() => {
      void refresh()
      scheduleMidnight()
    }, ms)
  }

  async function start(onHeartbeat?: () => void): Promise<void> {
    stop()
    applyCache()
    handle = connectSse({
      url: '/api/v1/stream',
      headers: () => {
        const t = getDeviceToken()
        const h: Record<string, string> = {}
        if (t) h['Authorization'] = `Bearer ${t}`
        return h
      },
      onStatus: (s) => {
        connection.value = s
      },
      onMessage: (m) => onMessage(m, onHeartbeat),
    })
    await Promise.all([refresh(), loadDeviceId()])
    scheduleMidnight()
  }

  function stop(): void {
    handle?.stop()
    handle = null
    clearTimeout(midnight)
    clearTimeout(retry)
  }

  return {
    snapshot,
    savedAt,
    deviceId,
    householdName,
    members,
    connection,
    lastHeartbeat,
    elevatedUntil,
    stale,
    loadError,
    showStaleBadge,
    weekData,
    showWeek,
    refresh,
    handleEvent,
    complete,
    reopen,
    tickStep,
    addTask,
    resetChecks,
    start,
    stop,
  }
})
