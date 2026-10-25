import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { LiveEventData, LiveEventKind } from '../composables/useLiveStream'
import { connectSse, type SseHandle, type SseMessage, type SseStatus } from '../kiosk/sse'
import { useAwayStore } from './away'
import { useCalendarStore } from './calendar'
import { useDefinitionsStore } from './definitions'
import { useHouseholdStore } from './household'
import { useListsStore } from './lists'
import { useMembersStore } from './members'
import { useNotifyPrefsStore } from './notifyPrefs'
import { useRoutinesStore } from './routines'
import { useSourcesStore } from './sources'
import { useTasksStore } from './tasks'
import { useWeatherStore } from './weather'

/** The M2 kinds are added here (the shared `LiveEventKind` union is not part of this app's paths). */
export type AppLiveKind = LiveEventKind | 'tasks.updated' | 'routines.updated'

const KINDS: AppLiveKind[] = [
  'calendar.updated',
  'weather.updated',
  'advisory.updated',
  'household.updated',
  'device.updated',
  'source.updated',
  'member.updated',
  'tasks.updated',
  'routines.updated',
  'resync',
]

/** Fans one SSE event out to every store; each refetches only what it holds (Frontend.md §5). */
export function dispatchLive(kind: AppLiveKind, data?: LiveEventData): Promise<unknown> {
  // The M0/M1 stores ignore the M2 kinds, so narrowing for them is safe.
  const base = kind as LiveEventKind
  return Promise.all([
    useCalendarStore().invalidate(base, data),
    useWeatherStore().invalidate(base),
    useHouseholdStore().invalidate(base),
    useSourcesStore().invalidate(base),
    useMembersStore().invalidate(base),
    useTasksStore().invalidate(kind),
    useListsStore().invalidate(kind),
    useDefinitionsStore().invalidate(kind),
    useRoutinesStore().invalidate(kind),
    useAwayStore().invalidate(kind),
    useNotifyPrefsStore().invalidate(kind),
  ])
}

export function parseLive(m: SseMessage): { kind: AppLiveKind; data?: LiveEventData } | null {
  if (!(KINDS as string[]).includes(m.event)) return null
  let data: LiveEventData | undefined
  try {
    data = m.data ? (JSON.parse(m.data) as LiveEventData) : undefined
  } catch {
    data = undefined
  }
  return { kind: m.event as AppLiveKind, data }
}

/** The single app-level subscription (cookie session). Start after login, stop on logout. */
export const useLiveStore = defineStore('live', () => {
  const status = ref<SseStatus>('closed')
  let handle: SseHandle | null = null
  let wasOpen = false

  function start(): void {
    if (handle) return
    handle = connectSse({
      url: '/api/v1/stream',
      onStatus: (s) => {
        // Events may have been missed while we were away, so catch up after a reconnect.
        if (s === 'open' && wasOpen) void dispatchLive('resync')
        if (s === 'open') wasOpen = true
        status.value = s
      },
      onMessage: (m) => {
        const ev = parseLive(m)
        if (ev) void dispatchLive(ev.kind, ev.data)
      },
    })
  }

  function stop(): void {
    handle?.stop()
    handle = null
    wasOpen = false
    status.value = 'closed'
  }

  return { status, start, stop }
})
