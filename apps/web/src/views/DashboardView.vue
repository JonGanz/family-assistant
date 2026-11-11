<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { api } from '../api/client'
import { ApiError } from '../api/http'
import PinPad from '../components/PinPad.vue'
import { useClock } from '../composables/useClock'
import DashboardCanvas from '../dashboard/DashboardCanvas.vue'
import Screensaver from '../dashboard/Screensaver.vue'
import { useTimers } from '../dashboard/timers/useTimers'
import TaskFlow from '../dashboard/TaskFlow.vue'
import { S } from '../dashboard/strings'
import { getDeviceToken } from '../kiosk/deviceToken'
import { startKeepAlive } from '../kiosk/keepAlive'
import { startWatchdog, type Watchdog } from '../kiosk/watchdog'
import { watchForUpdates } from '../lib/appUpdate'
import { useDashboardStore } from '../stores/dashboard'
import UiIcon from '../ui/UiIcon.vue'

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

// Fullscreen (for the Echo Show's browser bar): a button, since browsers only allow it on a tap.
// A ringing kitchen timer keeps the screensaver away so the card is visible.
const timers = useTimers()
const fullscreen = ref(false)
const fullscreenSupported =
  typeof document !== 'undefined' && !!document.documentElement.requestFullscreen
const syncFullscreen = () => (fullscreen.value = document.fullscreenElement !== null)
async function toggleFullscreen(): Promise<void> {
  try {
    if (document.fullscreenElement) await document.exitFullscreen()
    else await document.documentElement.requestFullscreen()
  } catch {
    /* refused (no gesture, or unsupported): stay as is */
  }
  syncFullscreen()
}

let stopUpdates: (() => void) | undefined
onMounted(() => {
  // The page itself must not scroll: a drag that scrolls it brings the browser bar back.
  document.documentElement.classList.add('kiosk-lock')
  document.addEventListener('fullscreenchange', syncFullscreen)
  document.addEventListener('pointerdown', onDocDown)
  document.addEventListener('keydown', onKey)
  window.addEventListener('resize', place)
  if (!hasToken.value) return
  stopKeepAlive = startKeepAlive()
  watchdog = startWatchdog()
  // A deploy (D-123): reload as soon as nobody is mid-action (no dialog open, not unlocked with a PIN).
  stopUpdates = watchForUpdates({
    eager: true,
    canReload: () => {
      const until = store.elevatedUntil
      const elevated = until !== null && until > Date.now()
      return !elevated && !document.querySelector('[role="dialog"], dialog[open]')
    },
  }).stop
  void store.start(() => watchdog?.heartbeat())
})
onUnmounted(() => {
  document.documentElement.classList.remove('kiosk-lock')
  document.removeEventListener('fullscreenchange', syncFullscreen)
  document.removeEventListener('pointerdown', onDocDown)
  document.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', place)
  clearInterval(remainingTimer)
  stopKeepAlive?.()
  watchdog?.stop()
  stopUpdates?.()
  store.stop()
})

// Elevation (D-019): pick an adult, enter the PIN, show the remaining time.
const showElevation = ref(false)
const pickedId = ref<string | null>(null)
const elevMsg = ref('')
const expiresAt = ref<number | null>(null)
const remaining = ref(0)
let remainingTimer: ReturnType<typeof setInterval> | undefined

const elevated = computed(() => !!store.snapshot?.elevated)
const lockBtn = ref<HTMLElement | null>(null)
const popover = ref<HTMLElement | null>(null)
const popStyle = ref<Record<string, string>>({})

/** Anchor the popover under the lock button, right-aligned and kept inside the viewport. */
function place(): void {
  const b = lockBtn.value?.getBoundingClientRect()
  if (!b) return
  const gap = 8
  const w = popover.value?.offsetWidth ?? 0
  const h = popover.value?.offsetHeight ?? 0
  const vw = window.innerWidth
  const vh = window.innerHeight
  let left = Math.min(Math.max(gap, b.right - w), Math.max(gap, vw - w - gap))
  let top = b.bottom + gap
  if (top + h > vh - gap) top = Math.max(gap, b.top - gap - h)
  popStyle.value = { left: `${left}px`, top: `${top}px`, maxHeight: `${vh - 2 * gap}px` }
}
function openPopover(): void {
  showElevation.value = true
  popStyle.value = { right: '8px', top: '8px' }
  void nextTick(place)
}
function stopElevation(): void {
  clearInterval(remainingTimer)
  expiresAt.value = null
  remaining.value = 0
  store.elevatedUntil = null
  showElevation.value = false
  pickedId.value = null
}
function onLock(): void {
  if (elevated.value) stopElevation()
  else if (showElevation.value) showElevation.value = false
  else openPopover()
}
function onDocDown(e: Event): void {
  if (!showElevation.value) return
  const t = e.target as Node
  if (popover.value?.contains(t) || lockBtn.value?.contains(t)) return
  showElevation.value = false
}
function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape' && showElevation.value) showElevation.value = false
}

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
      :food-slots="store.mealSlots"
      @week="(from: string | null) => store.showWeek(from)"
      @pick="pick"
      @complete="(id: string) => flow?.beginComplete(id)"
      @tick="(r: string, st: string, d: boolean) => store.tickStep(r, st, d)"
      @add="flow?.beginAdd()"
    >
      <template #nav-end>
        <!-- Far right (portrait) reads: lock, fullscreen, connection. -->
        <span
          v-if="link !== 'open'"
          class="conn"
          :class="link"
          data-testid="connection"
          :data-status="link"
        >
          ● {{ connLabel }}
        </span>
        <span v-if="store.showStaleBadge && updatedAt" class="stale" data-testid="stale">
          {{ S.lastUpdated(updatedAt) }}
        </span>
        <button
          v-if="fullscreenSupported"
          type="button"
          class="navbtn"
          data-testid="fullscreen"
          :aria-pressed="fullscreen"
          :aria-label="fullscreen ? S.exitFullscreen : S.fullscreen"
          :title="fullscreen ? S.exitFullscreen : S.fullscreen"
          @click="toggleFullscreen"
        >
          <UiIcon :name="fullscreen ? 'fullscreen_exit' : 'fullscreen'" :size="24" />
        </button>
        <button
          ref="lockBtn"
          type="button"
          class="navbtn"
          data-testid="unlock"
          :aria-pressed="elevated"
          :aria-expanded="showElevation"
          :aria-label="elevated ? S.lock : S.elevate"
          :title="elevated ? S.lock : S.elevate"
          @click="onLock"
        >
          <UiIcon :name="elevated ? 'lock_open' : 'lock'" :size="24" />
        </button>
      </template>
    </DashboardCanvas>
    <div v-else class="loading">
      <p>{{ S.loading }}</p>
      <p v-if="store.loadError" role="alert" data-testid="load-error">
        {{ S.loadFailed(store.loadError) }}
      </p>
    </div>
    <TaskFlow ref="flow" />
    <Screensaver
      :minutes="store.snapshot?.settings.screensaver_minutes ?? 15"
      :tz="store.snapshot?.household.tz"
      :hold="timers.ringing.value"
    />

    <div
      v-if="showElevation"
      ref="popover"
      class="elevation popover"
      role="dialog"
      aria-label="Elevation"
      data-testid="elevation"
      :style="popStyle"
    >
      <button
        type="button"
        class="close"
        data-testid="elevation-close"
        :aria-label="S.close"
        @click="showElevation = false"
      >
        <UiIcon name="close" :size="24" />
      </button>
      <div v-if="expiresAt && remaining > 0" data-testid="elevated">{{ S.elevated(mmss) }}</div>
      <template v-else>
        <div class="pick">
          <button
            v-for="m in adults"
            :key="m.id"
            type="button"
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
  position: fixed;
  z-index: 20;
  max-width: calc(100vw - 16px);
  overflow-y: auto;
  padding: var(--space-4);
  border-radius: var(--ui-radius-xl);
  background: var(--color-layer-2);
  color: var(--color-text);
  box-shadow: 0 8px 32px rgb(0 0 0 / 35%);
  font-size: var(--kiosk-font-base);
}
.elevation .pick {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0 var(--min-touch) var(--space-3) 0;
}
.pick button {
  min-height: var(--min-touch);
  padding: 0 var(--space-4);
  border: 0;
  border-radius: var(--ui-radius-full);
  background: var(--color-layer-3);
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.pick button.on {
  outline: 3px solid var(--color-accent);
}
.close {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
  display: grid;
  place-items: center;
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  padding: 0;
  border: 0;
  border-radius: var(--ui-radius-full);
  background: var(--color-layer-3);
  color: inherit;
  cursor: pointer;
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

/* Same look as the Home/Week/Food nav buttons. */
.navbtn {
  display: grid;
  place-items: center;
  width: 6rem;
  min-height: 5rem;
  padding-inline: var(--space-2);
  border: 0;
  border-radius: var(--ui-radius-xl);
  background: var(--color-layer-2);
  color: inherit;
  cursor: pointer;
}
.navbtn :deep(svg) {
  width: 2.4rem;
  height: 2.4rem;
}
</style>
