<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { reducedMotion } from './confettiModel'
import { SNOW_PARAMS, type SnowMode } from './snowModel'

const props = defineProps<{ mode: SnowMode }>()
const params = () => SNOW_PARAMS[props.mode === 'none' ? 'light' : props.mode]

/** One flake per this many square pixels, capped. */
const AREA_PER_FLAKE = 14_000
interface Flake {
  x: number
  y: number
  r: number
  vy: number
  drift: number
  phase: number
}
const cv = ref<HTMLCanvasElement | null>(null)
let flakes: Flake[] = []
let raf = 0
let last = 0
let w = 0
let h = 0
let ro: ResizeObserver | undefined

const spawn = (anywhere: boolean): Flake => ({
  x: Math.random() * w,
  y: anywhere ? Math.random() * h : -10,
  r: 1.5 + Math.random() * 3,
  vy: 18 + Math.random() * 30,
  drift: 6 + Math.random() * 14,
  phase: Math.random() * Math.PI * 2,
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
  const want = Math.min(p.max, Math.round(((w * h) / AREA_PER_FLAKE) * p.density))
  while (flakes.length < want) flakes.push(spawn(true))
  flakes.length = Math.min(flakes.length, want)
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
  ctx.fillStyle = 'rgba(255,255,255,0.85)'
  ctx.shadowColor = 'rgba(0,0,0,0.25)'
  ctx.shadowBlur = 2
  const p = params()
  // Flurries gust: the wind swells and fades.
  const wind = p.gust * (0.6 + 0.4 * Math.sin(t / 1700))
  for (const f of flakes) {
    f.y += f.vy * p.speed * dt
    f.phase += dt * Math.min(p.speed, 1.5)
    f.x += wind * dt
    if (f.x > w + 10) f.x -= w + 20
    const x = f.x + Math.sin(f.phase) * f.drift * p.drift
    if (f.y > h + 10) Object.assign(f, spawn(false))
    ctx.beginPath()
    ctx.arc(x, f.y, f.r, 0, Math.PI * 2)
    ctx.fill()
  }
  raf = requestAnimationFrame(frame)
}

function start() {
  if (!raf && !document.hidden) {
    last = performance.now()
    raf = requestAnimationFrame(frame)
  }
}
function onVisibility() {
  if (document.hidden) {
    if (raf) cancelAnimationFrame(raf)
    raf = 0
  } else start()
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
// Live mode changes: refit the flake count; the layer is unmounted for 'none' by the canvas,
// but stop and clear here too.
watch(
  () => props.mode,
  (m) => {
    if (m === 'none') {
      stop()
      flakes = []
      cv.value?.getContext('2d')?.clearRect(0, 0, w, h)
    } else if (w && h) {
      fit()
      start()
    }
  },
)
onBeforeUnmount(() => {
  if (raf) cancelAnimationFrame(raf)
  ro?.disconnect()
  document.removeEventListener('visibilitychange', onVisibility)
})
</script>

<template>
  <canvas ref="cv" class="snow-layer" aria-hidden="true" tabindex="-1" data-testid="snow-layer" />
</template>

<style scoped>
.snow-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 0;
}
</style>
