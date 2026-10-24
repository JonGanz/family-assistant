import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type Member } from '../api/client'
import { getDeviceToken } from '../kiosk/deviceToken'
import { connectSse, type SseHandle, type SseMessage, type SseStatus } from '../kiosk/sse'

const CACHE_KEY = 'fa.dashboard.snapshot'

interface Snapshot {
  householdName: string
  members: Member[]
  savedAt: number
}

function readCache(): Snapshot | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY)
    return raw ? (JSON.parse(raw) as Snapshot) : null
  } catch {
    return null
  }
}

function writeCache(s: Snapshot): void {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(s))
  } catch {
    /* storage unavailable */
  }
}

export const useDashboardStore = defineStore('dashboard', () => {
  const householdName = ref('')
  const members = ref<Member[]>([])
  const connection = ref<SseStatus>('connecting')
  const lastHeartbeat = ref<string | null>(null)
  /** True while showing cached data because the API could not be reached. */
  const stale = ref(false)
  let handle: SseHandle | null = null

  async function refresh(): Promise<void> {
    try {
      const [me, list] = await Promise.all([api.me(true), api.members(true)])
      householdName.value = me.household.name
      members.value = list
      stale.value = false
      writeCache({ householdName: me.household.name, members: list, savedAt: Date.now() })
    } catch {
      const c = readCache()
      if (c && !householdName.value) {
        householdName.value = c.householdName
        members.value = c.members
      }
      stale.value = true
    }
  }

  function onMessage(m: SseMessage, onHeartbeat?: () => void): void {
    if (m.event === 'heartbeat') {
      lastHeartbeat.value = m.data
      onHeartbeat?.()
    } else if (m.event === 'member.updated') {
      void refresh()
    }
  }

  async function start(onHeartbeat?: () => void): Promise<void> {
    stop()
    const c = readCache()
    if (c) {
      householdName.value = c.householdName
      members.value = c.members
    }
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
    await refresh()
  }

  function stop(): void {
    handle?.stop()
    handle = null
  }

  return { householdName, members, connection, lastHeartbeat, stale, refresh, start, stop }
})
