import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { LiveEventData, LiveEventKind } from '../composables/useLiveStream'
import { connectSse, type SseHandle, type SseMessage, type SseStatus } from '../kiosk/sse'
import { useCalendarStore } from './calendar'
import { useHouseholdStore } from './household'
import { useMembersStore } from './members'
import { useSourcesStore } from './sources'
import { useWeatherStore } from './weather'

const KINDS: LiveEventKind[] = [
  'calendar.updated',
  'weather.updated',
  'advisory.updated',
  'household.updated',
  'device.updated',
  'source.updated',
  'member.updated',
  'resync',
]

/** Fans one SSE event out to every store; each refetches only what it holds (Frontend.md §5). */
export function dispatchLive(kind: LiveEventKind, data?: LiveEventData): Promise<unknown> {
  return Promise.all([
    useCalendarStore().invalidate(kind, data),
    useWeatherStore().invalidate(kind),
    useHouseholdStore().invalidate(kind),
    useSourcesStore().invalidate(kind),
    useMembersStore().invalidate(kind),
  ])
}

export function parseLive(m: SseMessage): { kind: LiveEventKind; data?: LiveEventData } | null {
  if (!(KINDS as string[]).includes(m.event)) return null
  let data: LiveEventData | undefined
  try {
    data = m.data ? (JSON.parse(m.data) as LiveEventData) : undefined
  } catch {
    data = undefined
  }
  return { kind: m.event as LiveEventKind, data }
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
