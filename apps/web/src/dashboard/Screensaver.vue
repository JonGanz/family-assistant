<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useClock } from '../composables/useClock'
import { formatTime } from '../lib/time'
import { S } from './strings'

const props = defineProps<{
  /** Minutes without interaction before it starts; 0 turns it off. */
  minutes: number
  /** Household time zone for the clock; the browser's when unknown. */
  tz?: string
  /** Keeps the screen awake (a ringing timer): wakes it at once and holds it off while true. */
  hold?: boolean
}>()

const SPEED = 110 // px per second, on both axes

// The bouncing clock. Its box is measured each frame, so a wider time (10:00 vs 9:59) still bounces
// off the edges exactly.
const now = useClock(1000)
const time = computed(() =>
  formatTime(now.value, props.tz || Intl.DateTimeFormat().resolvedOptions().timeZone),
)
const clockEl = ref<HTMLElement | null>(null)
const size = () => ({
  w: clockEl.value?.offsetWidth || 240,
  h: clockEl.value?.offsetHeight || 96,
})

const active = ref(false)
const pos = ref({ x: 0, y: 0 })
let timer: ReturnType<typeof setTimeout> | undefined
let frame = 0
let last = 0
let vx = SPEED
let vy = SPEED

function arm() {
  clearTimeout(timer)
  if (props.minutes > 0 && !active.value) {
    timer = setTimeout(start, props.minutes * 60_000)
  }
}

/** Any touch, click or key press counts as interaction; while the screensaver shows it only wakes on a tap. */
function onInteract() {
  if (!active.value) arm()
}

function start() {
  if (props.hold) return arm()
  active.value = true
  const w = window.innerWidth
  const h = window.innerHeight
  const { w: cw, h: ch } = size()
  pos.value = { x: Math.random() * Math.max(1, w - cw), y: Math.random() * Math.max(1, h - ch) }
  vx = Math.random() < 0.5 ? SPEED : -SPEED
  vy = Math.random() < 0.5 ? SPEED : -SPEED
  last = 0
  frame = requestAnimationFrame(step)
}

function step(t: number) {
  const dt = last ? Math.min(0.1, (t - last) / 1000) : 0
  last = t
  const { w: cw, h: ch } = size()
  const maxX = Math.max(0, window.innerWidth - cw)
  const maxY = Math.max(0, window.innerHeight - ch)
  let { x, y } = pos.value
  x += vx * dt
  y += vy * dt
  if (x <= 0) {
    x = 0
    vx = Math.abs(vx)
  } else if (x >= maxX) {
    x = maxX
    vx = -Math.abs(vx)
  }
  if (y <= 0) {
    y = 0
    vy = Math.abs(vy)
  } else if (y >= maxY) {
    y = maxY
    vy = -Math.abs(vy)
  }
  pos.value = { x, y }
  frame = requestAnimationFrame(step)
}

/** Dismiss on `click`, so the tap that wakes the screen does not also press a button beneath. */
function wake() {
  active.value = false
  cancelAnimationFrame(frame)
  arm()
}

const EVENTS = ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart'] as const
onMounted(() => {
  for (const e of EVENTS) window.addEventListener(e, onInteract, { passive: true })
  arm()
})
onBeforeUnmount(() => {
  for (const e of EVENTS) window.removeEventListener(e, onInteract)
  clearTimeout(timer)
  cancelAnimationFrame(frame)
})
watch(
  () => props.minutes,
  () => arm(),
)
watch(
  () => props.hold,
  (h) => {
    if (h && active.value) wake()
  },
)
</script>

<template>
  <div
    v-if="active"
    class="screensaver"
    role="button"
    tabindex="0"
    :aria-label="S.screensaverWake"
    data-testid="screensaver"
    @click="wake"
    @keydown.enter="wake"
  >
    <div
      ref="clockEl"
      class="clock"
      data-testid="screensaver-clock"
      :style="{ transform: `translate(${pos.x}px, ${pos.y}px)` }"
    >
      {{ time }}
    </div>
  </div>
</template>

<style scoped>
.screensaver {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: #000;
  overflow: hidden;
  cursor: none;
}
.clock {
  position: absolute;
  top: 0;
  left: 0;
  will-change: transform;
  white-space: nowrap;
  /* Dim grey on black: readable across the room without lighting it up at night. */
  color: #9a9a9a;
  font-size: clamp(3rem, 10vmin, 7rem);
  font-weight: 300;
  font-variant-numeric: tabular-nums;
  line-height: 1;
  letter-spacing: 0.02em;
}
</style>
