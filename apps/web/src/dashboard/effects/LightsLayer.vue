<script setup lang="ts">
import { reducedMotion } from './confettiModel'

/** Christmas lights: a swagged string of bulbs along the top edge, each fading gently. */
const COLORS = ['#e5484d', '#3fb950', '#f5c542', '#4c9bf5', '#e07bd1']
const COUNT = 28
const bulbs = Array.from({ length: COUNT }, (_, i) => {
  // A sagging wire: bulbs hang lowest half-way between the hooks (every 7th bulb).
  const sag = Math.sin(((i % 7) / 7) * Math.PI)
  return {
    i,
    left: `${((i + 0.5) / COUNT) * 100}%`,
    top: `${4 + sag * 14}px`,
    color: COLORS[i % COLORS.length]!,
    delay: `${((i * 0.37) % 2.6).toFixed(2)}s`,
    dur: `${(2.4 + ((i * 7) % 5) * 0.35).toFixed(2)}s`,
  }
})
const still = reducedMotion()
</script>

<template>
  <div class="lights" :class="{ still }" aria-hidden="true" data-testid="lights-layer">
    <i
      v-for="b in bulbs"
      :key="b.i"
      class="bulb"
      :style="{
        left: b.left,
        top: b.top,
        '--c': b.color,
        animationDelay: b.delay,
        animationDuration: b.dur,
      }"
    />
  </div>
</template>

<style scoped>
.lights {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3rem;
  pointer-events: none;
  /* Above the menu bar so the top nav does not hide the string. */
  z-index: 2;
}
.bulb {
  position: absolute;
  width: 0.8rem;
  height: 1.1rem;
  margin-left: -0.4rem;
  border-radius: 50% 50% 55% 55%;
  background: var(--c);
  box-shadow: 0 0 0.9rem 0.25rem var(--c);
  opacity: 0.85;
  animation: twinkle 3s ease-in-out infinite;
}
.still .bulb {
  animation: none;
}
@keyframes twinkle {
  0%,
  100% {
    opacity: 0.95;
  }
  50% {
    opacity: 0.35;
  }
}
</style>
