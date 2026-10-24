<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from '../api/client'
import { ApiError } from '../api/http'
import PinPad from '../components/PinPad.vue'
import { getDeviceToken } from '../kiosk/deviceToken'
import { startKeepAlive } from '../kiosk/keepAlive'
import { startWatchdog, type Watchdog } from '../kiosk/watchdog'
import { useDashboardStore } from '../stores/dashboard'

const store = useDashboardStore()
const hasToken = ref(!!getDeviceToken())

const now = ref(new Date())
const clock = computed(() =>
  now.value.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
)
const date = computed(() =>
  now.value.toLocaleDateString([], {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }),
)

const connLabel = computed(
  () =>
    ({
      open: 'Connected',
      connecting: 'Connecting',
      reconnecting: 'Reconnecting',
      closed: 'Offline',
    })[store.connection],
)

let timer: ReturnType<typeof setInterval> | undefined
let stopKeepAlive: (() => void) | undefined
let watchdog: Watchdog | undefined

onMounted(() => {
  if (!hasToken.value) return
  timer = setInterval(() => (now.value = new Date()), 1000)
  stopKeepAlive = startKeepAlive()
  watchdog = startWatchdog()
  void store.start(() => watchdog?.heartbeat())
})
onUnmounted(() => {
  clearInterval(timer)
  clearInterval(remainingTimer)
  stopKeepAlive?.()
  watchdog?.stop()
  store.stop()
})

// Elevation demo (D-019): pick a member, enter the PIN, show the remaining time.
const showElevation = ref(false)
const pickedId = ref<string | null>(null)
const elevMsg = ref('')
const expiresAt = ref<number | null>(null)
const remaining = ref(0)
let remainingTimer: ReturnType<typeof setInterval> | undefined

const adults = computed(() => store.members.filter((m) => m.role === 'adult'))

async function submitPin(pin: string) {
  if (!pickedId.value) return
  elevMsg.value = ''
  try {
    const r = await api.elevate(pickedId.value, pin)
    expiresAt.value = Date.parse(r.expires_at)
    clearInterval(remainingTimer)
    const tick = () => {
      remaining.value = Math.max(0, Math.round(((expiresAt.value ?? 0) - Date.now()) / 1000))
      if (remaining.value === 0) clearInterval(remainingTimer)
    }
    tick()
    remainingTimer = setInterval(tick, 1000)
  } catch (e) {
    expiresAt.value = null
    if (e instanceof ApiError && e.status === 423) elevMsg.value = 'Locked. Try again later.'
    else if (e instanceof ApiError && e.status === 401) elevMsg.value = 'Wrong PIN.'
    else elevMsg.value = 'Elevation failed.'
  }
}

const mmss = computed(
  () => `${Math.floor(remaining.value / 60)}:${String(remaining.value % 60).padStart(2, '0')}`,
)
</script>

<template>
  <section v-if="!hasToken" class="pair">
    <h1>Pair this screen</h1>
    <p>Ask an adult to open Settings → Devices.</p>
  </section>

  <section v-else class="dashboard">
    <header>
      <div>
        <div class="household" data-testid="household">{{ store.householdName }}</div>
        <div class="date">{{ date }}</div>
      </div>
      <div class="clock" data-testid="clock">{{ clock }}</div>
    </header>

    <ul class="strip">
      <li v-for="m in store.members" :key="m.id">
        <span class="avatar" :style="{ background: m.color ?? 'var(--color-border)' }">
          {{ m.avatar || m.display_name.slice(0, 1) }}
        </span>
        <span>{{ m.display_name }}</span>
      </li>
    </ul>

    <div v-if="showElevation" class="elevation">
      <div v-if="expiresAt && remaining > 0" data-testid="elevated">Elevated: {{ mmss }} left</div>
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
    </div>

    <footer>
      <span
        class="conn"
        :class="store.connection"
        data-testid="connection"
        :data-status="store.connection"
      >
        ● {{ connLabel }}<template v-if="store.stale"> (showing last known state)</template>
      </span>
      <span v-if="store.lastHeartbeat" class="hb" data-testid="heartbeat">
        Last heartbeat {{ store.lastHeartbeat }}
      </span>
      <button class="try" @click="showElevation = !showElevation">Try elevation</button>
    </footer>
  </section>
</template>

<style scoped>
.pair {
  padding: 3rem;
}
.pair h1,
.clock {
  font-size: var(--kiosk-font-title);
  margin: 0;
}
.dashboard {
  padding: 2rem 3rem;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.household {
  font-size: var(--kiosk-font-title);
}
.date {
  font-size: var(--kiosk-font-base);
  color: var(--color-muted);
}
.strip {
  display: flex;
  gap: 2rem;
  list-style: none;
  padding: 0;
  margin: 0;
}
.strip li {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: var(--kiosk-font-base);
}
.avatar {
  width: 4rem;
  height: 4rem;
  border-radius: 50%;
  display: grid;
  place-items: center;
  color: #fff;
}
.elevation {
  font-size: var(--kiosk-font-base);
}
.pick button.on {
  outline: 3px solid var(--color-accent);
}
footer {
  margin-top: auto;
  display: flex;
  gap: 2rem;
  align-items: center;
  font-size: var(--kiosk-font-small);
}
.conn.open {
  color: #27ae60;
}
.conn.reconnecting,
.conn.connecting {
  color: #e67e22;
}
.conn.closed {
  color: #c0392b;
}
.try {
  margin-left: auto;
  cursor: pointer;
  font-size: var(--kiosk-font-small);
}
</style>
