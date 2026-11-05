<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{ kind?: 'linear' | 'circular'; value?: number; size?: number; label?: string }>(),
  { kind: 'linear', size: 40 },
)
const pct = computed(() =>
  props.value === undefined ? undefined : Math.min(100, Math.max(0, props.value)),
)
const circ = computed(() => 2 * Math.PI * 16)
</script>

<template>
  <div
    v-if="kind === 'linear'"
    class="ui-progress"
    :class="{ 'is-indeterminate': pct === undefined }"
    role="progressbar"
    :aria-label="label ?? 'Loading'"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-valuenow="pct"
  >
    <div class="ui-progress__bar" :style="pct === undefined ? undefined : { width: `${pct}%` }" />
  </div>
  <svg
    v-else
    class="ui-progress-c"
    :class="{ 'is-indeterminate': pct === undefined }"
    :width="size"
    :height="size"
    viewBox="0 0 40 40"
    role="progressbar"
    :aria-label="label ?? 'Loading'"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-valuenow="pct"
  >
    <circle class="track" cx="20" cy="20" r="16" fill="none" stroke-width="4" />
    <circle
      class="arc"
      cx="20"
      cy="20"
      r="16"
      fill="none"
      stroke-width="4"
      stroke-linecap="round"
      :stroke-dasharray="
        pct === undefined ? `${circ * 0.3} ${circ}` : `${(circ * pct) / 100} ${circ}`
      "
      transform="rotate(-90 20 20)"
    />
  </svg>
</template>

<style scoped>
.ui-progress {
  height: 4px;
  border-radius: var(--ui-radius-full);
  background: var(--ui-primary-container);
  overflow: hidden;
}
.ui-progress__bar {
  height: 100%;
  background: var(--ui-primary);
  border-radius: inherit;
  transition: width var(--ui-motion-medium) var(--ui-ease);
}
.ui-progress.is-indeterminate .ui-progress__bar {
  width: 40%;
  animation: ui-slide 1.4s var(--ui-ease) infinite;
}
.ui-progress-c .track {
  stroke: var(--ui-primary-container);
}
.ui-progress-c .arc {
  stroke: var(--ui-primary);
}
.ui-progress-c.is-indeterminate {
  animation: ui-spin 1.2s linear infinite;
}
@keyframes ui-slide {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(250%);
  }
}
@keyframes ui-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .ui-progress.is-indeterminate .ui-progress__bar,
  .ui-progress-c.is-indeterminate {
    animation-duration: 4s;
  }
}
</style>
