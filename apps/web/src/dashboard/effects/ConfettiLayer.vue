<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { onConfetti, reducedMotion, scatterPlan, type ConfettiRequest } from './confettiModel'

const PIECES = 48
const LIFE_MS = 2800
interface Piece {
  id: number
  style: Record<string, string>
}
const layer = ref<HTMLElement | null>(null)
const pieces = ref<Piece[]>([])
let seq = 0
const timers = new Set<number>()

/** The normal burst from the card, plus staggered bursts at random places over the screen. */
function fire(req: ConfettiRequest) {
  burst(req)
  const host = layer.value?.getBoundingClientRect()
  if (!host || !req.scatter || reducedMotion()) return
  for (const s of scatterPlan(req.scatter, host.width, host.height)) {
    const t = window.setTimeout(() => {
      timers.delete(t)
      burst({
        rect: { left: host.left + s.x, top: host.top + s.y, width: 0, height: 0 },
        colors: req.colors,
      })
    }, s.delay)
    timers.add(t)
  }
}

function burst(req: ConfettiRequest) {
  const host = layer.value?.getBoundingClientRect()
  if (!host || reducedMotion()) return
  const cx = req.rect.left - host.left + req.rect.width / 2
  const cy = req.rect.top - host.top + req.rect.height / 2
  const made: Piece[] = []
  for (let i = 0; i < PIECES; i++) {
    const angle = Math.random() * Math.PI * 2
    const dist = 80 + Math.random() * Math.min(260, Math.max(req.rect.width, 200))
    made.push({
      id: ++seq,
      style: {
        left: `${cx}px`,
        top: `${cy}px`,
        background: req.colors[i % req.colors.length]!,
        '--dx': `${Math.cos(angle) * dist}px`,
        '--dy': `${Math.sin(angle) * dist - 60}px`,
        '--rot': `${Math.round(Math.random() * 720 - 360)}deg`,
        animationDelay: `${Math.round(Math.random() * 120)}ms`,
      },
    })
  }
  pieces.value.push(...made)
  const ids = new Set(made.map((p) => p.id))
  const t = window.setTimeout(() => {
    pieces.value = pieces.value.filter((p) => !ids.has(p.id))
    timers.delete(t)
  }, LIFE_MS)
  timers.add(t)
}

let off: (() => void) | undefined
onMounted(() => (off = onConfetti(fire)))
onBeforeUnmount(() => {
  off?.()
  timers.forEach((t) => clearTimeout(t))
})
</script>

<template>
  <div ref="layer" class="confetti-layer" aria-hidden="true" data-testid="confetti-layer">
    <span v-for="p in pieces" :key="p.id" class="piece" :style="p.style" />
  </div>
</template>

<style scoped>
.confetti-layer {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
  z-index: 5;
}
.piece {
  position: absolute;
  width: 0.7rem;
  height: 1.1rem;
  border-radius: 2px;
  pointer-events: none;
  opacity: 0;
  animation: confetti-fly 2.6s cubic-bezier(0.2, 0.7, 0.3, 1) forwards;
}
@keyframes confetti-fly {
  0% {
    opacity: 1;
    transform: translate(0, 0) rotate(0);
  }
  70% {
    opacity: 1;
  }
  100% {
    opacity: 0;
    transform: translate(var(--dx), calc(var(--dy) + 220px)) rotate(var(--rot));
  }
}
</style>
