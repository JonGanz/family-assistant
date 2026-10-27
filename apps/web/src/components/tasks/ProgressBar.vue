<script setup lang="ts">
import { computed } from 'vue'
import { progress } from '../../lib/tasks'

const props = defineProps<{
  done: number
  total: number
  label?: string
  size?: 'normal' | 'kiosk'
}>()
const p = computed(() => progress(props.done, props.total))
</script>

<template>
  <div class="progress" :class="[size ?? 'normal', { complete: p.complete }]">
    <div
      class="track"
      role="progressbar"
      :aria-label="label ?? 'Progress'"
      aria-valuemin="0"
      :aria-valuemax="total"
      :aria-valuenow="done"
      :aria-valuetext="`${done} of ${total}`"
    >
      <div class="fill" :style="{ width: `${p.percent}%` }"></div>
    </div>
    <span class="count" aria-hidden="true">{{ done }}/{{ total }}</span>
  </div>
</template>

<style scoped>
.progress {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.track {
  flex: 1;
  height: 0.6rem;
  background: var(--color-border);
  border-radius: 999px;
  overflow: hidden;
}
.fill {
  height: 100%;
  min-width: 0;
  background: var(--color-accent);
  border-radius: 999px;
  transition:
    width 0.4s ease,
    background-color 0.3s ease;
}
.complete .fill {
  background: var(--color-success);
}
.count {
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
  color: var(--color-muted);
}
.complete .count {
  color: var(--color-success);
  font-weight: 600;
}
.kiosk .track {
  height: 1.4rem;
}
.kiosk .count {
  font-size: var(--kiosk-font-small);
}
@media (prefers-reduced-motion: reduce) {
  .fill {
    transition: none;
  }
}
</style>
