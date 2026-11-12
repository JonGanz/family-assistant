<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { reducedMotion } from './confettiModel'

/** One flake per this many square pixels, capped. */
const AREA_PER_FLAKE = 14_000
const MAX_FLAKES = 140
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
  const want = Math.min(MAX_FLAKES, Math.round((w * h) / AREA_PER_FLAKE))
  while (flakes.length < want) flakes.push(spawn(true))
  flakes.length = Math.min(flakes.length, want)
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
  for (const f of flakes) {
    f.y += f.vy * dt
    f.phase += dt
    const x = f.x + Math.sin(f.phase) * f.drift
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
