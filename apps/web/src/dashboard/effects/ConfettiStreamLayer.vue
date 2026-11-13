<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import {
  onStreamsChange,
  reducedMotion,
  STREAM_INTERVAL_MS,
  STREAM_LIFE_S,
  STREAM_MAX_PIECES,
  streamPiece,
  streamSources,
} from './confettiModel'

/**
 * A gentle stream of confetti bubbling up from behind each completed routine card. It sits at
 * z-index 0 like the snow, so the cards (z-index 1) cover its source. One timer runs only while
 * some card is streaming; with none, nothing runs at all.
 */
interface Piece {
  id: number
  style: Record<string, string>
}
const layer = ref<HTMLElement | null>(null)
const pieces = ref<Piece[]>([])
let seq = 0
let timer = 0
const timers = new Set<number>()

function spawn() {
  const host = layer.value?.getBoundingClientRect()
  if (!host) return
  for (const src of streamSources()) {
    if (pieces.value.length >= STREAM_MAX_PIECES) return
    const rect = src.rect()
    if (!rect || rect.width <= 0) continue
    const p = streamPiece(rect)
    const id = ++seq
    pieces.value.push({
      id,
      style: {
        left: `${p.x - host.left}px`,
        top: `${p.y - host.top}px`,
        background: src.colors[id % src.colors.length]!,
        '--rise': `${p.rise}px`,
        '--sway': `${p.sway}px`,
        animationDuration: `${STREAM_LIFE_S + Math.random() * 2}s`,
      },
    })
    const t = window.setTimeout(
      () => {
        pieces.value = pieces.value.filter((x) => x.id !== id)
        timers.delete(t)
      },
      (STREAM_LIFE_S + 2) * 1000 + 100,
    )
    timers.add(t)
  }
}

function sync() {
  const on = streamSources().length > 0 && !reducedMotion()
  if (on && !timer) {
    spawn()
    timer = window.setInterval(spawn, STREAM_INTERVAL_MS)
  } else if (!on && timer) {
    clearInterval(timer)
    timer = 0
  }
}

let off: (() => void) | undefined
onMounted(() => {
  off = onStreamsChange(sync)
  sync()
})
onBeforeUnmount(() => {
  off?.()
  if (timer) clearInterval(timer)
  timers.forEach((t) => clearTimeout(t))
})
</script>

<template>
  <div ref="layer" class="stream-layer" aria-hidden="true" data-testid="confetti-stream-layer">
    <span v-for="p in pieces" :key="p.id" class="bit" :style="p.style" />
  </div>
</template>

<style scoped>
.stream-layer {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
  z-index: 0;
}
.bit {
  position: absolute;
  width: 0.55rem;
  height: 0.8rem;
  border-radius: 2px;
  pointer-events: none;
  opacity: 0;
  animation: confetti-rise 7s ease-out forwards;
}
@keyframes confetti-rise {
  0% {
    opacity: 0;
    transform: translate(0, 0) rotate(0);
  }
  15% {
    opacity: 0.9;
  }
  70% {
    opacity: 0.8;
  }
  100% {
    opacity: 0;
    transform: translate(var(--sway), calc(var(--rise) * -1)) rotate(240deg);
  }
}
</style>
