<script setup lang="ts">
import { computed } from 'vue'
import { UiProgress } from '../../ui'
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
    <UiProgress
      class="bar"
      :value="p.percent"
      :label="label ?? 'Progress'"
      :aria-valuenow="done"
      :aria-valuemax="total"
      :aria-valuetext="`${done} of ${total}`"
    />
    <span class="count" aria-hidden="true">{{ done }}/{{ total }}</span>
  </div>
</template>

<style scoped>
.progress {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.bar {
  flex: 1;
}
.count {
  font: var(--ui-type-label-md);
  font-variant-numeric: tabular-nums;
  color: var(--ui-on-surface-variant);
}
.complete .count {
  color: var(--ui-success);
  font-weight: 600;
}
.kiosk .bar {
  height: 1.4rem;
}
.kiosk .count {
  font-size: var(--kiosk-font-small);
}
</style>
