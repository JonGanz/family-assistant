<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { reducedMotion } from './confettiModel'
import { RAIN_PARAMS, type RainMode } from './rainModel'

const props = defineProps<{ mode: RainMode }>()
const params = () => RAIN_PARAMS[props.mode === 'none' ? 'light' : props.mode]

/** One drop per this many square pixels, capped. */
const AREA_PER_DROP = 9_000
/** Rain leans a little: horizontal px per vertical px. */
const SLANT = 0.12
interface Drop {
  x: number
  y: number
  len: number
  vy: number
}
const cv = ref<HTMLCanvasElement | null>(null)
let drops: Drop[] = []
let raf = 0
let last = 0
let w = 0
let h = 0
let ro: ResizeObserver | undefined

const spawn = (anywhere: boolean): Drop => ({
  x: Math.random() * (w + h * SLANT),
  y: anywhere ? Math.random() * h : -20,
  len: 10 + Math.random() * 14,
  vy: 420 + Math.random() * 260,
})

function resize() {
  const c = cv.value
  const el = c?.parentElement
  if (!c || !el) return
  w = el.clientWidth
  h = el.clientHeight
  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  c.width = Math.max(1, Math.floor(w * dpr))
  c.height = Math.max(1, Math.floor(h * dpr))
  c.getContext('2d')?.setTransform(dpr, 0, 0, dpr, 0, 0)
  fit()
}

function fit() {
  const p = params()
  const want = Math.min(p.max, Math.round(((w * h) / AREA_PER_DROP) * p.density))
  while (drops.length < want) drops.push(spawn(true))
  drops.length = Math.min(drops.length, want)
}

function stop() {
  if (raf) cancelAnimationFrame(raf)
  raf = 0
}

function frame(t: number) {
  raf = 0
  const ctx = cv.value?.getContext('2d')
  if (!ctx || document.hidden) return
  const dt = Math.min(0.1, (t - last) / 1000 || 0.016)
  last = t
  ctx.clearRect(0, 0, w, h)
  ctx.strokeStyle = 'rgba(150,190,235,0.55)'
  ctx.lineWidth = 1.3
  ctx.lineCap = 'round'
  const p = params()
  ctx.beginPath()
  for (const d of drops) {
    d.y += d.vy * p.speed * dt
    if (d.y - d.len > h) Object.assign(d, spawn(false))
    const x = d.x - d.y * SLANT
    ctx.moveTo(x, d.y)
    ctx.lineTo(x - d.len * SLANT, d.y - d.len)
  }
  ctx.stroke()
  raf = requestAnimationFrame(frame)
}

function start() {
  if (!raf && !document.hidden) {
    last = performance.now()
    raf = requestAnimationFrame(frame)
  }
}
function onVisibility() {
  if (document.hidden) stop()
  else start()
}

onMounted(() => {
  const c = cv.value
  // jsdom and reduced-motion: no canvas, no animation.
  if (!c || reducedMotion() || !c.getContext('2d')) return
  resize()
  if (typeof ResizeObserver !== 'undefined' && c.parentElement) {
    ro = new ResizeObserver(resize)
    ro.observe(c.parentElement)
  }
  document.addEventListener('visibilitychange', onVisibility)
  start()
})
// A live change of intensity refits the drop count.
watch(
  () => props.mode,
  (m) => {
    if (m === 'none') {
      stop()
      drops = []
      cv.value?.getContext('2d')?.clearRect(0, 0, w, h)
    } else if (w && h) {
      fit()
      start()
    }
  },
)
onBeforeUnmount(() => {
  stop()
  ro?.disconnect()
  document.removeEventListener('visibilitychange', onVisibility)
})
</script>

<template>
  <canvas ref="cv" class="rain-layer" aria-hidden="true" tabindex="-1" data-testid="rain-layer" />
</template>

<style scoped>
.rain-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 0;
}
</style>
