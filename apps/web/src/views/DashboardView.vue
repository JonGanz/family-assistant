<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from '../api/client'
import { ApiError } from '../api/http'
import PinPad from '../components/PinPad.vue'
import { useClock } from '../composables/useClock'
import DashboardCanvas from '../dashboard/DashboardCanvas.vue'
import Screensaver from '../dashboard/Screensaver.vue'
import TaskFlow from '../dashboard/TaskFlow.vue'
import { S } from '../dashboard/strings'
import { getDeviceToken } from '../kiosk/deviceToken'
import { startKeepAlive } from '../kiosk/keepAlive'
import { startWatchdog, type Watchdog } from '../kiosk/watchdog'
import { useDashboardStore } from '../stores/dashboard'

const store = useDashboardStore()
const hasToken = ref(!!getDeviceToken())
const now = useClock()
const flow = ref<InstanceType<typeof TaskFlow> | null>(null)

/** The server sends a heartbeat every 15 s: an open stream silent for three of them is stalled. */
const STALLED_AFTER_MS = 45_000
const link = computed(() =>
  store.connection === 'open' &&
  store.lastSignalAt !== null &&
  now.value.getTime() - store.lastSignalAt > STALLED_AFTER_MS
    ? 'stalled'
    : store.connection,
)
const connLabel = computed(
  () =>
    ({
      open: 'Connected',
      stalled: 'No data',
      connecting: 'Connecting',
      reconnecting: 'Reconnecting',
      closed: 'Offline',
    })[link.value],
)
const updatedAt = computed(() =>
  store.savedAt
    ? new Intl.DateTimeFormat(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        timeZone: store.snapshot?.household.tz,
      }).format(store.savedAt)
    : '',
)

let stopKeepAlive: (() => void) | undefined
let watchdog: Watchdog | undefined

onMounted(() => {
  if (!hasToken.value) return
  stopKeepAlive = startKeepAlive()
  watchdog = startWatchdog()
  void store.start(() => watchdog?.heartbeat())
})
onUnmounted(() => {
  clearInterval(remainingTimer)
  stopKeepAlive?.()
  watchdog?.stop()
  store.stop()
})

// Elevation (D-019): pick an adult, enter the PIN, show the remaining time.
const showElevation = ref(false)
const pickedId = ref<string | null>(null)
const elevMsg = ref('')
const expiresAt = ref<number | null>(null)
const remaining = ref(0)
let remainingTimer: ReturnType<typeof setInterval> | undefined

const adults = computed(() => store.members.filter((m) => m.role === 'adult'))

function pick(id: string) {
  const m = store.members.find((x) => x.id === id)
  if (m?.role !== 'adult') return
  pickedId.value = id
  showElevation.value = true
}

async function submitPin(pin: string) {
  if (!pickedId.value) return
  elevMsg.value = ''
  try {
    const r = await api.elevate(pickedId.value, pin)
    expiresAt.value = Date.parse(r.expires_at)
    store.elevatedUntil = expiresAt.value
    clearInterval(remainingTimer)
    const tick = () => {
      remaining.value = Math.max(0, Math.round(((expiresAt.value ?? 0) - Date.now()) / 1000))
      if (remaining.value === 0) {
        clearInterval(remainingTimer)
        store.elevatedUntil = null
      }
    }
    tick()
    remainingTimer = setInterval(tick, 1000)
  } catch (e) {
    expiresAt.value = null
    store.elevatedUntil = null
    if (e instanceof ApiError && e.status === 423) elevMsg.value = S.locked
    else if (e instanceof ApiError && e.status === 401) elevMsg.value = S.wrongPin
    else if (e instanceof ApiError && e.status === 403) elevMsg.value = S.pinOffOnLan
    else elevMsg.value = S.elevationFailed
  }
}

const mmss = computed(
  () => `${Math.floor(remaining.value / 60)}:${String(remaining.value % 60).padStart(2, '0')}`,
)
</script>

<template>
  <section v-if="!hasToken" class="pair kiosk">
    <h1>{{ S.pairTitle }}</h1>
    <p>{{ S.pairBody }}</p>
  </section>

  <section v-else class="dashboard kiosk">
    <DashboardCanvas
      v-if="store.snapshot"
      :data="store.snapshot"
      :now="now"
      burn-in
      page
      :week-data="store.weekData ?? undefined"
      @week="(from: string | null) => store.showWeek(from)"
      @pick="pick"
      @complete="(id: string) => flow?.beginComplete(id)"
      @tick="(r: string, st: string, d: boolean) => store.tickStep(r, st, d)"
      @add="flow?.beginAdd()"
    >
      <template #nav-end>
        <span class="conn" :class="link" data-testid="connection" :data-status="link">
          ● {{ connLabel }}
        </span>
        <span v-if="store.showStaleBadge && updatedAt" class="stale" data-testid="stale">
          {{ S.lastUpdated(updatedAt) }}
        </span>
        <button class="try" @click="showElevation = !showElevation">{{ S.elevate }}</button>
      </template>
    </DashboardCanvas>
    <div v-else class="loading">
      <p>{{ S.loading }}</p>
      <p v-if="store.loadError" role="alert" data-testid="load-error">
        {{ S.loadFailed(store.loadError) }}
      </p>
    </div>
    <TaskFlow ref="flow" />
    <Screensaver :minutes="store.snapshot?.settings.screensaver_minutes ?? 15" />

    <div v-if="showElevation" class="elevation" role="dialog" aria-label="Elevation">
      <div v-if="expiresAt && remaining > 0" data-testid="elevated">{{ S.elevated(mmss) }}</div>
      <template v-else>
        <div class="pick">
          <button
            v-for="m in adults"
            :key="m.id"
            :class="{ on: pickedId === m.id }"
            @click="pickedId = m.id"
          >
            {{ m.display_name }}
          </button>
        </div>
        <PinPad v-if="pickedId" @submit="submitPin" />
        <p v-if="elevMsg" role="alert">{{ elevMsg }}</p>
      </template>
      <button class="close" @click="showElevation = false">✕</button>
    </div>
  </section>
</template>

<style scoped>
.pair {
  padding: 3rem;
}
.pair h1 {
  font-size: var(--kiosk-font-title);
  margin: 0;
}
.dashboard {
  position: relative;
  width: 100%;
  height: 100%;
}
.loading {
  padding: 3rem;
  color: var(--color-muted);
}
.elevation {
  position: absolute;
  inset: auto 2rem 4rem auto;
  z-index: 5;
  padding: var(--space-4);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: 0 8px 32px rgb(0 0 0 / 35%);
  font-size: var(--font-size-lg);
}
.pick button.on {
  outline: 3px solid var(--color-accent);
}
.close {
  position: absolute;
  top: 0;
  right: 0;
  min-width: var(--min-touch);
  min-height: var(--min-touch);
}
.conn.open {
  color: var(--color-success);
}
.conn.stalled,
.conn.reconnecting,
.conn.connecting {
  color: var(--color-warning);
}
.conn.closed {
  color: var(--color-danger);
}
.stale {
  padding: 0 var(--space-3);
  border-radius: 0;
  background: var(--color-warning);
  color: var(--color-bg);
  font-size: var(--font-size-sm);
  font-weight: 600;
}

.try {
  min-height: var(--min-touch);
  padding: var(--space-1) var(--space-2);
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: inherit;
  font: inherit;
  cursor: pointer;
}
</style>
