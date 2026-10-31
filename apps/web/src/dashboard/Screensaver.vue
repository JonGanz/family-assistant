<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { S } from './strings'

const props = defineProps<{
  /** Minutes without interaction before it starts; 0 turns it off. */
  minutes: number
}>()

const ICON = 96
const SPEED = 110 // px per second, on both axes

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
  active.value = true
  const w = window.innerWidth
  const h = window.innerHeight
  pos.value = { x: Math.random() * Math.max(1, w - ICON), y: Math.random() * Math.max(1, h - ICON) }
  vx = Math.random() < 0.5 ? SPEED : -SPEED
  vy = Math.random() < 0.5 ? SPEED : -SPEED
  last = 0
  frame = requestAnimationFrame(step)
}

function step(t: number) {
  const dt = last ? Math.min(0.1, (t - last) / 1000) : 0
  last = t
  const maxX = Math.max(0, window.innerWidth - ICON)
  const maxY = Math.max(0, window.innerHeight - ICON)
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
    <img
      class="icon"
      src="/icon-192.png"
      alt=""
      :width="ICON"
      :height="ICON"
      :style="{ transform: `translate(${pos.x}px, ${pos.y}px)` }"
    />
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
.icon {
  position: absolute;
  top: 0;
  left: 0;
  will-change: transform;
}
</style>
