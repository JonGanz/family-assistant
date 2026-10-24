import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '../api/client'
import type { DashboardData } from '../dashboard/types'
import { getDeviceToken } from '../kiosk/deviceToken'
import { connectSse, type SseHandle, type SseMessage, type SseStatus } from '../kiosk/sse'
import { addDays, dayKey, msUntilNextMidnight, startOfDay } from '../lib/time'

export const CACHE_KEY = 'fa.dashboard.snapshot'
const CACHE_VERSION = 2

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

/** Days to fetch: the agenda window, or a full week when a week panel needs it (API caps 1..14). */
export function daysToFetch(settings: DashboardData['settings'] | undefined): number {
  const agenda = settings?.agenda_days ?? 3
  const needsWeek = settings?.panels.some((p) => p.kind === 'week') ?? false
  return Math.min(14, Math.max(1, agenda, needsWeek ? 7 : 1))
}

export const useDashboardStore = defineStore('dashboard', () => {
  const snapshot = ref<DashboardData | null>(null)
  const savedAt = ref<number | null>(null)
  const deviceId = ref<string | null>(null)
  const connection = ref<SseStatus>('connecting')
  const lastHeartbeat = ref<string | null>(null)
  /** True while showing cached data because the API could not be reached. */
  const stale = ref(false)
  let handle: SseHandle | null = null
  let midnight: ReturnType<typeof setTimeout> | undefined
  let inflight: Promise<void> | null = null

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

  async function doRefresh(): Promise<void> {
    try {
      const days = daysToFetch(snapshot.value?.settings)
      let data = await api.dashboard(days)
      const wanted = daysToFetch(data.settings)
      if (wanted !== days) data = await api.dashboard(wanted)
      snapshot.value = data
      savedAt.value = Date.now()
      stale.value = false
      writeCache(data, savedAt.value)
    } catch {
      applyCache()
      stale.value = true
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
        if (w && !Number.isNaN(from) && !Number.isNaN(to) && !(from < w.to && w.from < to)) {
          return false
        }
        break
      }
      case 'device.updated':
        if (p.id && deviceId.value && p.id !== deviceId.value) return false
        break
      case 'weather.updated':
      case 'advisory.updated':
      case 'member.updated':
      case 'household.updated':
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

  async function dismiss(id: string): Promise<void> {
    if (snapshot.value) {
      snapshot.value = {
        ...snapshot.value,
        advisories: snapshot.value.advisories.filter((a) => a.id !== id),
      }
    }
    try {
      await api.dismissAdvisory(id, true)
    } finally {
      await refresh()
    }
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
  }

  return {
    snapshot,
    savedAt,
    deviceId,
    householdName,
    members,
    connection,
    lastHeartbeat,
    stale,
    showStaleBadge,
    refresh,
    handleEvent,
    dismiss,
    start,
    stop,
  }
})
